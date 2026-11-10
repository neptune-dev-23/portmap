use std::collections::VecDeque;
use std::time::{Duration, Instant};

pub const SAMPLE_INTERVAL: Duration = Duration::from_secs(15);
const WINDOW: Duration = Duration::from_secs(60 * 60);
// One point before the window boundary, one at the end, and 240 intervals.
const MAX_SAMPLES: usize = 242;

struct Sample {
    at: Instant,
    cpu_ns: u64,
    energy_nj: Option<u64>,
}

#[derive(Default)]
pub struct CpuHistory {
    started: Option<u64>,
    samples: VecDeque<Sample>,
}

#[derive(Default)]
pub struct CpuMetrics {
    pub current: Option<f64>,
    pub average: Option<f64>,
    pub seconds: f64,
    pub power_watts: Option<f64>,
}

impl CpuHistory {
    pub fn record(
        &mut self,
        started: u64,
        cpu_ns: u64,
        energy_nj: Option<u64>,
        at: Instant,
    ) -> CpuMetrics {
        // PIDs are reusable. Never combine counters from different processes,
        // counter resets, or a gap longer than the entire history window.
        if self.started != Some(started)
            || self
                .samples
                .back()
                .is_some_and(|last| cpu_ns < last.cpu_ns || at.duration_since(last.at) > WINDOW)
        {
            self.samples.clear();
        }
        self.started = Some(started);
        let current = self.samples.back().and_then(|last| {
            let elapsed = at.duration_since(last.at).as_secs_f64();
            (elapsed > 0.0).then(|| percent(cpu_ns - last.cpu_ns, elapsed))
        });
        let power_watts = self.samples.back().and_then(|last| {
            let elapsed = at.duration_since(last.at).as_secs_f64();
            let delta = energy_nj?.checked_sub(last.energy_nj?)?;
            (elapsed > 0.0).then(|| delta as f64 / 1_000_000_000.0 / elapsed)
        });
        self.samples.push_back(Sample {
            at,
            cpu_ns,
            energy_nj,
        });
        let cutoff = at.checked_sub(WINDOW).unwrap_or(at);
        while self.samples.len() > 2 && self.samples[1].at <= cutoff {
            self.samples.pop_front();
        }
        while self.samples.len() > MAX_SAMPLES {
            self.samples.pop_front();
        }

        let first = &self.samples[0];
        let mut start = first.at;
        let mut initial_cpu = first.cpu_ns as f64;
        if first.at < cutoff && self.samples.len() > 1 {
            let next = &self.samples[1];
            let fraction = cutoff.duration_since(first.at).as_secs_f64()
                / next.at.duration_since(first.at).as_secs_f64();
            initial_cpu += (next.cpu_ns - first.cpu_ns) as f64 * fraction;
            start = cutoff;
        }
        let seconds = at.duration_since(start).as_secs_f64();
        CpuMetrics {
            current,
            power_watts,
            average: (seconds > 0.0)
                .then(|| (cpu_ns as f64 - initial_cpu) / 1_000_000_000.0 / seconds * 100.0),
            seconds,
        }
    }
}

fn percent(cpu_ns: u64, seconds: f64) -> f64 {
    cpu_ns as f64 / 1_000_000_000.0 / seconds * 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warms_up_and_weights_by_elapsed_time() {
        let at = Instant::now();
        let mut history = CpuHistory::default();
        let first = history.record(1, 100_000_000_000, None, at);
        assert!(first.average.is_none());
        assert!(first.current.is_none());
        history.record(1, 130_000_000_000, None, at + Duration::from_secs(15));
        let result = history.record(1, 130_000_000_000, None, at + Duration::from_secs(60));
        assert_eq!(result.current, Some(0.0));
        assert_eq!(result.average, Some(50.0));
        assert_eq!(result.seconds, 60.0);
    }

    #[test]
    fn rolls_off_old_activity_at_the_hour_boundary() {
        let at = Instant::now();
        let mut history = CpuHistory::default();
        history.record(1, 0, None, at);
        history.record(1, 30_000_000_000, None, at + Duration::from_secs(30));
        // Half the initial busy interval is outside the last hour.
        let result = history.record(1, 30_000_000_000, None, at + Duration::from_secs(3615));
        assert_eq!(result.seconds, 3600.0);
        assert!((result.average.unwrap() - 15.0 / 3600.0 * 100.0).abs() < 0.0001);
    }

    #[test]
    fn resets_for_pid_reuse_counter_reset_and_long_gaps() {
        let at = Instant::now();
        let mut history = CpuHistory::default();
        history.record(1, 100, None, at);
        assert!(history
            .record(2, 200, None, at + SAMPLE_INTERVAL)
            .average
            .is_none());
        assert!(history
            .record(2, 0, None, at + SAMPLE_INTERVAL * 2)
            .average
            .is_none());
        assert!(history
            .record(2, 100, None, at + WINDOW * 2)
            .average
            .is_none());
    }

    #[test]
    fn storage_stays_bounded() {
        let at = Instant::now();
        let mut history = CpuHistory::default();
        for i in 0..1000 {
            history.record(1, i as u64, None, at + SAMPLE_INTERVAL * i);
        }
        assert!(history.samples.len() <= MAX_SAMPLES);
    }

    #[test]
    fn power_uses_energy_delta_and_handles_unavailable_counters() {
        let at = Instant::now();
        let mut history = CpuHistory::default();
        assert!(history.record(1, 0, Some(1), at).power_watts.is_none());
        let sample = history.record(1, 0, Some(30_000_000_001), at + SAMPLE_INTERVAL);
        assert_eq!(sample.power_watts, Some(2.0));
        assert!(history
            .record(1, 0, None, at + SAMPLE_INTERVAL * 2)
            .power_watts
            .is_none());
        assert!(history
            .record(1, 0, Some(40_000_000_000), at + SAMPLE_INTERVAL * 3)
            .power_watts
            .is_none());
        assert!(history
            .record(1, 0, Some(1), at + SAMPLE_INTERVAL * 4)
            .power_watts
            .is_none());
    }
}

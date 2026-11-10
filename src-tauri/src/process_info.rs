//! Small, unprivileged libproc queries; no process or thread enumeration.

#[derive(Default)]
pub struct ProcessInfo {
    pub executable_path: Option<String>,
    pub cwd: Option<String>,
    pub usage: Option<Usage>,
}

pub struct Usage {
    pub started: u64,
    pub cpu_ns: u64,
    pub memory_bytes: u64,
    pub energy_nj: Option<u64>,
}

// libc currently exposes V4 only. V6 is the V4 prefix followed by these
// fields from the public macOS sys/resource.h. Older kernels reject flavor 6;
// read_usage then falls back to V0 for CPU and memory.
#[cfg(target_os = "macos")]
#[repr(C)]
struct RusageInfoV6 {
    v4: libc::rusage_info_v4,
    flags: u64,
    user_ptime: u64,
    system_ptime: u64,
    pinstructions: u64,
    pcycles: u64,
    energy_nj: u64,
    penergy_nj: u64,
    secure_time_in_system: u64,
    secure_ptime_in_system: u64,
    neural_footprint: u64,
    lifetime_max_neural_footprint: u64,
    interval_max_neural_footprint: u64,
    conclave_footprint: u64,
    page_wait_time_mach: u64,
    page_cache_hits: u64,
    reserved: [u64; 6],
}

#[cfg(target_os = "macos")]
// libc deprecated these bindings in favor of mach2; the system ABI is stable.
// Keep this one tiny query here rather than adding another runtime dependency.
#[allow(deprecated)]
fn ticks_to_ns(ticks: u64) -> u64 {
    use std::sync::OnceLock;
    static TIMEBASE: OnceLock<(u32, u32)> = OnceLock::new();
    let &(numer, denom) = TIMEBASE.get_or_init(|| {
        let mut info = libc::mach_timebase_info { numer: 0, denom: 0 };
        // SAFETY: points to a properly sized, initialized timebase buffer.
        let status = unsafe { libc::mach_timebase_info(&mut info) };
        assert!(status == 0 && info.denom != 0, "Mach timebase unavailable");
        (info.numer, info.denom)
    });
    ((ticks as u128 * numer as u128) / denom as u128).min(u64::MAX as u128) as u64
}

#[cfg(target_os = "macos")]
fn read_usage(pid: i32) -> Option<Usage> {
    // SAFETY: these C structs consist entirely of integers and arrays; zero is
    // valid, and libproc writes at most the size of the requested version.
    unsafe {
        let mut v6: RusageInfoV6 = std::mem::zeroed();
        if libc::proc_pid_rusage(pid, 6, (&mut v6 as *mut RusageInfoV6).cast()) == 0 {
            return Some(Usage {
                started: v6.v4.ri_proc_start_abstime,
                cpu_ns: ticks_to_ns(v6.v4.ri_user_time.saturating_add(v6.v4.ri_system_time)),
                memory_bytes: v6.v4.ri_phys_footprint,
                // An all-zero lifetime counter can mean unsupported hardware.
                energy_nj: (cfg!(target_arch = "aarch64") && v6.energy_nj > 0)
                    .then_some(v6.energy_nj),
            });
        }
        let mut v0: libc::rusage_info_v0 = std::mem::zeroed();
        (libc::proc_pid_rusage(
            pid,
            libc::RUSAGE_INFO_V0,
            (&mut v0 as *mut libc::rusage_info_v0).cast(),
        ) == 0)
            .then(|| Usage {
                started: v0.ri_proc_start_abstime,
                cpu_ns: ticks_to_ns(v0.ri_user_time.saturating_add(v0.ri_system_time)),
                memory_bytes: v0.ri_phys_footprint,
                energy_nj: None,
            })
    }
}

#[cfg(target_os = "macos")]
pub fn inspect(pid: u32) -> ProcessInfo {
    use std::mem::{size_of, zeroed};

    let Ok(pid) = i32::try_from(pid) else {
        return ProcessInfo::default();
    };
    let mut path = [0u8; libc::PROC_PIDPATHINFO_MAXSIZE as usize];
    // SAFETY: all buffers have the sizes and layouts required by libproc. These
    // calls copy data into our buffers and do not retain any pointers.
    let executable_path = unsafe {
        (libc::proc_pidpath(pid, path.as_mut_ptr().cast(), path.len() as u32) > 0)
            .then(|| nul_string(&path))
            .flatten()
    };
    let mut vnode: libc::proc_vnodepathinfo = unsafe { zeroed() };
    let cwd = unsafe {
        let size = size_of::<libc::proc_vnodepathinfo>() as i32;
        if libc::proc_pidinfo(
            pid,
            libc::PROC_PIDVNODEPATHINFO,
            0,
            (&mut vnode as *mut libc::proc_vnodepathinfo).cast(),
            size,
        ) == size
        {
            // libc represents this contiguous MAXPATHLEN array as 32 x 32.
            let bytes = std::slice::from_raw_parts(
                vnode.pvi_cdir.vip_path.as_ptr().cast::<u8>(),
                size_of_val(&vnode.pvi_cdir.vip_path),
            );
            nul_string(bytes)
        } else {
            None
        }
    };
    ProcessInfo {
        executable_path,
        cwd,
        usage: read_usage(pid),
    }
}

#[cfg(target_os = "macos")]
fn nul_string(bytes: &[u8]) -> Option<String> {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    (end > 0).then(|| String::from_utf8_lossy(&bytes[..end]).into_owned())
}

#[cfg(not(target_os = "macos"))]
pub fn inspect(_pid: u32) -> ProcessInfo {
    ProcessInfo::default()
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    #[test]
    fn queries_current_process_without_privileges() {
        let info = inspect(std::process::id());
        assert_eq!(
            info.executable_path
                .as_ref()
                .and_then(|p| std::fs::canonicalize(p).ok()),
            std::env::current_exe()
                .ok()
                .and_then(|p| p.canonicalize().ok())
        );
        assert_eq!(
            info.cwd
                .as_ref()
                .and_then(|p| std::fs::canonicalize(p).ok()),
            std::env::current_dir()
                .ok()
                .and_then(|p| p.canonicalize().ok())
        );
        assert!(info.usage.unwrap().memory_bytes > 0);
    }

    #[test]
    fn inaccessible_process_is_unavailable() {
        let info = inspect(u32::MAX);
        assert!(info.executable_path.is_none());
        assert!(info.cwd.is_none());
        assert!(info.usage.is_none());
    }

    #[test]
    fn cpu_units_match_getrusage_under_load() {
        fn cpu_ns() -> u64 {
            let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
            assert_eq!(unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut usage) }, 0);
            let micros = (usage.ru_utime.tv_sec + usage.ru_stime.tv_sec) * 1_000_000
                + i64::from(usage.ru_utime.tv_usec)
                + i64::from(usage.ru_stime.tv_usec);
            micros as u64 * 1000
        }
        let before = cpu_ns();
        let initial = read_usage(std::process::id() as i32).unwrap();
        let start = std::time::Instant::now();
        while start.elapsed() < std::time::Duration::from_millis(100) {
            std::hint::spin_loop();
        }
        let final_usage = read_usage(std::process::id() as i32).unwrap();
        let expected = cpu_ns() - before;
        let actual = final_usage.cpu_ns - initial.cpu_ns;
        assert!(expected > 0);
        assert!((actual as f64 / expected as f64 - 1.0).abs() < 0.15);
    }
}

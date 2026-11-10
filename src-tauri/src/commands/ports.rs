use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use crate::models::PortEntry;
use crate::process_info;
use crate::resource_history::{CpuHistory, CpuMetrics, SAMPLE_INTERVAL};

use super::project::infer_project_name;

struct Listener {
    port: u16,
    pid: u32,
    name: String,
}

struct CachedProcess {
    checked: Instant,
    executable_path: Option<String>,
    cwd: Option<String>,
    project_name: Option<String>,
    history: CpuHistory,
    cpu: CpuMetrics,
    memory_bytes: Option<u64>,
}

impl CachedProcess {
    fn sample(pid: u32, mut history: CpuHistory) -> Self {
        let info = process_info::inspect(pid);
        let checked = Instant::now();
        let (cpu, memory_bytes) = match info.usage {
            Some(usage) => (
                history.record(usage.started, usage.cpu_ns, usage.energy_nj, checked),
                Some(usage.memory_bytes),
            ),
            None => {
                // Do not retain stale readings after exit or permission loss.
                history = CpuHistory::default();
                (CpuMetrics::default(), None)
            }
        };
        let project_name = info.cwd.as_deref().and_then(infer_project_name);
        Self {
            checked,
            executable_path: info.executable_path,
            cwd: info.cwd,
            project_name,
            history,
            cpu,
            memory_bytes,
        }
    }
}

#[tauri::command]
pub async fn list_ports(min_port: u16, max_port: u16) -> Result<Vec<PortEntry>, String> {
    // Process launch and filesystem reads must not block the async runtime.
    tauri::async_runtime::spawn_blocking(move || scan_ports(min_port, max_port))
        .await
        .map_err(|e| e.to_string())?
}

fn scan_ports(min_port: u16, max_port: u16) -> Result<Vec<PortEntry>, String> {
    static CACHE: OnceLock<Mutex<HashMap<u32, CachedProcess>>> = OnceLock::new();
    let mut cache = CACHE
        .get_or_init(Mutex::default)
        .lock()
        .map_err(|e| e.to_string())?;
    // Exactly one subprocess per port scan. Machine fields preserve spaces and
    // full command names; NUL terminators avoid display-column parsing.
    let output = Command::new("lsof")
        .args(["-iTCP", "-sTCP:LISTEN", "-n", "-P", "+c", "0", "-F0pcn"])
        .output()
        .map_err(|e| e.to_string())?;
    // lsof exits 1 with no diagnostics when there are no matching sockets.
    if !output.status.success() && !output.stderr.is_empty() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    let listeners = parse_listeners(&output.stdout, min_port, max_port);
    let active: HashSet<u32> = listeners.iter().map(|l| l.pid).collect();
    // Bound memory to processes currently owning an in-range listener. Multiple
    // ports owned by one PID share both queries and the same CPU history.
    cache.retain(|pid, _| active.contains(pid));
    for pid in active {
        if cache
            .get(&pid)
            .is_none_or(|entry| entry.checked.elapsed() >= SAMPLE_INTERVAL)
        {
            let history = cache.remove(&pid).map(|p| p.history).unwrap_or_default();
            cache.insert(pid, CachedProcess::sample(pid, history));
        }
    }

    let mut entries: Vec<PortEntry> = listeners
        .into_iter()
        .map(|listener| {
            let process = &cache[&listener.pid];
            let process_name = process
                .executable_path
                .as_deref()
                .and_then(|path| Path::new(path).file_name())
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or(listener.name);
            PortEntry {
                port: listener.port,
                pid: listener.pid,
                process_name,
                executable_path: process.executable_path.clone(),
                project_name: process.project_name.clone(),
                cwd: process.cwd.clone(),
                cpu_percent: process.cpu.current,
                cpu_average_percent: process.cpu.average,
                cpu_history_seconds: process.cpu.seconds,
                memory_bytes: process.memory_bytes,
                power_watts: process.cpu.power_watts,
            }
        })
        .collect();
    entries.sort_by_key(|e| (e.port, e.pid));
    Ok(entries)
}

fn parse_listeners(output: &[u8], min_port: u16, max_port: u16) -> Vec<Listener> {
    let mut pid = None;
    let mut name = String::new();
    let mut seen = HashSet::new();
    let mut listeners = Vec::new();
    for field in output.split(|&b| b == 0) {
        // lsof inserts a newline between each process/file record, even in -F0.
        let field = field.strip_prefix(b"\n").unwrap_or(field);
        let Some((&kind, value)) = field.split_first() else {
            continue;
        };
        let value = String::from_utf8_lossy(value);
        match kind {
            b'p' => {
                pid = value.parse::<u32>().ok();
                name.clear();
            }
            b'c' => name = value.into_owned(),
            b'n' => {
                let Some(pid) = pid else { continue };
                let Some(port) = value.rsplit(':').next().and_then(|p| p.parse::<u16>().ok())
                else {
                    continue;
                };
                if (min_port..=max_port).contains(&port) && seen.insert((pid, port)) {
                    listeners.push(Listener {
                        port,
                        pid,
                        name: name.clone(),
                    });
                }
            }
            _ => {}
        }
    }
    listeners
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn machine_fields_keep_full_names_and_deduplicate_ipv4_ipv6() {
        let output = b"p123\0cExample Helper (Renderer)\0\nf4\0n127.0.0.1:3773\0\nf5\0n[::1]:3773\0\np456\0cAnother App\0\nf6\0n*:8765\0\n";
        let entries = parse_listeners(output, 3000, 9999);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "Example Helper (Renderer)");
        assert_eq!(entries[0].port, 3773);
        assert_eq!(entries[1].pid, 456);
        assert_eq!(parse_listeners(output, 8000, 9000).len(), 1);
    }

    #[test]
    fn malformed_records_do_not_reuse_a_previous_pid() {
        let output = b"p1\0cfirst\0\nn*:5000\0\npinvalid\0\nn*:6000\0\np2\0csecond\0\nninvalid\0\nn*:99999\0\n";
        let entries = parse_listeners(output, 1, 65535);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].pid, 1);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn discovers_and_enriches_a_real_listener() {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let entries = scan_ports(port, port).unwrap();
        let entry = entries
            .iter()
            .find(|entry| entry.pid == std::process::id())
            .unwrap();
        assert_eq!(entry.port, port);
        assert!(entry.executable_path.is_some());
        assert!(entry.memory_bytes.is_some_and(|bytes| bytes > 0));
        assert!(entry.cwd.is_some());
        // A repeated port scan reuses its resource sample instead of producing
        // an invalid CPU percentage from an almost-zero measurement interval.
        let again = scan_ports(port, port).unwrap();
        let again = again
            .iter()
            .find(|entry| entry.pid == std::process::id())
            .unwrap();
        assert_eq!(entry.cpu_history_seconds, again.cpu_history_seconds);
        assert_eq!(entry.memory_bytes, again.memory_bytes);
    }
}

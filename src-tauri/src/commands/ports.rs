use std::collections::HashMap;
use std::process::Command;

use crate::models::PortEntry;

use super::project::infer_project_name;

#[tauri::command]
pub async fn list_ports(min_port: u16, max_port: u16) -> Result<Vec<PortEntry>, String> {
    let output = Command::new("lsof")
        .args(["-iTCP", "-sTCP:LISTEN", "-n", "-P"])
        .output()
        .map_err(|e| e.to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut seen: HashMap<(u32, u16), PortEntry> = HashMap::new();

    for line in stdout.lines().skip(1) {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 9 {
            continue;
        }
        let process_name = cols[0].to_string();
        let pid: u32 = match cols[1].parse() {
            Ok(p) => p,
            Err(_) => continue,
        };
        let name_col = cols[8];
        let port: u16 = match name_col.rsplit(':').next().and_then(|p| p.parse().ok()) {
            Some(p) => p,
            None => continue,
        };

        if port < min_port || port > max_port {
            continue;
        }

        let key = (pid, port);
        if seen.contains_key(&key) {
            continue;
        }

        let cwd = get_process_cwd(pid);
        let project_name = cwd.as_deref().and_then(infer_project_name);

        seen.insert(
            key,
            PortEntry {
                port,
                pid,
                process_name,
                project_name,
                cwd,
            },
        );
    }

    let mut entries: Vec<PortEntry> = seen.into_values().collect();
    entries.sort_by_key(|e| e.port);
    Ok(entries)
}

fn get_process_cwd(pid: u32) -> Option<String> {
    let output = Command::new("lsof")
        .args(["-p", &pid.to_string(), "-a", "-d", "cwd"])
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines().skip(1) {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() >= 9 {
            return Some(cols[8..].join(" "));
        }
    }
    None
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortEntry {
    pub port: u16,
    pub pid: u32,
    pub process_name: String,
    pub executable_path: Option<String>,
    pub project_name: Option<String>,
    pub cwd: Option<String>,
    pub cpu_percent: Option<f64>,
    pub cpu_average_percent: Option<f64>,
    pub cpu_history_seconds: f64,
    pub memory_bytes: Option<u64>,
    pub power_watts: Option<f64>,
}

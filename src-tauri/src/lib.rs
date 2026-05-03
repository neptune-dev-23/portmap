mod commands;
mod models;

use commands::{ports::list_ports, process::kill_process};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .invoke_handler(tauri::generate_handler![list_ports, kill_process])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

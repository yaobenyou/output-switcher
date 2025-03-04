use tauri::AppHandle;
use tauri_plugin_window_state::{AppHandleExt, StateFlags};

pub mod audio;
pub mod error;
pub mod init;
pub mod sender;

#[tauri::command]
pub fn quit(app: AppHandle) {
    let try_save = app.save_window_state(StateFlags::all());
    if let Err(e) = try_save {
        eprintln!("Failed to save window state: {:?}", e);
    }

    app.exit(0)
}

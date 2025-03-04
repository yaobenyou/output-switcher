// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod ipc;

use std::process::exit;
use tauri::{
    async_runtime::Sender, AppHandle, CustomMenuItem, Manager, State, SystemTray, SystemTrayEvent,
    SystemTrayMenu,
};

use anyhow::Result;
use ipc::{
    error::{APIError, UnexpectedErr},
    init::{prepare_backend, setup, BackendPrepareRet, IPCHandlers},
    quit,
};

#[derive(Clone, serde::Serialize)]
struct Payload {
    args: Vec<String>,
    cwd: String,
}

fn handle_window(event: tauri::GlobalWindowEvent) {
    match event.event() {
        tauri::WindowEvent::CloseRequested { .. } => {
            let app = event.window().app_handle();
            quit(app);
        }
        _ => {}
    }
}

fn create_task_tray() -> SystemTray {
    let quit = CustomMenuItem::new("quit".to_string(), "終了");

    let tray = SystemTrayMenu::new().add_item(quit);
    let system_tray = SystemTray::new().with_menu(tray);

    system_tray
}

fn handle_system_tray(app: &AppHandle, event: SystemTrayEvent) {
    match event {
        SystemTrayEvent::LeftClick { .. } => {
            let window = app.get_window("main").unwrap();

            window.show().unwrap();
            window.set_focus().unwrap();
        }
        SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
            "quit" => {
                exit(0);
            }
            _ => {}
        },
        _ => {}
    }
}

#[tauri::command]
async fn query(tx: State<'_, Sender<IPCHandlers>>, query: IPCHandlers) -> Result<(), APIError> {
    log::info!("query: {:?}", query);
    tx.send(query).await.map_err(|_| APIError::Unexpected {
        inner: UnexpectedErr::MPSCClosedError,
    })?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let BackendPrepareRet {
        relay_thread,
        backend_thread,
        ipc_tx,
        ipc_rx,
    } = prepare_backend().await?;

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            println!("{}, {argv:?}, {cwd}", app.package_info().name);
            app.emit_all("single-instance", Payload { args: argv, cwd })
                .unwrap();
        }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_context_menu::init())
        .invoke_handler(tauri::generate_handler![query, quit])
        .system_tray(create_task_tray())
        .on_window_event(handle_window)
        .on_system_tray_event(handle_system_tray)
        .manage(ipc_tx)
        .setup(|app| {
            setup(app, ipc_rx);

            #[cfg(debug_assertions)]
            {
                // let main_window = app.get_window("main").unwrap();
                // main_window.open_devtools();
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    let e = relay_thread.await;
    if let Err(e) = e {
        log::error!("relay_thread end with Error: {:?}", e);
    }

    let e = backend_thread.await;
    if let Err(e) = e {
        log::error!("backend_thread end with Error: {:?}", e);
    }

    Ok(())
}

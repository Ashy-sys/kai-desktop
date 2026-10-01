#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use tauri::{menu::{Menu, MenuItem}, tray::TrayIconBuilder, Manager, WebviewWindow};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WindowPosition { x: i32, y: i32 }

fn position_file(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|error| error.to_string())?;
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    Ok(dir.join("kai-position.json"))
}

#[tauri::command]
fn save_window_position(app: tauri::AppHandle, x: i32, y: i32) -> Result<(), String> {
    let file = position_file(&app)?;
    let content = serde_json::to_string(&WindowPosition { x, y }).map_err(|error| error.to_string())?;
    fs::write(file, content).map_err(|error| error.to_string())?;
    log::info!("Saved Kai position: {x}, {y}");
    Ok(())
}

#[tauri::command]
fn load_window_position(app: tauri::AppHandle) -> Result<Option<WindowPosition>, String> {
    let file = position_file(&app)?;
    match fs::read_to_string(file) {
        Ok(content) => {
            let saved: WindowPosition = serde_json::from_str(&content).map_err(|error| error.to_string())?;
            // Keep an old monitor arrangement from hiding Kai permanently off-screen.
            let visible = app.available_monitors().map(|monitors| monitors.iter().any(|monitor| {
                let origin = monitor.position();
                let size = monitor.size();
                saved.x >= origin.x - 280
                    && saved.x <= origin.x + size.width as i32 - 40
                    && saved.y >= origin.y - 240
                    && saved.y <= origin.y + size.height as i32 - 40
            })).unwrap_or(true);
            if visible { Ok(Some(saved)) } else {
                log::warn!("Discarded an off-screen Kai position");
                Ok(None)
            }
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

#[tauri::command]
fn quit_app(app: tauri::AppHandle) { app.exit(0); }

fn show_window(window: &WebviewWindow) { let _ = window.show(); let _ = window.set_focus(); }

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::default().level(log::LevelFilter::Info).build())
        .setup(|app| {
            let show = MenuItem::with_id(app, "show", "Show Kai", true, None::<&str>)?;
            let hide = MenuItem::with_id(app, "hide", "Hide Kai", true, None::<&str>)?;
            let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &hide, &settings, &quit])?;
            let handle = app.handle().clone();
            TrayIconBuilder::with_id("kai-tray").menu(&menu).tooltip("Kai Desktop").on_menu_event(move |_tray, event| {
                match event.id.as_ref() {
                    "show" | "settings" => if let Some(window) = handle.get_webview_window("main") { show_window(&window); },
                    "hide" => if let Some(window) = handle.get_webview_window("main") { let _ = window.hide(); },
                    "quit" => handle.exit(0),
                    _ => {}
                }
            }).build(app)?;
            log::info!("Kai Desktop started");
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event { api.prevent_close(); let _ = window.hide(); }
        })
        .invoke_handler(tauri::generate_handler![save_window_position, load_window_position, quit_app])
        .run(tauri::generate_context!())
        .expect("error while running Kai Desktop");
}

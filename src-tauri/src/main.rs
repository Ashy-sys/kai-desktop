#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::{fs, path::{Path, PathBuf}, process::Command};
use tauri::{menu::{Menu, MenuItem}, tray::TrayIconBuilder, Emitter, Manager, WebviewWindow};
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WindowPosition { x: i32, y: i32 }
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Settings { show_companion: bool, show_island: bool, island_always_visible: bool, auto_hide_fullscreen: bool, reduced_motion: bool, launch_at_login: bool, search_provider: String }
impl Default for Settings { fn default() -> Self { Self { show_companion:true, show_island:true, island_always_visible:true, auto_hide_fullscreen:false, reduced_motion:false, launch_at_login:false, search_provider:"auto".into() } } }
#[derive(Serialize)] #[serde(rename_all = "camelCase")]
struct SearchResult { name:String, path:String, extension:Option<String>, size:Option<u64>, modified:Option<String>, is_directory:bool }
#[derive(Serialize)] struct EverythingStatus { status:String }

fn data_file(app:&tauri::AppHandle, name:&str)->Result<PathBuf,String>{ let dir=app.path().app_data_dir().map_err(|e|e.to_string())?;fs::create_dir_all(&dir).map_err(|e|e.to_string())?;Ok(dir.join(name)) }

fn position_file(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    data_file(app,"kai-position.json")
}
fn settings_file(app:&tauri::AppHandle)->Result<PathBuf,String>{data_file(app,"settings.json")}
#[tauri::command] fn get_settings(app:tauri::AppHandle)->Result<Settings,String>{let file=settings_file(&app)?;match fs::read_to_string(file){Ok(v)=>serde_json::from_str(&v).map_err(|e|e.to_string()),Err(e)if e.kind()==std::io::ErrorKind::NotFound=>Ok(Settings::default()),Err(e)=>Err(e.to_string())}}
#[tauri::command] fn save_settings(app:tauri::AppHandle,settings:Settings)->Result<(),String>{ if !["auto","everything","fallback"].contains(&settings.search_provider.as_str()){return Err("invalid search provider".into())} fs::write(settings_file(&app)?,serde_json::to_string(&settings).map_err(|e|e.to_string())?).map_err(|e|e.to_string()) }
#[tauri::command] fn publish_kai_event(app:tauri::AppHandle,event:serde_json::Value)->Result<(),String>{app.emit("kai-event",event).map_err(|e|e.to_string())}

fn validate_query(query:&str)->Result<String,String>{let q=query.trim();if q.is_empty()||q.len()>120||q.chars().any(|c|c.is_control()) {Err("Search query must be 1–120 printable characters".into())}else{Ok(q.to_lowercase())}}
fn search_roots()->Vec<PathBuf>{let mut roots=Vec::new();if let Ok(home)=std::env::var("USERPROFILE"){for name in ["Desktop","Documents","Downloads"]{let p=PathBuf::from(&home).join(name);if p.is_dir(){roots.push(p)}}}roots}
#[tauri::command] fn search_files(query:String)->Result<Vec<SearchResult>,String>{let q=validate_query(&query)?;let mut out=Vec::new();for root in search_roots(){for entry in WalkDir::new(root).follow_links(false).max_depth(8).into_iter().filter_map(Result::ok){if out.len()>=80{break}let name=entry.file_name().to_string_lossy();if !name.to_lowercase().contains(&q){continue}let metadata=entry.metadata().ok();out.push(SearchResult{name:name.into_owned(),path:entry.path().display().to_string(),extension:entry.path().extension().map(|v|v.to_string_lossy().into_owned()),size:metadata.as_ref().filter(|m|m.is_file()).map(|m|m.len()),modified:None,is_directory:entry.file_type().is_dir()});}}Ok(out)}
#[tauri::command] fn everything_status()->EverythingStatus{let found=["ProgramFiles","ProgramFiles(x86)"].iter().filter_map(|key|std::env::var(key).ok()).map(|base|PathBuf::from(base).join("Everything").join("Everything.exe")).any(|p|p.exists());EverythingStatus{status:if found{"detected — local IPC/CLI connection not configured".into()}else{"not detected — using safe local fallback".into()}}}
fn valid_existing(path:&str)->Result<&Path,String>{let p=Path::new(path);if !p.exists(){Err("File no longer exists".into())}else{Ok(p)}}
#[tauri::command] fn open_file(path:String)->Result<(),String>{let p=valid_existing(&path)?;Command::new("explorer.exe").arg(p).spawn().map(|_|()).map_err(|e|e.to_string())}
#[tauri::command] fn reveal_file(path:String)->Result<(),String>{let p=valid_existing(&path)?;Command::new("explorer.exe").arg(format!("/select,{}",p.display())).spawn().map(|_|()).map_err(|e|e.to_string())}
#[tauri::command] fn launch_app(app:String)->Result<(),String>{let executable=match app.as_str(){"calculator"=>"calc.exe","notepad"=>"notepad.exe","explorer"=>"explorer.exe","settings"=>"ms-settings:",_=>return Err("Application is not allowlisted".into())};Command::new(executable).spawn().map(|_|()).map_err(|e|e.to_string())}
#[tauri::command] fn system_info()->serde_json::Value{let disks=sysinfo::Disks::new_with_refreshed_list();let free: u64=disks.iter().map(|d|d.available_space()).sum();serde_json::json!({"freeDiskBytes":free,"connectivity":"unknown"})}

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
            if let Some(island) = app.get_webview_window("island") {
                if let Some(monitor) = island.current_monitor().ok().flatten().or_else(|| app.primary_monitor().ok().flatten()) {
                    let size = monitor.size(); let pos = monitor.position();
                    let x = pos.x + (size.width as i32 - 430) / 2;
                    let _ = island.set_position(tauri::Position::Physical(tauri::PhysicalPosition::new(x, pos.y + 8)));
                }
            }
            log::info!("Kai Desktop started");
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event { api.prevent_close(); let _ = window.hide(); }
        })
        .invoke_handler(tauri::generate_handler![save_window_position, load_window_position, quit_app, get_settings, save_settings, publish_kai_event, search_files, everything_status, open_file, reveal_file, launch_app, system_info])
        .run(tauri::generate_context!())
        .expect("error while running Kai Desktop");
}

#[cfg(test)]
mod tests {
    use super::validate_query;
    #[test] fn accepts_normal_search() { assert_eq!(validate_query(" invoice.pdf ").unwrap(), "invoice.pdf"); }
    #[test] fn rejects_malformed_search() { assert!(validate_query("").is_err()); assert!(validate_query("a\n").is_err()); }
}

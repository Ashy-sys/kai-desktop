#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::{fs, io::{Read, Write}, net::{SocketAddr, TcpStream}, path::{Path, PathBuf}, process::Command, time::Duration};
use sysinfo::System;
use winreg::{enums::HKEY_CURRENT_USER, RegKey};
use tauri::{menu::{Menu, MenuItem}, tray::TrayIconBuilder, Emitter, Manager, WebviewWindow};
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WindowPosition { x: i32, y: i32 }
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Settings { #[serde(default)] display_name: String, show_companion: bool, show_island: bool, island_always_visible: bool, auto_hide_fullscreen: bool, reduced_motion: bool, launch_at_login: bool, search_provider: String, #[serde(default)] search_roots: Vec<String>, #[serde(default)] online_ai_enabled: bool }
impl Default for Settings { fn default() -> Self { Self { display_name:String::new(), show_companion:true, show_island:true, island_always_visible:true, auto_hide_fullscreen:false, reduced_motion:false, launch_at_login:false, search_provider:"auto".into(), search_roots:Vec::new(), online_ai_enabled:false } } }
#[derive(Debug, Clone, Serialize, Deserialize)] #[serde(rename_all = "camelCase")]
struct SearchResult { name:String, path:String, extension:Option<String>, size:Option<u64>, modified:Option<String>, is_directory:bool }
#[derive(Serialize)] #[serde(rename_all="camelCase")] struct EverythingStatus { status:String, installed:bool, http_available:bool, active_provider:String }
#[derive(Debug, Serialize, Deserialize, Default)] struct Memory { #[serde(default)] recent_commands:Vec<String>, #[serde(default)] recent_searches:Vec<String> }
#[derive(Serialize)] #[serde(tag="kind",rename_all="camelCase")] enum IntentResponse { Search { query:String, results:Vec<SearchResult>, provider:String }, Message { title:String, body:String, data:serde_json::Value }, Unsupported { message:String } }

fn data_file(app:&tauri::AppHandle, name:&str)->Result<PathBuf,String>{ let dir=app.path().app_data_dir().map_err(|e|e.to_string())?;fs::create_dir_all(&dir).map_err(|e|e.to_string())?;Ok(dir.join(name)) }

fn position_file(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    data_file(app,"kai-position.json")
}
fn settings_file(app:&tauri::AppHandle)->Result<PathBuf,String>{data_file(app,"settings.json")}
fn memory_file(app:&tauri::AppHandle)->Result<PathBuf,String>{data_file(app,"memory.json")}
#[tauri::command] fn get_settings(app:tauri::AppHandle)->Result<Settings,String>{let file=settings_file(&app)?;let mut settings=match fs::read_to_string(file){Ok(v)=>serde_json::from_str(&v).map_err(|e|e.to_string())?,Err(e)if e.kind()==std::io::ErrorKind::NotFound=>Settings::default(),Err(e)=>return Err(e.to_string())};settings.launch_at_login=startup_enabled();Ok(settings)}
fn set_startup(enabled:bool)->Result<(),String>{let hkcu=RegKey::predef(HKEY_CURRENT_USER);let (key,_)=hkcu.create_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run").map_err(|e|e.to_string())?;if enabled{let exe=std::env::current_exe().map_err(|e|e.to_string())?;key.set_value("Kai Desktop",&format!("\"{}\"",exe.display())).map_err(|e|e.to_string())?}else{match key.delete_value("Kai Desktop"){Ok(_)=>{},Err(e)if e.kind()==std::io::ErrorKind::NotFound=>{},Err(e)=>return Err(e.to_string())}}Ok(())}
fn startup_enabled()->bool{RegKey::predef(HKEY_CURRENT_USER).open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run").ok().and_then(|k:RegKey|k.get_value::<String,_>("Kai Desktop").ok()).is_some()}
#[tauri::command] fn save_settings(app:tauri::AppHandle,mut settings:Settings)->Result<(),String>{ if !["auto","everything","fallback"].contains(&settings.search_provider.as_str()){return Err("invalid search provider".into())} for root in &settings.search_roots{let p=Path::new(root);if !p.is_absolute()||!p.is_dir(){return Err("Search roots must be existing folders".into())}}set_startup(settings.launch_at_login)?;settings.launch_at_login=startup_enabled();fs::write(settings_file(&app)?,serde_json::to_string_pretty(&settings).map_err(|e|e.to_string())?).map_err(|e|e.to_string()) }
#[tauri::command] fn startup_status()->bool{startup_enabled()}
fn read_memory(app:&tauri::AppHandle)->Memory{memory_file(app).ok().and_then(|p|fs::read_to_string(p).ok()).and_then(|v|serde_json::from_str(&v).ok()).unwrap_or_default()}
fn remember(app:&tauri::AppHandle,value:&str,search:bool){let mut memory=read_memory(app);let list=if search{&mut memory.recent_searches}else{&mut memory.recent_commands};list.retain(|v|v!=value);list.insert(0,value.into());list.truncate(20);if let Ok(path)=memory_file(app){let _=fs::write(path,serde_json::to_string_pretty(&memory).unwrap_or_default());}}
#[tauri::command] fn get_memory(app:tauri::AppHandle)->Memory{read_memory(&app)}
#[tauri::command] fn clear_memory(app:tauri::AppHandle)->Result<(),String>{let path=memory_file(&app)?;match fs::remove_file(path){Ok(_)=>Ok(()),Err(e)if e.kind()==std::io::ErrorKind::NotFound=>Ok(()),Err(e)=>Err(e.to_string())}}
#[tauri::command] fn publish_kai_event(app:tauri::AppHandle,event:serde_json::Value)->Result<(),String>{app.emit("kai-event",event).map_err(|e|e.to_string())}

fn validate_query(query:&str)->Result<String,String>{if query.chars().any(|c|c.is_control()) {return Err("Search query must be 1–120 printable characters".into())} let q=query.trim();if q.is_empty()||q.len()>120 {Err("Search query must be 1–120 printable characters".into())}else{Ok(q.to_string())}}
fn search_roots(settings:&Settings)->Vec<PathBuf>{let mut roots=Vec::new();if let Ok(home)=std::env::var("USERPROFILE"){for name in ["Desktop","Documents","Downloads","Pictures","Videos","Music"]{let p=PathBuf::from(&home).join(name);if p.is_dir(){roots.push(p)}}}for root in &settings.search_roots{let p=PathBuf::from(root);if p.is_dir()&&!roots.contains(&p){roots.push(p)}}roots}
fn everything_installed()->bool{["ProgramFiles","ProgramFiles(x86)","LOCALAPPDATA"].iter().filter_map(|key|std::env::var(key).ok()).map(|base|PathBuf::from(base).join("Everything").join("Everything.exe")).any(|p|p.exists())}
fn everything_http(query:&str)->Result<Vec<SearchResult>,String>{let address=SocketAddr::from(([127,0,0,1],80));let mut stream=TcpStream::connect_timeout(&address,Duration::from_millis(180)).map_err(|_|"Everything HTTP server is unavailable".to_string())?;stream.set_read_timeout(Some(Duration::from_millis(700))).ok();let uri=format!("/?search={}&count=80&json=1&path_column=1&size_column=1&date_modified_column=1",urlencoding::encode(query));let request=format!("GET {uri} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");stream.write_all(request.as_bytes()).map_err(|e|e.to_string())?;let mut response=String::new();stream.read_to_string(&mut response).map_err(|e|e.to_string())?;let body=response.split("\r\n\r\n").nth(1).ok_or("Invalid Everything HTTP response")?;#[derive(Deserialize)]struct HttpResult{r#type:String,name:String,path:String,size:Option<String>,date_modified:Option<String>}#[derive(Deserialize)]struct HttpResponse{results:Vec<HttpResult>}let data:HttpResponse=serde_json::from_str(body).map_err(|e|e.to_string())?;Ok(data.results.into_iter().map(|r|{let full=PathBuf::from(&r.path).join(&r.name);SearchResult{name:r.name,extension:full.extension().map(|v|v.to_string_lossy().into_owned()),path:full.display().to_string(),size:r.size.and_then(|v|v.parse().ok()),modified:r.date_modified,is_directory:r.r#type.eq_ignore_ascii_case("folder")}}).collect())}
fn fallback_search(query:&str,settings:&Settings)->Vec<SearchResult>{let q=query.to_lowercase();let wildcard_ext=q.strip_prefix("*.").map(str::to_string);let mut out=Vec::new();for root in search_roots(settings){for entry in WalkDir::new(root).follow_links(false).max_depth(8).into_iter().filter_map(Result::ok){if out.len()>=80{break}let name=entry.file_name().to_string_lossy();let matched=wildcard_ext.as_ref().map(|ext|entry.path().extension().map(|v|v.to_string_lossy().eq_ignore_ascii_case(ext)).unwrap_or(false)).unwrap_or_else(||name.to_lowercase().contains(&q));if !matched{continue}let metadata=entry.metadata().ok();let modified=metadata.as_ref().and_then(|m|m.modified().ok()).and_then(|t|t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d|d.as_secs().to_string());out.push(SearchResult{name:name.into_owned(),path:entry.path().display().to_string(),extension:entry.path().extension().map(|v|v.to_string_lossy().into_owned()),size:metadata.as_ref().filter(|m|m.is_file()).map(|m|m.len()),modified,is_directory:entry.file_type().is_dir()});}}out}
fn perform_search(app:&tauri::AppHandle,query:&str)->Result<(Vec<SearchResult>,String),String>{let query=validate_query(query)?;let settings=get_settings(app.clone())?;remember(app,&query,true);if settings.search_provider!="fallback"{if let Ok(results)=everything_http(&query){return Ok((results,"Everything HTTP".into()))}if settings.search_provider=="everything"{log::warn!("Everything HTTP unavailable; using fallback")}}Ok((fallback_search(&query,&settings),"Local fallback".into()))}
#[tauri::command] fn search_files(app:tauri::AppHandle,query:String)->Result<Vec<SearchResult>,String>{perform_search(&app,&query).map(|v|v.0)}
#[tauri::command] fn everything_status()->EverythingStatus{let installed=everything_installed();let http_available=TcpStream::connect_timeout(&SocketAddr::from(([127,0,0,1],80)),Duration::from_millis(100)).is_ok();EverythingStatus{status:if http_available{"Everything whole-PC search ready".into()}else if installed{"Everything detected; enable its HTTP server for Kai whole-PC search".into()}else{"Everything not detected; using local fallback".into()},installed,http_available,active_provider:if http_available{"Everything HTTP".into()}else{"Local fallback".into()}}}
fn valid_existing(path:&str)->Result<&Path,String>{let p=Path::new(path);if !p.exists(){Err("File no longer exists".into())}else{Ok(p)}}
#[tauri::command] fn open_file(path:String)->Result<(),String>{let p=valid_existing(&path)?;Command::new("explorer.exe").arg(p).spawn().map(|_|()).map_err(|e|e.to_string())}
#[tauri::command] fn reveal_file(path:String)->Result<(),String>{let p=valid_existing(&path)?;Command::new("explorer.exe").arg(format!("/select,{}",p.display())).spawn().map(|_|()).map_err(|e|e.to_string())}
fn app_target(app:&str)->Option<&'static str>{match app{"calculator"=>Some("calc.exe"),"notepad"=>Some("notepad.exe"),"explorer"=>Some("explorer.exe"),"settings"=>Some("ms-settings:"),_=>None}}
#[tauri::command] fn launch_app(app:String)->Result<(),String>{let executable=app_target(&app).ok_or("Application is not allowlisted")?;Command::new(executable).spawn().map(|_|()).map_err(|e|e.to_string())}
fn connectivity()->&'static str{let mut flags=0u32;let online=unsafe{windows_sys::Win32::Networking::WinInet::InternetGetConnectedState(&mut flags,0)}!=0;if online{"online"}else{"offline"}}
#[tauri::command] fn system_info()->serde_json::Value{let mut system=System::new_all();std::thread::sleep(Duration::from_millis(220));system.refresh_cpu_usage();system.refresh_memory();let disks=sysinfo::Disks::new_with_refreshed_list();let total: u64=disks.iter().map(|d|d.total_space()).sum();let free: u64=disks.iter().map(|d|d.available_space()).sum();let mut power:windows_sys::Win32::System::Power::SYSTEM_POWER_STATUS=unsafe{std::mem::zeroed()};let has_power=unsafe{windows_sys::Win32::System::Power::GetSystemPowerStatus(&mut power)}!=0;serde_json::json!({"cpuPercent":system.global_cpu_usage().round(),"memoryUsedBytes":system.used_memory(),"memoryTotalBytes":system.total_memory(),"diskFreeBytes":free,"diskTotalBytes":total,"batteryPercent":if has_power&&power.BatteryLifePercent<=100{Some(power.BatteryLifePercent)}else{None},"charging":if has_power{Some(power.ACLineStatus==1)}else{None},"connectivity":connectivity()})}

#[derive(Debug,PartialEq)] enum LocalIntent{Search(String),Launch(&'static str),OpenKnown(&'static str),SystemInfo,Connectivity,Unsupported}
fn parse_intent(input:&str)->LocalIntent{let lower=input.trim().to_lowercase();let normalized=lower.trim_end_matches(['?','.']);if let Some(rest)=normalized.strip_prefix("find ").or_else(||normalized.strip_prefix("search for ")).or_else(||normalized.strip_prefix("search ")){let query=match rest{"mp4 files"=>"*.mp4","jpg files"=>"*.jpg","videos"=>"*.mp4",other=>other};return LocalIntent::Search(query.into())}match normalized{"open calculator"=>LocalIntent::Launch("calculator"),"open notepad"=>LocalIntent::Launch("notepad"),"open file explorer"|"open explorer"=>LocalIntent::Launch("explorer"),"open settings"|"open windows settings"=>LocalIntent::Launch("settings"),"open downloads"=>LocalIntent::OpenKnown("Downloads"),"open documents"=>LocalIntent::OpenKnown("Documents"),"how much disk space do i have"|"how much storage is left"|"show storage"|"system info"|"battery"|"what's my battery"|"memory usage"|"how much ram am i using"=>LocalIntent::SystemInfo,"am i offline"|"connectivity"|"network status"=>LocalIntent::Connectivity,_=>LocalIntent::Unsupported}}
#[tauri::command] fn execute_local_command(app:tauri::AppHandle,request:String)->Result<IntentResponse,String>{if request.len()>300||request.chars().any(|c|c.is_control()){return Err("Request must be short printable text".into())}remember(&app,&request,false);match parse_intent(&request){LocalIntent::Search(query)=>{let(results,provider)=perform_search(&app,&query)?;Ok(IntentResponse::Search{query,results,provider})},LocalIntent::Launch(target)=>{launch_app(target.into())?;Ok(IntentResponse::Message{title:"Opened".into(),body:format!("{} is ready.",target),data:serde_json::Value::Null})},LocalIntent::OpenKnown(folder)=>{let home=std::env::var("USERPROFILE").map_err(|_|"User folder unavailable")?;let path=PathBuf::from(home).join(folder);if !path.is_dir(){return Err("That folder is unavailable".into())}Command::new("explorer.exe").arg(path).spawn().map_err(|e|e.to_string())?;Ok(IntentResponse::Message{title:"Opened".into(),body:format!("{} is ready.",folder),data:serde_json::Value::Null})},LocalIntent::SystemInfo=>Ok(IntentResponse::Message{title:"Your PC".into(),body:"Here’s the latest snapshot.".into(),data:system_info()}),LocalIntent::Connectivity=>Ok(IntentResponse::Message{title:"Connection".into(),body:if connectivity()=="online"{"You’re online.".into()}else{"You’re offline. Local Kai still works.".into()},data:serde_json::json!({"connectivity":connectivity()})}),LocalIntent::Unsupported=>Ok(IntentResponse::Unsupported{message:"I can search this PC, open a few Windows apps, or check system info. Online AI isn’t configured yet.".into()})}}

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
fn center_island(app: &tauri::AppHandle, width: i32) {
    if let Some(island) = app.get_webview_window("island") {
        if let Some(monitor) = island.current_monitor().ok().flatten().or_else(|| app.primary_monitor().ok().flatten()) {
            let size = monitor.size(); let pos = monitor.position();
            let _ = island.set_position(tauri::Position::Physical(tauri::PhysicalPosition::new(pos.x + (size.width as i32 - width) / 2, pos.y + 8)));
        }
    }
}
#[tauri::command]
fn resize_island(app:tauri::AppHandle, mode:String)->Result<(),String>{
    let (width,height) = match mode.as_str() { "micro" => (68.0,52.0), "compact" => (88.0,68.0), "stretch" => (400.0,68.0), "expanded" => (400.0,248.0), "content" => (400.0,430.0), _ => return Err("Unknown Island presentation".into()) };
    let island=app.get_webview_window("island").ok_or("Island window unavailable")?;
    island.set_size(tauri::LogicalSize::new(width,height)).map_err(|e|e.to_string())?;
    center_island(&app,island.outer_size().map_err(|e|e.to_string())?.width as i32);
    Ok(())
}

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
                    "show" | "settings" => if let Some(window) = handle.get_webview_window("island") { show_window(&window); },
                    "hide" => if let Some(window) = handle.get_webview_window("island") { let _ = window.hide(); },
                    "quit" => handle.exit(0),
                    _ => {}
                }
            }).build(app)?;
            resize_island(app.handle().clone(), "compact".into())?;
            log::info!("Kai Desktop started");
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event { api.prevent_close(); let _ = window.hide(); }
        })
        .invoke_handler(tauri::generate_handler![save_window_position, load_window_position, quit_app, get_settings, save_settings, startup_status, get_memory, clear_memory, publish_kai_event, search_files, everything_status, open_file, reveal_file, launch_app, system_info, execute_local_command, resize_island])
        .run(tauri::generate_context!())
        .expect("error while running Kai Desktop");
}

#[cfg(test)]
mod tests {
    use super::{app_target, parse_intent, validate_query, LocalIntent};
    #[test] fn accepts_normal_search() { assert_eq!(validate_query(" invoice.pdf ").unwrap(), "invoice.pdf"); }
    #[test] fn rejects_malformed_search() { assert!(validate_query("").is_err()); assert!(validate_query("a\n").is_err()); }
    #[test] fn parses_offline_intents() { assert_eq!(parse_intent("Find invoice.pdf"),LocalIntent::Search("invoice.pdf".into()));assert_eq!(parse_intent("find mp4 files"),LocalIntent::Search("*.mp4".into()));assert_eq!(parse_intent("open Calculator"),LocalIntent::Launch("calculator"));assert_eq!(parse_intent("show storage"),LocalIntent::SystemInfo); }
    #[test] fn rejects_unknown_apps() { assert_eq!(app_target("calculator"),Some("calc.exe"));assert_eq!(app_target("powershell"),None); }
}

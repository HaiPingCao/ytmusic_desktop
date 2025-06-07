// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use tauri::{
    http::{ResponseBuilder, Uri},
    CustomMenuItem, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu, SystemTrayMenuItem,
};
use tokio::sync::Mutex;

use std::{str::FromStr, sync::Arc, time::SystemTime};

use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};

fn get_sys_time_in_secs() -> u64 {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(n) => n.as_secs(),
        Err(_) => panic!("SystemTime before UNIX EPOCH!"),
    }
}

fn create_tray() -> SystemTray {
    let previous = CustomMenuItem::new("previous".to_string(), "Previous Song");
    let play_pause = CustomMenuItem::new("play_pause".to_string(), "Play");
    let next = CustomMenuItem::new("next".to_string(), "Next Song");
    let reload_discord = CustomMenuItem::new("reload_discord".to_string(), "Reload Discord RPC");
    let quit = CustomMenuItem::new("quit".to_string(), "Quit");
    let tray_menu = SystemTrayMenu::new()
        .add_item(previous)
        .add_item(play_pause)
        .add_item(next)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(reload_discord)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(quit);
    SystemTray::new().with_menu(tray_menu)
}

fn update_status(dipc_client: Arc<Mutex<DiscordIpcClient>>, data: PlayerState) {
    if dipc_client.try_lock().is_err() {
        println!("Discord IPC client is not available");
        return;
    }
    let status_activity =
        activity::Activity::new().activity_type(activity::ActivityType::Listening);
    if data.is_distroyed {
        let _ = dipc_client
            .blocking_lock()
            .set_activity(status_activity.details("idle not playing"));
    } else {
        let video_data = data.video_data.unwrap();
        let acess = activity::Assets::new();
        let time_stam = activity::Timestamps::new();
        let start = get_sys_time_in_secs() - video_data.current_duration as u64;
        let end = start + video_data.duration as u64;
        let _ = dipc_client.blocking_lock().set_activity(
            status_activity
                .details(&video_data.title)
                .state(&video_data.artist)
                .assets(
                    acess
                        .large_image(&video_data.album_art)
                        .small_image(if data.is_playing { "play" } else { "pause" }),
                )
                .timestamps(if data.is_playing {
                    time_stam.start(start as i64).end(end as i64)
                } else {
                    time_stam
                })
                .buttons(vec![activity::Button::new(
                    "Play on YouTube Music",
                    &video_data.url,
                )]),
        );
    }
}

fn system_tray_event(app_handle: tauri::AppHandle, event: tauri::SystemTrayEvent) {
    match event {
        SystemTrayEvent::LeftClick { .. } => {
            let main = app_handle.get_window("main").unwrap();
            main.unminimize().unwrap();
            main.show().unwrap();
            main.set_focus().unwrap();
        }
        SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
            "quit" => {
                let dipc_client = app_handle.state::<Arc<Mutex<DiscordIpcClient>>>().inner();
                dipc_client.blocking_lock().close().unwrap();
                std::process::exit(0);
            }
            "play_pause" => {
                app_handle
                    .emit_to("main", "control_player", "play_pause")
                    .unwrap();
            }
            "previous" => {
                app_handle
                    .emit_to("main", "control_player", "previous")
                    .unwrap();
            }
            "next" => {
                app_handle
                    .emit_to("main", "control_player", "next")
                    .unwrap();
            }
            "reload_discord" => {
                let dipc_client = app_handle.state::<Arc<Mutex<DiscordIpcClient>>>().inner();
                if dipc_client.try_lock().is_err() {
                    println!("Discord IPC client is not available");
                    return;
                }
                match dipc_client.blocking_lock().reconnect() {
                    Ok(_) => println!("Discord RPC reloaded successfully"),
                    Err(e) => println!("Failed to reload Discord RPC: {}", e),
                }
                app_handle.emit_to("main", "status_update_req", "").unwrap();
            }
            _ => {}
        },
        _ => {}
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug, PartialEq)]
struct VideoData {
    pub title: String,
    pub artist: String,
    pub url: String,
    pub album_art: String,
    pub current_duration: f64,
    pub duration: f64,
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug, PartialEq)]
struct PlayerState {
    pub is_playing: bool,
    pub is_distroyed: bool,
    pub video_data: Option<VideoData>,
}

#[tauri::command]
fn update_state(app: tauri::AppHandle, data: PlayerState) {
    let dipc_client = app.state::<Arc<Mutex<DiscordIpcClient>>>().inner();

    app.tray_handle()
        .get_item("play_pause")
        .set_enabled(!data.is_distroyed)
        .unwrap();
    app.tray_handle()
        .get_item("previous")
        .set_enabled(!data.is_distroyed)
        .unwrap();
    app.tray_handle()
        .get_item("next")
        .set_enabled(!data.is_distroyed)
        .unwrap();

    app.tray_handle()
        .get_item("play_pause")
        .set_title(if data.is_playing { "Pause" } else { "Play" })
        .unwrap();

    update_status(dipc_client.clone(), data.clone());
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct PluginMetadata {
    pub name: String,
    pub export: Vec<String>,
    pub main_dir: String,
}

#[tauri::command]
fn get_plugin_list() -> Vec<String> {
    let working_dir = get_working_dir();
    let plugin_dir = working_dir.join("plugins");
    if !plugin_dir.exists() {
        std::fs::create_dir_all(&plugin_dir).unwrap();
        return vec![];
    }
    let mut plugins = vec![];
    for entry in std::fs::read_dir(plugin_dir).unwrap() {
        let entry = entry.unwrap();
        if !entry.file_type().unwrap().is_dir() {
            println!("Skipping non-directory entry: {:?}", entry.path());
            continue;
        }
        let dir_path = entry.path();
        let metadata_path = dir_path.join("metadata.json");
        if !metadata_path.exists() {
            println!("Metadata file does not exist for plugin: {:?}", dir_path);
            continue;
        }
        let metadata: PluginMetadata =
            serde_json::from_str(&std::fs::read_to_string(metadata_path).unwrap()).unwrap();
        let export = metadata
            .export
            .iter()
            .map(|s| {
                format!(
                    "{}/{}/{}",
                    entry.file_name().to_string_lossy(),
                    metadata.main_dir,
                    s.trim_start_matches('/')
                )
            })
            .collect::<Vec<String>>();

        plugins.extend(export.clone());
        println!(
            "Found plugin: {} with {} files",
            metadata.name,
            export.len()
        );
    }
    plugins
}

#[cfg(dev)]
fn get_working_dir() -> std::path::PathBuf {
    let mut path = std::env::current_dir().unwrap();
    path.push("../test_workspace");
    if !path.exists() {
        std::fs::create_dir_all(&path).unwrap();
    }
    path
}

#[cfg(not(dev))]
fn get_working_dir() -> std::path::PathBuf {
    let path = std::env::current_exe().unwrap();
    path
}

fn main() {
    let drpc_client = Arc::new(Mutex::new(DiscordIpcClient::new("1049275932239728672")));
    let drpc_client_th = drpc_client.clone();
    tauri::Builder::default()
        .register_uri_scheme_protocol("plugin", |_app, request| {
            let uri = Uri::from_str(request.uri()).unwrap();
            let path = uri.path().trim_start_matches('/');
            let plugin_dir = get_working_dir().join("plugins");
            if !plugin_dir.exists() {
                return ResponseBuilder::new().status(404).body("Not Found".into());
            }
            if path.is_empty() {
                return ResponseBuilder::new().status(404).body("Not Found".into());
            }
            let path_parts: Vec<&str> = path.split(|c| c == '/' || c == '\\').collect();

            let safe_parts: Vec<&str> = path_parts
                .iter()
                .filter(|&part| *part != ".." && *part != "." && !part.is_empty())
                .cloned()
                .collect();

            if safe_parts.is_empty() || safe_parts.len() != path_parts.len() {
                return ResponseBuilder::new()
                    .status(403)
                    .body("Forbidden: Path traversal attempt detected".into());
            }

            let safe_path = safe_parts.join(std::path::MAIN_SEPARATOR_STR);
            let plugin_path = plugin_dir.join(safe_path);

            if !plugin_path.starts_with(&plugin_dir) {
                return ResponseBuilder::new()
                    .status(403)
                    .body("Forbidden: Path outside plugin directory".into());
            }

            if !plugin_path.exists() {
                return ResponseBuilder::new().status(404).body("Not Found".into());
            }

            let content_type = if path.ends_with(".js") {
                "application/javascript"
            } else if path.ends_with(".css") {
                "text/css"
            } else {
                "text/plain"
            };
            let content = std::fs::read(&plugin_path).unwrap_or_else(|_| "File not found".into());
            ResponseBuilder::new()
                .status(200)
                .header("Content-Type", content_type)
                .body(content)
        })
        .setup(|app| {
            let app_handle = app.handle();
            std::thread::spawn(move || {
                drpc_client_th.blocking_lock().connect().unwrap();
                println!("Connected to discord rpc");
                app_handle.emit_to("main", "status_update_req", "").unwrap();
            });
            Ok(())
        })
        .manage(drpc_client)
        .system_tray(create_tray())
        .plugin(tauri_plugin_single_instance::init(|app_handle, _, _| {
            let main = app_handle.get_window("main").unwrap();
            main.unminimize().unwrap();
            main.show().unwrap();
            main.set_focus().unwrap();
        }))
        .on_system_tray_event(|a, e| system_tray_event(a.clone(), e))
        .on_window_event(|event| match event.event() {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                event.window().minimize().unwrap();
                event.window().hide().unwrap();
            }
            _ => {}
        })
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .on_page_load(|window, _page_load_payload| {
            let js_script = "
            if (window.trustedTypes && window.trustedTypes.createPolicy) { // Feature testing
                window.trustedTypes.createPolicy('default', {
                    createHTML: (string) => DOMPurify.sanitize(string, {RETURN_TRUSTED_TYPE: true}),
                    createScriptURL: string => string, // warning: this is unsafe!
                    createScript: string => string, // warning: this is unsafe!
                });
            }
            let script = document.createElement(\"script\");
            script.type = \"module\";
            script.src = \"https://tauri.localhost/load_lib.js\";
            document.head.appendChild(script);
            ";
            window.eval(&js_script).unwrap();
        })
        .invoke_handler(tauri::generate_handler![update_state, get_plugin_list])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

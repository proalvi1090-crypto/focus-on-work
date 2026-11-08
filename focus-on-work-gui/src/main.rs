use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use focus_on_work_core::process;
use focus_on_work_core::session::Session;
use notify_rust::Notification;
use serde::{Deserialize, Serialize};
use slint::{CloseRequestResponse, Timer, TimerMode};
use tray_icon::{
    Icon, TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuId, MenuItem},
};

slint::include_modules!();

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum Request {
    Start {
        duration: String,
        sites: Vec<String>,
        apps: Vec<String>,
    },
    Status,
}

#[derive(Debug, Deserialize)]
struct Response {
    ok: bool,
    error: Option<String>,
    session: Option<Session>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GuiConfig {
    profile: String,
    duration_minutes: i32,
    allow_list_mode: bool,
    #[serde(default = "default_social_media")]
    social_media_enabled: bool,
    #[serde(default)]
    games_enabled: bool,
    #[serde(default)]
    videos_enabled: bool,
    #[serde(default)]
    custom_sites: String,
    #[serde(default)]
    custom_apps: String,
    sites: Vec<String>,
    apps: Vec<String>,
}

fn default_social_media() -> bool {
    true
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    gtk::init()?;
    let ui = AppWindow::new()?;
    let socket = env::var_os("FOW_SOCKET_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/run/focus-on-work/focus-on-work.sock"));
    let config_path = env::var_os("FOW_GUI_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("focus-on-work.json"));

    install_callbacks(&ui, socket.clone(), config_path);
    let (_tray, show_menu_id, quit_menu_id) = create_tray()?;
    let weak = ui.as_weak();
    let timer = Timer::default();
    timer.start(TimerMode::Repeated, Duration::from_secs(1), move || {
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if show_menu_id == event.id() {
                if let Some(ui) = weak.upgrade() {
                    let _ = ui.window().show();
                }
            } else if quit_menu_id == event.id() {
                let _ = slint::quit_event_loop();
            }
        }
        if let Some(ui) = weak.upgrade() {
            match request(&socket, Request::Status) {
                Ok(response) if response.ok => update_status(&ui, response.session),
                Ok(response) => ui.set_status_text(
                    response
                        .error
                        .unwrap_or_else(|| "Daemon rejected status request".into())
                        .into(),
                ),
                Err(error) => ui.set_status_text(format!("Daemon unavailable: {error}").into()),
            }
        }
    });

    let close_weak = ui.as_weak();
    ui.window().on_close_requested(move || {
        if let Some(ui) = close_weak.upgrade() {
            let _ = ui.window().hide();
            let _ = Notification::new()
                .summary("Focus on Work")
                .body("Still running in the background. The active block remains protected.")
                .show();
        }
        CloseRequestResponse::KeepWindowShown
    });
    ui.run()?;
    Ok(())
}

fn create_tray() -> Result<(TrayIcon, MenuId, MenuId), Box<dyn std::error::Error>> {
    let menu = Menu::new();
    let show = MenuItem::new("Show Focus on Work", true, None);
    let quit = MenuItem::new("Quit", true, None);
    let show_menu_id = show.id().clone();
    let quit_menu_id = quit.id().clone();
    menu.append(&show)?;
    menu.append(&quit)?;
    let icon = Icon::from_rgba(vec![0x38, 0xbd, 0xf8, 0xff], 1, 1)?;
    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("Focus on Work")
        .with_icon(icon)
        .build()?;
    Ok((tray, show_menu_id, quit_menu_id))
}

fn start_request_apps(request: &Request) -> Vec<String> {
    match request {
        Request::Start { apps, .. } => apps.clone(),
        Request::Status => Vec::new(),
    }
}

fn install_callbacks(ui: &AppWindow, socket: PathBuf, config_path: PathBuf) {
    let start_weak = ui.as_weak();
    let start_socket = socket.clone();
    ui.on_start_session(move || {
        if let Some(ui) = start_weak.upgrade() {
            let minutes = ui.get_duration_minutes().max(15);
            let start_request = Request::Start {
                duration: format!("{minutes}m"),
                sites: selected_sites(
                    ui.get_active_profile().as_str(),
                    ui.get_social_media_enabled(),
                    ui.get_videos_enabled(),
                    ui.get_allow_list_mode(),
                    ui.get_custom_sites().as_str(),
                ),
                apps: selected_apps(
                    ui.get_games_enabled(),
                    ui.get_allow_list_mode(),
                    ui.get_custom_apps().as_str(),
                ),
            };
            if let Err(error) = process::validate_app_entries(&start_request_apps(&start_request)) {
                ui.set_status_text(error.into());
                return;
            }
            match request(&start_socket, start_request) {
                Ok(response) if response.ok => {
                    ui.set_status_text("Focus session started".into());
                    let _ = Notification::new()
                        .summary("Focus on Work")
                        .body("Focus session started.")
                        .show();
                    update_status(&ui, response.session);
                }

                Ok(response) => ui.set_status_text(
                    response
                        .error
                        .unwrap_or_else(|| "Could not start session".into())
                        .into(),
                ),
                Err(error) => ui.set_status_text(format!("Daemon unavailable: {error}").into()),
            }
        }
    });

    let status_weak = ui.as_weak();
    let status_socket = socket.clone();
    ui.on_refresh_status(move || {
        if let Some(ui) = status_weak.upgrade() {
            match request(&status_socket, Request::Status) {
                Ok(response) if response.ok => update_status(&ui, response.session),
                Ok(response) => ui.set_status_text(
                    response
                        .error
                        .unwrap_or_else(|| "Status request failed".into())
                        .into(),
                ),
                Err(error) => ui.set_status_text(format!("Daemon unavailable: {error}").into()),
            }
        }
    });

    let export_weak = ui.as_weak();
    let export_path = config_path.clone();
    ui.on_export_config(move || {
        if let Some(ui) = export_weak.upgrade() {
            let config = GuiConfig {
                profile: ui.get_active_profile().to_string(),
                duration_minutes: ui.get_duration_minutes(),
                allow_list_mode: ui.get_allow_list_mode(),
                social_media_enabled: ui.get_social_media_enabled(),
                games_enabled: ui.get_games_enabled(),
                videos_enabled: ui.get_videos_enabled(),
                custom_sites: ui.get_custom_sites().to_string(),
                custom_apps: ui.get_custom_apps().to_string(),
                sites: selected_sites(
                    ui.get_active_profile().as_str(),
                    ui.get_social_media_enabled(),
                    ui.get_videos_enabled(),
                    ui.get_allow_list_mode(),
                    ui.get_custom_sites().as_str(),
                ),
                apps: selected_apps(
                    ui.get_games_enabled(),
                    ui.get_allow_list_mode(),
                    ui.get_custom_apps().as_str(),
                ),
            };
            match serde_json::to_vec_pretty(&config)
                .map_err(std::io::Error::other)
                .and_then(|bytes| fs::write(&export_path, bytes))
            {
                Ok(()) => ui.set_status_text(
                    format!("Configuration saved to {}", export_path.display()).into(),
                ),
                Err(error) => {
                    ui.set_status_text(format!("Could not export configuration: {error}").into())
                }
            }
        }
    });

    let import_weak = ui.as_weak();
    ui.on_import_config(move || {
        if let Some(ui) = import_weak.upgrade() {
            match fs::read(&config_path).and_then(|bytes| {
                serde_json::from_slice::<GuiConfig>(&bytes).map_err(std::io::Error::other)
            }) {
                Ok(config) => {
                    ui.set_active_profile(config.profile.into());
                    ui.set_duration_minutes(config.duration_minutes.clamp(15, 480));
                    ui.set_allow_list_mode(config.allow_list_mode);
                    ui.set_social_media_enabled(config.social_media_enabled);
                    ui.set_games_enabled(config.games_enabled);
                    ui.set_videos_enabled(config.videos_enabled);
                    ui.set_custom_sites(
                        if !config.custom_sites.is_empty() {
                            config.custom_sites
                        } else {
                            config.sites.join(", ")
                        }
                        .into(),
                    );
                    ui.set_custom_apps(
                        if !config.custom_apps.is_empty() {
                            config.custom_apps
                        } else {
                            config.apps.join(", ")
                        }
                        .into(),
                    );
                    ui.set_status_text("Configuration imported".into());
                }
                Err(error) => {
                    ui.set_status_text(format!("Could not import configuration: {error}").into())
                }
            }
        }
    });

    let lock_weak = ui.as_weak();
    ui.on_toggle_lock(move || {
        if let Some(ui) = lock_weak.upgrade() {
            ui.set_lock_enabled(!ui.get_lock_enabled());
            ui.set_status_text(
                if ui.get_lock_enabled() {
                    "Settings locked for 10 minutes"
                } else {
                    "Settings unlocked"
                }
                .into(),
            );
        }
    });
}

fn request(socket: &PathBuf, request: Request) -> Result<Response, String> {
    let mut stream = UnixStream::connect(socket).map_err(|error| error.to_string())?;
    serde_json::to_writer(&mut stream, &request).map_err(|error| error.to_string())?;
    stream.write_all(b"\n").map_err(|error| error.to_string())?;
    let mut line = String::new();
    BufReader::new(stream)
        .read_line(&mut line)
        .map_err(|error| error.to_string())?;
    serde_json::from_str(&line).map_err(|error| error.to_string())
}

fn update_status(ui: &AppWindow, session: Option<Session>) {
    match session {
        Some(session) => {
            let remaining = session.end_unix.saturating_sub(now_unix());
            ui.set_session_active(remaining > 0);
            ui.set_timer_text(format_timer(remaining).into());
            ui.set_status_text("Focus session active".into());
        }
        None => {
            ui.set_session_active(false);
            ui.set_timer_text("--:--:--".into());
            ui.set_status_text("No active session".into());
        }
    }
}

fn selected_sites(
    profile: &str,
    social_media: bool,
    videos: bool,
    custom_only: bool,
    custom: &str,
) -> Vec<String> {
    let mut sites = Vec::new();
    if !custom_only {
        if social_media {
            sites.extend(
                [
                    "facebook.com",
                    "instagram.com",
                    "tiktok.com",
                    "x.com",
                    "twitter.com",
                ]
                .into_iter()
                .map(str::to_owned),
            );
        }
        if videos {
            sites.extend(
                ["youtube.com", "netflix.com", "twitch.tv"]
                    .into_iter()
                    .map(str::to_owned),
            );
        }
        if profile == "Sleep" {
            sites.extend(
                ["reddit.com", "news.ycombinator.com"]
                    .into_iter()
                    .map(str::to_owned),
            );
        }
    }
    sites.extend(split_entries(custom));
    unique(sites)
}

fn selected_apps(games: bool, custom_only: bool, custom: &str) -> Vec<String> {
    let mut apps = Vec::new();
    if games && !custom_only {
        apps.extend(["steam", "lutris", "heroic"].into_iter().map(str::to_owned));
    }
    apps.extend(split_entries(custom));
    unique(apps)
}

fn split_entries(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(str::to_owned)
        .collect()
}

fn unique(entries: Vec<String>) -> Vec<String> {
    let mut result = Vec::new();
    for entry in entries {
        if !result.iter().any(|existing| existing == &entry) {
            result.push(entry);
        }
    }
    result
}

fn format_timer(seconds: u64) -> String {
    format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        (seconds / 60) % 60,
        seconds % 60
    )
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_entries_are_added_and_deduplicated() {
        assert_eq!(
            selected_sites(
                "Work",
                false,
                false,
                false,
                "reddit.com, example.com, reddit.com"
            ),
            vec!["reddit.com", "example.com"]
        );
        assert_eq!(
            selected_apps(true, false, "steam, discord, steam"),
            vec!["steam", "lutris", "heroic", "discord"]
        );
    }

    #[test]
    fn custom_only_excludes_presets() {
        assert_eq!(
            selected_sites("Study", true, true, true, "example.com"),
            vec!["example.com"]
        );
        assert_eq!(selected_apps(true, true, "discord"), vec!["discord"]);
    }
}

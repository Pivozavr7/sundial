use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, sync::Mutex};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub close_to_tray: bool,
    pub volume: f64,
    pub hotkeys_enabled: bool,
    pub dynamic_color: bool,
    pub accent_seed: String,
    pub ui_theme: String,
    pub animated_bg: bool,
    pub wave_speed: f64,
    pub wave_amp: f64,
    pub ui_fps: u32,
    pub window: WindowGeom,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct WindowGeom {
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub maximized: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            close_to_tray: true,
            volume: 1.0,
            hotkeys_enabled: true,
            dynamic_color: true,
            accent_seed: "#7D5700".into(),
            ui_theme: "dark".into(),
            animated_bg: true,
            wave_speed: 0.55,
            wave_amp: 3.0,
            ui_fps: 60,
            window: WindowGeom::default(),
        }
    }
}

pub struct SettingsState(pub Mutex<Settings>);

fn path(app: &AppHandle) -> PathBuf {
    let dir = app.path().app_data_dir().expect("app data dir");
    let _ = fs::create_dir_all(&dir);
    dir.join("settings.json")
}

pub fn init(app: &AppHandle) -> tauri::Result<()> {
    let s: Settings = fs::read_to_string(path(app))
        .ok()
        .and_then(|txt| serde_json::from_str(&txt).ok())
        .unwrap_or_default();
    app.manage(SettingsState(Mutex::new(s)));
    Ok(())
}

pub fn get(app: &AppHandle) -> Settings {
    app.state::<SettingsState>().0.lock().unwrap().clone()
}

pub fn set(app: &AppHandle, s: Settings) {
    *app.state::<SettingsState>().0.lock().unwrap() = s.clone();
    save(app, &s);
}

fn save(app: &AppHandle, s: &Settings) {
    let p = path(app);
    if let Ok(txt) = serde_json::to_string_pretty(s) {
        let tmp = p.with_extension("json.tmp");
        if fs::write(&tmp, txt)
            .and_then(|_| fs::rename(&tmp, &p))
            .is_err()
        {
            eprintln!("settings: не удалось сохранить");
        }
    }
}

pub fn snapshot_window(app: &AppHandle) {
    let Some(w) = app.get_webview_window("main") else {
        return;
    };
    let mut s = get(app);
    if let Ok(p) = w.outer_position() {
        s.window.x = Some(p.x as f64);
        s.window.y = Some(p.y as f64);
    }
    if let Ok(sz) = w.outer_size() {
        s.window.width = Some(sz.width as f64);
        s.window.height = Some(sz.height as f64);
    }
    s.window.maximized = w.is_maximized().unwrap_or(false);
    set(app, s);
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Settings {
    get(&app)
}

#[tauri::command]
pub fn set_settings(app: AppHandle, settings: Settings) -> Result<(), String> {
    set(&app, settings);
    Ok(())
}

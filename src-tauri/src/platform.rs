const CSD_WMS: &[&str] = &[
    "gnome", "unity", "pantheon", "sway", "wayfire", "river", "hyprland", "niri", "mango", "labwc",
    "cosmic", "cage", "phosh",
];
const SSD_WMS: &[&str] = &[
    "kde", "plasma", "xfce", "cinnamon", "mate", "lxde", "lxqt", "budgie", "deepin",
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Desktop {
    Csd,
    Ssd,
    Unknown,
}

pub fn detect() -> Desktop {
    if let Ok(s) = std::env::var("PEAR_DECO") {
        match s.to_lowercase().as_str() {
            "csd" => return Desktop::Csd,
            "ssd" => return Desktop::Ssd,
            _ => {}
        }
    }

    let vars = [
        "XDG_CURRENT_DESKTOP",
        "XDG_SESSION_DESKTOP",
        "DESKTOP_SESSION",
    ];
    let mut names = Vec::new();
    for v in vars {
        if let Ok(s) = std::env::var(v) {
            names.extend(s.to_lowercase().split(':').map(String::from));
        }
    }
    eprintln!("platform: сессия = {names:?}");

    if names.iter().any(|n| CSD_WMS.iter().any(|c| n.contains(c))) {
        return Desktop::Csd;
    }
    if names.iter().any(|n| SSD_WMS.iter().any(|c| n.contains(c))) {
        return Desktop::Ssd;
    }
    Desktop::Unknown
}

#[tauri::command]
pub fn window_deco_mode() -> String {
    match detect() {
        Desktop::Csd => "csd".into(),
        Desktop::Ssd => "ssd".into(),
        Desktop::Unknown => "unknown".into(),
    }
}

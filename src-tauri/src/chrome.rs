use tauri::{Manager, WebviewWindow};

#[tauri::command]
pub fn window_action(window: WebviewWindow, action: String) -> Result<(), String> {
    match action.as_str() {
        "min" => window.minimize().map_err(|e| e.to_string()),
        "max" => if window.is_maximized().unwrap_or(false) {
            window.unmaximize()
        } else {
            window.maximize()
        }
        .map_err(|e| e.to_string()),
        "drag" => window.start_dragging().map_err(|e| e.to_string()),
        "close" => {
            let app = window.app_handle();
            if crate::settings::get(app).close_to_tray {
                crate::settings::snapshot_window(app);
                window.hide().map_err(|e| e.to_string())
            } else {
                window.close().map_err(|e| e.to_string())
            }
        }
        _ => Err(format!("неизвестное действие: {action}")),
    }
}

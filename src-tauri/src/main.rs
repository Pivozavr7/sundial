#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod bridge;
mod chrome;
#[cfg(feature = "hotkeys")]
mod hotkeys;
mod library;
#[cfg(feature = "mpris")]
mod mpris;
mod platform;
mod settings;
mod state;
#[cfg(feature = "tray")]
mod tray;
mod windows;

use tauri::Manager;

fn main() {
    std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    if std::env::var("APPIMAGE").is_ok() {
        std::env::set_var("GST_PLUGIN_SYSTEM_PATH", "/usr/lib/gstreamer-1.0");
    }

    let port = portpicker::pick_unused_port().expect("не нашёл свободный порт");

    tauri::Builder::default()
        .plugin(tauri_plugin_localhost::Builder::new(port).build())
        .setup(move |app| {
            let handle = app.handle().clone();
            settings::init(&handle)?;
            app.manage(state::PlayerState::default());

            #[cfg(dev)]
            let url = tauri::WebviewUrl::App("index.html".into());

            #[cfg(not(dev))]
            let url = {
                let url: tauri::Url = format!("http://localhost:{}", port).parse().unwrap();
                tauri::WebviewUrl::External(url)
            };

            windows::create_main_with_url(&handle, url)?;

            #[cfg(feature = "tray")]
            tray::init(&handle)?;
            #[cfg(feature = "mpris")]
            mpris::init(&handle)?;
            #[cfg(feature = "hotkeys")]
            hotkeys::init(&handle)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if settings::get(window.app_handle()).close_to_tray {
                    api.prevent_close();
                    settings::snapshot_window(window.app_handle());
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            bridge::ytm_search,
            bridge::player_sync,
            chrome::window_action,
            settings::get_settings,
            settings::set_settings,
            platform::window_deco_mode,
            library::pl_list,
            library::pl_save,
            library::pl_add,
            library::pl_delete,
            library::pl_remove_track,
            library::pl_save_source,
            library::ytm_playlist,
        ])
        .run(tauri::generate_context!())
        .expect("sundial: ошибка запуска");
}

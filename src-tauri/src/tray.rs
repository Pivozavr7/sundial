use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

pub struct TrayMenu {
    playpause: MenuItem<tauri::Wry>,
}

pub fn init(app: &AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Показать окно", true, None::<&str>)?;
    let hide = MenuItem::with_id(app, "hide", "Скрыть", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let prev = MenuItem::with_id(app, "prev", "Предыдущий трек", true, None::<&str>)?;
    let playpause = MenuItem::with_id(app, "playpause", "Пауза", true, None::<&str>)?;
    let next = MenuItem::with_id(app, "next", "Следующий трек", true, None::<&str>)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Выход", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[&show, &hide, &sep1, &prev, &playpause, &next, &sep2, &quit],
    )?;
    app.manage(TrayMenu {
        playpause: playpause.clone(),
    });

    let icon = app
        .default_window_icon()
        .cloned()
        .expect("нет иконки — запусти: cargo tauri icon");

    TrayIconBuilder::with_id("main")
        .icon(icon)
        .tooltip("Sundial")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => show_window(app),
            "hide" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.hide();
                }
            }
            "playpause" => crate::bridge::host_cmd(app, "play_pause", None),
            "next" => crate::bridge::host_cmd(app, "next", None),
            "prev" => crate::bridge::host_cmd(app, "prev", None),
            "quit" => {
                crate::settings::snapshot_window(app);
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

fn show_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = (w.show(), w.unminimize(), w.set_focus());
    }
}

fn toggle(app: &AppHandle) {
    let Some(w) = app.get_webview_window("main") else {
        return;
    };
    if w.is_visible().unwrap_or(false) {
        let _ = w.hide();
    } else {
        show_window(app);
    }
}

pub fn update_play_label(app: &AppHandle, playing: bool) {
    if let Some(tm) = app.try_state::<TrayMenu>() {
        let _ = tm.playpause.set_text(if playing {
            "Пауза"
        } else {
            "Играть"
        });
    }
}

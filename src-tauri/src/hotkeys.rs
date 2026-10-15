use tauri::AppHandle;
use tauri_plugin_global_shortcut::{Builder, GlobalShortcutExt, Shortcut, ShortcutState};

const BINDINGS: &[(&str, &str)] = &[
    ("CmdOrCtrl+Alt+P", "play_pause"),
    ("CmdOrCtrl+Alt+Shift+Right", "next"),
    ("CmdOrCtrl+Alt+Shift+Left", "prev"),
];

pub fn init(app: &AppHandle) -> tauri::Result<()> {
    let binds: Vec<(Shortcut, &'static str)> = BINDINGS
        .iter()
        .filter_map(|(k, a)| match k.parse::<Shortcut>() {
            Ok(s) => Some((s, *a)),
            Err(e) => {
                eprintln!("hotkeys: разбор «{k}»: {e}");
                None
            }
        })
        .collect();

    app.plugin(
        Builder::new()
            .with_handler(move |app: &AppHandle, shortcut: &Shortcut, event| {
                if event.state() == ShortcutState::Pressed {
                    if let Some((_, act)) = binds.iter().find(|(s, _)| s == shortcut) {
                        crate::bridge::host_cmd(app, act, None);
                    }
                }
            })
            .build(),
    )?;

    let gs = app.global_shortcut();
    for (k, _) in BINDINGS {
        if let Err(e) = gs.register(*k) {
            eprintln!("hotkeys: «{k}» занят системой ({e})");
        }
    }
    Ok(())
}

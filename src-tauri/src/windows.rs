use tauri::{AppHandle, WebviewUrl, WebviewWindowBuilder};

pub fn create_main(app: &AppHandle) -> tauri::Result<()> {
    let s = crate::settings::get(app);
    let csd = !matches!(crate::platform::detect(), crate::platform::Desktop::Ssd);

    let mut b = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .title("Sundial")
        .decorations(csd)
        .min_inner_size(760.0, 480.0)
        .inner_size(
            s.window.width.unwrap_or(1100.0),
            s.window.height.unwrap_or(720.0),
        );
    if let (Some(x), Some(y)) = (s.window.x, s.window.y) {
        b = b.position(x, y);
    }
    let w = b.build()?;
    if s.window.maximized {
        let _ = w.maximize();
    }
    Ok(())
}

use serde::{Deserialize, Serialize};
use std::fs;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlTrack {
    pub video_id: String,
    pub title: String,
    pub subtitle: String,
    pub thumb: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    pub name: String,
    pub tracks: Vec<PlTrack>,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Library {
    playlists: Vec<Playlist>,
}

fn path(app: &AppHandle) -> std::path::PathBuf {
    let dir = app.path().app_data_dir().expect("app data dir");
    let _ = fs::create_dir_all(&dir);
    dir.join("library.json")
}

fn load(app: &AppHandle) -> Library {
    fs::read_to_string(path(app))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn persist(app: &AppHandle, lib: &Library) {
    let p = path(app);
    if let Ok(t) = serde_json::to_string_pretty(lib) {
        let tmp = p.with_extension("json.tmp");
        let _ = fs::write(&tmp, t).and_then(|_| fs::rename(&tmp, &p));
    }
}

#[tauri::command]
pub fn pl_list(app: AppHandle) -> Vec<Playlist> {
    load(&app).playlists
}

#[tauri::command]
pub fn pl_save(app: AppHandle, name: String, tracks: Vec<PlTrack>) -> Result<(), String> {
    pl_save_src(app, name, tracks, None)
}

#[tauri::command]
pub fn pl_save_source(
    app: AppHandle,
    name: String,
    tracks: Vec<PlTrack>,
    source: Option<String>,
) -> Result<(), String> {
    pl_save_src(app, name, tracks, source)
}

fn pl_save_src(
    app: AppHandle,
    name: String,
    tracks: Vec<PlTrack>,
    source: Option<String>,
) -> Result<(), String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("пустое имя".into());
    }
    let mut lib = load(&app);
    match lib.playlists.iter_mut().find(|p| p.name == name) {
        Some(p) => {
            p.tracks = tracks;
            p.source = source;
        }
        None => lib.playlists.push(Playlist {
            name: name.clone(),
            tracks,
            source,
        }),
    }
    persist(&app, &lib);
    Ok(())
}

#[tauri::command]
pub fn pl_add(app: AppHandle, name: String, track: PlTrack) -> Result<(), String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("нет названия".into());
    }
    let mut lib = load(&app);
    match lib.playlists.iter_mut().find(|p| p.name == name) {
        Some(p) => p.tracks.push(track),
        None => lib.playlists.push(Playlist {
            name,
            tracks: vec![track],
            source: None,
        }),
    }
    persist(&app, &lib);
    Ok(())
}

#[tauri::command]
pub fn pl_delete(app: AppHandle, name: String) -> Result<(), String> {
    let mut lib = load(&app);
    lib.playlists.retain(|p| p.name != name);
    persist(&app, &lib);
    Ok(())
}

#[tauri::command]
pub fn pl_remove_track(app: AppHandle, name: String, index: usize) -> Result<(), String> {
    let mut lib = load(&app);
    if let Some(p) = lib.playlists.iter_mut().find(|p| p.name == name) {
        if index < p.tracks.len() {
            p.tracks.remove(index);
        }
        persist(&app, &lib);
        Ok(())
    } else {
        Err("такого плейлиста нет".into())
    }
}

const INNERTUBE_KEY: &str = "AIzaSyC9XL3ZjWddXya6X74dJoCTL-WEYFDNX30";
const CLIENT_VERSION: &str = "1.20240403.01.00";
const UA: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36";

fn runs_text(v: &serde_json::Value) -> String {
    let mut s = String::new();
    if let Some(runs) = v.pointer("/text/runs").and_then(|r| r.as_array()) {
        for r in runs {
            if let Some(t) = r.get("text").and_then(|t| t.as_str()) {
                s.push_str(t);
            }
        }
    }
    s.trim().to_string()
}

fn collect_tracks(v: &serde_json::Value, out: &mut Vec<PlTrack>) {
    match v {
        serde_json::Value::Array(a) => a.iter().for_each(|x| collect_tracks(x, out)),
        serde_json::Value::Object(m) => {
            if let Some(r) = m.get("musicResponsiveListItemRenderer") {
                let cols = r.get("flexColumns").and_then(|c| c.as_array());
                let title = cols
                    .and_then(|c| c.first())
                    .map(|c| {
                        c.pointer("/musicResponsiveListItemFlexColumnRenderer")
                            .map(runs_text)
                            .unwrap_or_default()
                    })
                    .unwrap_or_default();
                let subtitle = cols
                    .and_then(|c| c.get(1))
                    .map(|c| {
                        c.pointer("/musicResponsiveListItemFlexColumnRenderer")
                            .map(runs_text)
                            .unwrap_or_default()
                    })
                    .unwrap_or_default();
                let video_id = r
                    .pointer("/playlistItemData/videoId")
                    .and_then(|x| x.as_str())
                    .map(String::from)
                    .or_else(|| {
                        r.pointer("/flexColumns/0/musicResponsiveListItemFlexColumnRenderer/text/runs/0/navigationEndpoint/watchEndpoint/videoId")
                            .and_then(|x| x.as_str())
                            .map(String::from)
                    });
                let thumb = r
                    .pointer("/thumbnail/musicThumbnailRenderer/thumbnail/thumbnails")
                    .and_then(|t| t.as_array())
                    .and_then(|t| t.last())
                    .and_then(|t| t.get("url"))
                    .and_then(|u| u.as_str())
                    .map(String::from);
                if let Some(video_id) = video_id {
                    if !title.is_empty() {
                        out.push(PlTrack {
                            video_id,
                            title,
                            subtitle,
                            thumb,
                        });
                    }
                }
            }
            m.values().for_each(|x| collect_tracks(x, out));
        }
        _ => {}
    }
}

#[tauri::command]
pub async fn ytm_playlist(link_or_id: String) -> Result<(String, Vec<PlTrack>), String> {
    let id = {
        let s = link_or_id.trim();
        if let Some(pos) = s.find("list=") {
            let rest = &s[pos + 5..];
            let end = rest.find('&').unwrap_or(rest.len());
            rest[..end].to_string()
        } else if let Some(stripped) = s.strip_prefix("VL") {
            stripped.to_string()
        } else {
            s.to_string()
        }
    };
    if id.is_empty() {
        return Err("неверный ID".into());
    }

    let cli = reqwest::Client::builder()
        .user_agent(UA)
        .build()
        .map_err(|e| e.to_string())?;

    let resp = cli
        .post(format!(
            "https://music.youtube.com/youtubei/v1/browse?prettyPrint=false&key={INNERTUBE_KEY}"
        ))
        .json(&serde_json::json!({
            "context": { "client": {
                "clientName": "WEB_REMIX",
                "clientVersion": CLIENT_VERSION,
                "hl": "ru", "gl": "RU"
            }},
            "browseId": format!("VL{id}")
        }))
        .send()
        .await
        .map_err(|e| format!("сеть: {e}"))?;

    let data: serde_json::Value = resp.json().await.map_err(|e| format!("json: {e}"))?;

    let name = data
        .pointer("/header/musicDetailHeaderRenderer/title/runs/0/text")
        .and_then(|t| t.as_str())
        .unwrap_or("Импорт из YTM")
        .to_string();

    let mut tracks = Vec::new();
    collect_tracks(&data, &mut tracks);
    let mut seen = std::collections::HashSet::new();
    tracks.retain(|t| seen.insert(t.video_id.clone()));

    if tracks.is_empty() {
        return Err("плейлист пуст или скрыт".into());
    }
    Ok((name, tracks))
}

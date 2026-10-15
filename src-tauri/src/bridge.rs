use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};

use crate::state::{PlayerState, Track};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchItem {
    pub video_id: String,
    pub title: String,
    pub subtitle: String,
    pub thumb: Option<String>,
}

const INNERTUBE_KEY: &str = "AIzaSyC9XL3ZjWddXya6X74dJoCTL-WEYFDNX30";
const CLIENT_VERSION: &str = "1.20240403.01.00";
const UA: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36";

#[tauri::command]
pub async fn ytm_search(query: String) -> Result<Vec<SearchItem>, String> {
    let cli = reqwest::Client::builder()
        .user_agent(UA)
        .build()
        .map_err(|e| e.to_string())?;

    let resp = cli
        .post(format!(
            "https://music.youtube.com/youtubei/v1/search?prettyPrint=false&key={INNERTUBE_KEY}"
        ))
        .json(&json!({
            "context": { "client": {
                "clientName": "WEB_REMIX",
                "clientVersion": CLIENT_VERSION,
                "hl": "ru",
                "gl": "RU"
            }},
            "query": query
        }))
        .send()
        .await
        .map_err(|e| format!("сеть: {e}"))?;

    let data: serde_json::Value = resp.json().await.map_err(|e| format!("json: {e}"))?;
    let mut out = Vec::new();
    collect_items(&data, &mut out);

    let mut seen = std::collections::HashSet::new();
    out.retain(|i| seen.insert(i.video_id.clone()));
    out.truncate(40);
    Ok(out)
}

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

fn collect_items(v: &serde_json::Value, out: &mut Vec<SearchItem>) {
    match v {
        serde_json::Value::Array(a) => a.iter().for_each(|x| collect_items(x, out)),
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
                        r.pointer("/navigationEndpoint/watchEndpoint/videoId")
                            .and_then(|x| x.as_str())
                            .map(String::from)
                    })
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
                        out.push(SearchItem {
                            video_id,
                            title,
                            subtitle,
                            thumb,
                        });
                    }
                }
            }
            m.values().for_each(|x| collect_items(x, out));
        }
        _ => {}
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncPayload {
    pub track: Option<Track>,
    pub playing: bool,
    pub pos: u32,
}

#[tauri::command]
pub fn player_sync(app: AppHandle, payload: SyncPayload) {
    let st = app.state::<PlayerState>();
    *st.track.lock().unwrap() = payload.track;
    *st.playing.lock().unwrap() = payload.playing;
    *st.pos_secs.lock().unwrap() = payload.pos;
    drop(st);

    #[cfg(feature = "tray")]
    crate::tray::update_play_label(&app, payload.playing);
    #[cfg(feature = "mpris")]
    {
        crate::mpris::push_track(&app);
        crate::mpris::push_status(&app);
    }
}

pub fn host_cmd(app: &AppHandle, action: &str, value: Option<f64>) {
    let _ = app.emit_to("main", "host", json!({ "action": action, "value": value }));
}

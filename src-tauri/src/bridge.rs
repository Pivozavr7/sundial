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

#[tauri::command]
pub async fn ytm_search(query: String) -> Result<Vec<SearchItem>, String> {
    let q = query.trim();
    if q.is_empty() { return Ok(vec![]); }

    let out = run_ytdlp(vec![
        format!("ytsearch30:{q}"),
        "--dump-json".into(),
        "--flat-playlist".into(),
        "--no-warnings".into(),
        "--ignore-errors".into(),
    ]).await?;

    let stdout = String::from_utf8_lossy(&out.stdout);
    if stdout.is_empty() { return Err("yt-dlp: no results".into()); }

    let mut items = parse_flat(&stdout, q);
    items.sort_by(|a, b| b.score.cmp(&a.score));
    let items: Vec<SearchItem> = items.into_iter().map(|s| s.item).collect();
    Ok(items)
}

async fn run_ytdlp(args: Vec<String>) -> Result<std::process::Output, String> {
    tokio::process::Command::new("yt-dlp")
        .args(&args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .await
        .map_err(|e| format!("yt-dlp: {e}"))
}

struct Scored { item: SearchItem, score: i64 }

fn parse_flat(stdout: &str, query: &str) -> Vec<Scored> {
    let q = query.to_lowercase();
    let qlen = q.split_whitespace().count().max(1);
    let mut out = Vec::new();

    for line in stdout.lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else { continue };
        if !v["url"].as_str().is_some_and(|u| u.contains("/watch")) { continue; }
        let (Some(id), Some(title)) = (v["id"].as_str(), v["title"].as_str()) else { continue };
        let dur = v["duration"].as_f64().unwrap_or(0.0);
        if !(0.5..=1500.0).contains(&dur) { continue; }
        let channel = v["channel"].as_str().or_else(|| v["uploader"].as_str()).unwrap_or("");
        if channel.is_empty() { continue; }

        let t = title.to_lowercase();
        let c = channel.to_lowercase();
        let mut score = 0i64;

        // official audio uploads win
        if channel.ends_with("- Topic") { score += 60; }
        // artist name in the channel (queries are usually "artist song")
        if q.split_whitespace().any(|w| w.len() > 2 && c.contains(w)) { score += 25; }
        // query words present in the title
        let hits = q.split_whitespace()
            .filter(|w| w.len() > 2 && t.contains(w))
            .count();
        score += (hits * 15) as i64;
        // strong signal: all words of the query in the title
        if hits >= qlen { score += 20; }
        // verified channel bonus
        if v["channel_is_verified"].as_bool().unwrap_or(false) { score += 15; }
        // music-length videos preferred over 15-min clips
        if (60.0..=480.0).contains(&dur) { score += 10; }
        // mild junk penalty — demote, never delete
        for j in ["full album", "reaction", "review", "1 hour", "10 hours", "mix -", "episode"] {
            if t.contains(j) { score -= 30; break; }
        }
        // view count as a weak tiebreaker (log-ish buckets)
        if let Some(vc) = v["view_count"].as_u64() {
            score += (vc as f64).log10() as i64;
        }

        out.push(Scored {
            item: SearchItem {
                video_id: id.to_string(),
                title: title.to_string(),
                subtitle: if let Some(stripped) = channel.strip_suffix(" - Topic") {
                    stripped.to_string()
                } else { channel.to_string() },
                thumb: last_thumb(&v),
            },
            score,
        });
    }
    out
}

fn last_thumb(v: &serde_json::Value) -> Option<String> {
    v["thumbnails"].as_array().and_then(|t| t.last())
        .and_then(|t| t["url"].as_str()).map(String::from)
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

use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct Track {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub art_url: Option<String>,
    pub duration_secs: u32,
}

#[derive(Default)]
pub struct PlayerState {
    pub track: Mutex<Option<Track>>,
    pub playing: Mutex<bool>,
    pub pos_secs: Mutex<u32>,
    pub volume: Mutex<f64>,
}

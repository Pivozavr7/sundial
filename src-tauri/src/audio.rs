use std::io::Cursor;
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rodio::{OutputStream, Sink, Source};
use tauri::{AppHandle, Emitter, Manager};

const STOPPED: u8 = 0;
const PLAYING: u8 = 1;
const PAUSED: u8 = 2;

#[derive(Default)]
pub struct AudioHandle {
    state: Arc<AtomicU8>,
    pos_ms: Arc<AtomicU64>,
    dur_ms: Arc<AtomicU64>,
    vol: Arc<Mutex<f32>>,
    sink: Arc<Mutex<Option<Arc<Sink>>>>,
    gen: Arc<AtomicU64>,
    cur_video: Mutex<String>,
    cache: Arc<Mutex<Option<(String, Arc<Vec<u8>>)>>>,
}

fn fetch_audio_sync(video_id: &str, cache_dir: &std::path::Path, bypass: bool) -> Result<std::path::PathBuf, String> {
    let target = cache_dir.join(format!("{video_id}.m4a"));
    if target.exists() {
        return Ok(target); // cached from previous play
    }
    let mut args = vec![
        format!("https://www.youtube.com/watch?v={video_id}"),
        "-f".into(), "bestaudio/best".into(),
        "-x".into(),
        "--audio-format".into(), "m4a".into(),
        "-o".into(), format!("{}/%(id)s.%(ext)s", cache_dir.display()),
        "--no-warnings".into(),
        "--no-playlist".into(),
    ];
    if bypass {
        args.push("--force-ipv4".into());
        args.push("--geo-bypass".into());
    }
    let st = std::process::Command::new("yt-dlp")
        .args(&args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map_err(|e| format!("yt-dlp: {e}"))?;
    if !st.success() || !target.exists() {
        return Err("yt-dlp: download failed".into());
    }
    Ok(target)
}

fn spawn_play(app: &AppHandle, video_id: &str, skip_ms: u64) {
    let h = app.state::<AudioHandle>();
    let my_gen = h.gen.fetch_add(1, Ordering::Relaxed) + 1;

    if let Some(s) = h.sink.lock().unwrap().take() {
        s.stop();
    }
    h.state.store(STOPPED, Ordering::Relaxed);
    h.pos_ms.store(skip_ms, Ordering::Relaxed);
    h.dur_ms.store(0, Ordering::Relaxed);
    *h.cur_video.lock().unwrap() = video_id.to_string();

    let (state, pos_ms, dur_ms, vol, sink_slot, gen, cache) = (
        h.state.clone(), h.pos_ms.clone(), h.dur_ms.clone(),
        h.vol.clone(), h.sink.clone(), h.gen.clone(), h.cache.clone(),
    );
    let app2 = app.clone();
    let vid = video_id.to_string();

    std::thread::spawn(move || {
        if let Err(e) = play_thread(&app2, &vid, skip_ms, state, pos_ms, dur_ms, vol, sink_slot, gen, my_gen, cache) {
            eprintln!("[audio] {e}");
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn play_thread(
    app: &AppHandle,
    video_id: &str,
    skip_ms: u64,
    state: Arc<AtomicU8>,
    pos_ms: Arc<AtomicU64>,
    dur_ms: Arc<AtomicU64>,
    vol: Arc<Mutex<f32>>,
    sink_slot: Arc<Mutex<Option<Arc<Sink>>>>,
    gen: Arc<AtomicU64>,
    my_gen: u64,
    cache: Arc<Mutex<Option<(String, Arc<Vec<u8>>)>>>,
) -> Result<(), String> {
    let cached = {
        let c = cache.lock().unwrap();
        c.as_ref().filter(|(id, _)| id == video_id).map(|(_, b)| b.clone())
    };

    let cache_dir = std::path::PathBuf::from(
        dirs::cache_dir().unwrap_or_else(|| std::path::PathBuf::from("/tmp")).join("sundial")
    );
    let _ = std::fs::create_dir_all(&cache_dir);

    let path = fetch_audio_sync(video_id, &cache_dir, false)
        .or_else(|_| fetch_audio_sync(video_id, &cache_dir, true))?;

    if gen.load(Ordering::Relaxed) != my_gen {
        return Ok(());
    }

    let file = std::fs::File::open(&path).map_err(|e| format!("open: {e}"))?;
    let (_stream, handle) = OutputStream::try_default().map_err(|e| format!("device: {e}"))?;
    let sink = Arc::new(Sink::try_new(&handle).map_err(|e| format!("sink: {e}"))?);
    sink.set_volume(*vol.lock().unwrap());

    let src = rodio::Decoder::new(std::io::BufReader::new(file))
        .map_err(|e| format!("decode: {e}"))?;

    if gen.load(Ordering::Relaxed) != my_gen {
        return Ok(());
    }

    let (_stream, handle) = OutputStream::try_default().map_err(|e| format!("device: {e}"))?;
    let sink = Arc::new(Sink::try_new(&handle).map_err(|e| format!("sink: {e}"))?);
    sink.set_volume(*vol.lock().unwrap());

    let data = (*bytes).clone();
    let src = rodio::Decoder::new(Cursor::new(data)).map_err(|e| format!("decode: {e}"))?;
    if let Some(d) = src.total_duration() {
        dur_ms.store(d.as_millis() as u64, Ordering::Relaxed);
    }
    let src = src.skip_duration(Duration::from_millis(skip_ms));

    state.store(PLAYING, Ordering::Relaxed);
    pos_ms.store(skip_ms, Ordering::Relaxed);
    *sink_slot.lock().unwrap() = Some(sink.clone());
    sink.append(PosTracker { src, pos_ms: pos_ms.clone(), count: 0, offset_ms: skip_ms });
    sink.sleep_until_end();

    *sink_slot.lock().unwrap() = None;
    if gen.load(Ordering::Relaxed) == my_gen {
        state.store(STOPPED, Ordering::Relaxed);
        pos_ms.store(0, Ordering::Relaxed);
        let _ = app.emit_to("main", "track-ended", ());
    }
    Ok(())
}

struct PosTracker<S> {
    src: S,
    pos_ms: Arc<AtomicU64>,
    count: u64,
    offset_ms: u64,
}

impl<S> Iterator for PosTracker<S>
where
    S: rodio::Source,
    S::Item: rodio::Sample,
{
    type Item = S::Item;
    fn next(&mut self) -> Option<Self::Item> {
        let item = self.src.next()?;
        self.count += 1;
        if self.count % 4096 == 0 {
            let rate = self.src.sample_rate() as u64;
            let ch = self.src.channels() as u64;
            if rate > 0 && ch > 0 {
                let p = self.offset_ms + self.count * 1000 / (rate * ch);
                self.pos_ms.store(p, Ordering::Relaxed);
            }
        }
        Some(item)
    }
}

impl<S> rodio::Source for PosTracker<S>
where
    S: rodio::Source,
    S::Item: rodio::Sample,
{
    fn current_frame_len(&self) -> Option<usize> { self.src.current_frame_len() }
    fn sample_rate(&self) -> u32 { self.src.sample_rate() }
    fn channels(&self) -> u16 { self.src.channels() }
    fn total_duration(&self) -> Option<Duration> { self.src.total_duration() }
}

#[tauri::command]
pub async fn audio_play(app: AppHandle, video_id: String) {
    spawn_play(&app, &video_id, 0);
}

#[tauri::command]
pub async fn audio_seek(app: AppHandle, sec: f64) -> Result<(), String> {
    let h = app.state::<AudioHandle>();
    let vid = h.cur_video.lock().unwrap().clone();
    drop(h);
    if vid.is_empty() {
        return Err("nothing playing".into());
    }
    let ms = (sec.max(0.0) * 1000.0) as u64;
    spawn_play(&app, &vid, ms);
    Ok(())
}

#[tauri::command]
pub async fn audio_pause(app: AppHandle) {
    let h = app.state::<AudioHandle>();
    let guard = h.sink.lock().unwrap();
    if let Some(s) = guard.as_ref() {
        s.pause();
        h.state.store(PAUSED, Ordering::Relaxed);
    }
}

#[tauri::command]
pub async fn audio_resume(app: AppHandle) {
    let h = app.state::<AudioHandle>();
    let guard = h.sink.lock().unwrap();
    if let Some(s) = guard.as_ref() {
        s.play();
        h.state.store(PLAYING, Ordering::Relaxed);
    }
}

#[tauri::command]
pub async fn audio_stop(app: AppHandle) {
    let h = app.state::<AudioHandle>();
    h.gen.fetch_add(1, Ordering::Relaxed);
    if let Some(s) = h.sink.lock().unwrap().take() {
        s.stop();
    }
    h.state.store(STOPPED, Ordering::Relaxed);
    h.pos_ms.store(0, Ordering::Relaxed);
    *h.cur_video.lock().unwrap() = String::new();
}

#[tauri::command]
pub async fn audio_volume(app: AppHandle, v: f32) {
    let h = app.state::<AudioHandle>();
    let v = v.clamp(0.0, 1.0);
    *h.vol.lock().unwrap() = v;
    let guard = h.sink.lock().unwrap();
    if let Some(s) = guard.as_ref() {
        s.set_volume(v);
    }
}

#[tauri::command]
pub async fn audio_pos(app: AppHandle) -> u64 {
    app.state::<AudioHandle>().pos_ms.load(Ordering::Relaxed)
}

#[tauri::command]
pub async fn audio_dur(app: AppHandle) -> u64 {
    app.state::<AudioHandle>().dur_ms.load(Ordering::Relaxed)
}

use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rodio::{OutputStream, Sink, Source};
use tauri::{AppHandle, Emitter, Manager};

const STOPPED: u8 = 0;
const PLAYING: u8 = 1;
const PAUSED: u8 = 2;

pub struct AudioHandle {
    state: Arc<AtomicU8>,
    pos_ms: Arc<AtomicU64>,
    dur_ms: Arc<AtomicU64>,
    vol: Arc<Mutex<f32>>,
    sink: Arc<Mutex<Option<Arc<Sink>>>>,
    gen: Arc<AtomicU64>,
    cur_video: Mutex<String>,
}

impl Default for AudioHandle {
    fn default() -> Self {
        Self {
            state: Arc::new(AtomicU8::new(STOPPED)),
            pos_ms: Arc::new(AtomicU64::new(0)),
            dur_ms: Arc::new(AtomicU64::new(0)),
            // громкость НЕ 0: derive(Default) давал 0.0 и первый трек молчал
            vol: Arc::new(Mutex::new(1.0)),
            sink: Arc::new(Mutex::new(None)),
            gen: Arc::new(AtomicU64::new(0)),
            cur_video: Mutex::new(String::new()),
        }
    }
}

/// Накатить громкость из настроек — вызывать после manage(AudioHandle).
pub fn init_volume(app: &AppHandle) {
    let saved = crate::settings::get(app).volume;
    let h = app.state::<AudioHandle>();
    *h.vol.lock().unwrap() = (saved as f32).clamp(0.0, 1.0);
}

fn fetch_audio_sync(
    video_id: &str,
    cache_dir: &std::path::Path,
    bypass: bool,
) -> Result<std::path::PathBuf, String> {
    let target = cache_dir.join(format!("{video_id}.m4a"));
    if target.exists() {
        let size = std::fs::metadata(&target).map(|m| m.len()).unwrap_or(0);
        if size >= 10 * 1024 {
            eprintln!("[audio] кэш валиден: {video_id}.m4a ({size} байт)");
            return Ok(target); // cached from previous play
        }
        // недокачанный/пустой файл от прошлого неудачного прогона — иначе
        // `target.exists()` принимал бы его вечно и декодер молча падал бы
        eprintln!("[audio] кэш битый ({size} байт) — стираю и качаю заново");
        let _ = std::fs::remove_file(&target);
    }
    eprintln!("[audio] yt-dlp: качаю {video_id} (bypass={bypass})");
    let mut args = vec![
        format!("https://www.youtube.com/watch?v={video_id}"),
        "-f".into(),
        "bestaudio/best".into(),
        "-x".into(),
        "--audio-format".into(),
        "m4a".into(),
        "-o".into(),
        format!("{}/%(id)s.%(ext)s", cache_dir.display()),
        "--no-warnings".into(),
        "--no-playlist".into(),
        "--socket-timeout".into(),
        "15".into(),
        "--retries".into(),
        "3".into(),
    ];
    if bypass {
        args.push("--force-ipv4".into());
        args.push("--geo-bypass".into());
    }
    let out = std::process::Command::new("yt-dlp")
        .args(&args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                "yt-dlp: не найден — установи его (pip install yt-dlp или пакет дистрибутива)".into()
            } else {
                format!("yt-dlp: {e}")
            }
        })?;
    if !out.status.success() || !target.exists() {
        // хвост stderr — там yt-dlp пишет причину (бот-чек, сеть, 403 и т.п.)
        let err = String::from_utf8_lossy(&out.stderr);
        let tail: String = err
            .lines()
            .rev()
            .take(4)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join(" | ");
        let tail = tail.trim();
        if tail.is_empty() {
            return Err("yt-dlp: не удалось скачать аудио".into());
        }
        return Err(format!("yt-dlp: {tail}"));
    }
    let size = std::fs::metadata(&target).map(|m| m.len()).unwrap_or(0);
    eprintln!("[audio] yt-dlp: скачано {video_id}.m4a ({size} байт)");
    Ok(target)
}

fn spawn_play(app: &AppHandle, video_id: &str, skip_ms: u64) {
    eprintln!("[audio] ▶ play {video_id} (skip {skip_ms} ms)");
    let h = app.state::<AudioHandle>();
    let my_gen = h.gen.fetch_add(1, Ordering::Relaxed) + 1;

    if let Some(s) = h.sink.lock().unwrap().take() {
        s.stop();
    }
    h.state.store(STOPPED, Ordering::Relaxed);
    h.pos_ms.store(skip_ms, Ordering::Relaxed);
    h.dur_ms.store(0, Ordering::Relaxed);
    *h.cur_video.lock().unwrap() = video_id.to_string();

    let (state, pos_ms, dur_ms, vol, sink_slot, gen) = (
        h.state.clone(),
        h.pos_ms.clone(),
        h.dur_ms.clone(),
        h.vol.clone(),
        h.sink.clone(),
        h.gen.clone(),
    );
    let app2 = app.clone();
    let vid = video_id.to_string();

    std::thread::spawn(move || {
        if let Err(e) =
            play_thread(&app2, &vid, skip_ms, state, pos_ms, dur_ms, vol, sink_slot, gen, my_gen)
        {
            eprintln!("[audio] {e}");
            // продублируем ошибку в интерфейс — там её видно без терминала
            let _ = app2.emit_to("main", "audio-error", e);
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
) -> Result<(), String> {
    let cache_dir = dirs::cache_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
        .join("sundial");
    let _ = std::fs::create_dir_all(&cache_dir);

    let path = fetch_audio_sync(video_id, &cache_dir, false)
        .or_else(|_| fetch_audio_sync(video_id, &cache_dir, true))?;

    if gen.load(Ordering::Relaxed) != my_gen {
        return Ok(()); // трек уже сменили, пока качали
    }

    // Один поток вывода на весь трек: stream должен жить дольше sink.
    let (_stream, handle) = OutputStream::try_default().map_err(|e| format!("device: {e}"))?;
    eprintln!("[audio] устройство вывода ок");
    let sink = Arc::new(Sink::try_new(&handle).map_err(|e| format!("sink: {e}"))?);
    let v = *vol.lock().unwrap();
    sink.set_volume(v);
    eprintln!("[audio] sink ок, громкость = {v}");

    let file = std::fs::File::open(&path).map_err(|e| format!("open: {e}"))?;
    let src = rodio::Decoder::new(std::io::BufReader::new(file))
        .map_err(|e| format!("decode: {e}"))?;

    if gen.load(Ordering::Relaxed) != my_gen {
        return Ok(());
    }

    match src.total_duration() {
        Some(d) => {
            dur_ms.store(d.as_millis() as u64, Ordering::Relaxed);
            eprintln!(
                "[audio] декодировано: {} Hz, {} ch, ~{} мс",
                src.sample_rate(),
                src.channels(),
                d.as_millis()
            );
        }
        None => eprintln!(
            "[audio] декодировано: {} Hz, {} ch, длительность неизвестна",
            src.sample_rate(),
            src.channels()
        ),
    }
    let src = src.skip_duration(Duration::from_millis(skip_ms));

    // Пауза могла быть нажата, пока шла загрузка — тогда не стартуем звук.
    let paused_while_loading = state.load(Ordering::Relaxed) == PAUSED;
    state.store(PLAYING, Ordering::Relaxed);
    pos_ms.store(skip_ms, Ordering::Relaxed);
    *sink_slot.lock().unwrap() = Some(sink.clone());
    if paused_while_loading {
        sink.pause();
        state.store(PAUSED, Ordering::Relaxed);
    }
    sink.append(PosTracker { src, pos_ms: pos_ms.clone(), count: 0, offset_ms: skip_ms });
    eprintln!("[audio] играю...");
    sink.sleep_until_end();

    // из слота убираем только свой sink: если юзер быстро переключил трек,
    // новый поток уже мог положить туда свой sink — затирать его нельзя
    {
        let mut slot = sink_slot.lock().unwrap();
        if slot.as_ref().is_some_and(|s| Arc::ptr_eq(s, &sink)) {
            *slot = None;
        }
    }
    let natural_end = gen.load(Ordering::Relaxed) == my_gen;
    if natural_end {
        state.store(STOPPED, Ordering::Relaxed);
        pos_ms.store(0, Ordering::Relaxed);
        let _ = app.emit_to("main", "track-ended", ());
    }
    eprintln!(
        "[audio] воспроизведение завершилось: {}",
        if natural_end { "трек доиграл до конца" } else { "сменили трек или остановили" }
    );
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
    fn current_frame_len(&self) -> Option<usize> {
        self.src.current_frame_len()
    }
    fn sample_rate(&self) -> u32 {
        self.src.sample_rate()
    }
    fn channels(&self) -> u16 {
        self.src.channels()
    }
    fn total_duration(&self) -> Option<Duration> {
        self.src.total_duration()
    }
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
// okaki melstro pido - guva ras ebabibib tutut pipipi puupupuuuuuu idor ass
#[tauri::command]
pub async fn audio_pause(app: AppHandle) {
    let h = app.state::<AudioHandle>();
    let guard = h.sink.lock().unwrap();
    if let Some(s) = guard.as_ref() {
        s.pause();
        h.state.store(PAUSED, Ordering::Relaxed);
    } else if !h.cur_video.lock().unwrap().is_empty() {
        // sink ещё не поднят (идёт загрузка) — запоминаем пожелание
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
    } else if h.state.load(Ordering::Relaxed) == PAUSED {
        // отмена паузы, нажатой во время загрузки
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

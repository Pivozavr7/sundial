use mpris_server::{
    zbus, LocalPlayerInterface, LocalRootInterface, LocalServer, LoopStatus, Metadata,
    PlaybackRate, PlaybackStatus, Time, TrackId, Volume,
};
use tauri::{AppHandle, Manager};

use crate::state::PlayerState;

fn playing_of(app: &AppHandle) -> bool {
    *app.state::<PlayerState>().playing.lock().unwrap()
}

fn pos_secs(app: &AppHandle) -> u32 {
    *app.state::<PlayerState>().pos_secs.lock().unwrap()
}

fn track_of(app: &AppHandle) -> Option<crate::state::Track> {
    app.state::<PlayerState>().track.lock().unwrap().clone()
}

fn to_time(secs: u64) -> Time {
    Time::from_micros((secs * 1_000_000) as i64)
}

fn time_to_secs(t: Time) -> f64 {
    t.as_micros() as f64 / 1e6
}

fn build_metadata(app: &AppHandle) -> Metadata {
    let mut b = Metadata::builder()
        .trackid(TrackId::try_from("/dev/sunder/sundial/current").expect("track id"));
    if let Some(t) = track_of(app) {
        let artists: Vec<String> = t
            .artist
            .split(&[',', ';'][..])
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();
        b = b
            .title(t.title)
            .artist(artists)
            .album(t.album)
            .length(to_time(t.duration_secs as u64));
        if let Some(url) = t.art_url {
            b = b.art_url(url);
        }
    }
    b.build()
}

#[derive(Clone)]
pub struct PearMpris {
    app: AppHandle,
}

impl LocalRootInterface for PearMpris {
    async fn identity(&self) -> zbus::fdo::Result<String> {
        Ok("Sundial".into())
    }
    async fn desktop_entry(&self) -> zbus::fdo::Result<String> {
        Ok("pearnext".into())
    }
    async fn can_quit(&self) -> zbus::fdo::Result<bool> {
        Ok(true)
    }
    async fn quit(&self) -> zbus::fdo::Result<()> {
        self.app.exit(0);
        Ok(())
    }
    async fn can_raise(&self) -> zbus::fdo::Result<bool> {
        Ok(true)
    }
    async fn raise(&self) -> zbus::fdo::Result<()> {
        if let Some(w) = self.app.get_webview_window("main") {
            let _ = (w.show(), w.unminimize(), w.set_focus());
        }
        Ok(())
    }
    async fn can_set_fullscreen(&self) -> zbus::fdo::Result<bool> {
        Ok(false)
    }
    async fn fullscreen(&self) -> zbus::fdo::Result<bool> {
        Ok(false)
    }
    async fn set_fullscreen(&self, _fullscreen: bool) -> zbus::Result<()> {
        Ok(())
    }
    async fn has_track_list(&self) -> zbus::fdo::Result<bool> {
        Ok(false)
    }
    async fn supported_uri_schemes(&self) -> zbus::fdo::Result<Vec<String>> {
        Ok(vec![])
    }
    async fn supported_mime_types(&self) -> zbus::fdo::Result<Vec<String>> {
        Ok(vec![])
    }
}

impl LocalPlayerInterface for PearMpris {
    async fn playback_status(&self) -> zbus::fdo::Result<PlaybackStatus> {
        Ok(if playing_of(&self.app) {
            PlaybackStatus::Playing
        } else {
            PlaybackStatus::Paused
        })
    }
    async fn loop_status(&self) -> zbus::fdo::Result<LoopStatus> {
        Ok(LoopStatus::None)
    }
    async fn set_loop_status(&self, _: LoopStatus) -> zbus::Result<()> {
        Ok(())
    }
    async fn shuffle(&self) -> zbus::fdo::Result<bool> {
        Ok(false)
    }
    async fn set_shuffle(&self, _: bool) -> zbus::Result<()> {
        Ok(())
    }
    async fn rate(&self) -> zbus::fdo::Result<PlaybackRate> {
        Ok(1.0)
    }
    async fn set_rate(&self, _: PlaybackRate) -> zbus::Result<()> {
        Ok(())
    }
    async fn minimum_rate(&self) -> zbus::fdo::Result<PlaybackRate> {
        Ok(1.0)
    }
    async fn maximum_rate(&self) -> zbus::fdo::Result<PlaybackRate> {
        Ok(1.0)
    }
    async fn metadata(&self) -> zbus::fdo::Result<Metadata> {
        Ok(build_metadata(&self.app))
    }
    async fn volume(&self) -> zbus::fdo::Result<Volume> {
        Ok(*self.app.state::<PlayerState>().volume.lock().unwrap())
    }
    async fn set_volume(&self, volume: Volume) -> zbus::Result<()> {
        let v = volume.clamp(0.0, 1.0);
        *self.app.state::<PlayerState>().volume.lock().unwrap() = v;
        crate::bridge::host_cmd(&self.app, "volume", Some(v));
        Ok(())
    }
    async fn position(&self) -> zbus::fdo::Result<Time> {
        Ok(to_time(pos_secs(&self.app) as u64))
    }
    async fn can_go_next(&self) -> zbus::fdo::Result<bool> {
        Ok(true)
    }
    async fn can_go_previous(&self) -> zbus::fdo::Result<bool> {
        Ok(true)
    }
    async fn can_play(&self) -> zbus::fdo::Result<bool> {
        Ok(true)
    }
    async fn can_pause(&self) -> zbus::fdo::Result<bool> {
        Ok(true)
    }
    async fn can_seek(&self) -> zbus::fdo::Result<bool> {
        Ok(true)
    }
    async fn can_control(&self) -> zbus::fdo::Result<bool> {
        Ok(true)
    }

    async fn play(&self) -> zbus::fdo::Result<()> {
        crate::bridge::host_cmd(&self.app, "play", None);
        Ok(())
    }
    async fn pause(&self) -> zbus::fdo::Result<()> {
        crate::bridge::host_cmd(&self.app, "pause", None);
        Ok(())
    }
    async fn play_pause(&self) -> zbus::fdo::Result<()> {
        crate::bridge::host_cmd(&self.app, "play_pause", None);
        Ok(())
    }
    async fn stop(&self) -> zbus::fdo::Result<()> {
        crate::bridge::host_cmd(&self.app, "pause", None);
        Ok(())
    }
    async fn next(&self) -> zbus::fdo::Result<()> {
        crate::bridge::host_cmd(&self.app, "next", None);
        Ok(())
    }
    async fn previous(&self) -> zbus::fdo::Result<()> {
        crate::bridge::host_cmd(&self.app, "prev", None);
        Ok(())
    }
    async fn seek(&self, offset: Time) -> zbus::fdo::Result<()> {
        crate::bridge::host_cmd(&self.app, "seek_by", Some(time_to_secs(offset)));
        Ok(())
    }
    async fn set_position(&self, _track_id: TrackId, position: Time) -> zbus::fdo::Result<()> {
        crate::bridge::host_cmd(&self.app, "seek", Some(time_to_secs(position)));
        Ok(())
    }
    async fn open_uri(&self, _uri: String) -> zbus::fdo::Result<()> {
        Ok(())
    }
}

pub fn init(app: &AppHandle) -> tauri::Result<()> {
    let app2 = app.clone();
    std::thread::Builder::new()
        .name("mpris".into())
        .spawn(move || {
            futures_lite::future::block_on(async move {
                let server = match LocalServer::new("sundial", PearMpris { app: app2 }).await {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("mpris: не удалось занять шину: {e}");
                        return;
                    }
                };
                server.run().await;
            });
        })
        .expect("mpris: запустить поток");
    Ok(())
}

pub fn push_track(_app: &AppHandle) {}
pub fn push_status(_app: &AppHandle) {}

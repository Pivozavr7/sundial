import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { createPlayer } from './yt';

export interface SearchItem {
  videoId: string; title: string; subtitle: string; thumb: string | null;
}
export interface Track {
  id: string; title: string; artist: string; album: string;
  artUrl: string | null; durationSecs: number;
}

class Player {
  results = $state<SearchItem[]>([]);
  searching = $state(false);
  searchError = $state<string | null>(null);
  lastQuery = $state('');

  track = $state<Track | null>(null);
  playing = $state(false);
  pos = $state(0);
  volume = $state(1);

  queue = $state<SearchItem[]>([]);
  qidx = $state(-1);

  #anchorPos = 0;
  #anchorAt = 0;

  #yt: any = null;
  #ytReady: Promise<any> | null = null;
  #lastVol = 1;

  get estPos(): number {
    if (!this.playing) return this.#anchorPos;
    const est = this.#anchorPos + (performance.now() - this.#anchorAt) / 1000;
    const d = this.track?.durationSecs ?? 0;
    return d > 0 ? Math.min(est, d) : est;
  }

  #anchor(t: number) {
    this.#anchorPos = Math.max(0, t);
    this.#anchorAt = performance.now();
  }

  async init() {
    const inTauri = typeof (window as any).__TAURI_INTERNALS__ !== 'undefined';

    if (inTauri) {
      try {
        await listen<{ action: string; value: number | null }>('host', (e) => {
          const { action, value } = e.payload;
          if (action === 'play_pause') this.playPause();
          else if (action === 'play') this.#yt?.playVideo();
          else if (action === 'pause') this.#yt?.pauseVideo();
          else if (action === 'next') this.next();
          else if (action === 'prev') this.prev();
          else if (action === 'seek' && value != null) this.seekTo(value);
          else if (action === 'seek_by' && value != null) this.seekTo(this.estPos + value);
          else if (action === 'volume' && value != null) this.setVolume(value);
        });
      } catch (e) {
        console.warn('[pear] IPC-мост не подключился:', e);
      }
    }

    setInterval(() => {
      if (this.playing && this.#yt) {
        const t = this.#yt.getCurrentTime() || 0;
        const d = this.#yt.getDuration() || 0;
        this.#anchor(t);
        this.pos = this.estPos;
        if (d > 0 && this.track) this.track.durationSecs = d;
        this.#sync();
      }
    }, 2000);
  }

  #ensurePlayer(): Promise<any> {
    if (!this.#ytReady) {
      this.#ytReady = createPlayer({
        onState: (s) => {
          if (s === 0) {
            // ended: скипаем только если трек дошёл до конца честно.
            // если «кончился» на середине — тракт аудио сломан, очередь не уносим.
            const p = this.#yt;
            if (p && p.getDuration && p.getCurrentTime
                && p.getDuration() > 0
                && p.getCurrentTime() < p.getDuration() - 2) {
              console.warn('[pear] ended раньше времени — аудио-тракт сломан, не скипаю');
              return;
            }
            this.next();
            return;
          }
          this.playing = s === 1;
          const t = this.#yt?.getCurrentTime?.() ?? null;
          if (t != null) this.#anchor(t);
          this.pos = this.estPos;
          this.#sync();
        },
        onError: (code) => {
          console.error(
            `[pear] Ошибка YouTube: ${code}, трек: ${this.track?.title}`
          );
          this.playing = false;
          this.#sync();
        },
      }).then((p) => {
        this.#yt = p;
        p.setVolume(Math.round(this.volume * 100));
        return p;
      });
    }
    return this.#ytReady;
  }

  async search(q: string) {
    const s = q.trim();
    if (!s) return;
    this.searching = true;
    this.searchError = null;
    this.lastQuery = s;
    try {
      this.results = await invoke<SearchItem[]>('ytm_search', { query: s });
    } catch (e: any) {
      this.results = [];
      this.searchError = String(e);
    } finally {
      this.searching = false;
    }
  }

  playItem(item: SearchItem) {
    this.queue = [...this.results];
    const i = this.queue.findIndex((r) => r.videoId === item.videoId);
    this.playAt(Math.max(0, i));
  }

  playQueue(tracks: SearchItem[], start = 0) {
    this.queue = [...tracks];
    this.playAt(start);
  }

  playAt(i: number) {
    if (i < 0 || i >= this.queue.length) return;
    this.qidx = i;
    this.#play(this.queue[i]);
  }

  #play(item: SearchItem) {
    this.track = {
      id: item.videoId,
      title: item.title,
      artist: item.subtitle || 'YouTube Music',
      album: '',
      artUrl: item.thumb,
      durationSecs: 0,
    };
    this.#anchor(0);
    this.pos = 0;
    this.#ensurePlayer()
      .then((p) => p.loadVideoById(item.videoId))
      .catch((e) => console.error('[pear] плеер не поднялся:', e));
    this.#sync();
  }

  next() {
    if (this.qidx < this.queue.length - 1) this.playAt(this.qidx + 1);
  }

  prev() {
    if (this.qidx > 0) this.playAt(this.qidx - 1);
  }

  playPause() {
    if (!this.#yt) return;
    this.playing ? this.#yt.pauseVideo() : this.#yt.playVideo();
  }

  seekTo(sec: number) {
    if (this.#yt && Number.isFinite(sec)) this.#yt.seekTo(sec, true);
    this.#anchor(sec);
    this.pos = sec;
  }

  setVolume(v: number) {
    const c = Math.min(1, Math.max(0, v));
    this.volume = c;
    if (c > 0) this.#lastVol = c;
    if (this.#yt) this.#yt.setVolume(Math.round(c * 100));
  }

  toggleMute() {
    if (this.volume > 0) this.setVolume(0);
    else this.setVolume(this.#lastVol || 1);
  }

  #sync() {
    if (typeof (window as any).__TAURI_INTERNALS__ === 'undefined') return;
    invoke('player_sync', {
      payload: { track: this.track, playing: this.playing, pos: this.pos },
    }).catch(() => {});
  }
}

export const player = new Player();

export const fmt = (s: number) => {
  if (!Number.isFinite(s) || s <= 0) return '0:00';
  return `${Math.floor(s / 60)}:${String(Math.floor(s % 60)).padStart(2, '0')}`;
};

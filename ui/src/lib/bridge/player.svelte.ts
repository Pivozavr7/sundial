import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';

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
    try {
      await listen<{ action: string; value: number | null }>('host', (e) => {
        const { action, value } = e.payload;
        if (action === 'play_pause') this.playPause();
        else if (action === 'play') { invoke('audio_resume'); this.playing = true; }
        else if (action === 'pause') { invoke('audio_pause'); this.playing = false; }
        else if (action === 'next') this.next();
        else if (action === 'prev') this.prev();
        else if (action === 'seek' && value != null) this.seekTo(value);
        else if (action === 'seek_by' && value != null) this.seekTo(this.estPos + value);
        else if (action === 'volume' && value != null) this.setVolume(value);
      });
      await listen('track-ended', () => this.next());
    } catch (e) {
      console.warn('[pear] IPC:', e);
    }

    setInterval(async () => {
      try {
        const [p, d] = await Promise.all([invoke<number>('audio_pos'), invoke<number>('audio_dur')]);
        if (p > 0 || d > 0) {
          this.#anchor(p / 1000);
          this.pos = this.estPos;
          if (d > 0 && this.track) this.track.durationSecs = d / 1000;
          this.#sync();
        }
      } catch {}
    }, 500);
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
      artist: item.subtitle || 'YouTube',
      album: '',
      artUrl: item.thumb,
      durationSecs: 0,
    };
    this.#anchor(0);
    this.pos = 0;
    this.playing = true;
    invoke('audio_play', { videoId: item.videoId }).catch((e) => {
      console.error('[pear] play:', e);
      this.playing = false;
    });
    this.#sync();
  }

  next() {
    if (this.qidx < this.queue.length - 1) this.playAt(this.qidx + 1);
  }

  prev() {
    if (this.qidx > 0) this.playAt(this.qidx - 1);
  }

  playPause() {
    if (!this.track) return;
    if (this.playing) { invoke('audio_pause'); this.playing = false; }
    else { invoke('audio_resume'); this.playing = true; }
    this.#anchor(this.estPos);
    this.#sync();
  }

  seekTo(sec: number) {
    if (!Number.isFinite(sec) || sec < 0) return;
    invoke('audio_seek', { sec }).catch(() => {});
    this.#anchor(sec);
    this.pos = sec;
    this.playing = true;
  }

  setVolume(v: number) {
    const c = Math.min(1, Math.max(0, v));
    this.volume = c;
    if (c > 0) this.#lastVol = c;
    invoke('audio_volume', { v: c }).catch(() => {});
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

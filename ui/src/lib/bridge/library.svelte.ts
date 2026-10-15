import { invoke } from '@tauri-apps/api/core';

export interface PlTrack {
  videoId: string; title: string; subtitle: string; thumb: string | null;
}
export interface Playlist {
  name: string; tracks: PlTrack[]; source?: string | null;
}

class LibraryStore {
  playlists = $state<Playlist[]>([]);
  loaded = $state(false);

  async refresh() {
    try {
      this.playlists = await invoke<Playlist[]>('pl_list');
    } catch (e) {
      console.warn('[pear] library:', e);
    }
    this.loaded = true;
  }

  async save(name: string, tracks: PlTrack[]) {
    await invoke('pl_save', { name, tracks });
    await this.refresh();
  }
  async add(name: string, track: PlTrack) {
    await invoke('pl_add', { name, track });
    await this.refresh();
  }
  async del(name: string) {
    await invoke('pl_delete', { name });
    await this.refresh();
  }
  async removeTrack(name: string, index: number) {
    await invoke('pl_remove_track', { name, index });
    await this.refresh();
  }
  async importYtm(link: string): Promise<string> {
    const [name, tracks] = await invoke<[string, PlTrack[]]>('ytm_playlist', { linkOrId: link });
    await invoke('pl_save_source', { name, tracks, source: `ytm:${name}` });
    await this.refresh();
    return name;
  }
}

export const library = new LibraryStore();

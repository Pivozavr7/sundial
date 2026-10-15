import { mount } from 'svelte';
import './app.css';
import { applySeed } from './lib/md/theme';
import { player } from './lib/bridge/player.svelte';
import App from './App.svelte';

async function start() {
  let dark = true, seed = '#6750A4', darkAcc: string | undefined;
  try {
    if (typeof (window as any).__TAURI_INTERNALS__ !== 'undefined') {
      const s = await (await import('@tauri-apps/api/core')).invoke('get_settings');
      (window as any).__pearSettings = s;
      seed = s.accentSeed || seed;
      dark = s.uiTheme !== 'light';
      const { PALETTES } = await import('./lib/md/palette');
      const p = PALETTES.find((x) => x.seed.toLowerCase() === seed.toLowerCase());
      if (p) darkAcc = p.dark;
    }
  } catch {}

  applySeed(seed, dark, darkAcc);
  player.init();
  mount(App, { target: document.getElementById('app')! });
  document.getElementById('pear-boot')?.remove();
}
start();

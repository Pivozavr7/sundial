# sundial

![status](https://img.shields.io/badge/status-work%20in%20progress-orange)
![version](https://img.shields.io/badge/version-0.1.0-blue)
![platform](https://img.shields.io/badge/platform-linux%20only-lightgrey)
![license](https://img.shields.io/badge/license-MIT-green)
![rust](https://img.shields.io/badge/rust-✓-brown?logo=rust)
![svelte](https://img.shields.io/badge/svelte-5-orange?logo=svelte)
![typescript](https://img.shields.io/badge/typescript-✓-blue?logo=typescript)

sundial is a desktop client for youtube music. built on tauri v2: a rusty core and a svelte frontend. the idea is simple — listen to music without a browser tab sitting around, with a tray icon, hotkeys and proper media keys, like a real app.

this is the author's first project, so the code doesn't apologize for anything.

## features

- search youtube music right from the app
- playback, playlists, your own local library
- tray icon, closing to tray instead of quitting
- mpris — media keys and system notifications (linux)
- global hotkeys
- the theme picks colors from the track cover (material you)
- two languages: русский и english (see `ui/src/lib/locales/`)

## building

```bash
# frontend
cd ui
npm install

# dev mode (from repo root)
cargo tauri dev

# release build — deb and appimage
cargo tauri build
```

linux only. in `src-tauri/Cargo.toml` the `tray`, `mpris` and `hotkeys` features are on by default — the app builds without them, but then it's just a bare player.

## repo layout

```javascript
sundial/
├── ui/         # frontend: svelte 5 + typescript + vite
└── src-tauri/  # core: rust, tauri v2 (bridge, mpris, tray, hotkeys, library)
```

## localization

strings live in `ui/src/lib/locales/en.json` and `ru.json`, all lowercase, with the occasional keyboard mash as a placeholder. to wire it up:

```ts
import { t, locale, setLocale } from './lib/i18n.svelte.ts';
// placeholder="что включить?" → placeholder={t('search.placeholder')}
// tabs: ['search','поиск'] → ['search', t('tabs.search')]
```

add new strings to both json files, keep the keys in english. want a third language? copy `en.json` and go.

## honest list of downsides

since it's a first project, no pretending:

- **version 0.1.0.** everything is rough, bugs are guaranteed, something may not even start.
- **linux only.** deb/appimage builds, mpris — all linux stuff. no windows, no macos, and no real plans for them yet.
- **undocumented api.** search goes through youtube music's internal innertube api. google can change something tomorrow and search just breaks. the key and the region (RU, russian locale) are hardcoded.
- **no releases, ci or tests.** manual testing only.
- **monolithic ui.** the whole frontend lives in one `app.svelte` (~29 kb). a refactor is coming, someday, eventually.
- **comments and error messages are half in russian.** it's a feature, probably.
- **vibecode.**
## roadmap

- proper releases with prebuilt bundles
- split the ui into components
- configurable search region and language
- maybe other platforms, eventually
- more languages, if anyone asks

## contributing

issues and prs welcome. it's a pet project, but if the idea clicks — why not.

## license

mit

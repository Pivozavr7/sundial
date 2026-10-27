<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { player } from './lib/bridge/player.svelte';
  import { library, type PlTrack } from './lib/bridge/library.svelte';
  import { applySeed } from './lib/md/theme';
  import { PALETTES } from './lib/md/palette';
  import TitleBar from './components/TitleBar.svelte';
  import PlayerBar from './components/PlayerBar.svelte';
  import CookieBg from './components/CookieBg.svelte';
  import { t, locale, setLocale } from './lib/i18n.svelte.ts';

  let q = $state('');
  let view = $state<'search' | 'queue' | 'playlists' | 'settings'>('search');
  let openPl = $state<string | null>(null);
  let newName = $state('');
  let addModal = $state<PlTrack | null>(null);
  let saveOpen = $state(false);

  let importLink = $state('');
  let importing = $state(false);
  let importMsg = $state<{ ok: boolean; text: string } | null>(null);

  let settings = $state<any>((window as any).__pearSettings ?? {
    accentSeed: '#7D5700', uiTheme: 'dark', animatedBg: true, closeToTray: true,
    waveSpeed: 0.55, waveAmp: 3.0, uiFps: 60,
  });

  library.refresh();

  function applyTheme(seed: string, dark: boolean) {
    const p = PALETTES.find((x) => x.seed.toLowerCase() === seed.toLowerCase());
    applySeed(seed, dark, p?.dark);
  }

  $effect(() => {
    applyTheme(settings.accentSeed || '#7D5700', settings.uiTheme !== 'light');
  });

  function setAccent(seed: string) {
    save({ accentSeed: seed });
  }

  async function save(patch: any) {
    settings = { ...settings, ...patch };
    (window as any).__pearSettings = settings;
    try { await invoke('set_settings', { settings }); } catch {}
  }

  function submit(e: SubmitEvent) {
    e.preventDefault();
    player.search(q);
    view = 'search';
  }

  const open = $derived(
    view === 'playlists' && openPl ? library.playlists.find((p) => p.name === openPl) : null
  );

  const toPl = (i: { videoId: string; title: string; subtitle: string; thumb: string | null }): PlTrack =>
    ({ videoId: i.videoId, title: i.title, subtitle: i.subtitle, thumb: i.thumb });

  async function addTo(p: { name: string }) {
    if (!addModal) return;
    await library.add(p.name, addModal);
    addModal = null;
  }
  async function createAndAdd() {
    if (!addModal || !newName.trim()) return;
    await library.add(newName.trim(), addModal);
    newName = '';
    addModal = null;
  }
  async function saveQueueConfirm() {
    if (!newName.trim() || player.queue.length === 0) return;
    await library.save(newName.trim(), player.queue.map(toPl));
    newName = '';
    saveOpen = false;
  }

  async function doImport() {
    const l = importLink.trim();
    if (!l || importing) return;
    importing = true;
    importMsg = null;
    try {
      const name = await library.importYtm(l);
      importMsg = { ok: true, text: t('playlists.imported', { name }) };
      importLink = '';
    } catch (e: any) {
      importMsg = { ok: false, text: String(e) };
    } finally {
      importing = false;
    }
  }

  let tabEls: Record<string, HTMLElement> = {};
  let indicator = $state({ left: 0, width: 0 });

  function measureTab(key: string) {
    const el = tabEls[key];
    if (el) indicator = { left: el.offsetLeft, width: el.offsetWidth };
  }

  $effect(() => {
    measureTab(view);
  });

  $effect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        if (addModal) addModal = null;
        else if (saveOpen) saveOpen = false;
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });

</script>

<div class="app">
  <TitleBar>
    <form class="search" onsubmit={submit} role="search">
      <svg width="18" height="18" viewBox="0 0 24 24"><path fill="currentColor" d="M15.5 14h-.79l-.28-.27A6.47 6.47 0 0 0 16 9.5 6.5 6.5 0 1 0 9.5 16c1.61 0 3.09-.59 4.23-1.57l.27.28v.79l5 4.99L20.49 19zm-6 0C7.01 14 5 11.99 5 9.5S7.01 5 9.5 5 14 7.01 14 9.5 11.99 14 9.5 14"/></svg>
      <input placeholder={t('search.placeholder')} bind:value={q} />
    </form>
  </TitleBar>

  <nav class="tabs">
    {#each [['search', t('tabs.search')], ['queue', t('tabs.queue')], ['playlists', t('tabs.playlists')], ['settings', t('tabs.settings')]] as [key, label] (key)}
      <button bind:this={tabEls[key]} class:active={view === key}
              onclick={() => (view = key as any)}>{label}</button>
    {/each}
    <div class="tab-indicator" style={`left:${indicator.left}px; width:${indicator.width}px`}></div>
  </nav>

  <main>
    <div class="bgfx" aria-hidden="true">
      {#if settings.animatedBg !== false}
        <div class="cookie c1"><CookieBg size={420} color="var(--md-primary-container)" opacity={0.35} /></div>
        <div class="cookie c2"><CookieBg size={260} color="var(--md-tertiary-container)" opacity={0.3} n={8} speed={0.8} /></div>
      {/if}
    </div>

    {#key view}
      {#if view === 'settings'}
        <div class="panel settings">
          <div class="row">
            <span>{t('settings.language')}</span>
            <div class="seg">
              <button class:sel={locale.current === 'ru'} onclick={() => setLocale('ru')}>русский</button>
              <button class:sel={locale.current === 'en'} onclick={() => setLocale('en')}>english</button>
            </div>
          </div>

          <h3>{t('settings.color')}</h3>
          <div class="swatches">
            {#each PALETTES as p (p.seed)}
              <button class="swatch" class:sel={(settings.accentSeed || '').toLowerCase() === p.seed}
                      style={`background:${p.dark}`}
                      title={p.name}
                      aria-label={`${t('settings.color')}: ${p.name}`}
                      onclick={() => setAccent(p.seed)}>
                {#if (settings.accentSeed || '').toLowerCase() === p.seed}
                  <svg width="16" height="16" viewBox="0 0 24 24"><path fill="var(--md-surface)" d="M9 16.2 4.8 12l-1.4 1.4L9 19 21 7l-1.4-1.4z"/></svg>
                {/if}
              </button>
            {/each}
          </div>

          <h3>{t('settings.wave')}</h3>
          <div class="row">
            <span>{t('settings.waveSpeed')}</span>
            <input type="range" class="setrange" min="0.2" max="1.2" step="0.05"
                   value={settings.waveSpeed ?? 0.55}
                   aria-label={t('settings.waveSpeed')}
                   oninput={(e) => save({ waveSpeed: Number((e.target as HTMLInputElement).value) })} />
          </div>
          <div class="row">
            <span>{t('settings.waveAmp')}</span>
            <input type="range" class="setrange" min="1.5" max="6" step="0.5"
                   value={settings.waveAmp ?? 3}
                   aria-label={t('settings.waveAmp')}
                   oninput={(e) => save({ waveAmp: Number((e.target as HTMLInputElement).value) })} />
          </div>
          <div class="row">
            <span>{t('settings.uiSmoothness')}</span>
            <div class="seg">
              <button class:sel={(settings.uiFps ?? 60) === 30}
                      onclick={() => save({ uiFps: 30 })}>30</button>
              <button class:sel={(settings.uiFps ?? 60) === 60}
                      onclick={() => save({ uiFps: 60 })}>60</button>
              <button class:sel={!(settings.uiFps ?? 60)}
                      onclick={() => save({ uiFps: 0 })}>{t('settings.max')}</button>
            </div>
          </div>

          <h3>{t('settings.looks')}</h3>
          <div class="row">
            <span>{t('settings.theme')}</span>
            <div class="seg">
              <button class:sel={settings.uiTheme !== 'light'}
                      onclick={() => save({ uiTheme: 'dark' })}>{t('settings.dark')}</button>
              <button class:sel={settings.uiTheme === 'light'}
                      onclick={() => save({ uiTheme: 'light' })}>{t('settings.light')}</button>
            </div>
          </div>
          <div class="row">
            <span>{t('settings.animatedBg')}</span>
            <button class="switch" class:on={settings.animatedBg !== false}
                    onclick={() => save({ animatedBg: !(settings.animatedBg !== false) })}
                    role="switch" aria-checked={settings.animatedBg !== false}
                    aria-label={t('settings.animatedBg')}></button>
          </div>

          <h3>{t('settings.behavior')}</h3>
          <div class="row">
            <span>{t('settings.closeToTray')}</span>
            <button class="switch" class:on={!!settings.closeToTray}
                    onclick={() => save({ closeToTray: !settings.closeToTray })}
                    role="switch" aria-checked={!!settings.closeToTray}
                    aria-label={t('settings.closeToTray')}></button>
          </div>

          <h3>{t('settings.about')}</h3>
          <div class="about">
            <svg width="40" height="40" viewBox="0 0 512 512" aria-hidden="true">
              <path d="M 307.3 115 A 150 150 0 1 1 204.7 115" fill="none"
                    stroke="var(--md-primary)" stroke-width="54" stroke-linecap="round"/>
              <path d="M 224 204 L 224 308 L 308 256 Z" fill="var(--md-primary)"/>
            </svg>
            <div>
              <p class="ab-name">Sundial <span class="ab-ver">0.1.0-alpha</span></p>
              <p class="ab-line">{t('settings.tagline')}</p>
              <p class="ab-line">{t('settings.origin')}</p>
              <div class="ab-links">
                <a class="ab-link" href="https://github.com/Pivozavr7/sundial" target="_blank">{t('settings.github')}</a>
                <a class="ab-link" href="https://t.me/vedmost" target="_blank">{t('settings.contact')}</a>
              </div>
              <p class="ab-note">{t('settings.disclaimer')}</p>
            </div>
          </div>
        </div>

      {:else if view === 'search'}
        {#if player.searching}
          <div class="center"><p>{t('search.loading', { q: player.lastQuery })}</p></div>
        {:else if player.searchError}
          <div class="center"><p class="big">{t('search.error')}</p><p class="hint">{player.searchError}</p></div>
        {:else if player.results.length > 0}
          <div class="results-head">
            <h2>{player.lastQuery}</h2>
            <span class="count">{t('search.count', { n: player.results.length })}</span>
          </div>
          <div class="list panel">
            {#each player.results as item (item.videoId)}
              <div class="item" role="button" tabindex="0" onclick={() => player.playItem(item)}
                   onkeydown={(e) => e.key === 'Enter' && player.playItem(item)}>
                {#if item.thumb}
                  <img class="art" src={item.thumb} alt="" loading="lazy" />
                {:else}
                  <div class="art empty"></div>
                {/if}
                <div class="txt">
                  <span class="t">{item.title}</span>
                  <span class="a">{item.subtitle || 'YouTube Music'}</span>
                </div>
                <button class="mini" title="В плейлист"
                        onclick={(e) => { e.stopPropagation(); addModal = toPl(item); }}>
                  <svg width="16" height="16" viewBox="0 0 24 24"><path fill="currentColor" d="M11 5h2v6h6v2h-6v6h-2v-6H5v-2h6z"/></svg>
                </button>
              </div>
            {/each}
          </div>
        {:else}
          <div class="center">
            <p class="big">{t('search.emptyTitle')}</p>
            <p class="hint">{t('search.emptyHint')}</p>
          </div>
        {/if}

      {:else if view === 'queue'}
        {#if player.queue.length === 0}
          <div class="center">
            <p class="big">{t('queue.emptyTitle')}</p>
            <p class="hint">{t('queue.emptyHint')}</p>
          </div>
        {:else}
          <div class="results-head">
            <h2>{t('queue.title')}</h2>
            <span class="count">{player.queue.length}</span>
            <span class="gap"></span>
            <button class="md-text" onclick={() => (saveOpen = true)}>{t('queue.saveAs')}</button>
          </div>
          <div class="list panel">
            {#each player.queue as item, i (item.videoId + '-' + i)}
              <div class="item" class:now={i === player.qidx} role="button" tabindex="0"
                   onclick={() => player.playAt(i)}
                   onkeydown={(e) => e.key === 'Enter' && player.playAt(i)}>
                {#if item.thumb}
                  <img class="art" src={item.thumb} alt="" loading="lazy" />
                {:else}
                  <div class="art empty"></div>
                {/if}
                <div class="txt">
                  <span class="t">{item.title}</span>
                  <span class="a">{item.subtitle || 'YouTube Music'}</span>
                </div>
                <button class="mini" title="В плейлист"
                        onclick={(e) => { e.stopPropagation(); addModal = toPl(item); }}>
                  <svg width="16" height="16" viewBox="0 0 24 24"><path fill="currentColor" d="M11 5h2v6h6v2h-6v6h-2v-6H5v-2h6z"/></svg>
                </button>
              </div>
            {/each}
          </div>
        {/if}

      {:else if !open}
        <div class="panel import">
          <form class="improw" onsubmit={(e) => { e.preventDefault(); doImport(); }}>
            <input placeholder={t('playlists.importPlaceholder')} bind:value={importLink} />
            <button class="md-filled" disabled={importing}>{importing ? t('playlists.importing') : t('playlists.import')}</button>
          </form>
          {#if importMsg}
            <p class="impmsg" class:err={!importMsg.ok}>{importMsg.text}</p>
          {/if}
        </div>

        {#if library.playlists.length === 0}
          <div class="center">
            <p class="big">{t('playlists.emptyTitle')}</p>
            <p class="hint">{t('playlists.emptyHint')}</p>
          </div>
        {:else}
          <div class="plgrid">
            {#each library.playlists as p (p.name)}
              <button class="plcard" onclick={() => (openPl = p.name)}>
                <span class="plname">{p.name}</span>
                <span class="plcount">{t('playlists.tracks', { n: p.tracks.length })}{p.source ? t('playlists.fromYtm') : ''}</span>
              </button>
            {/each}
          </div>
        {/if}

      {:else}
        <div class="results-head">
          <button class="back" onclick={() => (openPl = null)} title={t('playlists.back')} aria-label={t('playlists.back')}>
            <svg width="20" height="20" viewBox="0 0 24 24"><path fill="currentColor" d="M20 11H7.8l5.6-5.6L12 4l-8 8 8 8 1.4-1.4L7.8 13H20z"/></svg>
          </button>
          <h2>{open.name}</h2>
          <span class="count">{open.tracks.length}</span>
          <span class="gap"></span>
          <button class="md-text" onclick={() => player.playQueue(open.tracks.map((t) => ({ ...t })), 0)}>{t('playlists.play')}</button>
          <button class="md-text danger" onclick={() => { library.del(open.name); openPl = null; }}>{t('playlists.delete')}</button>
        </div>
        <div class="list panel">
          {#each open.tracks as item, i (item.videoId + '-' + i)}
            <div class="item" role="button" tabindex="0"
                 onclick={() => player.playQueue(open.tracks.map((t) => ({ ...t })), i)}
                 onkeydown={(e) => e.key === 'Enter' && player.playQueue(open.tracks.map((t) => ({ ...t })), i)}>
              {#if item.thumb}
                <img class="art" src={item.thumb} alt="" loading="lazy" />
              {:else}
                <div class="art empty"></div>
              {/if}
              <div class="txt">
                <span class="t">{item.title}</span>
                <span class="a">{item.subtitle || 'YouTube Music'}</span>
              </div>
              <button class="mini" title="убрать"
                      onclick={(e) => { e.stopPropagation(); library.removeTrack(open.name, i); }}>
                <svg width="16" height="16" viewBox="0 0 24 24"><path fill="currentColor" d="M19 6.4 17.6 5 12 10.6 6.4 5 5 6.4 10.6 12 5 17.6 19 19 17.6 13.4 12z"/></svg>
              </button>
            </div>
          {/each}
        </div>
      {/if}
    {/key}
  </main>

  {#if player.error}
    <div class="play-error" role="alert">
      <span class="pe-txt">{player.error}</span>
      <button class="pe-x" onclick={() => (player.error = null)} aria-label="закрыть">×</button>
    </div>
  {/if}
  <PlayerBar waveSpeed={settings.waveSpeed ?? 0.55} waveAmp={settings.waveAmp ?? 3.0} />
</div>

{#if addModal}
  <div class="scrimwrap">
    <button class="scrim" aria-label="Закрыть" onclick={() => (addModal = null)}></button>
    <div class="modal" role="dialog" aria-modal="true">
      <h3>{t('modal.addToPlaylist')}</h3>
      <p class="mtrack">{addModal.title}</p>
      <div class="pllist">
        {#each library.playlists as p (p.name)}
          <button class="plrow" onclick={() => addTo(p)}>{p.name}<span>{p.tracks.length}</span></button>
        {/each}
      </div>
      <div class="newpl">
        <input placeholder={t('modal.newPlaylist')} bind:value={newName} />
        <button class="md-filled" onclick={createAndAdd}>{t('modal.create')}</button>
      </div>
    </div>
  </div>
{/if}

{#if saveOpen}
  <div class="scrimwrap">
    <button class="scrim" aria-label="Закрыть" onclick={() => (saveOpen = false)}></button>
    <div class="modal" role="dialog" aria-modal="true">
      <h3>{t('modal.saveQueue')}</h3>
      <div class="newpl">
        <input placeholder={t('modal.name')} bind:value={newName} />
        <button class="md-filled" onclick={saveQueueConfirm}>{t('modal.save')}</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .app { height: 100%; display: flex; flex-direction: column; }

  .search {
    display: flex; align-items: center; gap: 12px;
    width: min(560px, 46vw); height: 42px; padding: 0 20px;
    border-radius: 21px;
    background: color-mix(in srgb, var(--md-surface) 70%, transparent);
    border: 2px solid color-mix(in srgb, var(--md-outline) 70%, transparent);
    color: var(--md-on-surface-variant);
    transition: border-color .15s, box-shadow .15s;
  }
  .search:focus-within {
    border-color: var(--md-primary);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--md-primary) 18%, transparent);
  }
  .search input {
    flex: 1; border: none; outline: none; background: transparent;
    color: var(--md-on-surface); font: inherit;
  }
  .search input::placeholder { color: var(--md-on-surface-variant); }

  .tabs {
    position: relative;
    display: flex; gap: 6px; padding: 2px 16px 0;
    background: var(--md-surface); flex-shrink: 0;
  }
  .tabs button { padding: 12px 18px; font-weight: 500; color: var(--md-on-surface-variant); transition: color .15s; }
  .tabs button.active { color: var(--md-primary); }
  .tab-indicator {
    position: absolute;
    bottom: 0;
    height: 3px;
    border-radius: 3px 3px 0 0;
    background: var(--md-primary);
    transition: left .3s cubic-bezier(.2, 0, 0, 1), width .3s cubic-bezier(.2, 0, 0, 1);
  }

  main {
    flex: 1; position: relative; overflow: hidden;
    background: var(--md-surface-container-low);
    display: flex; flex-direction: column;
  }

  .bgfx { position: absolute; inset: 0; overflow: hidden; pointer-events: none; }
  .cookie { position: absolute; }
  .c1 { top: -90px; right: -70px; animation: drift1 26s ease-in-out infinite alternate; }
  .c2 { bottom: -70px; left: -50px; animation: drift2 32s ease-in-out infinite alternate; }
  @keyframes drift1 { to { transform: translate(-40px, 30px) rotate(18deg); } }
  @keyframes drift2 { to { transform: translate(30px, -25px) rotate(-14deg); } }

  .panel {
    position: relative; z-index: 1;
    margin: 0 16px 18px;
    padding: 6px;
    background: color-mix(in srgb, var(--md-surface) 78%, transparent);
    border: 1px solid color-mix(in srgb, var(--md-outline-variant) 55%, transparent);
    border-radius: 22px;
    flex: 1; min-height: 0;
    display: flex; flex-direction: column;
    overflow: hidden;
    animation: rise .3s cubic-bezier(.2, 0, 0, 1) both;
  }

  .settings { padding: 16px 20px; overflow-y: auto; display: block; }
  .settings h3 { font-size: 14px; font-weight: 500; color: var(--md-primary); margin: 18px 0 6px; }
  .settings h3:first-child { margin-top: 0; }
  .row { display: flex; align-items: center; justify-content: space-between; padding: 10px 4px; }
  .seg { display: flex; border: 1px solid var(--md-outline-variant); border-radius: 100px; overflow: hidden; }
  .seg button { padding: 8px 18px; color: var(--md-on-surface-variant); }
  .seg button.sel { background: var(--md-secondary-container); color: var(--md-on-secondary-container); }

  .swatches { display: flex; gap: 10px; padding: 8px 4px; }
  .swatch {
    width: 40px; height: 40px;
    border-radius: 14px;
    display: grid; place-items: center;
    border: 2px solid transparent;
    transition: border-radius .25s cubic-bezier(.2, 0, 0, 1), transform .15s, border-color .15s;
  }
  .swatch:hover { border-radius: 50%; transform: scale(1.08); }
  .swatch.sel { border-color: var(--md-on-surface); animation: jelly .4s cubic-bezier(.2, 0, 0, 1); }
  @keyframes jelly {
    0% { transform: scale(1); }
    35% { transform: scale(1.15, .85); }
    65% { transform: scale(.92, 1.08); }
    100% { transform: scale(1); }
  }
  .setrange { width: 160px; accent-color: var(--md-primary); }

  .switch {
    position: relative; width: 52px; height: 32px;
    border-radius: 16px;
    background: var(--md-surface-container-highest);
    border: 2px solid var(--md-outline);
    transition: background .2s, border-color .2s;
    flex-shrink: 0;
  }
  .switch::after {
    content: ''; position: absolute; top: 50%; left: 5px;
    translate: 0 -50%;
    width: 16px; height: 16px; border-radius: 50%;
    background: var(--md-on-surface-variant);
    transition: left .2s cubic-bezier(.2, 0, 0, 1), width .2s, height .2s, background .2s;
  }
  .switch.on { background: var(--md-primary); border-color: var(--md-primary); }
  .switch.on::after { left: calc(100% - 23px); width: 24px; height: 24px; background: var(--md-on-primary); }

  .about { display: flex; gap: 14px; padding: 10px 4px; align-items: flex-start; }
  .ab-name { font-weight: 500; }
  .ab-ver { font-size: 12px; color: var(--md-on-surface-variant); margin-left: 6px; }
  .ab-line { font-size: 12px; color: var(--md-on-surface-variant); }
  .ab-note { font-size: 11px; color: var(--md-outline); margin-top: 4px; }
  .ab-links { display: flex; gap: 14px; padding: 8px 0 2px; }
  .ab-link { color: var(--md-primary); font-size: 13px; font-weight: 500; }
  .ab-link:hover { text-decoration: underline; }

  .center {
    position: relative; z-index: 1;
    height: 100%;
    display: flex; flex-direction: column; align-items: center; justify-content: center;
    gap: 12px; text-align: center; padding: 24px;
  }
  .big { font-size: 18px; font-weight: 500; }
  .hint { color: var(--md-on-surface-variant); font-size: 12px; max-width: 360px; }

  .results-head {
    position: relative; z-index: 1;
    display: flex; align-items: baseline; gap: 12px;
    padding: 20px 30px 10px;
    animation: rise .3s cubic-bezier(.2, 0, 0, 1) both;
  }
  .results-head h2 { font-size: 18px; font-weight: 500; }
  .count { font-size: 12px; color: var(--md-on-surface-variant); }
  .gap { flex: 1; }
  .back {
    width: 34px; height: 34px; border-radius: 50%;
    display: grid; place-items: center;
    color: var(--md-on-surface-variant);
    align-self: center;
  }
  .back:hover { background: color-mix(in srgb, var(--md-on-surface) 8%, transparent); }
  .danger { color: var(--md-error); }

  .list {
    display: flex; flex-direction: column; gap: 2px;
    overflow-y: auto; min-height: 0; flex: 1;
    padding: 4px;
    scrollbar-width: thin;
    scrollbar-color: var(--md-outline-variant) transparent;
  }
  .list::-webkit-scrollbar { width: 8px; }
  .list::-webkit-scrollbar-thumb {
    background: color-mix(in srgb, var(--md-outline-variant) 70%, transparent);
    border-radius: 4px;
  }
  .list::-webkit-scrollbar-track { background: transparent; }

  .item {
    display: flex; align-items: center; gap: 14px;
    padding: 8px 12px; border-radius: 14px;
    cursor: pointer;
    transition: background .12s, transform .15s cubic-bezier(.2, 0, 0, 1);
    animation: rise .3s cubic-bezier(.2, 0, 0, 1) both;
  }
  .item:nth-child(1) { animation-delay: .02s; }
  .item:nth-child(2) { animation-delay: .05s; }
  .item:nth-child(3) { animation-delay: .08s; }
  .item:nth-child(4) { animation-delay: .11s; }
  .item:nth-child(5) { animation-delay: .14s; }
  .item:nth-child(n+6) { animation-delay: .16s; }
  .item:hover { background: color-mix(in srgb, var(--md-on-surface) 6%, transparent); transform: translateX(3px); }
  .item.now { background: color-mix(in srgb, var(--md-primary) 14%, transparent); }
  .item.now .t { color: var(--md-primary); }
  .art { width: 48px; height: 48px; object-fit: cover; flex-shrink: 0; }
  .art.empty { background: var(--md-surface-container-highest); }
  .txt { display: flex; flex-direction: column; min-width: 0; flex: 1; gap: 2px; }
  .t { font-weight: 500; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .a { font-size: 12px; color: var(--md-on-surface-variant); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .mini {
    width: 36px; height: 36px;
    display: grid; place-items: center;
    color: var(--md-on-surface-variant);
    flex-shrink: 0;
    border-radius: 34%;
    transition: border-radius .25s cubic-bezier(.2, 0, 0, 1), background .12s, color .12s;
  }
  .mini:hover { border-radius: 50%; background: color-mix(in srgb, var(--md-primary) 16%, transparent); color: var(--md-primary); }

  .import { padding: 14px 16px; margin-top: 14px; flex: none; display: block; }
  .improw { display: flex; gap: 8px; }
  .improw input {
    flex: 1; height: 40px; padding: 0 14px;
    border-radius: 20px; border: none; outline: none;
    background: var(--md-surface-container-highest);
    color: var(--md-on-surface); font: inherit;
  }
  .impmsg { margin-top: 8px; font-size: 12px; color: var(--md-primary); }
  .impmsg.err { color: var(--md-error); }

  .plgrid {
    position: relative; z-index: 1;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: 12px;
    padding: 16px 24px 24px;
  }
  .plcard {
    display: flex; flex-direction: column; align-items: flex-start; gap: 4px;
    padding: 16px;
    border-radius: 22px;
    background: color-mix(in srgb, var(--md-surface) 78%, transparent);
    border: 1px solid color-mix(in srgb, var(--md-outline-variant) 55%, transparent);
    text-align: left;
    transition: background .12s, border-radius .25s cubic-bezier(.2, 0, 0, 1);
    animation: rise .3s cubic-bezier(.2, 0, 0, 1) both;
  }
  .plcard:hover { background: color-mix(in srgb, var(--md-primary) 10%, var(--md-surface)); border-radius: 28px; }
  .plname { font-weight: 500; }
  .plcount { font-size: 12px; color: var(--md-on-surface-variant); }

  .scrimwrap {
    position: fixed; inset: 0; z-index: 100;
    display: grid; place-items: center;
  }
  .scrim {
    position: absolute; inset: 0;
    background: rgba(0, 0, 0, .45);
    border: none; padding: 0;
    cursor: default;
    animation: fade .2s ease both;
  }
  .modal {
    position: relative;
    width: min(380px, 90vw);
    max-height: 70vh; overflow-y: auto;
    background: var(--md-surface-container-high);
    border-radius: 24px;
    padding: 20px;
    display: flex; flex-direction: column; gap: 12px;
    animation: pop .25s cubic-bezier(.2, 0, 0, 1) both;
  }
  @keyframes fade { from { opacity: 0; } }
  @keyframes pop { from { opacity: 0; transform: scale(.94); } }
  @keyframes rise { from { opacity: 0; transform: translateY(8px); } }

  .modal h3 { font-size: 16px; font-weight: 500; }
  .mtrack { color: var(--md-on-surface-variant); font-size: 13px; }
  .pllist { display: flex; flex-direction: column; gap: 2px; }
  .plrow {
    display: flex; justify-content: space-between; align-items: center;
    padding: 10px 12px; border-radius: 12px;
  }
  .plrow:hover { background: color-mix(in srgb, var(--md-on-surface) 8%, transparent); }
  .plrow span { font-size: 12px; color: var(--md-on-surface-variant); }
  .newpl { display: flex; gap: 8px; }
  .newpl input {
    flex: 1; height: 40px; padding: 0 14px;
    border-radius: 20px; border: none; outline: none;
    background: var(--md-surface-container-highest);
    color: var(--md-on-surface); font: inherit;
  }
</style>

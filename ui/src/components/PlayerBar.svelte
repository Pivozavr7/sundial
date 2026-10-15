<script lang="ts">
  import { onMount } from 'svelte';
  import { player, fmt } from '../lib/bridge/player.svelte';
  import { t } from '../lib/i18n.svelte.ts';
  let { waveSpeed = 0.55, waveAmp = 3.0 }: { waveSpeed?: number; waveAmp?: number } = $props();

  const H = 18;
  const WAVE_LEN = 34;
  const THICK = 3;
  const STEP = 4;

  let box: HTMLDivElement | undefined = $state();
  let w = $state(0);
  let hovering = $state(false);

  let pathEl: SVGPathElement | undefined = $state();
  let lineEl: SVGLineElement | undefined = $state();
  let dotEl: SVGCircleElement | undefined = $state();

  let raf = 0;
  let amp = 0;
  let phase = 0;
  let prevT = 0;
  let lastBuild = 0;
  let paused = false;
  let reduced = false;

  const dur = $derived(player.track?.durationSecs ?? 0);

  const uiFps = $derived((window as any).__pearSettings?.uiFps ?? 60);
  const frameMin = $derived(uiFps > 0 ? 1000 / uiFps : 0);

  function cookieClip(points = 9): string {
    const cx = 24, cy = 24;
    const rBase = 19, rPeak = 24, sigma = 0.16;
    const TAU = Math.PI * 2;
    const step = TAU / points;
    const samples = points * 24;

    const wrap = (a: number) => {
      a = ((a % TAU) + TAU) % TAU;
      return a > Math.PI ? a - TAU : a;
    };

    let p = '';
    for (let s = 0; s <= samples; s++) {
      const a = (s / samples) * TAU;
      let bump = 0;
      for (let i = 0; i < points; i++) {
        const dist = wrap(a - i * step);
        bump = Math.max(bump, Math.exp(-(dist * dist) / (2 * sigma * sigma)));
      }
      const r = rBase + (rPeak - rBase) * bump;
      const x = cx + r * Math.cos(a);
      const y = cy + r * Math.sin(a);
      p += (s === 0 ? 'M' : 'L') + ` ${x.toFixed(1)} ${y.toFixed(1)}`;
    }
    return `path('${p} Z')`;
  }
  const artClip = cookieClip(9);

  function rebuild(prog: number) {
    if (!w) return;
    const mid = H / 2;
    const xEnd = Math.max(1, prog * w);

    let d = '';
    if (amp > 0.08) {
      for (let x = 0; x < xEnd; x += STEP) {
        const y = mid + amp * Math.sin((x / WAVE_LEN) * Math.PI * 2 - phase * Math.PI * 2);
        d += (d === '' ? 'M' : ' L') + ` ${x.toFixed(1)} ${y.toFixed(2)}`;
      }
      const y = mid + amp * Math.sin((xEnd / WAVE_LEN) * Math.PI * 2 - phase * Math.PI * 2);
      d += ` L ${xEnd.toFixed(1)} ${y.toFixed(2)}`;
    } else {
      d = `M 0 ${mid} L ${xEnd.toFixed(1)} ${mid}`;
    }
    pathEl?.setAttribute('d', d);
    lineEl?.setAttribute('x1', String(xEnd));
    dotEl?.setAttribute('cx', String(xEnd));
  }

  function frame(t: number) {
    const dt = prevT ? Math.min(0.05, (t - prevT) / 1000) : 0.016;
    prevT = t;

    if (player.track) {
      const est = player.estPos;
      player.pos = est;
      const prog = dur > 0 ? Math.min(1, est / dur) : 0;

      const target = player.playing && dur > 0 ? waveAmp : 0;
      amp += (target - amp) * (1 - Math.exp(-dt * 12));
      if (!reduced && player.playing && dur > 0) phase += dt * waveSpeed;

      const now = performance.now();
      if (now - lastBuild >= frameMin) {
        lastBuild = now;
        rebuild(prog);
      }
    } else {
      amp = 0;
      const now = performance.now();
      if (now - lastBuild >= frameMin) {
        lastBuild = now;
        rebuild(0);
      }
    }

    if (!paused) {
      raf = requestAnimationFrame(frame);
    }
  }

  function wake() {
    paused = false;
    prevT = 0;
    if (!raf) raf = requestAnimationFrame(frame);
  }

  function sleep() {
    paused = true;
    cancelAnimationFrame(raf);
    raf = 0;
  }

  onMount(() => {
    reduced = matchMedia('(prefers-reduced-motion: reduce)').matches;
    document.addEventListener('visibilitychange', () => (document.hidden ? sleep() : wake()));
    wake();
    return () => {
      sleep();
      document.removeEventListener('visibilitychange', () => {});
    };
  });

  function seek(e: PointerEvent) {
    if (!box || dur <= 0) return;
    const r = box.getBoundingClientRect();
    player.seekTo(((e.clientX - r.left) / r.width) * dur);
  }

  function keySeek(e: KeyboardEvent) {
    if (dur <= 0) return;
    if (e.key === 'ArrowRight') { player.seekTo(Math.min(dur, player.estPos + 5)); e.preventDefault(); }
    if (e.key === 'ArrowLeft')  { player.seekTo(Math.max(0, player.estPos - 5)); e.preventDefault(); }
  }
</script>

<footer class="pb">
  <div class="meta">
    {#if player.track}
      {#if player.track.artUrl}
        <img class="art" class:beat={player.playing} src={player.track.artUrl} alt="" style={`clip-path:${artClip}`} />
      {:else}
        <div class="art empty" style={`clip-path:${artClip}`}></div>
      {/if}
      <div class="txt">
        <span class="t" title={player.track.title}>{player.track.title || '—'}</span>
        <span class="a" title={player.track.artist}>{player.track.artist || '—'}</span>
      </div>
    {:else}
      <div class="art empty" style={`clip-path:${artClip}`}>
        <svg class="qmark" width="24" height="24" viewBox="0 0 24 24" aria-hidden="true">
          <path fill="currentColor" d="M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20zm0 18a8 8 0 1 1 0-16 8 8 0 0 1 0 16zm-1-5h2v2h-2zm1.2-9.5c-1.9 0-3.3 1.1-3.6 2.9l1.8.4c.15-.9.8-1.6 1.8-1.6.9 0 1.6.6 1.6 1.4 0 .6-.3 1-1.1 1.6-.9.7-1.9 1.5-1.9 3h2c0-.9.5-1.3 1.3-2 .8-.7 1.7-1.5 1.7-2.8 0-1.7-1.4-2.9-3.6-2.9z"/>
        </svg>
      </div>
      <div class="txt">
        <span class="t">{t('player.silence')}</span>
        <span class="a">{t('player.silenceHint')}</span>
      </div>
    {/if}
  </div>

  <div class="center">
    <div class="controls">
      <button class="cbtn tonal" title={t('player.prev')} onclick={() => player.prev()}>
        <svg width="20" height="20" viewBox="0 0 24 24"><path fill="currentColor" d="M6 6h2v12H6zm3.5 6l8.5 6V6z"/></svg>
      </button>
      <button class="cbtn play" title={player.playing ? t('player.pause') : t('player.play')}
              onclick={() => player.playPause()}>
        {#key player.playing}
          <span class="picon">
            {#if player.playing}
              <svg width="24" height="24" viewBox="0 0 24 24"><path fill="currentColor" d="M6 19h4V5H6v14zm8-14v14h4V5h-4z"/></svg>
            {:else}
              <svg width="24" height="24" viewBox="0 0 24 24"><path fill="currentColor" d="M8 5v14l11-7z"/></svg>
            {/if}
          </span>
        {/key}
      </button>
      <button class="cbtn tonal" title={t('player.next')} onclick={() => player.next()}>
        <svg width="20" height="20" viewBox="0 0 24 24"><path fill="currentColor" d="M6 18l8.5-6L6 6v12zM16 6v12h2V6h-2z"/></svg>
      </button>
    </div>
    <div class="progress">
      <span class="time">{fmt(player.pos)}</span>
      <div class="slider" bind:this={box} bind:clientWidth={w}
           onpointerdown={seek}
           onpointerenter={() => (hovering = true)}
           onpointerleave={() => (hovering = false)}
           onkeydown={keySeek} tabindex="0"
           role="slider" aria-label={t('player.position')}
           aria-valuemin="0" aria-valuemax={dur} aria-valuenow={player.pos}>
        <svg width={w} height={H} viewBox={`0 0 ${w} ${H}`}>
          <line bind:this={lineEl} x1="0" y1={H / 2} x2={w} y2={H / 2}
                stroke="var(--md-outline-variant)" stroke-width={THICK}
                stroke-linecap="round" opacity="0.5" />
          <path bind:this={pathEl} fill="none"
                stroke="var(--md-primary)" stroke-width={THICK}
                stroke-linecap="round" />
          {#if hovering && dur > 0}
            <circle bind:this={dotEl} cy={H / 2} r="5" fill="var(--md-primary)" />
          {/if}
        </svg>
      </div>
      <span class="time">{fmt(dur)}</span>
    </div>
  </div>

  <div class="side">
    <button class="cbtn" title={player.volume === 0 ? t('player.unmute') : t('player.mute')}
            onclick={() => player.toggleMute()}>
      {#if player.volume === 0}
        <svg width="20" height="20" viewBox="0 0 24 24"><path fill="currentColor" d="M16.5 12c0-1.77-1.02-3.29-2.5-4.03v2.21l2.45 2.45c.03-.2.05-.41.05-.63zm2.5 0c0 .94-.2 1.82-.54 2.64l1.51 1.51A8.8 8.8 0 0 0 21 12c0-4.28-2.99-7.86-7-8.77v2.06c2.89.86 5 3.54 5 6.71zM4.27 3L3 4.27 7.73 9H3v6h4l5 5v-6.73l4.25 4.25c-.67.52-1.42.93-2.25 1.18v2.06a8.99 8.99 0 0 0 3.69-1.81L19.73 21 21 19.73l-9-9L4.27 3zM12 4L9.91 6.09 12 8.18V4z"/></svg>
      {:else}
        <svg width="20" height="20" viewBox="0 0 24 24"><path fill="currentColor" d="M3 9v6h4l5 5V4L7 9H3zm13.5 3A4.5 4.5 0 0 0 14 7.97v8.05A4.47 4.47 0 0 0 16.5 12zM14 3.23v2.06c2.89.86 5 3.54 5 6.71s-2.11 5.85-5 6.71v2.06c4.01-.91 7-4.49 7-8.77s-2.99-7.86-7-8.77z"/></svg>
      {/if}
    </button>
    <input class="vol" type="range" min="0" max="1" step="0.01"
           style={`--fill: ${Math.round(player.volume * 100)}%`}
           value={player.volume}
           aria-label={t('player.volume')}
           oninput={(e) => player.setVolume(Number((e.target as HTMLInputElement).value))} />
  </div>
</footer>

<style>
  .pb {
    height: 96px;
    padding: 10px 20px;
    flex-shrink: 0;
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: center;
    gap: 16px;
    background: var(--md-surface-container);
    border-top: 2px solid color-mix(in srgb, var(--md-outline) 60%, transparent);
  }

  .meta { display: flex; align-items: center; gap: 16px; min-width: 0; }
  .art { width: 52px; height: 52px; object-fit: cover; flex-shrink: 0; transition: transform .3s cubic-bezier(.2, 0, 0, 1); }
  .art.empty {
    background: var(--md-surface-container-highest);
    display: grid;
    place-items: center;
    color: var(--md-on-surface-variant);
    opacity: .55;
  }
  .art.beat { animation: beat 3.2s ease-in-out infinite; }
  @keyframes beat {
    0%, 100% { transform: scale(1); }
    50% { transform: scale(1.04) rotate(1.5deg); }
  }
  .qmark { animation: floaty 3s ease-in-out infinite; }
  @keyframes floaty {
    0%, 100% { transform: translateY(0); }
    50% { transform: translateY(-2px); }
  }
  .txt { display: flex; flex-direction: column; min-width: 0; }
  .t { font-weight: 500; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .a { font-size: 12px; color: var(--md-on-surface-variant); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }

  .center { display: flex; flex-direction: column; align-items: center; gap: 0; }

  .controls { display: flex; align-items: center; gap: 10px; padding-bottom: 0; }
  .cbtn {
    height: 40px;
    display: grid; place-items: center;
    color: var(--md-on-surface-variant);
    transition: background .15s, color .15s,
                border-radius .25s cubic-bezier(.2, 0, 0, 1),
                transform .2s, filter .15s, box-shadow .15s;
  }
  .cbtn.tonal {
    width: 44px;
    border-radius: 15px;
    background: var(--md-secondary-container);
    color: var(--md-on-secondary-container);
  }
  .cbtn.tonal:hover { border-radius: 50%; filter: brightness(1.06); }
  .cbtn.tonal:active { transform: scale(.92); }

  .cbtn.play {
    width: 52px; height: 52px;
    border-radius: 14px;
    background: var(--md-primary);
    color: var(--md-on-primary);
  }
  .cbtn.play:hover {
    border-radius: 50%;
    filter: brightness(1.08);
    transform: scale(1.04);
  }
  .cbtn.play:active { transform: scale(.94); border-radius: 50%; }

  .picon { display: grid; place-items: center; animation: iconpop .25s cubic-bezier(.2, 0, 0, 1); }
  @keyframes iconpop {
    0% { transform: scale(.6) rotate(-25deg); opacity: 0; }
    100% { transform: scale(1) rotate(0); opacity: 1; }
  }

  .progress { display: flex; align-items: center; gap: 10px; width: 460px; max-width: 40vw; }
  .time { font-size: 11px; color: var(--md-on-surface-variant); font-variant-numeric: tabular-nums; min-width: 34px; }
  .time:last-child { text-align: right; }

  .slider {
    flex: 1;
    display: block;
    padding: 4px 0;
    cursor: pointer;
    border-radius: 8px;
    outline: none;
  }
  .slider:focus-visible { box-shadow: 0 0 0 2px color-mix(in srgb, var(--md-primary) 60%, transparent); }

  .side { display: flex; align-items: center; gap: 6px; justify-content: flex-end; }

  .vol {
    -webkit-appearance: none;
    appearance: none;
    width: 96px; height: 20px;
    margin: 0;
    border: none;
    outline: none;
    cursor: pointer;
    border-radius: 10px;
    background: linear-gradient(
        to right,
        var(--md-primary) var(--fill, 100%),
        color-mix(in srgb, var(--md-outline-variant) 55%, transparent) var(--fill, 100%)
      )
      center / calc(100% - 14px) 4px no-repeat;
  }
  .vol::-webkit-slider-thumb {
    -webkit-appearance: none;
    width: 14px; height: 14px;
    border-radius: 50%;
    border: none;
    background: var(--md-primary);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--md-primary) 22%, transparent);
    transition: transform .15s, box-shadow .15s;
  }
  .vol:hover::-webkit-slider-thumb { transform: scale(1.25); }
  .vol:active::-webkit-slider-thumb { transform: scale(1.1); box-shadow: 0 0 0 5px color-mix(in srgb, var(--md-primary) 22%, transparent); }
</style>

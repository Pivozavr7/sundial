<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import { t } from '../lib/i18n.svelte.ts';

  let { children } = $props();

  // 'csd' | 'ssd' | 'unknown'; при ssd свои кнопки не рисуем
  let mode = $state<'csd' | 'ssd' | 'unknown'>('unknown');

  onMount(async () => {
    try { mode = await invoke('window_deco_mode'); } catch {}
  });

  const act = (a: string) => invoke('window_action', { action: a });
</script>

<header class="tb" data-tauri-drag-region>
  <div class="brand" data-tauri-drag-region>
    <svg width="18" height="18" viewBox="0 0 512 512" aria-hidden="true">
      <path d="M 307.3 115 A 150 150 0 1 1 204.7 115" fill="none"
            stroke="var(--md-primary)" stroke-width="54" stroke-linecap="round"/>
      <path d="M 224 204 L 224 308 L 308 256 Z" fill="var(--md-primary)"/>
    </svg>
    <span data-tauri-drag-region>Sundial</span>
  </div>

  <div class="center-slot">
    {@render children?.()}
  </div>

  <div class="spacer" data-tauri-drag-region></div>

  {#if mode === 'unknown'}
  <button class="wbtn" title={t('window.min')} onclick={() => act('min')}>
      <svg width="12" height="12" viewBox="0 0 12 12"><path d="M1 6h10" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/></svg>
    </button>
    <button class="wbtn" title={t('window.max')} onclick={() => act('max')}>
      <svg width="12" height="12" viewBox="0 0 12 12"><rect x="1.8" y="1.8" width="8.4" height="8.4" rx="1.5" fill="none" stroke="currentColor" stroke-width="1.6"/></svg>
    </button>
    <button class="wbtn close" title={t('window.close')} onclick={() => act('close')}>
      <svg width="12" height="12" viewBox="0 0 12 12"><path d="M2 2l8 8M10 2l-8 8" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/></svg>
    </button>
  {/if}
</header>

<style>
  .tb {
    height: 48px;
    display: flex; align-items: center; gap: 12px;
    position: relative
    flex-shrink: 0;
    background: var(--md-surface);
  }
  .brand { display: flex; align-items: center; gap: 8px; padding-left: 14px; font-weight: 500; }
  .spacer { flex: 1; align-self: stretch; }
  .wbtn {
    width: 44px; height: 48px;
    display: grid; place-items: center;
    color: var(--md-on-surface-variant);
    transition: background .12s;
  }
  .wbtn:hover { background: color-mix(in srgb, var(--md-on-surface) 8%, transparent); }
  .wbtn.close:hover { background: #E8354A; color: #fff; }
  .center-slot {
    position: absolute;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
  }
</style>

<script lang="ts">
    let { size = 420, color = 'var(--md-primary-container)', opacity = 0.35, n = 9, speed = 1, breathe = true } = $props();
    const uiFps = (window as any).__pearSettings?.uiFps ?? 30;
  let d = $state('');
  let rot = 0;
  let t = 0;
  let raf = 0;
  let prev = 0;
  let acc = 0;

  const TAU = Math.PI * 2;

  function build() {
    const C = 150;
    const rBase = 95;
    const rPeak = 122;
    const sigma = 0.16;
    const step = TAU / n;
    const samples = n * 36;

    const wrap = (a: number) => {
      a = ((a % TAU) + TAU) % TAU;
      return a > Math.PI ? a - TAU : a;
    };

    let p = '';
    for (let s = 0; s <= samples; s++) {
      const a = (s / samples) * TAU;
      let bump = 0;
      for (let i = 0; i < n; i++) {
        const dist = wrap(a - (rot + i * step));
        const amp = breathe ? 1 + 0.06 * Math.sin(t * 1.2 + i * 0.7) : 1;
        bump = Math.max(bump, amp * Math.exp(-(dist * dist) / (2 * sigma * sigma)));
      }
      const r = rBase + (rPeak - rBase) * bump;
      const x = C + r * Math.cos(a);
      const y = C + r * Math.sin(a);
      p += (s === 0 ? 'M' : 'L') + ` ${x.toFixed(1)} ${y.toFixed(1)}`;
    }
    d = p + ' Z';
  }

  function frame(now: number) {
    const dt = prev ? Math.min(0.05, (now - prev) / 1000) : 0.016;
    prev = now;
    if (!document.hidden) {
      t += dt * speed;
      rot += dt * 0.1 * speed;
      acc += dt;
      if (acc > 1 / uiFps) { acc = 0; build(); }
    }
    raf = requestAnimationFrame(frame);
  }

  $effect(() => {
    build();
    raf = requestAnimationFrame(frame);
    return () => cancelAnimationFrame(raf);
  });
</script>

<svg viewBox="0 0 300 300" width={size} height={size} style={`opacity:${opacity}`} aria-hidden="true">
  <path {d} fill={color} />
</svg>

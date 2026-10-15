
export type YtState = { onState: (s: number) => void; onError: (code: number) => void };

let api: Promise<any> | null = null;

function loadApi(): Promise<any> {
  if (!api) {
    api = new Promise((resolve, reject) => {
      const t = setTimeout(() => reject(new Error('YouTube IFrame API недоступна')), 12000);
      (window as any).onYouTubeIframeAPIReady = () => { clearTimeout(t); resolve((window as any).YT); };
      const s = document.createElement('script');
      s.src = 'https://www.youtube.com/iframe_api';
      s.onerror = () => { clearTimeout(t); reject(new Error('не загрузить iframe_api')); };
      document.head.append(s);
    });
  }
  return api;
}

export async function createPlayer(ev: YtState): Promise<any> {
  const YT = await loadApi();
  return new Promise((resolve, reject) => {
    const host = document.createElement('div');
    host.id = 'yt-host';
    host.style.cssText = 'position:fixed;left:0;bottom:0;width:1px;height:1px;opacity:.01;pointer-events:none;overflow:hidden;';
    document.body.append(host);
    const t = setTimeout(() => reject(new Error('плеер не инициализировался')), 12000);
    new YT.Player('yt-host', {
      width: 1, height: 1,
      playerVars: { autoplay: 0, controls: 0, disablekb: 1, playsinline: 1 },
      events: {
        onReady: (e: any) => { clearTimeout(t); resolve(e.target); },
        onStateChange: (e: any) => ev.onState(e.data),
        onError: (e: any) => ev.onError(e.data),
      },
    });
  });
}

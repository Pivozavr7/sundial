import en from './locales/en.json';
import ru from './locales/ru.json';

const dicts: Record<string, any> = { en, ru };
export type Lang = 'en' | 'ru';

export const locale = $state<{ current: Lang }>({
  current:
    ((window as any).__pearSettings?.language as Lang) ??
    (localStorage.getItem('sundial-lang') as Lang) ??
    'en',
});

document.documentElement.lang = locale.current;

const translate = $derived((key: string, vars?: Record<string, string | number>): string => {
  const d = dicts[locale.current] ?? dicts.en;
  let s = key.split('.').reduce((o: any, k) => o?.[k], d) ?? key;
  if (vars) for (const [k, v] of Object.entries(vars)) s = s.replaceAll(`{${k}}`, String(v));
  return s;
});

export function t(key: string, vars?: Record<string, string | number>): string {
  return translate(key, vars);
}

export function setLocale(l: Lang) {
  locale.current = l;
  localStorage.setItem('sundial-lang', l);
  document.documentElement.lang = l;
}

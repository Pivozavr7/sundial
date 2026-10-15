import { argbFromHex, hexFromArgb, themeFromSourceColor } from '@material/material-color-utilities';

const kebab = (s: string) => s.replace(/[A-Z]/g, (c) => '-' + c.toLowerCase());

export function applySeed(seed: string, dark: boolean, darkAccent?: string) {
  const theme = themeFromSourceColor(argbFromHex(seed));
  const scheme = theme.schemes[dark ? 'dark' : 'light'].toJSON();
  const r = document.documentElement;
  for (const [role, argb] of Object.entries(scheme)) {
    r.style.setProperty(`--md-${kebab(role)}`, hexFromArgb(argb as number));
  }
  if (dark && darkAccent) {
    const on = themeFromSourceColor(argbFromHex(darkAccent)).schemes.dark.toJSON();
    r.style.setProperty('--md-primary', hexFromArgb(on.primary));
    r.style.setProperty('--md-on-primary', hexFromArgb(on.onPrimary));
  }
  r.style.colorScheme = dark ? 'dark' : 'light';
  document.body.dataset.theme = dark ? 'dark' : 'light';
}

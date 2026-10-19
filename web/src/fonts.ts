import type { Language } from './i18n';

// The Noto styles declare about 100 Unicode-range faces per weight. Each Chinese page loads
// only its own styles, so the English page does not download or parse about 600 faces.
const STYLES: Record<Exclude<Language, 'en'>, () => Promise<unknown>> = {
  'zh-Hans': () =>
    Promise.all([
      import('@fontsource/noto-sans-sc/400.css'),
      import('@fontsource/noto-sans-sc/500.css'),
      import('@fontsource/noto-sans-sc/700.css'),
    ]),
  'zh-Hant': () =>
    Promise.all([
      import('@fontsource/noto-sans-tc/400.css'),
      import('@fontsource/noto-sans-tc/500.css'),
      import('@fontsource/noto-sans-tc/700.css'),
    ]),
};

const loaded = new Set<Language>();

/**
 * Loads the font styles that `language` needs. Until they apply, and if they fail to load,
 * the text uses a system font.
 */
export function loadFonts(language: Language): void {
  if (language === 'en' || loaded.has(language)) return;
  loaded.add(language);
  STYLES[language]().catch(() => loaded.delete(language));
}

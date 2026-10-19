import { describe, expect, it } from 'vitest';
import { MODESET_TOKEN } from './cameras';
import { matchLanguage, STRINGS } from './i18n';

describe('matchLanguage', () => {
  it('maps Chinese tags by script and region', () => {
    expect(matchLanguage(['zh-CN'])).toBe('zh-Hans');
    expect(matchLanguage(['zh'])).toBe('zh-Hans');
    expect(matchLanguage(['zh-Hant-TW'])).toBe('zh-Hant');
    expect(matchLanguage(['zh-HK'])).toBe('zh-Hant');
    expect(matchLanguage(['zh-TW'])).toBe('zh-Hant');
  });

  it('maps Japanese tags', () => {
    expect(matchLanguage(['ja'])).toBe('ja');
    expect(matchLanguage(['ja-JP'])).toBe('ja');
  });

  it('uses the first supported tag and falls back to English', () => {
    expect(matchLanguage(['ja-JP', 'zh-TW', 'en'])).toBe('ja');
    expect(matchLanguage(['ko-KR', 'zh-TW', 'ja'])).toBe('zh-Hant');
    expect(matchLanguage(['fr', 'ja', 'en'])).toBe('ja');
    expect(matchLanguage(['fr', 'en-GB'])).toBe('en');
    expect(matchLanguage(['ko'])).toBe('en');
    expect(matchLanguage([])).toBe('en');
  });
});

describe('STRINGS', () => {
  it('gives each language the same install steps and effect looks', () => {
    const shape = (language: keyof typeof STRINGS) => ({
      steps: STRINGS[language].steps.map((parts) => parts.filter((p) => p.startsWith('`'))),
      looks: Object.keys(STRINGS[language].looks).sort(),
    });
    expect(shape('zh-Hans')).toEqual(shape('en'));
    expect(shape('zh-Hant')).toEqual(shape('en'));
    expect(shape('ja')).toEqual(shape('en'));
  });

  it('names the ModeSet file through the camera token once in each language', () => {
    for (const language of Object.keys(STRINGS) as (keyof typeof STRINGS)[]) {
      const parts = STRINGS[language].steps.flat();
      expect(parts.filter((part) => part === MODESET_TOKEN)).toHaveLength(1);
    }
  });
});

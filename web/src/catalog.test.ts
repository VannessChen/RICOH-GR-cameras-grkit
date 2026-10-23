import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const catalog = {
  schema: 'grkit-site-effects-v1',
  target: 'gr3-2.10',
  effects: [],
  preview: { input: 'linear-srgb16', shaper: 'srgb', order: 'rgb', size: 2, baseline: 'base.lut' },
};

beforeEach(() => {
  vi.resetModules();
  vi.stubGlobal('document', { baseURI: 'https://grkit.test/' });
});
afterEach(() => {
  vi.unstubAllGlobals();
});

describe('loadCatalog', () => {
  it('requests the catalog again after a failure', async () => {
    const fetch = vi
      .fn()
      .mockRejectedValueOnce(new TypeError('offline'))
      .mockImplementation(async () => Response.json(catalog));
    vi.stubGlobal('fetch', fetch);
    const { loadCatalog } = await import('./catalog');

    const failed = loadCatalog('gr3-2.10');
    await expect(failed).rejects.toThrow('offline');
    const loaded = loadCatalog('gr3-2.10');
    expect(loaded).not.toBe(failed);
    await expect(loaded).resolves.toMatchObject({ target: 'gr3-2.10', lutSize: 2 });
    // A successful request stays cached.
    expect(loadCatalog('gr3-2.10')).toBe(loaded);
    expect(fetch).toHaveBeenCalledTimes(2);
  });
});

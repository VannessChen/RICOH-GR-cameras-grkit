import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { Source } from './render';

const image = { width: 1, height: 1, data: new Uint16Array([100, 200, 300]) };
const effect = { id: 'film', table: '/film.lut', baseline: '/standard.lut', size: 2 };

vi.mock('./renderer', () => ({ workerRender: () => async () => new Blob(['preview']) }));

beforeEach(() => {
  vi.resetModules();
  vi.stubGlobal(
    'fetch',
    vi.fn(async () => new Response(new Uint8Array(24))),
  );
  vi.spyOn(URL, 'createObjectURL');
  vi.spyOn(URL, 'revokeObjectURL');
});
afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe('preview lifetime', () => {
  it('shares decoded pixels and the baseline across effects', async () => {
    const { preview, forget } = await import('./render');
    const source: Source = { key: 'sample', load: vi.fn(async () => image) };
    const first = preview(source, effect);
    expect(preview(source, effect)).toBe(first);
    const a = await first;
    // Different target URLs with the same effect ID must not reuse the result.
    const b = await preview(source, { ...effect, table: '/other-target/film.lut' });
    expect(b.original).toBe(a.original);
    expect(b.processed).not.toBe(a.processed);
    expect(source.load).toHaveBeenCalledOnce();
    expect(fetch).toHaveBeenCalledTimes(3);
    forget(source);
    await Promise.resolve();
    expect(URL.revokeObjectURL).toHaveBeenCalledTimes(3);
  });

  it('keeps rejected promises stable so Suspense reaches the error boundary', async () => {
    const { preview, forget } = await import('./render');
    const source: Source = {
      key: 'broken',
      load: async () => {
        throw new Error('Broken sample');
      },
    };
    const result = preview(source, effect);
    await expect(result).rejects.toThrow('Broken sample');
    expect(preview(source, effect)).toBe(result);
    forget(source);
  });

  it('releases URLs that complete after a source is replaced', async () => {
    const { preview, forget } = await import('./render');
    let finish!: (value: typeof image) => void;
    const pending = new Promise<typeof image>((resolve) => {
      finish = resolve;
    });
    const source: Source = { key: 'replaced', load: () => pending };
    const result = preview(source, effect);
    forget(source);
    finish(image);
    const done = await result;
    await Promise.resolve();
    expect(URL.revokeObjectURL).toHaveBeenCalledWith(done.original);
    expect(URL.revokeObjectURL).toHaveBeenCalledWith(done.processed);
  });

  it('recovers from a failed baseline request when the user retries', async () => {
    const { preview, forget } = await import('./render');
    let baselineRequests = 0;
    vi.stubGlobal(
      'fetch',
      vi.fn(async (url: string) => {
        if (url === effect.baseline && ++baselineRequests === 1) throw new Error('Offline');
        return new Response(new Uint8Array(24));
      }),
    );
    const source: Source = { key: 'retry', load: async () => image };
    const failed = preview(source, effect);
    await expect(failed).rejects.toThrow('Offline');
    expect(preview(source, effect)).toBe(failed);
    forget(source);
    const retried = await preview(source, effect);
    expect(retried.original).toMatch(/^blob:/);
    expect(baselineRequests).toBe(2);
    forget(source);
  });
});

import { afterEach, describe, expect, it, vi } from 'vitest';
import { loadRaw, MAX_RAW_BYTES, PhotoLoader, validateRawFile } from './raw';
import type { LinearPicture } from './linear';

const image: LinearPicture = { width: 1, height: 1, data: new Uint16Array([1, 2, 3]) };
const file = new File([new Uint8Array(16)], 'photo.DNG');

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}

afterEach(() => {
  vi.unstubAllGlobals();
  vi.useRealTimers();
});

describe('RAW uploads', () => {
  it('accepts a DNG name and rejects other extensions and oversized files', () => {
    expect(() => validateRawFile(file)).not.toThrow();
    expect(() => validateRawFile({ name: 'photo.jpg', size: 16 })).toThrow();
    expect(() => validateRawFile({ name: 'photo.dng', size: MAX_RAW_BYTES + 1 })).toThrow();
    expect(() => validateRawFile({ name: 'photo.dng', size: 0 })).toThrow();
  });

  it('ignores an older success after another photo was selected', async () => {
    const first = deferred<LinearPicture>();
    const second = deferred<LinearPicture>();
    const decode = vi.fn().mockReturnValueOnce(first.promise).mockReturnValueOnce(second.promise);
    const loader = new PhotoLoader(decode);
    const a = loader.open(file);
    const signal = decode.mock.calls[0]![1] as AbortSignal;
    const b = loader.open(file);
    expect(signal.aborted).toBe(true);
    second.resolve(image);
    expect(await b).toBe(image);
    first.resolve(image);
    expect(await a).toBeNull();
  });

  it('ignores a pending error after returning to the sample or unmounting', async () => {
    const pending = deferred<LinearPicture>();
    const loader = new PhotoLoader(() => pending.promise);
    const result = loader.open(file);
    loader.cancel();
    pending.reject(new Error('Old failure'));
    expect(await result).toBeNull();
  });

  it('reports a current error and permits retry', async () => {
    const decode = vi
      .fn()
      .mockRejectedValueOnce(new Error('Decode failed'))
      .mockResolvedValueOnce(image);
    const loader = new PhotoLoader(decode);
    await expect(loader.open(file)).rejects.toThrow('Decode failed');
    expect(await loader.open(file)).toBe(image);
  });

  it('terminates a worker after success, cancellation, a crash, or timeout', async () => {
    vi.useFakeTimers();
    const workers: FakeWorker[] = [];
    class FakeWorker {
      onmessage?: (event: { data: { image: LinearPicture } }) => void;
      onerror?: () => void;
      postMessage = vi.fn();
      terminate = vi.fn();
      constructor() {
        workers.push(this);
      }
    }
    vi.stubGlobal('Worker', FakeWorker);
    for (const mode of ['success', 'cancel', 'error', 'timeout']) {
      const controller = new AbortController();
      const pending = loadRaw(file, controller.signal);
      const outcome = pending.then(
        () => 'ok',
        () => 'error',
      );
      await vi.waitFor(() => expect(workers.at(-1)?.postMessage).toHaveBeenCalled());
      const worker = workers.at(-1)!;
      if (mode === 'success') worker.onmessage?.({ data: { image } });
      if (mode === 'cancel') controller.abort();
      if (mode === 'error') worker.onerror?.();
      if (mode === 'timeout') await vi.advanceTimersByTimeAsync(60_000);
      expect(await outcome).toBe(mode === 'success' ? 'ok' : 'error');
      expect(worker.terminate).toHaveBeenCalledOnce();
      workers.pop();
    }
  });
});

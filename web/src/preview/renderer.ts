import type { LinearPicture } from './linear';
import type { RenderRequest, RenderResponse } from './render.worker';

/** Applies a preview table to linear pixels and encodes the result as a JPEG. */
export type Render = (image: LinearPicture, lut: Uint8Array, size: number) => Promise<Blob>;

interface Pending {
  resolve: (blob: Blob) => void;
  reject: (error: Error) => void;
}

/**
 * Renders in a worker, so the table pass and the JPEG encoder do not block scrolling or input.
 * Each request copies the pixels (about 6 MB) and leaves the caller's copy intact.
 * If the worker fails, its requests reject and the next request starts a new worker.
 */
export function workerRender(): Render {
  let worker: Worker | undefined;
  const pending = new Map<number, Pending>();
  let next = 0;

  const start = (): Worker => {
    const started = new Worker(new URL('./render.worker.ts', import.meta.url), { type: 'module' });
    const fail = (message: string) => {
      if (worker === started) worker = undefined;
      started.terminate();
      for (const request of pending.values()) request.reject(new Error(message));
      pending.clear();
    };
    started.onmessage = ({ data }: MessageEvent<RenderResponse>) => {
      const request = pending.get(data.id);
      if (!request) return;
      pending.delete(data.id);
      if (data.blob) request.resolve(data.blob);
      else request.reject(new Error(data.error ?? 'Invalid renderer response'));
    };
    started.onerror = () => fail('Preview renderer failed');
    started.onmessageerror = () => fail('Invalid renderer response');
    return started;
  };

  return (image, lut, size) =>
    new Promise((resolve, reject) => {
      worker ??= start();
      const id = ++next;
      pending.set(id, { resolve, reject });
      worker.postMessage({ id, image, lut, size } satisfies RenderRequest);
    });
}

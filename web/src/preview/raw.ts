import type { LinearPicture } from './linear';

const DECODE_TIMEOUT = 60_000;

/** Any DNG. The decoder bounds the sensor size, so the file size needs no limit. */
export function validateRawFile(file: Pick<File, 'name' | 'size'>): void {
  if (!/\.dng$/i.test(file.name) || file.size < 8) {
    throw new Error('Choose a DNG file');
  }
}

export async function loadRaw(file: File, signal: AbortSignal): Promise<LinearPicture> {
  validateRawFile(file);
  signal.throwIfAborted();
  const bytes = await file.arrayBuffer();
  signal.throwIfAborted();
  return new Promise((resolve, reject) => {
    const worker = new Worker(new URL('./raw.worker.ts', import.meta.url), { type: 'module' });
    const finish = (image?: LinearPicture, error?: unknown) => {
      clearTimeout(timeout);
      signal.removeEventListener('abort', abort);
      worker.terminate();
      if (image) resolve(image);
      else reject(error);
    };
    const abort = () => finish(undefined, signal.reason);
    const timeout = setTimeout(
      () => finish(undefined, new Error('RAW decode timed out')),
      DECODE_TIMEOUT,
    );
    signal.addEventListener('abort', abort, { once: true });
    worker.onmessage = ({ data }: MessageEvent<{ image?: LinearPicture; error?: string }>) => {
      finish(data.image, new Error(data.error ?? 'Invalid decoder response'));
    };
    worker.onerror = () => finish(undefined, new Error('RAW decoder failed'));
    worker.onmessageerror = () => finish(undefined, new Error('Invalid decoder response'));
    worker.postMessage(bytes, [bytes]);
  });
}

/** Only the newest request can change the displayed photo or its error state. */
export class PhotoLoader {
  private current?: AbortController;

  constructor(private readonly decode = loadRaw) {}

  cancel(): void {
    this.current?.abort();
    this.current = undefined;
  }

  async open(file: File): Promise<LinearPicture | null> {
    this.cancel();
    const controller = new AbortController();
    this.current = controller;
    try {
      const image = await this.decode(file, controller.signal);
      return this.current === controller ? image : null;
    } catch (error) {
      if (this.current !== controller) return null;
      throw error;
    } finally {
      if (this.current === controller) this.current = undefined;
    }
  }
}

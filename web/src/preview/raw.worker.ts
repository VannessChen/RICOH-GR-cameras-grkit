import wasmUrl from '@colorhythm/libraw-wasm/libraw.wasm?url';
import { decodeRaw } from './raw-decode';
import { resizeLinear } from './linear';

// Each upload owns one worker. Termination releases the decoder's entire WASM heap.
self.onmessage = async ({ data }: MessageEvent<ArrayBuffer>) => {
  try {
    const response = await fetch(wasmUrl);
    if (!response.ok) throw new Error('Cannot load RAW decoder');
    const image = resizeLinear(await decodeRaw(new Uint8Array(data), await response.arrayBuffer()));
    self.postMessage({ image }, { transfer: [image.data.buffer] });
  } catch (error) {
    self.postMessage({ error: error instanceof Error ? error.message : 'Cannot decode RAW photo' });
  }
};

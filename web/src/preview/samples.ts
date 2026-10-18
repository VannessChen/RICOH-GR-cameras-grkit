import { siteUrl } from '../catalog';
import { decodeLinear, MAX_EDGE } from './linear';
import type { Source } from './render';

/** The sample is developed from a CC0 GR III DNG with the same settings as uploads. */
export const DEFAULT_SAMPLE: Source = {
  key: 'gr-iii-cc0-v1',
  load: async () => {
    const response = await fetch(siteUrl('samples/gr-iii.grlinear'));
    if (!response.ok || !response.body) throw new Error('Cannot load sample photo');
    const reader = response.body.pipeThrough(new DecompressionStream('gzip')).getReader();
    const chunks: Uint8Array[] = [];
    let length = 0;
    try {
      while (true) {
        const { value, done } = await reader.read();
        if (done) break;
        length += value.byteLength;
        if (length > 16 + MAX_EDGE * MAX_EDGE * 6) throw new Error('Sample photo is too large');
        chunks.push(value);
      }
    } finally {
      await reader.cancel();
      reader.releaseLock();
    }
    const bytes = new Uint8Array(length);
    let offset = 0;
    for (const chunk of chunks) {
      bytes.set(chunk, offset);
      offset += chunk.byteLength;
    }
    return decodeLinear(bytes.buffer);
  },
};

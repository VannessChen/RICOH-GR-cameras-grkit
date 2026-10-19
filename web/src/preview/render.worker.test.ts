import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { RenderRequest, RenderResponse } from './render.worker';

const image = { width: 1, height: 1, data: new Uint16Array([0, 32768, 65535]) };
const posted = vi.fn<(response: RenderResponse) => void>();
let drawn: Uint8ClampedArray | undefined;

beforeEach(() => {
  vi.resetModules();
  posted.mockClear();
  drawn = undefined;
  vi.stubGlobal('self', { postMessage: posted });
  vi.stubGlobal(
    'ImageData',
    class {
      constructor(readonly data: Uint8ClampedArray) {}
    },
  );
  vi.stubGlobal(
    'OffscreenCanvas',
    class {
      getContext() {
        return { putImageData: (pixels: { data: Uint8ClampedArray }) => (drawn = pixels.data) };
      }
      async convertToBlob() {
        return new Blob(['preview']);
      }
    },
  );
});
afterEach(() => vi.unstubAllGlobals());

async function send(request: RenderRequest): Promise<RenderResponse> {
  await import('./render.worker');
  const handler = (self as unknown as { onmessage: (event: { data: RenderRequest }) => unknown })
    .onmessage;
  await handler({ data: request });
  return posted.mock.calls[0]![0];
}

describe('preview render worker', () => {
  it('answers a request with the encoded preview of the table applied to the pixels', async () => {
    // An identity table with two points: grid point (r, g, b) holds its own coordinates.
    const identity = new Uint8Array(24);
    for (let i = 0; i < 8; i++) {
      identity.set([i & 1, (i >> 1) & 1, i >> 2].map((v) => v * 255), i * 3);
    }
    const response = await send({ id: 7, image, lut: identity, size: 2 });
    expect(response.id).toBe(7);
    expect(response.blob).toBeInstanceOf(Blob);
    // Linear 0.5 is sRGB 0.735, which is 188 of 255. Alpha becomes opaque.
    expect(Array.from(drawn!)).toEqual([0, 188, 255, 255]);
  });

  it('answers a bad table with an error for the same request', async () => {
    const response = await send({ id: 3, image, lut: new Uint8Array(5), size: 2 });
    expect(response).toEqual({ id: 3, error: 'Preview table size mismatch' });
  });
});

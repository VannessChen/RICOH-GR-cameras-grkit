import type { LinearPicture } from './linear';
import { applyLinearLut } from './lut';

export interface RenderRequest {
  id: number;
  image: LinearPicture;
  lut: Uint8Array;
  size: number;
}

export interface RenderResponse {
  id: number;
  blob?: Blob;
  error?: string;
}

async function render({ image, lut, size }: RenderRequest): Promise<Blob> {
  const pixels = new ImageData(applyLinearLut(image.data, lut, size), image.width, image.height);
  const canvas = new OffscreenCanvas(image.width, image.height);
  const context = canvas.getContext('2d');
  if (!context) throw new Error('2D canvas is not available');
  context.putImageData(pixels, 0, 0);
  return canvas.convertToBlob({ type: 'image/jpeg', quality: 0.9 });
}

self.onmessage = async ({ data }: MessageEvent<RenderRequest>) => {
  let response: RenderResponse;
  try {
    response = { id: data.id, blob: await render(data) };
  } catch (error) {
    response = { id: data.id, error: error instanceof Error ? error.message : 'Render failed' };
  }
  self.postMessage(response);
};

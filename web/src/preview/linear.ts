/** Camera-white-balanced, scene-linear sRGB, with unsigned 16-bit RGB samples. */
export interface LinearPicture {
  width: number;
  height: number;
  data: Uint16Array;
}

export const MAX_EDGE = 1200;
const MAGIC = 'GRLIN001';

export function validatePicture(image: LinearPicture, maxPixels = 30_000_000): void {
  const { width, height, data } = image;
  if (
    !Number.isInteger(width) ||
    !Number.isInteger(height) ||
    width < 1 ||
    height < 1 ||
    width * height > maxPixels ||
    !(data instanceof Uint16Array) ||
    data.length !== width * height * 3
  ) {
    throw new Error('Invalid linear image');
  }
}

/** Area resampling in linear light, before tone curves or 8-bit quantization. */
export function resizeLinear(image: LinearPicture): LinearPicture {
  validatePicture(image);
  const scale = Math.min(1, MAX_EDGE / Math.max(image.width, image.height));
  if (scale === 1) return image;
  const width = Math.max(1, Math.round(image.width * scale));
  const height = Math.max(1, Math.round(image.height * scale));
  const data = new Uint16Array(width * height * 3);
  const sx = image.width / width;
  const sy = image.height / height;
  for (let y = 0; y < height; y++) {
    const top = y * sy;
    const bottom = (y + 1) * sy;
    for (let x = 0; x < width; x++) {
      const left = x * sx;
      const right = (x + 1) * sx;
      const sum = [0, 0, 0];
      for (let iy = Math.floor(top); iy < Math.min(image.height, Math.ceil(bottom)); iy++) {
        const wy = Math.min(bottom, iy + 1) - Math.max(top, iy);
        for (let ix = Math.floor(left); ix < Math.min(image.width, Math.ceil(right)); ix++) {
          const weight = wy * (Math.min(right, ix + 1) - Math.max(left, ix));
          const source = (iy * image.width + ix) * 3;
          for (let c = 0; c < 3; c++) sum[c] = sum[c]! + image.data[source + c]! * weight;
        }
      }
      const target = (y * width + x) * 3;
      for (let c = 0; c < 3; c++) data[target + c] = Math.round(sum[c]! / (sx * sy));
    }
  }
  return { width, height, data };
}

/** GRLIN001: eight magic bytes, LE u32 width/height, then LE u16 RGB pixels. */
export function encodeLinear(image: LinearPicture): Uint8Array {
  validatePicture(image, MAX_EDGE * MAX_EDGE);
  const bytes = new Uint8Array(16 + image.data.length * 2);
  bytes.set(new TextEncoder().encode(MAGIC));
  const view = new DataView(bytes.buffer);
  view.setUint32(8, image.width, true);
  view.setUint32(12, image.height, true);
  image.data.forEach((value, i) => view.setUint16(16 + i * 2, value, true));
  return bytes;
}

export function decodeLinear(bytes: ArrayBuffer): LinearPicture {
  if (bytes.byteLength < 16 || new TextDecoder().decode(bytes.slice(0, 8)) !== MAGIC) {
    throw new Error('Invalid linear sample header');
  }
  const view = new DataView(bytes);
  const width = view.getUint32(8, true);
  const height = view.getUint32(12, true);
  if (
    width < 1 ||
    height < 1 ||
    Math.max(width, height) > MAX_EDGE ||
    bytes.byteLength !== 16 + width * height * 6
  ) {
    throw new Error('Invalid linear sample dimensions');
  }
  const data = new Uint16Array(width * height * 3);
  for (let i = 0; i < data.length; i++) data[i] = view.getUint16(16 + i * 2, true);
  return { width, height, data };
}

import { describe, expect, it } from 'vitest';
import { cropRawImage, readRawCrop } from './raw-crop.ts';

function fixture(type: 3 | 4 | 5 = 4, little = true, subIfd = true) {
  const data = new Uint8Array(256);
  const view = new DataView(data.buffer);
  const u16 = (offset: number, value: number) => view.setUint16(offset, value, little);
  const u32 = (offset: number, value: number) => view.setUint32(offset, value, little);
  data.set(little ? [73, 73] : [77, 77]);
  u16(2, 42);
  u32(4, 8);
  const directory = subIfd ? 64 : 8;
  if (subIfd) {
    u16(8, 1);
    u16(10, 330);
    u16(12, 4);
    u32(14, 1);
    u32(18, directory);
  }
  u16(directory, 3);
  const photo = directory + 2;
  u16(photo, 262);
  u16(photo + 2, 3);
  u32(photo + 4, 1);
  u16(photo + 8, 32803);
  for (const [index, tag, values] of [
    [1, 50719, [10, 12]],
    [2, 50720, [6000, 4000]],
  ] as const) {
    const entry = directory + 2 + index * 12;
    u16(entry, tag);
    u16(entry + 2, type);
    u32(entry + 4, 2);
    const start = type === 3 ? entry + 8 : 128 + index * 32;
    if (type !== 3) u32(entry + 8, start);
    values.forEach((value, position) => {
      if (type === 3) u16(start + position * 2, value);
      else if (type === 4) u32(start + position * 4, value);
      else {
        u32(start + position * 8, value * 2);
        u32(start + position * 8 + 4, 2);
      }
    });
  }
  return { data, view, directory, u32, u16 };
}

describe('readRawCrop', () => {
  it.each([3, 4, 5] as const)('reads TIFF type %i from the RAW SubIFD', (type) => {
    expect(readRawCrop(fixture(type).data)).toEqual({ x: 10, y: 12, width: 6000, height: 4000 });
  });
  it('supports big-endian IFD0 raw metadata and byte subviews', () => {
    const source = fixture(5, false, false).data;
    const bytes = new Uint8Array(source.length + 8);
    bytes.set(source, 4);
    expect(readRawCrop(bytes.subarray(4, -4))).toEqual({ x: 10, y: 12, width: 6000, height: 4000 });
  });
  it('rejects a cyclic directory chain', () => {
    const { data, u32, directory } = fixture();
    u32(directory + 2 + 3 * 12, 8);
    expect(() => readRawCrop(data)).toThrow('crop metadata');
  });
  it('rejects out-of-range crop arrays', () => {
    const { data, u32, directory } = fixture();
    u32(directory + 2 + 12 + 8, 252);
    expect(() => readRawCrop(data)).toThrow('crop metadata');
  });
  it('rejects a zero rational denominator', () => {
    const { data, u32 } = fixture(5);
    u32(164, 0);
    expect(() => readRawCrop(data)).toThrow('crop metadata');
  });
  it('requires default crop on the CFA image', () => {
    const { data, u16, directory } = fixture();
    u16(directory + 2 + 24, 50000);
    expect(() => readRawCrop(data)).toThrow('crop metadata');
  });
  it('rejects a truncated directory', () => {
    expect(() => readRawCrop(fixture().data.subarray(0, 80))).toThrow('crop metadata');
  });
});

function image(width: number, height: number, pixels: number[]) {
  return { width, height, data: new Uint16Array(pixels.flatMap((pixel) => [pixel, pixel, pixel])) };
}

describe('cropRawImage', () => {
  const crop = { x: 2, y: 2, width: 4, height: 2 };
  it('removes inactive photo borders after half-size decoding', () => {
    expect(
      cropRawImage(
        image(
          4,
          3,
          Array.from({ length: 12 }, (_, i) => i),
        ),
        crop,
        8,
        6,
        0,
      ),
    ).toEqual(image(2, 1, [5, 6]));
  });
  it('rotates the crop for a clockwise portrait image', () => {
    const rotated = image(3, 4, [8, 4, 0, 9, 5, 1, 10, 6, 2, 11, 7, 3]);
    expect(cropRawImage(rotated, crop, 8, 6, 6)).toEqual(image(1, 2, [5, 6]));
  });
  it('reflects the crop for a mirrored image', () => {
    const mirrored = image(4, 3, [3, 2, 1, 0, 7, 6, 5, 4, 11, 10, 9, 8]);
    expect(cropRawImage(mirrored, { ...crop, x: 0 }, 8, 6, 1)).toEqual(image(2, 1, [5, 4]));
  });
  it('rejects crop outside the active sensor', () => {
    expect(() =>
      cropRawImage(image(4, 3, Array(12).fill(0)), { ...crop, x: 7 }, 8, 6, 0),
    ).toThrow();
  });
});

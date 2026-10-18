/**
 * Applies an 8-bit RGB table with `size` points on each axis, red fastest, to RGBA pixels.
 * Trilinear interpolation. Alpha stays unchanged.
 */
export function applyLut(pixels: Uint8ClampedArray, table: Uint8Array, size: number): void {
  transform(pixels, pixels, 4, table, size, 256, (value) => value / 255);
}

/** The LUT grid uses sRGB transfer solely as a shaper, preserving shadow precision. */
export function srgbCoordinate(linear: number): number {
  return linear <= 0.0031308 ? linear * 12.92 : 1.055 * linear ** (1 / 2.4) - 0.055;
}

export function applyLinearLut(
  pixels: Uint16Array,
  table: Uint8Array,
  size: number,
): Uint8ClampedArray<ArrayBuffer> {
  if (pixels.length % 3 !== 0) throw new Error('Expected RGB pixels');
  const result = new Uint8ClampedArray((pixels.length / 3) * 4);
  transform(pixels, result, 3, table, size, 65536, (value) => srgbCoordinate(value / 65535));
  return result;
}

function transform(
  input: Uint8ClampedArray | Uint16Array,
  pixels: Uint8ClampedArray,
  channels: number,
  table: Uint8Array,
  size: number,
  levels: number,
  coordinate: (value: number) => number,
): void {
  if (
    !Number.isInteger(size) ||
    size < 2 ||
    size > 256 ||
    table.length !== size * size * size * 3
  ) {
    throw new Error('Preview table size mismatch');
  }
  const stride = [3, size * 3, size * size * 3] as const;
  // Per input level: the lower grid index and the weight of the upper index.
  const low = new Uint16Array(levels);
  const weight = new Float32Array(levels);
  for (let v = 0; v < levels; v++) {
    const x = coordinate(v) * (size - 1);
    const i = Math.min(Math.floor(x), size - 2);
    low[v] = i;
    weight[v] = x - i;
  }
  for (let p = 0, source = 0; p < pixels.length; p += 4, source += channels) {
    const r = input[source]!;
    const g = input[source + 1]!;
    const b = input[source + 2]!;
    if (channels === 3) pixels[p + 3] = 255;
    const fr = weight[r]!;
    const fg = weight[g]!;
    const fb = weight[b]!;
    const base = low[r]! * stride[0] + low[g]! * stride[1] + low[b]! * stride[2];
    for (let c = 0; c < 3; c++) {
      const o = base + c;
      const c00 = table[o]! + (table[o + stride[0]]! - table[o]!) * fr;
      const c10 =
        table[o + stride[1]]! + (table[o + stride[1] + stride[0]]! - table[o + stride[1]]!) * fr;
      const c01 =
        table[o + stride[2]]! + (table[o + stride[2] + stride[0]]! - table[o + stride[2]]!) * fr;
      const c11 =
        table[o + stride[2] + stride[1]]! +
        (table[o + stride[2] + stride[1] + stride[0]]! - table[o + stride[2] + stride[1]]!) * fr;
      const c0 = c00 + (c10 - c00) * fg;
      const c1 = c01 + (c11 - c01) * fg;
      pixels[p + c] = c0 + (c1 - c0) * fb;
    }
  }
}

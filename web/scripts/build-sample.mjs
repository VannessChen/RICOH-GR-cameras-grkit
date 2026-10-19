// Builds the default sample from a camera JPEG. The preview applies tables to scene-linear
// pixels, so the script estimates them: it inverts the Standard development of the preview
// (preview/standard.lut) for each pixel. The baseline of the sample then shows the JPEG, and
// each effect starts from the same pixels as on a RAW photo. The full JPEG stays outside the
// repository.
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { gzipSync } from 'node:zlib';
import jpeg from 'jpeg-js';
import { encodeLinear, resizeLinear } from '../src/preview/linear.ts';
import { srgbCoordinate } from '../src/preview/lut.ts';

const SOURCE = {
  url: 'https://upload.wikimedia.org/wikipedia/commons/9/97/Snow_in_Akihabara_2022-01-06.jpg',
  page: 'https://www.flickr.com/photos/cafetricot/51803682755/',
  sha256: 'a4f15f37167735876bfc8c7dcd8bf480a7551c51a00d38045ceb4c5d4cecb598',
};
// Fixed-point steps of the inversion. More steps change the result by less than one 8-bit level.
const STEPS = 8;

const [input, tablePath] = process.argv.slice(2);
if (!input || !tablePath) {
  throw new Error(
    'Usage: node scripts/build-sample.mjs /path/to/Snow_in_Akihabara_2022-01-06.jpg /path/to/effects/gr3-2.10/preview/standard.lut',
  );
}
const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');
const source = await readFile(input);
if (sha256(source) !== SOURCE.sha256) throw new Error('Expected the Snow in Akihabara JPEG');
const table = await readFile(tablePath);
const size = Math.round(Math.cbrt(table.length / 3));
if (size ** 3 * 3 !== table.length) throw new Error('Expected an RGB table');

/** The Standard development of linear RGB in 0..1, as 0..255 floats (the table, trilinear). */
function develop(r, g, b, out) {
  const at = [r, g, b].map((v) => srgbCoordinate(Math.min(1, Math.max(0, v))) * (size - 1));
  const low = at.map((x) => Math.min(Math.floor(x), size - 2));
  const [fr, fg, fb] = at.map((x, c) => x - low[c]);
  const stride = [3, size * 3, size * size * 3];
  const base = low[0] * stride[0] + low[1] * stride[1] + low[2] * stride[2];
  for (let c = 0; c < 3; c++) {
    const t = (dr, dg, db) => table[base + c + dr * stride[0] + dg * stride[1] + db * stride[2]];
    const c00 = t(0, 0, 0) + (t(1, 0, 0) - t(0, 0, 0)) * fr;
    const c10 = t(0, 1, 0) + (t(1, 1, 0) - t(0, 1, 0)) * fr;
    const c01 = t(0, 0, 1) + (t(1, 0, 1) - t(0, 0, 1)) * fr;
    const c11 = t(0, 1, 1) + (t(1, 1, 1) - t(0, 1, 1)) * fr;
    const c0 = c00 + (c10 - c00) * fg;
    const c1 = c01 + (c11 - c01) * fg;
    out[c] = c0 + (c1 - c0) * fb;
  }
  return out;
}

// The tone curve of each channel on the neutral axis, sampled in the table's sRGB coordinate.
const SAMPLES = 4096;
const linear = (s) => (s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4);
const curves = [[], [], []];
const out = [0, 0, 0];
for (let i = 0; i <= SAMPLES; i++) {
  const v = linear(i / SAMPLES);
  develop(v, v, v, out);
  for (let c = 0; c < 3; c++) curves[c].push(out[c]);
}
/** The linear value whose neutral development in channel c is y (0..255). */
function invert(c, y) {
  const curve = curves[c];
  let lo = 0;
  let hi = SAMPLES;
  while (hi - lo > 1) {
    const mid = (lo + hi) >> 1;
    if (curve[mid] < y) lo = mid;
    else hi = mid;
  }
  const span = curve[hi] - curve[lo];
  const s = (lo + (span > 0 ? Math.min(1, Math.max(0, (y - curve[lo]) / span)) : 0)) / SAMPLES;
  return linear(s);
}

// 1. Each channel through the inverse neutral curve, at full size, then the same area
//    resampling in linear light as a RAW upload.
const decoded = jpeg.decode(source, { useTArray: true, formatAsRGBA: false, maxMemoryUsageInMB: 1024 });
const inverse = [0, 1, 2].map((c) => Uint16Array.from({ length: 256 }, (_, y) => Math.round(invert(c, y) * 65535)));
const full = new Uint16Array(decoded.width * decoded.height * 3);
for (let i = 0; i < full.length; i++) full[i] = inverse[i % 3][decoded.data[i]];
const image = resizeLinear({ width: decoded.width, height: decoded.height, data: full });

// 2. The full table mixes the channels and adds saturation. For each pixel, find the linear
//    value x whose development is the neutral development of the resampled pixel x0:
//    x += x0 - g(develop(x)), where g is the inverse neutral curve.
let error = 0;
const x = [0, 0, 0];
const x0 = [0, 0, 0];
for (let p = 0; p < image.data.length; p += 3) {
  for (let c = 0; c < 3; c++) x0[c] = x[c] = image.data[p + c] / 65535;
  const neutral = [0, 1, 2].map((c) => develop(x0[c], x0[c], x0[c], [0, 0, 0])[c]);
  for (let step = 0; step < STEPS; step++) {
    develop(x[0], x[1], x[2], out);
    for (let c = 0; c < 3; c++) x[c] = Math.min(1, Math.max(0, x[c] + x0[c] - invert(c, out[c])));
  }
  develop(x[0], x[1], x[2], out);
  for (let c = 0; c < 3; c++) {
    error += Math.abs(out[c] - neutral[c]);
    image.data[p + c] = Math.round(x[c] * 65535);
  }
}
const meanError = error / image.data.length;

const bytes = gzipSync(encodeLinear(image), { level: 9 });
const output = new URL('../public/samples/', import.meta.url);
await mkdir(output, { recursive: true });
await writeFile(new URL('akihabara.grlinear', output), bytes);
await writeFile(
  new URL('akihabara.json', output),
  JSON.stringify(
    {
      source: SOURCE.url,
      sourcePage: SOURCE.page,
      sourceSha256: SOURCE.sha256,
      title: 'Snow in Akihabara',
      author: 'nagi usano',
      license: 'CC-BY-2.0',
      licenseUrl: 'https://creativecommons.org/licenses/by/2.0/',
      camera: 'RICOH GR IIIx',
      decoder: 'jpeg-js 0.4.4',
      settings: {
        input: 'sRGB JPEG',
        outputColor: 'linear sRGB, estimated by inverting preview/standard.lut',
        standardTableSha256: sha256(table),
        bits: 16,
        resize: 'area resampling in linear light',
      },
      width: image.width,
      height: image.height,
      sha256: sha256(bytes),
    },
    null,
    2,
  ) + '\n',
);
console.log(
  `Sample: ${image.width} x ${image.height}, ${bytes.length} bytes. ` +
    `Mean difference between the baseline and the JPEG: ${meanError.toFixed(2)} of 255.`,
);

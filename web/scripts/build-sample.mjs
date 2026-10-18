// Uses the browser's RAW decoder and resampler. The full DNG stays outside the repository.
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { gzipSync } from 'node:zlib';
import { fileURLToPath } from 'node:url';
import { decodeRaw } from '../src/preview/raw-decode.ts';
import { encodeLinear, resizeLinear } from '../src/preview/linear.ts';

const input = process.argv[2];
if (!input) throw new Error('Usage: node scripts/build-sample.mjs /path/to/gr-iii.dng');
const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');
const source = await readFile(input);
const sourceHash = '05513ee72f7cac4165534c9f64995412084f414a033c80c473c79fd94154292e';
if (sha256(source) !== sourceHash)
  throw new Error('Expected the raw.pixls.us GR III sample (record 3115)');
const wasm = await readFile(
  fileURLToPath(import.meta.resolve('@colorhythm/libraw-wasm/libraw.wasm')),
);
const decoded = await decodeRaw(
  source,
  wasm.buffer.slice(wasm.byteOffset, wasm.byteOffset + wasm.byteLength),
);
const image = resizeLinear(decoded);
const bytes = gzipSync(encodeLinear(image), { level: 9 });
const output = new URL('../public/samples/', import.meta.url);
await mkdir(output, { recursive: true });
await writeFile(new URL('gr-iii.grlinear', output), bytes);
await writeFile(
  new URL('gr-iii.json', output),
  JSON.stringify(
    {
      source:
        'https://raw.pixls.us/getfile.php/3115/nice/Ricoh%20-%20GR%20III%20-%2014bit%20(3%3A2).DNG',
      sourceRecord: 'https://raw.pixls.us/json/getrepository.php?set=all',
      sourceId: 3115,
      sourceSha256: sourceHash,
      license: 'CC0-1.0',
      licenseUrl: 'https://creativecommons.org/publicdomain/zero/1.0/',
      author: 'Not specified by raw.pixls.us',
      camera: 'RICOH GR III',
      decoder: '@colorhythm/libraw-wasm 1.1.1 (LibRaw 0.22.1)',
      settings: {
        halfSize: true,
        cameraWhiteBalance: true,
        outputColor: 'linear sRGB',
        bits: 16,
        autoBrightness: false,
        adjustMaximumThreshold: 0,
        crop: 'DNG default crop',
      },
      width: image.width,
      height: image.height,
      sha256: sha256(bytes),
    },
    null,
    2,
  ) + '\n',
);
console.log(`Sample: ${image.width} x ${image.height}, ${bytes.length} bytes`);

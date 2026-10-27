import { validatePicture, MAX_EDGE, type LinearPicture } from './linear';
import { workerRender, type Render } from './renderer';

/** A decoded RAW source. The key identifies it in the caches. */
export interface Source {
  key: string;
  load: () => Promise<LinearPicture>;
}

export interface PreviewEffect {
  id: string;
  table: string;
  baseline: string;
  size: number;
}

export interface Preview {
  original: string;
  processed: string;
}

const MAX_RESULTS = 12;
const pictures = new Map<string, Promise<LinearPicture>>();
const tables = new Map<string, Promise<Uint8Array>>();
const baselines = new Map<string, Promise<string>>();
const results = new Map<string, Promise<Preview>>();

/** React retries must see the same promise, including a rejected one. */
function cached<T>(
  cache: Map<string, Promise<T>>,
  key: string,
  load: () => Promise<T>,
  retry = false,
): Promise<T> {
  let result = cache.get(key);
  if (!result) {
    result = load();
    cache.set(key, result);
    if (retry)
      result.catch(() => {
        if (cache.get(key) === result) cache.delete(key);
      });
  }
  return result;
}

export function picture(source: Source): Promise<LinearPicture> {
  return cached(
    pictures,
    source.key,
    async () => {
      const image = await source.load();
      validatePicture(image, MAX_EDGE * MAX_EDGE);
      return image;
    },
    true,
  );
}

function table(url: string): Promise<Uint8Array> {
  return cached(
    tables,
    url,
    async () => {
      const response = await fetch(url);
      if (!response.ok) throw new Error(`Preview table request failed: ${response.status}`);
      return new Uint8Array(await response.arrayBuffer());
    },
    true,
  );
}

/** Starts the download of a preview table, so a later preview of it starts sooner. */
export function prefetchTable(url: string): void {
  table(url).catch(() => undefined);
}

/**
 * Makes `bytes` the table of `key`, for a table that the page made instead of fetching it, such
 * as the table of an imported LUT. A `PreviewEffect` then gives `key` as its `table`.
 */
export function provideTable(key: string, bytes: Uint8Array): void {
  tables.set(key, Promise.resolve(bytes));
}

let renderer: Render | undefined;

async function render(image: LinearPicture, lut: Uint8Array, size: number): Promise<string> {
  renderer ??= workerRender();
  return URL.createObjectURL(await renderer(image, lut, size));
}

/** Shared by all effects of a source. Both sides use the same decoded pixels. */
function baseline(source: Source, effect: PreviewEffect): Promise<string> {
  return cached(
    baselines,
    `${source.key}\n${effect.baseline}\n${effect.size}`,
    async () => {
      const [image, lut] = await Promise.all([picture(source), table(effect.baseline)]);
      return render(image, lut, effect.size);
    },
    true,
  );
}

function release(key: string, result: Promise<Preview>): void {
  results.delete(key);
  result.then((preview) => URL.revokeObjectURL(preview.processed)).catch(() => undefined);
}

/** A stable promise for React use(), keyed by the source and the actual table URLs. */
export function preview(source: Source, effect: PreviewEffect): Promise<Preview> {
  const key = `${source.key}\n${effect.table}\n${effect.baseline}\n${effect.size}`;
  const result = cached(results, key, async () => {
    const [image, lut, original] = await Promise.all([
      picture(source),
      table(effect.table),
      baseline(source, effect),
    ]);
    return { original, processed: await render(image, lut, effect.size) };
  });
  results.delete(key);
  results.set(key, result);
  for (const [oldKey, oldResult] of results) {
    if (results.size <= MAX_RESULTS) break;
    release(oldKey, oldResult);
  }
  return result;
}

/** Also releases resources that finish loading after a source has been replaced. */
export function forget(source: Source): void {
  pictures.delete(source.key);
  for (const [key, result] of baselines) {
    if (!key.startsWith(`${source.key}\n`)) continue;
    baselines.delete(key);
    result.then((url) => URL.revokeObjectURL(url)).catch(() => undefined);
  }
  for (const [key, result] of results) {
    if (key.startsWith(`${source.key}\n`)) release(key, result);
  }
}

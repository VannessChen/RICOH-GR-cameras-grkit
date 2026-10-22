import init, { guessInput, importLut } from '../wasm/grkit_wasm.js';
import wasmUrl from '../wasm/grkit_wasm_bg.wasm?url';
import { GRADES, type Grade } from '../catalog';
import { INPUTS, type Input } from './names';

export interface ImportRequest {
  kind: 'import';
  bytes: ArrayBuffer;
  source: string;
  input: Input;
  baseGamma: Uint16Array;
  id: string;
  name: string;
  /** The JSON text of the effect.json `icon`. */
  icon: string;
}

export interface ImportResult {
  blob: Uint8Array;
  table: Uint8Array;
  icon: Uint8Array;
  monochrome: boolean;
  grade: Grade;
  warnings: string[];
}

/** Guesses the input that a LUT expects from its data (`fit::guess_input`). */
export interface GuessRequest {
  kind: 'guess';
  bytes: ArrayBuffer;
  baseGamma: Uint16Array;
}

export type ImportResponse =
  | { kind: 'import'; result: ImportResult }
  /** The progress of an import, a fraction from 0 to 1, before its result. */
  | { kind: 'progress'; fraction: number }
  | { kind: 'guess'; input: Input }
  | { kind: 'error'; message: string };

/** `value` as one of `values`, which the TypeScript types of grkit-wasm do not give. */
function oneOf<T extends string>(values: readonly T[], value: string): T {
  if (!(values as readonly string[]).includes(value)) {
    throw new Error(`Unexpected value from grkit-wasm: ${value}`);
  }
  return value as T;
}

/**
 * Fits one LUT, or guesses its input. The page starts a new worker for each request and
 * terminates it after.
 */
self.onmessage = async ({ data }: MessageEvent<ImportRequest | GuessRequest>) => {
  try {
    await init({ module_or_path: wasmUrl });
    if (data.kind === 'guess') {
      const input = oneOf(INPUTS, guessInput(new Uint8Array(data.bytes), data.baseGamma));
      self.postMessage({ kind: 'guess', input } satisfies ImportResponse);
      return;
    }
    // One message for each percent: the fit reports about 180 steps.
    let percent = 0;
    const effect = importLut(
      new Uint8Array(data.bytes),
      data.source,
      data.input,
      data.baseGamma,
      data.id,
      data.name,
      data.icon,
      (fraction) => {
        const next = Math.floor(fraction * 100);
        if (next <= percent) return;
        percent = next;
        self.postMessage({ kind: 'progress', fraction } satisfies ImportResponse);
      },
    );
    let result: ImportResult;
    try {
      result = {
        blob: effect.blob,
        table: effect.preview,
        icon: effect.icon,
        monochrome: effect.monochrome,
        grade: oneOf(GRADES, effect.grade),
        warnings: effect.warnings,
      };
    } finally {
      effect.free();
    }
    self.postMessage({ kind: 'import', result } satisfies ImportResponse, {
      transfer: [result.blob.buffer, result.table.buffer, result.icon.buffer],
    });
  } catch (error) {
    self.postMessage({
      kind: 'error',
      message: error instanceof Error ? error.message : 'The LUT cannot be imported',
    } satisfies ImportResponse);
  }
};

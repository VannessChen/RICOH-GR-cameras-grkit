import type { ImportedEffect } from '../catalog';
import { IMPORTS, run } from '../database';
import { provideTable } from '../preview/render';

// Keeps the effects that this page imported from LUTs in the browser, so a later visit has them
// without a new import. Each call fails quietly, and the page works without it.

/** The start of each compiled effect (`grkit_core::effects::MAGIC`). */
const MAGIC = 'GRFX0001';

interface Stored extends ImportedEffect {
  /** The time of the import, so the page shows the imports in their order. */
  savedAt: number;
}

/** A stored import that this version of the page can use: the fields and a compiled effect. */
function usable(value: unknown): value is Stored {
  const item = value as Partial<Stored> | null;
  const imported = item?.imported;
  return (
    typeof item?.id === 'string' &&
    typeof item.name === 'string' &&
    typeof item.icon === 'string' &&
    typeof item.preview === 'string' &&
    !!item.cover &&
    !!imported &&
    imported.blob instanceof Uint8Array &&
    imported.table instanceof Uint8Array &&
    new TextDecoder().decode(imported.blob.subarray(0, MAGIC.length)) === MAGIC
  );
}

/**
 * The stored imports, in import order. Each one has its preview table again, so the preview finds
 * it as after the import.
 */
export async function loadImports(): Promise<ImportedEffect[]> {
  try {
    const items = (await run(IMPORTS, 'readonly', (store) => store.getAll())).filter(usable);
    items.sort((a, b) => a.savedAt - b.savedAt);
    return items.map(({ savedAt: _, ...effect }) => {
      provideTable(effect.preview, effect.imported.table);
      return effect;
    });
  } catch (error) {
    console.warn('The imported effects cannot be read from this browser.', error);
    return [];
  }
}

export async function saveImport(effect: ImportedEffect): Promise<void> {
  try {
    await run(IMPORTS, 'readwrite', (store) => store.put({ ...effect, savedAt: Date.now() } satisfies Stored));
  } catch (error) {
    console.warn('The imported effect cannot be kept in this browser.', error);
  }
}

export async function deleteImport(id: string): Promise<void> {
  try {
    await run(IMPORTS, 'readwrite', (store) => store.delete(id));
  } catch (error) {
    console.warn('The imported effect cannot be deleted from this browser.', error);
  }
}

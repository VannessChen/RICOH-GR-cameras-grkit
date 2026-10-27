import type { CardDirectory } from './card';
import { CARDS, run } from './database';

// The last connected card, so a later visit reconnects it with one click. The browser keeps the
// folder handle; the permission to edit it is asked again. Each call fails quietly.

const LAST = 'last';

/** A stored card handle, as the browser gives it back. */
type StoredCard = CardDirectory & {
  requestPermission(options: { mode: 'readwrite' }): Promise<PermissionState>;
};

export async function loadLastCard(): Promise<CardDirectory | null> {
  try {
    const handle = await run<unknown>(CARDS, 'readonly', (store) => store.get(LAST));
    return handle && typeof handle === 'object' && 'requestPermission' in handle ? (handle as StoredCard) : null;
  } catch (error) {
    console.warn('The last card cannot be read from this browser.', error);
    return null;
  }
}

export async function saveLastCard(card: CardDirectory): Promise<void> {
  try {
    await run(CARDS, 'readwrite', (store) => store.put(card, LAST));
  } catch (error) {
    console.warn('The card cannot be kept in this browser.', error);
  }
}

export async function forgetLastCard(): Promise<void> {
  try {
    await run(CARDS, 'readwrite', (store) => store.delete(LAST));
  } catch (error) {
    console.warn('The card cannot be removed from this browser.', error);
  }
}

/** Asks again for the permission to edit a stored card. It must run in a click. */
export async function allowCard(card: CardDirectory): Promise<boolean> {
  return (await (card as StoredCard).requestPermission({ mode: 'readwrite' })) === 'granted';
}

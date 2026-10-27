/**
 * The page's database in this browser (IndexedDB): the imported effects and the last connected
 * card. It is a convenience for this browser only, so each caller fails quietly without it.
 */
const DATABASE = 'grkit';
const VERSION = 2;

export const IMPORTS = 'imports';
export const CARDS = 'cards';

function open(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(DATABASE, VERSION);
    request.onupgradeneeded = () => {
      const database = request.result;
      if (!database.objectStoreNames.contains(IMPORTS)) database.createObjectStore(IMPORTS, { keyPath: 'id' });
      if (!database.objectStoreNames.contains(CARDS)) database.createObjectStore(CARDS);
    };
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
}

/** Runs one request on `store` in its own transaction, and gives its result after the commit. */
export async function run<T>(
  store: string,
  mode: IDBTransactionMode,
  action: (store: IDBObjectStore) => IDBRequest<T>,
): Promise<T> {
  const database = await open();
  try {
    return await new Promise<T>((resolve, reject) => {
      const transaction = database.transaction(store, mode);
      const request = action(transaction.objectStore(store));
      transaction.oncomplete = () => resolve(request.result);
      transaction.onerror = () => reject(transaction.error);
      transaction.onabort = () => reject(transaction.error);
    });
  } finally {
    database.close();
  }
}

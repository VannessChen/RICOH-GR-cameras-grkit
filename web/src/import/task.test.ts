import { describe, expect, it } from 'vitest';
import { TaskSlot } from './task';

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}

describe('task slot', () => {
  it('aborts the previous task and marks its later result as not current', async () => {
    const slot = new TaskSlot();
    const first = deferred<string>();
    const second = deferred<string>();
    let firstSignal: AbortSignal | undefined;
    const a = slot.run((signal) => {
      firstSignal = signal;
      return first.promise;
    });
    const b = slot.run(() => second.promise);
    expect(firstSignal?.aborted).toBe(true);
    second.resolve('new');
    expect(await b).toEqual({ current: true, value: 'new' });
    first.resolve('old');
    expect(await a).toEqual({ current: false });
  });

  it('drops the error of a cancelled task and rejects with the error of a current one', async () => {
    const slot = new TaskSlot();
    const pending = deferred<string>();
    const cancelled = slot.run(() => pending.promise);
    slot.cancel();
    pending.reject(new Error('Old failure'));
    expect(await cancelled).toEqual({ current: false });
    await expect(slot.run(() => Promise.reject(new Error('Failure')))).rejects.toThrow('Failure');
    expect(await slot.run(() => Promise.resolve(null))).toEqual({ current: true, value: null });
  });
});

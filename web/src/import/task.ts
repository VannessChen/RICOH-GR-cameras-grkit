/** The result of a task, or only `current: false` when a newer task or `cancel` replaced it. */
export type TaskResult<T> = { current: true; value: T } | { current: false };

const STALE = { current: false } as const;

/**
 * Runs one task at a time, as `PhotoLoader` loads one photo: a new task aborts the one before,
 * and only the newest task gives its result or its error.
 */
export class TaskSlot {
  private controller?: AbortController;

  /** Aborts the running task, which then resolves as not current. */
  cancel(): void {
    this.controller?.abort();
    this.controller = undefined;
  }

  /** Runs `task` after it aborts the running task. An error rejects only while it is current. */
  async run<T>(task: (signal: AbortSignal) => Promise<T>): Promise<TaskResult<T>> {
    this.cancel();
    const controller = new AbortController();
    this.controller = controller;
    try {
      const value = await task(controller.signal);
      return this.controller === controller ? { current: true, value } : STALE;
    } catch (error) {
      if (this.controller !== controller) return STALE;
      throw error;
    } finally {
      if (this.controller === controller) this.controller = undefined;
    }
  }
}

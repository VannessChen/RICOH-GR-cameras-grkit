import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { STOCKS } from './stocks';

/** The target of an effect in its source, for example "Kodak Vision3 500T 5219 on Vision Premier 2393 print film". */
function target(id: string): string {
  const url = new URL(`../../effects/${id}/effect.json`, import.meta.url);
  return JSON.parse(readFileSync(url, 'utf8')).attribution.target;
}

describe('STOCKS', () => {
  it('gives the numbers that the target of each effect names', () => {
    for (const [id, { film, print }] of Object.entries(STOCKS)) {
      expect(target(id), id).toMatch(new RegExp(`\\b${film}\\b`));
      if (print) expect(target(id), id).toMatch(new RegExp(`\\b${print} print film\\b`));
    }
  });
});

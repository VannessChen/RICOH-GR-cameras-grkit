import { describe, expect, it } from 'vitest';
import { CAMERAS, release } from './cameras';
import { CAMERA_RELEASES } from './generated/cameras';

describe('cameras', () => {
  it('shows each runtime build of each camera release', () => {
    for (const [id, camera] of Object.entries(CAMERA_RELEASES)) {
      for (const { target } of camera.releases) {
        expect(
          CAMERAS.some((item) => item.id === id && item.support?.target === target),
          `${id} ${target}`,
        ).toBe(true);
      }
    }
  });

  it('lists the versions of a runtime build and stays experimental until one is tested', () => {
    expect(release('gr3', 'gr3-2.10')).toEqual({
      firmware: '2.10 / 2.20',
      target: 'gr3-2.10',
      modeset: '00078350.588',
      experimental: false,
    });
    expect(release('gr4', 'gr4-1.20').experimental).toBe(true);
    expect(() => release('gr3', 'gr4-1.11')).toThrow();
  });
});

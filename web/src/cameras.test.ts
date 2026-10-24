import { describe, expect, it } from 'vitest';
import { CAMERAS, releases } from './cameras';
import { CAMERA_RELEASES } from './generated/cameras';

describe('cameras', () => {
  it('packages each runtime build of each camera release', () => {
    for (const [id, camera] of Object.entries(CAMERA_RELEASES)) {
      for (const { target } of camera.releases) {
        expect(
          CAMERAS.some((item) => item.id === id && item.support?.targets.includes(target)),
          `${id} ${target}`,
        ).toBe(true);
      }
    }
  });

  it('shows each camera once', () => {
    const names = CAMERAS.map((item) => item.name);
    expect(new Set(names).size).toBe(names.length);
  });

  it('lists the versions and builds of a camera and stays experimental until one is tested', () => {
    expect(releases('gr3')).toEqual({
      firmware: '2.10 / 2.20',
      targets: ['gr3-2.10'],
      modeset: '00078350.588',
      experimental: false,
    });
    // Two builds of 1.20 show the version once.
    expect(releases('gr4')).toMatchObject({
      firmware: '1.11 / 1.20',
      targets: ['gr4-1.11', 'gr4-1.20', 'gr4-1.20-0528'],
      experimental: false,
    });
  });

  it('gives GR IV Monochrome the GR IV builds with its own versions', () => {
    const mono = CAMERAS.find((item) => item.key === 'gr4m')!;
    expect(mono.support).toMatchObject({
      firmware: '1.00 / 1.10',
      targets: ['gr4-1.11', 'gr4-1.20', 'gr4-1.20-0528'],
    });
  });
});

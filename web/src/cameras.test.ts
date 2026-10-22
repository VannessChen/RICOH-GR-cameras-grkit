import { describe, expect, it } from 'vitest';
import { CAMERAS, releases, selectedBuild } from './cameras';
import { CAMERA_RELEASES } from './generated/cameras';

describe('cameras', () => {
  it('shows each runtime build of each camera release', () => {
    for (const [id, camera] of Object.entries(CAMERA_RELEASES)) {
      for (const { target } of camera.releases) {
        expect(
          CAMERAS.some(
            (item) => item.id === id && item.support?.builds.some((build) => build.target === target),
          ),
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
      builds: [{ firmware: '2.10 / 2.20', target: 'gr3-2.10' }],
      modeset: '00078350.588',
      experimental: false,
    });
    expect(releases('gr4')).toMatchObject({
      firmware: '1.11 / 1.20',
      builds: [
        { firmware: '1.11', target: 'gr4-1.11' },
        { firmware: '1.20', target: 'gr4-1.20' },
      ],
      experimental: false,
    });
  });

  it('selects the only build, or the build of the selected firmware version', () => {
    const camera = (key: string) => CAMERAS.find((item) => item.key === key)!;
    expect(selectedBuild(camera('gr3'), undefined)?.target).toBe('gr3-2.10');
    expect(selectedBuild(camera('gr4'), undefined)).toBeUndefined();
    expect(selectedBuild(camera('gr4'), 'gr4-1.20')?.target).toBe('gr4-1.20');
    expect(selectedBuild(camera('gr4m'), undefined)?.target).toBe('gr4-1.11');
  });
});

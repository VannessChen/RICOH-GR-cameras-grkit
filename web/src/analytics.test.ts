import { describe, expect, it } from 'vitest';
import { downloadProps } from './analytics';
import { CAMERAS } from './cameras';
import { planModules, type ModuleKey } from './modules';

const camera = (key: string) => CAMERAS.find((item) => item.key === key)!;

describe('downloadProps', () => {
  it('gives the camera, the effect count, the language, and each module', () => {
    const gr3 = camera('gr3');
    const enabled = new Set<ModuleKey>(['imageControl', 'framing']);
    const plan = planModules(enabled, 2, gr3.support!.modules);
    expect(downloadProps(gr3, plan, 2, 'zh-Hans')).toEqual({
      camera: 'gr3',
      effects: '2',
      language: 'zh-Hans',
      imageControl: 'on',
      framing: 'on',
      guides: 'off',
      dateStamp: 'off',
      grain: 'off',
    });
  });

  it('gives Image Control as off when the package has no effects', () => {
    const gr3 = camera('gr3');
    const plan = planModules(new Set<ModuleKey>(['imageControl', 'grain']), 0, gr3.support!.modules);
    expect(downloadProps(gr3, plan, 0, 'en')).toMatchObject({ imageControl: 'off', grain: 'on' });
  });

  it('leaves out the modules that the site does not offer for the camera', () => {
    const gr4m = camera('gr4m');
    const support = gr4m.support!;
    const plan = planModules(new Set(support.modules), 0, support.modules, support.excluded);
    const props = downloadProps(gr4m, plan, 0, 'en');
    expect(props.camera).toBe('gr4m');
    expect(props).not.toHaveProperty('imageControl');
    expect(props.framing).toBe('on');
  });
});

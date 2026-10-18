import { describe, expect, it } from 'vitest';
import { applyLinearLut, applyLut, srgbCoordinate } from './lut';

function table(size: number, map: (rgb: [number, number, number]) => [number, number, number]) {
  const result = new Uint8Array(size ** 3 * 3);
  const level = (i: number) => Math.round((i * 255) / (size - 1));
  for (let b = 0; b < size; b++) {
    for (let g = 0; g < size; g++) {
      for (let r = 0; r < size; r++) {
        result.set(map([level(r), level(g), level(b)]), ((b * size + g) * size + r) * 3);
      }
    }
  }
  return result;
}

describe('applyLut', () => {
  it('keeps pixels with an identity table and keeps alpha', () => {
    const pixels = new Uint8ClampedArray([0, 0, 0, 255, 255, 255, 255, 10, 12, 130, 201, 77]);
    const before = pixels.slice();
    applyLut(
      pixels,
      table(33, (rgb) => rgb),
      33,
    );
    for (let i = 0; i < pixels.length; i++) {
      expect(Math.abs(pixels[i]! - before[i]!)).toBeLessThanOrEqual(1);
    }
    expect(pixels[7]).toBe(10);
  });

  it('reads the table with red fastest', () => {
    const pixels = new Uint8ClampedArray([255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255]);
    applyLut(
      pixels,
      table(2, ([r, g, b]) => [b, r, g]),
      2,
    );
    expect([...pixels]).toEqual([0, 255, 0, 255, 0, 0, 255, 255, 255, 0, 0, 255]);
  });

  it('interpolates between grid points', () => {
    const pixels = new Uint8ClampedArray([128, 128, 128, 255]);
    applyLut(
      pixels,
      table(2, ([r]) => [r, 0, 255 - r]),
      2,
    );
    expect([...pixels]).toEqual([128, 0, 127, 255]);
  });

  it('rejects a table of another size', () => {
    expect(() => applyLut(new Uint8ClampedArray(4), new Uint8Array(10), 33)).toThrow();
  });
});

describe('linear RAW preview', () => {
  it('shapes linear input before lookup without inverse Standard rendering', () => {
    const pixels = new Uint16Array([0, Math.round(0.18 * 65535), 65535]);
    const result = applyLinearLut(
      pixels,
      table(33, (rgb) => rgb),
      33,
    );
    expect(result[0]).toBe(0);
    expect(Math.abs(result[1]! - 118)).toBeLessThanOrEqual(1);
    expect(result[2]).toBe(255);
    expect(result[3]).toBe(255);
    expect([...pixels]).toEqual([0, Math.round(0.18 * 65535), 65535]);
  });

  it('preserves low linear values through the shaper', () => {
    expect(srgbCoordinate(0.001)).toBeCloseTo(0.01292, 6);
    expect(srgbCoordinate(1)).toBeCloseTo(1, 6);
    const result = applyLinearLut(
      new Uint16Array([100, 100, 100]),
      table(33, (rgb) => rgb),
      33,
    );
    expect(result[0]).toBeGreaterThan(0);
  });

  it('rejects malformed pixel arrays and table dimensions', () => {
    expect(() =>
      applyLinearLut(
        new Uint16Array(2),
        table(2, (rgb) => rgb),
        2,
      ),
    ).toThrow();
    expect(() => applyLinearLut(new Uint16Array(3), new Uint8Array(3), 1)).toThrow();
  });
});

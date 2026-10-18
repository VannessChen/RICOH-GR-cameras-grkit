import { describe, expect, it } from 'vitest';
import { decodeLinear, encodeLinear, resizeLinear } from './linear';

describe('linear samples', () => {
  it('round-trips 16-bit values in little-endian order', () => {
    const image = { width: 1, height: 1, data: new Uint16Array([1, 0x1234, 65535]) };
    const bytes = encodeLinear(image);
    expect([...bytes.slice(16)]).toEqual([1, 0, 0x34, 0x12, 255, 255]);
    expect(decodeLinear(bytes.buffer as ArrayBuffer)).toEqual(image);
  });

  it('rejects malformed, oversized, and truncated samples before allocating pixels', () => {
    expect(() => decodeLinear(new ArrayBuffer(8))).toThrow();
    const bytes = encodeLinear({ width: 1, height: 1, data: new Uint16Array(3) });
    expect(() => decodeLinear(bytes.slice(0, -1).buffer)).toThrow();
    new DataView(bytes.buffer).setUint32(8, 0xffffffff, true);
    expect(() => decodeLinear(bytes.buffer as ArrayBuffer)).toThrow();
    bytes[0] = 0;
    expect(() => decodeLinear(bytes.buffer as ArrayBuffer)).toThrow();
  });

  it('averages light before encoding and preserves aspect ratio', () => {
    const data = new Uint16Array(2400 * 2 * 3);
    for (let p = 0; p < data.length; p += 3) data.fill((p / 3) % 2 ? 65535 : 0, p, p + 3);
    const result = resizeLinear({ width: 2400, height: 2, data });
    expect([result.width, result.height]).toEqual([1200, 1]);
    expect(result.data.every((value) => value === 32768)).toBe(true);
  });

  it('does not upscale a small source', () => {
    const image = { width: 1, height: 1, data: new Uint16Array([100, 200, 300]) };
    expect(resizeLinear(image)).toBe(image);
  });
});

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { decodeRaw } from './raw-decode';

const mocks = vi.hoisted(() => {
  const decoder = {
    waitUntilReady: vi.fn(async () => {}),
    open: vi.fn(),
    getIParams: vi.fn(() => ({
      normalized_make: 'Ricoh',
      normalized_model: 'GR III',
      dng_version: 16908288,
      software: 'RICOH GR III Ver. 1.00',
    })),
    getRawWidth: vi.fn(() => 6112),
    getRawHeight: vi.fn(() => 4064),
    getActiveWidth: vi.fn(() => 6020),
    getActiveHeight: vi.fn(() => 4024),
    getFlip: vi.fn(() => 0),
    setHalfSize: vi.fn(),
    setUseCameraWb: vi.fn(),
    setOutputColor: vi.fn(),
    setOutputBps: vi.fn(),
    setGamma: vi.fn(),
    setNoAutoBright: vi.fn(),
    setAdjustMaximumThr: vi.fn(),
    unpack: vi.fn(),
    dcrawProcess: vi.fn(),
    dcrawMakeMemImage: vi.fn(() => ({
      width: 1,
      height: 1,
      bits: 16,
      colors: 3,
      data: new Uint8Array(new Uint16Array([1000, 2000, 3000]).buffer),
    })),
    dispose: vi.fn(),
  };
  return { decoder, initialize: vi.fn(async () => {}) };
});

vi.mock('@colorhythm/libraw-wasm', () => ({
  LibRaw: class {
    static initialize = mocks.initialize;
    constructor() {
      return mocks.decoder;
    }
  },
}));

vi.mock('./raw-crop.ts', () => ({
  readRawCrop: () => ({ x: 10, y: 12, width: 6000, height: 4000 }),
  cropRawImage: (image: { width: number; height: number; data: Uint16Array }) => image,
}));

describe('decodeRaw', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('uses the exact input view and returns owned linear 16-bit RGB', async () => {
    const input = new Uint8Array([9, 1, 2, 9]).subarray(1, 3);
    const result = await decodeRaw(input, new ArrayBuffer(1));
    expect(new Uint8Array(mocks.decoder.open.mock.calls[0]![0])).toEqual(new Uint8Array([1, 2]));
    expect([...result.data]).toEqual([1000, 2000, 3000]);
    expect(mocks.decoder.setGamma.mock.calls).toEqual([
      [0, 1],
      [1, 1],
    ]);
    expect(mocks.decoder.setNoAutoBright).toHaveBeenCalledWith(1);
    expect(mocks.decoder.setUseCameraWb).toHaveBeenCalledWith(1);
    expect(mocks.decoder.dispose).toHaveBeenCalledOnce();
  });

  it('rejects a raw file that is not a DNG before unpacking', async () => {
    const metadata = mocks.decoder.getIParams();
    mocks.decoder.getIParams.mockReturnValueOnce({ ...metadata, dng_version: 0 });
    await expect(decodeRaw(new Uint8Array([1]), new ArrayBuffer(1))).rejects.toThrow('DNG');
    expect(mocks.decoder.unpack).not.toHaveBeenCalled();
    expect(mocks.decoder.dispose).toHaveBeenCalledOnce();
  });

  it.each([
    { normalized_make: 'Canon', normalized_model: 'EOS R5' },
    { normalized_model: 'GR IV' },
    { software: 'Adobe DNG Converter' },
  ])('decodes a DNG from any camera or writer: %j', async (change) => {
    const metadata = mocks.decoder.getIParams();
    mocks.decoder.getIParams.mockReturnValueOnce({ ...metadata, ...change });
    await expect(decodeRaw(new Uint8Array([1]), new ArrayBuffer(1))).resolves.toBeDefined();
    expect(mocks.decoder.unpack).toHaveBeenCalledOnce();
  });

  it('bounds sensor allocation before unpacking', async () => {
    mocks.decoder.getRawWidth.mockReturnValueOnce(100_000);
    await expect(decodeRaw(new Uint8Array([1]), new ArrayBuffer(1))).rejects.toThrow('dimensions');
    expect(mocks.decoder.unpack).not.toHaveBeenCalled();
    expect(mocks.decoder.dispose).toHaveBeenCalledOnce();
  });

  it('releases the decoder when raw decoding fails', async () => {
    mocks.decoder.unpack.mockImplementationOnce(() => {
      throw new Error('Truncated file');
    });
    await expect(decodeRaw(new Uint8Array([1]), new ArrayBuffer(1))).rejects.toThrow(
      'Truncated file',
    );
    expect(mocks.decoder.dispose).toHaveBeenCalledOnce();
  });
});

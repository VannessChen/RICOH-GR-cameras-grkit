import { LibRaw } from '@colorhythm/libraw-wasm';
import { cropRawImage, readRawCrop } from './raw-crop.ts';

const MAX_RAW_PIXELS = 30_000_000;

export interface DecodedRaw {
  width: number;
  height: number;
  data: Uint16Array;
}

/** Decode sensor data in a Worker; embedded JPEG previews are never used. */
export async function decodeRaw(bytes: Uint8Array, wasm: ArrayBuffer): Promise<DecodedRaw> {
  await LibRaw.initialize(wasm);
  const decoder = new LibRaw();
  try {
    await decoder.waitUntilReady();
    // The input can be a view into a larger buffer. LibRaw must see only this file.
    decoder.open(bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength));
    const metadata = decoder.getIParams();
    const make = metadata.normalized_make.trim().toLowerCase();
    const model = metadata.normalized_model.trim().toLowerCase();
    if (!metadata.dng_version || make !== 'ricoh' || !['gr iii', 'gr iiix'].includes(model)) {
      throw new Error('Choose an original DNG from a RICOH GR III or GR IIIx.');
    }
    if (
      /adobe|lightroom|darktable|rawtherapee|capture one|dng converter/i.test(metadata.software)
    ) {
      throw new Error('Choose an original camera DNG, without converting or editing it.');
    }
    const crop = readRawCrop(bytes);
    const activeWidth = decoder.getActiveWidth();
    const activeHeight = decoder.getActiveHeight();
    const flip = decoder.getFlip();

    const width = decoder.getRawWidth();
    const height = decoder.getRawHeight();
    if (
      !Number.isSafeInteger(width) ||
      !Number.isSafeInteger(height) ||
      width <= 0 ||
      height <= 0 ||
      width * height > MAX_RAW_PIXELS
    ) {
      throw new Error('The DNG exceeds the supported sensor dimensions.');
    }

    decoder.setHalfSize(1);
    decoder.setUseCameraWb(1);
    decoder.setOutputColor(1);
    decoder.setOutputBps(16);
    decoder.setGamma(0, 1);
    decoder.setGamma(1, 1);
    decoder.setNoAutoBright(1);
    // Keep the camera white level instead of adjusting it from image content.
    decoder.setAdjustMaximumThr(0);
    decoder.unpack();
    decoder.dcrawProcess();
    // LibRaw applies the camera orientation and returns an owned byte buffer.
    const image = decoder.dcrawMakeMemImage();
    if (
      image.bits !== 16 ||
      image.colors !== 3 ||
      image.width <= 0 ||
      image.height <= 0 ||
      image.width * image.height > MAX_RAW_PIXELS ||
      image.data.byteLength !== image.width * image.height * 3 * 2
    ) {
      throw new Error('The DNG decoder returned an unsupported image.');
    }
    return cropRawImage(
      {
        width: image.width,
        height: image.height,
        data: new Uint16Array(image.data.buffer, image.data.byteOffset, image.data.byteLength / 2),
      },
      crop,
      activeWidth,
      activeHeight,
      flip,
    );
  } finally {
    decoder.dispose();
  }
}

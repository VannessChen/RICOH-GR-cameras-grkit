#ifndef GRKIT_ABI_H
#define GRKIT_ABI_H

/* Version 1 private memory layout, shared by the loader and ARM hooks. */
#define GRKIT_HOST_HEADER_SIZE 256
#define GRKIT_HOST_COUNT 0
#define GRKIT_HOST_RECORDS 4
#define GRKIT_HOST_ACTIVE 8
#define GRKIT_HOST_LAST_APPLIED 12
#define GRKIT_HOST_MAGIC 16
#define GRKIT_HOST_VERSION 20
#define GRKIT_HOST_OFF_ICON 84
#define GRKIT_HOST_BACK_PENDING 92
/* Guide style previewed on the ADJ canvas while GRKIT_HOST_FRAME_PREVIEW is set. */
#define GRKIT_HOST_GUIDE_PREVIEW 48
/* Date stamp style: 0 Off, 1 'YY M D, 2 'YY M D H:MM, 3 M D 'YY, 4 D M 'YY. */
#define GRKIT_HOST_STAMP 52
/* Grain level: 0 Off, 1 weak, 2 medium, 3 strong. The native grain stage of
   recorded pictures uses it instead of the native grain settings. */
#define GRKIT_HOST_GRAIN 72
/* Modules that MODULES.TXT hides from the menu: bit n is module n of
   runtime/modules.h. Bit 0 stays clear, because Image Control shows exactly
   when effects are installed. */
#define GRKIT_HOST_HIDDEN 56
/* Words 24..43 belong to the menu: two text descriptors (96..111) and a
   16-word event ring (112..175). */
#define GRKIT_HOST_PLUGINS_ICON 176
#define GRKIT_HOST_TOOLBAR_WIDGET 180
#define GRKIT_HOST_TOOLBAR_OWNER 184
#define GRKIT_HOST_MODULE 188
/* One icon chunk pointer: the runtime capacity fits a single chunk. */
#define GRKIT_HOST_ICON_CHUNKS 192
/* Aspect id 2 variant for the photo shown in playback: 0 none (shooting uses
   the Framing ratio), 1 not 65:24, 2 65:24. */
#define GRKIT_HOST_FRAME_FILE 196
/* Record of the last live-view build: an owned record, or 0 for native processing. */
#define GRKIT_HOST_LIVE_RECORD 200
/* Committed composition guide style: 0 Off, 1 thirds, 2 center, 3 golden ratio, 4 diagonals. */
#define GRKIT_HOST_GUIDE 204
#define GRKIT_HOST_FRAME_RATIO 208
/* ADJ Framing page preview: 0 none, else the highlighted ratio + 1. */
#define GRKIT_HOST_FRAME_PREVIEW 212
#define GRKIT_HOST_FRAME_DRAWS 216
/* ADJ canvas of the preview. It stays set until a full draw erases the mask. */
#define GRKIT_HOST_FRAME_PREVIEW_CANVAS 220
/* First module tile: the Framing tiles, then the Guides tiles, contiguous. */
#define GRKIT_HOST_FRAMING_ICON 224
#define GRKIT_HOST_FRAME_OWNER 228
#define GRKIT_HOST_FRAME_CANVAS 232
#define GRKIT_HOST_FRAME_BITMAP 236
#define GRKIT_HOST_FRAME_SKIPPED 240
/* Live-view-only override: 0 none, 1 native processing, else an owned record. */
#define GRKIT_HOST_PREVIEW 244
/* Record applied by the last non-live-view parameter build, or 0. A
   diagnostic: capture metadata and the encode read the shot ring. */
#define GRKIT_HOST_STILL 248
/* Return to GRKit Image Control after the native parameter page: 1 opening, 2 away. */
#define GRKIT_HOST_RETURN 252

#define GRKIT_RECORD_SIZE 0x740
#define GRKIT_RECORD_COEFFICIENTS 84
#define GRKIT_RECORD_NAME 0x668
#define GRKIT_RECORD_MONOCHROME 0x690
#define GRKIT_RECORD_ICON 0x694
#define GRKIT_RECORD_ID 0x698
/* The longest menu string, with its NUL: an Image Control summary is a prefix
   and a 16-character effect name. */
#define GRKIT_SUMMARY_UNITS 32
/* u16 ImageTone code recorded in photos: 0x8000 | hash of the effect ID. */
#define GRKIT_RECORD_CODE 0x6f8
/* GRF extension, zero for an effect without features. */
#define GRKIT_RECORD_FEATURES 0x700
#define GRKIT_FEATURE_POST 1U
#define GRKIT_FEATURE_SELECTIVE 2U
/* i16[9] signed Q13 RGB matrix after the tone curves, applied through the YCbCr
   matrix. */
#define GRKIT_RECORD_POST 0x704
/* Selective color for recorded pictures: i16 cos and sin of the target hue (Q12),
   u16 inner and outer tangents toward smaller hue angles, then toward larger
   ones (Q8), and u8 chroma gate c0 and c1. Bytes 0x724..0x72b are reserved and
   zero. */
#define GRKIT_RECORD_SELECTIVE 0x716
/* GR III 2.10. A target header can define a larger slot (see runtime/targets). */
#ifndef GRKIT_ICON_STRIDE
#define GRKIT_ICON_STRIDE 6408
#endif
/* Picture scratch after the records: the date stamp core and halo masks, then
   the shot ring. */
#define GRKIT_PICTURE_MASKS 141376
/* Shot ring: the record of each recent non-live-view parameter build, keyed by
   its shot settings pointer, which the maker-note setter (+0x18) and the JPEG
   encode configuration (+0x34) also hold. A later shot can start processing
   before an earlier one is written, so a single "last build" word is not
   enough. Word 0 counts the writes, and entry n is {key, record} at 8 + 8 n.
   The assembly computes the offset as 0x22800 + 0x40. */
#define GRKIT_SHOT_RING GRKIT_PICTURE_MASKS
#define GRKIT_SHOT_ENTRIES 8
#define GRKIT_PICTURE_SCRATCH (GRKIT_PICTURE_MASKS + 8 + 8 * GRKIT_SHOT_ENTRIES)

/* Menu text after the picture scratch, in each menu language: 0 English, 1
   Simplified Chinese, 2 Traditional Chinese. The installer copies the label
   table of the build to offset 0: for each language, GRKIT_TEXT_STRINGS u16
   byte offsets from the text start, then NUL-terminated UTF-16 strings. At
   GRKIT_TEXT_SUMMARY, the installer writes the Image Control summary of each
   record in each language, GRKIT_SUMMARY_UNITS units each, language-major. */
#define GRKIT_TEXT_SIZE 8192
/* Everything after the records: the script array must hold it
   (grkit_core::runtime::TAIL_SIZE). */
#define GRKIT_TAIL_SIZE (GRKIT_PICTURE_SCRATCH + GRKIT_TEXT_SIZE)
#define GRKIT_TEXT_SUMMARY 4096
#define GRKIT_TEXT_LANGUAGES 3
#define GRKIT_TEXT_RECORDS 20
#define GRKIT_TEXT(h) ((h)[GRKIT_HOST_RECORDS/4]+(h)[GRKIT_HOST_COUNT/4]*GRKIT_RECORD_SIZE+GRKIT_PICTURE_SCRATCH)
/* String IDs of the label table, in grkit_core::menu_text order. A run of IDs
   follows the rows or values that it names. */
#define TEXT_IMAGE_CONTROL 0
#define TEXT_FRAMING 1
#define TEXT_GUIDES 2
#define TEXT_DATE_STAMP 3
#define TEXT_GRAIN 4
#define TEXT_OFF 5
/* Framing rows 1..3: XPan 65:24, 16:9, 4:3. */
#define TEXT_RATIO 6
/* Guides rows 1..4: thirds, center, golden ratio, diagonals. */
#define TEXT_GUIDE 9
/* Date Stamp rows 1..4: the date styles. */
#define TEXT_STAMP 13
/* Grain rows 1..3: weak, medium, strong. */
#define TEXT_GRAIN_LEVEL 17
#define TEXT_IMAGE_OFF_SUMMARY 20
/* Summaries for each value, Off first. */
#define TEXT_FRAMING_SUMMARY 21
#define TEXT_GUIDES_SUMMARY 25
#define TEXT_STAMP_SUMMARY 30
#define TEXT_GRAIN_SUMMARY 35
/* The Image Control summary before an effect name, for the installer. */
#define TEXT_IMAGE_PREFIX 39
#define GRKIT_TEXT_STRINGS 40

#endif

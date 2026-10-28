#ifndef GRKIT_ASPECT_H
#define GRKIT_ASPECT_H
/* The capability CORE.ASPECT (grkit_core::capabilities): the aspect of still
   pictures. Its value is a ratio value: 0 for the native 3:2, then XPan
   65:24, 16:9, 4:3, 2.35:1, 1:1, 5:4, and 7:6 (GRKIT_CORE_ASPECT_VALUES). Each
   target offers the ratios of FRAMING_RATIO_ROWS.

   The core crops each ratio, also one that the camera has natively, so each
   ratio has the same live view and mask. The crop (GRKIT_HOST_FRAME_RATIO)
   is the capture area that the capture.aspect hooks and the overlay read.
   The camera keeps the native 3:2, and the resident loader saves the crop.

   The includer defines u8, u32, the header fields (runtime/abi.h), and
   aspect_native(). runtime/setting_store.h sets the value. */
/* The native still aspect id. Each component that includes this file
   defines it. */
static u32 aspect_native(void);
#ifndef CAPABILITY_USED
/* Whether a loaded module uses the capability `handle`. */
#define CAPABILITY_USED(h,handle) ((h)[GRKIT_HOST_CAPABILITIES/4]>>((handle)-GRKIT_CORE_SETTING_BASE)&1U)
#endif

static const u8 framing_rows[]={FRAMING_RATIO_ROWS};
/* The rows of the target, Off included, and the ratio value of a row. */
static __attribute__((unused)) u32 framing_ratios(void) { return sizeof(framing_rows); }
static __attribute__((unused)) u32 framing_ratio(u32 row) { return row<sizeof(framing_rows) ? framing_rows[row] : 0; }
/* The row of a ratio value, or 0 when the target does not offer it. */
static __attribute__((unused)) u32 framing_row(u32 value) {
    u32 row;
    for(row=1;row<sizeof(framing_rows);++row) if(framing_rows[row]==value) return row;
    return 0;
}
/* The committed crop: a ratio value that the target offers, or 0. Without a
   module that uses CORE.ASPECT, nothing crops. */
static __attribute__((unused)) u32 framing_crop(volatile u32 *h) {
    u32 ratio=h[GRKIT_HOST_FRAME_RATIO/4];
    if(!CAPABILITY_USED(h,GRKIT_CORE_ASPECT)) return 0;
    return ratio && framing_row(ratio) ? ratio : 0;
}
/* Whether the target offers a ratio value. */
static __attribute__((unused)) u32 aspect_offered(u32 value) { return !value || framing_row(value); }

#ifndef FRAMING_CROP_KEEPS
/* A crop that applies in a native aspect other than 3:2 (GR IV: XPan and
   2.35:1 also crop native 16:9 photos). */
#ifdef FRAMING_XPAN_NATIVE
#define FRAMING_CROP_KEEPS(native,crop) ((native)==FRAMING_XPAN_NATIVE && ((crop)==1 || (crop)==4))
#else
#define FRAMING_CROP_KEEPS(native,crop) 0
#endif
#endif

/* The value of CORE.ASPECT: the crop in native 3:2, or a crop that applies
   in the native aspect, else Off. A native aspect that the user sets in the
   camera menu is not a Framing ratio, so it reads as Off. */
static __attribute__((unused)) u32 aspect_value(volatile u32 *h) {
    u32 native=aspect_native(), crop=framing_crop(h);
    return !native || (crop && FRAMING_CROP_KEEPS(native,crop)) ? crop : 0;
}

#endif

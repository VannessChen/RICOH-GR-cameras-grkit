#ifndef GRKIT_ASPECT_H
#define GRKIT_ASPECT_H
/* The capability CORE.ASPECT (grkit_core::capabilities): the aspect of still
   pictures. Its value is a ratio value: 0 for the native 3:2, then XPan
   65:24, 16:9, 4:3, 2.35:1, 1:1, and 5:4 (GRKIT_CORE_ASPECT_VALUES). Each
   target offers the ratios of FRAMING_RATIO_ROWS.

   A ratio that the camera has natively is a native setting: the native
   viewfinder, photo, and DNG apply it. The core crops the other ratios: the
   crop (GRKIT_HOST_FRAME_RATIO) is the capture area that the capture.aspect
   hooks and the overlay read. The camera keeps the native aspect, and the
   resident loader saves the crop.

   The includer defines u8, u32, and the header fields (runtime/abi.h). A
   component that reads the value defines ASPECT_NATIVE() as the native
   still aspect id. A component that sets it also defines
   ASPECT_SET_NATIVE(id), which sets the native aspect as the native menu
   does, and ASPECT_REQUEST(), which sends the native aspect request again. */
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
/* The native aspect id that a ratio value sets, or FRAMING_CROP when the
   core crops it (FRAMING_NATIVE_IDS). Off sets the native 3:2. */
#define FRAMING_CROP 0xffU
static __attribute__((unused)) u32 framing_native(u32 value) {
    static const u8 ids[]={FRAMING_NATIVE_IDS};
    return value<sizeof(ids) ? ids[value] : FRAMING_CROP;
}
/* The committed crop: a ratio value that the target crops, or 0. Without a
   module that uses CORE.ASPECT, nothing crops. */
static __attribute__((unused)) u32 framing_crop(volatile u32 *h) {
    u32 ratio=h[GRKIT_HOST_FRAME_RATIO/4];
    if(!CAPABILITY_USED(h,GRKIT_CORE_ASPECT)) return 0;
    return ratio && framing_row(ratio) && framing_native(ratio)==FRAMING_CROP ? ratio : 0;
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

#ifdef ASPECT_NATIVE
/* The value of CORE.ASPECT: the crop in native 3:2, a crop that applies in
   the native aspect, else the ratio of the native aspect. A change of the
   native aspect in the camera menu therefore shows in the value. */
static __attribute__((unused)) u32 aspect_value(volatile u32 *h) {
    u32 native=ASPECT_NATIVE(), crop=framing_crop(h), value;
    if(!native || (crop && FRAMING_CROP_KEEPS(native,crop))) return crop;
    for(value=1;value<GRKIT_CORE_ASPECT_VALUES;++value)
        if(framing_row(value) && framing_native(value)==native) return value;
    return 0;
}
#endif

#ifdef ASPECT_SET_NATIVE
/* Set CORE.ASPECT to a value that the target offers. A native ratio (Off is
   native 3:2) clears the crop and sets its native aspect. A crop needs
   native 3:2: it sets that, then sends the request again, so the capture
   hook applies the new crop. */
static __attribute__((unused)) void aspect_store(volatile u32 *h,u32 value) {
    u32 native=framing_native(value);
    if(native!=FRAMING_CROP) {
        h[GRKIT_HOST_FRAME_RATIO/4]=0;
        if(ASPECT_NATIVE()!=native) ASPECT_SET_NATIVE(native);
        else ASPECT_REQUEST();
        return;
    }
    h[GRKIT_HOST_FRAME_RATIO/4]=value;
    if(ASPECT_NATIVE()) ASPECT_SET_NATIVE(0);
    ASPECT_REQUEST();
}
#endif
#endif

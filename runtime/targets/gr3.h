/* RICOH GR III family: the values that the shared runtime takes from each
   family, as GR III builds have them. A build header (gr3-<version>.h)
   defines the firmware addresses (FW_*) and the other values of its build,
   then includes this file. Comments here name no firmware address, because
   each build has its own; the build header gives them. */
#ifndef GRKIT_GR3_H
#define GRKIT_GR3_H

/* Icon slots hold an 8-byte descriptor, then 40 x 40 RGBA. */
#define GRKIT_ICON_STRIDE 6408

/* Framing: Off, XPan, 16:9, and 4:3. GR III has live-view modes only for 3:2
   and 1:1, so each ratio captures through a hidden aspect id from native 3:2. */
#define FRAMING_RATIO_COUNT 4U
/* Requested capture StillAspectRatio (CommonProperty +0x7b6). The property
   setter writes the next copy at [property+4]; readers see the current copy
   at [property] only after a later commit. Comparing the next copy sends
   each change once. */
#define FRAMING_CAPTURE_ASPECT() BYTE(WORD(WORD(WORD(((u32 (*)(void))FW_CAPTURE_PROPERTIES)(),0),0x518),4),0x7b6)
/* Native StillAspectRatio setter with the unchanged 3:2 UI value. It sends
   capture property 0xd4 again and posts no UI change event. */
#define FRAMING_REQUEST_ASPECT() (((u32 (*)(u32,u32))FW_STILL_ASPECT_SET)(FRAMING_SETTINGS(),0))
/* Resend the native StillAspectRatio value, as a native menu change does.
   The Framing capture hook maps a 3:2 request to the Framing ratio. */
#define MENU_FRAMING_REQUEST() do { u32 s=CALL0(FW_SETTINGS)(); CALL2(FW_STILL_ASPECT_SET)(s,CALL1(FW_STILL_ASPECT_GET)(s)); } while(0)
/* GetResolution id 2: the stock 16:9 height and the 65:24 height of each size. */
#define XPAN_HEIGHT_0 3376, 2216
#define XPAN_HEIGHT_1 2704, 1776
#define XPAN_HEIGHT_2 1888, 1240
#define XPAN_HEIGHT_3 1080, 712
/* The 65:24 sizes (width, height), largest first. */
#define XPAN_SIZES 6000, 2216, 4800, 1776, 3360, 1240, 1920, 712
/* The size classifier at its aspect BL: the frame is {r4-r7,fp,ip,lr,pc},
   r7 = width, r4 = height, r6 = class pointer. */
#define CLASSIFY_WIDTH r7
#define CLASSIFY_CLASS r6
#define CLASSIFY_RETURN ldm sp, {r4,r5,r6,r7,fp,sp,pc}

/* Visible-area id of a native aspect. GR III offers only 3:2 (0) and 1:1 (3). */
#define PICTURE_NATIVE_ID(native) ((native)==3 ? 3 : 0)

#endif

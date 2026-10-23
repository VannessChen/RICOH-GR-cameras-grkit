/* Framing: the menu page. Row 0 is Off, then each ratio (FRAMING_RATIO_ROWS),
   on every camera. A ratio that the camera has natively sets the native
   aspect (framing_native), and the native viewfinder and capture apply it.
   Framing crops the other ratios: the committed crop value
   (GRKIT_HOST_FRAME_RATIO) is the capture area that the core's
   capture.aspect hooks and the other plugins read. Labels, summaries, and
   tiles follow the value. */
#include "../../plugin.h"

#ifndef FRAMING_CROP_KEEPS
/* A crop that applies in a native aspect other than 3:2 (GR IV: XPan and
   2.35:1 also crop native 16:9 photos). */
#ifdef FRAMING_XPAN_NATIVE
#define FRAMING_CROP_KEEPS(native,crop) ((native)==FRAMING_XPAN_NATIVE && ((crop)==1 || (crop)==4))
#else
#define FRAMING_CROP_KEEPS(native,crop) 0
#endif
#endif

/* The ratio that the camera applies now: the crop in native 3:2, a crop that
   applies in the native aspect, else the ratio of the native aspect. A
   change of the native aspect in the camera menu therefore shows here. */
static u32 frame_ratio(volatile u32 *h) {
    u32 native=camera_aspect(), crop=framing_crop(h), value;
    if(!native || (crop && FRAMING_CROP_KEEPS(native,crop))) return crop;
    for(value=1;value<FRAMING_RATIO_MAX+1U;++value)
        if(framing_row(value) && framing_native(value)==native) return value;
    return 0;
}
static u32 frame_committed(volatile u32 *h) { return framing_row(frame_ratio(h)); }
static const u16 *frame_title(volatile u32 *h) { return text(h,TEXT_FRAMING); }
static const u16 *frame_summary(volatile u32 *h) { return text(h,TEXT_FRAMING_SUMMARY+frame_ratio(h)); }
static u32 frame_rows(volatile u32 *h) { (void)h; return framing_ratios(); }
/* 65:24 is named after the XPan panoramic format. */
static const u16 *frame_label(volatile u32 *h,u32 row) {
    u32 value=framing_ratio(row);
    return text(h,value ? TEXT_RATIO+value-1 : TEXT_OFF);
}
/* The root tile shows the selected ratio, like Image Control shows its effect. */
static u32 frame_icon(volatile u32 *h,u32 row) {
    if(h[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT) return GRKIT_ICON_TILE(frame_ratio(h));
    return row ? GRKIT_ICON_TILE(framing_ratio(row)) : GRKIT_ICON_OFF;
}
static u32 frame_checked(volatile u32 *h,u32 row) { return row==frame_committed(h); }
/* A native ratio (Off is native 3:2) clears the crop and sets its native
   aspect. A crop needs native 3:2: it sets that, then resends the request so
   the capture hook applies the new crop. */
static void frame_apply(volatile u32 *h,u32 row) {
    u32 value=framing_ratio(row), native=framing_native(value);
    if(native!=FRAMING_CROP) {
        h[GRKIT_HOST_FRAME_RATIO/4]=0;
        if(camera_aspect()!=native) set_camera_aspect(native);
        else request_native_aspect();
        return;
    }
    h[GRKIT_HOST_FRAME_RATIO/4]=value;
    if(camera_aspect()) set_camera_aspect(0);
    request_native_aspect();
}

/* The module tile, then a tile for each ratio value. */
#define FRAMING_PAGE {frame_title,frame_summary,frame_rows,frame_label,frame_icon, \
    frame_checked,frame_apply,frame_committed,0,0,FRAMING_TILES}

/* Framing: the menu page. Row 0 is Off, then the ratios of the target
   (XPan 65:24, 16:9, 4:3). The ratio is the capture area that the core's
   capture.aspect hooks and the other plugins read (GRKIT_HOST_FRAME_RATIO). */
#include "../../plugin.h"

static u32 frame_ratio(volatile u32 *h) {
    u32 value=h[GRKIT_HOST_FRAME_RATIO/4];
    return value<framing_ratios() ? value : 0;
}
static const u16 *frame_title(volatile u32 *h) { return text(h,TEXT_FRAMING); }
static const u16 *frame_summary(volatile u32 *h) { return text(h,TEXT_FRAMING_SUMMARY+frame_ratio(h)); }
static u32 frame_rows(volatile u32 *h) { (void)h; return framing_ratios(); }
/* 65:24 is named after the XPan panoramic format. */
static const u16 *frame_label(volatile u32 *h,u32 row) { return text(h,row ? TEXT_RATIO+row-1 : TEXT_OFF); }
/* The root tile shows the selected ratio, like Image Control shows its effect. */
static u32 frame_icon(volatile u32 *h,u32 row) {
    if(h[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT) return GRKIT_ICON_TILE(frame_ratio(h));
    return row ? GRKIT_ICON_TILE(row) : GRKIT_ICON_OFF;
}
static u32 frame_checked(volatile u32 *h,u32 row) { return row==frame_ratio(h); }
static void frame_apply(volatile u32 *h,u32 row) {
    h[GRKIT_HOST_FRAME_RATIO/4]=row;
    request_native_aspect();
}

/* The module tile, then XPan, 16:9, and 4:3. */
#define FRAMING_PAGE {frame_title,frame_summary,frame_rows,frame_label,frame_icon, \
    frame_checked,frame_apply,frame_ratio,0,0,4}

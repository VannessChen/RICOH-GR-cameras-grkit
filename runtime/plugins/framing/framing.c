/* Framing: the viewfinder.overlay layers and the capture.aspect answer. The
   Framing ratio (GRKIT_HOST_FRAME_RATIO) is the capture area, which the core
   and the other plugins read. */
#include "../../plugin.h"

/* The committed crop (runtime/plugin.h). A ratio that the camera has
   natively, or a value that the target does not offer, draws and crops
   nothing. */
static u32 framing_value(volatile u32 *h) { return framing_crop(h); }

/* Capture aspect ids of the crops: 0 = 3:2, 1 = 4:3, 2 = 16:9. XPan and 2.35:1
   use id 2 with their own heights, and 5:4 uses id 1 with its own widths
   (runtime/framing.S). GR III hides ids 1 and 2 from its menu, but its capture
   and playback code support them. 1:1 is native on each camera. */
GRKIT_MAYBE_UNUSED static u32 framing_capture_id(volatile u32 *h) {
    static const u8 ids[]={0,2,2,1,2,0,1};
    return ids[framing_value(h)];
}
/* The ratio of the badge layer. The badge shows only with the full display,
   like the top row of native settings. The mask stays in each display,
   because it shows the capture area. */
static u32 framing_badge_value(volatile u32 *h) {
    return native_display()==DISPLAY_FULL ? framing_value(h) : 0;
}
/* Black at 50% alpha outside the photo area. Each full draw paints the mask,
   so the loops write only the bars: the rows above and below the photo area,
   and the columns at its sides. */
#define FRAMING_MASK_COLOR 0x80000000U
static void mask_span(volatile u32 *row,u32 from,u32 to) {
    for(;from<to;++from) row[from]=FRAMING_MASK_COLOR;
}
static void framing_mask(const Surface *s,u32 value) {
    u32 y;
    volatile u32 *row;
    (void)value;
    if(!s->area) return;
    for(y=0;y<480;++y) {
        row=OVERLAY_PIXELS(s->data+y*s->stride*4);
        if(y<s->box[1] || y>=s->box[3]) { mask_span(row,0,720); continue; }
        mask_span(row,0,s->box[0]);
        mask_span(row,s->box[2],720);
    }
}

/* The ratio label at the left of the second row. */
static void framing_badge(const Surface *s,u32 value) {
    (void)value;
    if(s->area) overlay_label(s,LABEL_FRAMING(s->area),LABEL_LEFT,LABEL_TOP);
}

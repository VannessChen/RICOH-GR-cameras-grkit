/* Framing: the viewfinder.overlay layers and the capture.aspect answer. The
   Framing ratio (GRKIT_HOST_FRAME_RATIO) is the capture area, which the core
   and the other plugins read. */
#include "../../plugin.h"

/* Capture aspect ids: 0 = 3:2, 1 = 4:3, 2 = 16:9 and 65:24. GR III hides
   ids 1 and 2 from its menu, but its capture and playback code support them. */
GRKIT_MAYBE_UNUSED static u32 framing_capture_id(volatile u32 *h) {
    u32 ratio=h[GRKIT_HOST_FRAME_RATIO/4];
    return ratio==3 ? 1 : ratio==1 || ratio==2 ? 2 : 0;
}

/* The committed ratio, with an unknown value treated as Off. */
static u32 framing_value(volatile u32 *h) {
    u32 ratio=h[GRKIT_HOST_FRAME_RATIO/4];
    return ratio<framing_ratios() ? ratio : 0;
}
/* The ratio of the badge layer. The badge shows only with the full display,
   like the top row of native settings. The mask stays in each display,
   because it shows the capture area. */
static u32 framing_badge_value(volatile u32 *h) {
    return native_display()==DISPLAY_FULL ? framing_value(h) : 0;
}
/* Black at 50% alpha outside the photo area. */
static void framing_mask(const Surface *s,u32 value) {
    u32 x,y;
    volatile u32 *row;
    (void)value;
    if(!s->area) return;
    for(y=0;y<480;++y) {
        row=OVERLAY_PIXELS(s->data+y*s->stride*4);
        for(x=0;x<720;++x)
            if(y<s->box[1] || y>=s->box[3] || x<s->box[0] || x>=s->box[2]) row[x]=0x80000000;
    }
}

/* The ratio label at the left of the second row. */
static void framing_badge(const Surface *s,u32 value) {
    (void)value;
    if(s->area) overlay_label(s,LABEL_FRAMING(s->area),LABEL_LEFT,LABEL_TOP);
}

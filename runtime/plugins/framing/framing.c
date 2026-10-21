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

/* The ratio label at the top left. Original 5x7 column glyphs, bit zero at the
   top. Each pixel is 2x2 LCD pixels. */
static void framing_badge(const Surface *s,u32 value) {
    static const u8 glyphs[][5]={
        {0x00,0x42,0x7f,0x40,0x00}, /* 1 */
        {0x62,0x51,0x49,0x49,0x46}, /* 2 */
        {0x18,0x14,0x12,0x7f,0x10}, /* 4 */
        {0x27,0x45,0x45,0x45,0x39}, /* 5 */
        {0x3c,0x4a,0x49,0x49,0x30}, /* 6 */
        {0x06,0x49,0x49,0x29,0x1e}, /* 9 */
        {0x00,0x36,0x36,0x00,0x00}, /* : */
        {0x22,0x41,0x49,0x49,0x36}, /* 3 */
        {0x63,0x14,0x08,0x14,0x63}, /* X */
        {0x7f,0x09,0x09,0x09,0x06}, /* P */
        {0x20,0x54,0x54,0x54,0x78}, /* a */
        {0x7c,0x08,0x04,0x04,0x78}  /* n */
    };
    /* XPan, 16:9, 4:3 */
    static const u8 labels[][5]={{8,9,10,11,255},{0,4,6,5,255},{2,6,7,255,255}};
    u32 x,y,i,dy,ratio=s->area,data=s->data,stride=s->stride;
    volatile u32 *row;
    (void)value;
    if(!ratio) return;
    for(y=46;y<68;++y) {
        row=OVERLAY_PIXELS(data+y*stride*4);
        for(x=204;x<270;++x) row[x]=0x80000000;
    }
    for(i=0;labels[ratio-1][i]!=255;++i)
        for(x=0;x<5;++x) for(y=0;y<7;++y)
            if(glyphs[labels[ratio-1][i]][x]&(1U<<y))
                for(dy=0;dy<2;++dy) {
                    row=OVERLAY_PIXELS(data+(50+y*2+dy)*stride*4)+208+i*12+x*2;
                    row[0]=row[1]=0xffffffff;
                }
}

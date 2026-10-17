/* Guides: the viewfinder.overlay layer. Guides stay inside the photo area of
   the capture area. */
#include "../../plugin.h"

#define GUIDE_LAST 4U
/* The committed style, with an unknown value treated as Off. */
static u32 guides_value(volatile u32 *h) {
    u32 style=h[GRKIT_HOST_GUIDE/4];
    return style<=GUIDE_LAST ? style : 0;
}
/* Opaque gray is the same pixel with straight or premultiplied alpha. */
#define GUIDE_COLOR 0xffc0c0c0U
#define GUIDE_ARM 20U
static void guide_pixel(u32 data,u32 stride,u32 x,u32 y) { OVERLAY_PIXELS(data+y*stride*4)[x]=GUIDE_COLOR; }
/* A position at `fraction`/65536 of the span, rounded. The link has no division helpers. */
static u32 guide_at(u32 start,u32 end,u32 fraction) { return start+(((end-start)*fraction+0x8000)>>16); }
/* One vertical and one horizontal line at each fraction of the photo area. */
static void guide_grid(u32 data,u32 stride,const u16 *box,u32 fraction) {
    u32 i,j,f,x,y;
    for(i=0;i<2;++i) {
        f=i ? 65536-fraction : fraction;
        x=guide_at(box[0],box[2],f);
        y=guide_at(box[1],box[3],f);
        for(j=box[1];j<box[3];++j) guide_pixel(data,stride,x,j);
        for(j=box[0];j<box[2];++j) guide_pixel(data,stride,j,y);
    }
}
/* Corner-to-corner Bresenham lines. Every photo area is wider than tall, so
   each column has one pixel of each diagonal, rounded half up. */
static void guide_diagonals(u32 data,u32 stride,const u16 *box) {
    int run=box[2]-box[0]-1, rise=box[3]-box[1]-1, error=0;
    u32 x, y=box[1];
    for(x=0;x<=(u32)run;++x) {
        guide_pixel(data,stride,box[0]+x,y);
        guide_pixel(data,stride,box[2]-1-x,y);
        error+=rise;
        if(2*error>=run) { ++y; error-=run; }
    }
}
/* Styles: 1 thirds, 2 center, 3 golden ratio, 4 diagonals. Grid fractions are
   in 1/65536: 1/3 for thirds and 0.381966 for the golden section. */
static void guides_paint(const Surface *s,u32 style) {
    const u16 *box=s->box;
    u32 x,y,i,data=s->data,stride=s->stride;
    if(style==1) guide_grid(data,stride,box,21845);
    if(style==3) guide_grid(data,stride,box,25033);
    if(style==4) guide_diagonals(data,stride,box);
    if(style==2) {
        x=guide_at(box[0],box[2],32768);
        y=guide_at(box[1],box[3],32768);
        for(i=0;i<=2*GUIDE_ARM;++i) {
            guide_pixel(data,stride,x-GUIDE_ARM+i,y);
            guide_pixel(data,stride,x,y-GUIDE_ARM+i);
        }
    }
}

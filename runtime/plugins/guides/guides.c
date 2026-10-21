/* Guides: the viewfinder.overlay layer. Guides stay inside the photo area of
   the capture area. */
#include "../../plugin.h"

/* The committed style, with an unknown value treated as Off. The guides hide
   while DISP hides the native live-view information. */
static u32 guides_value(volatile u32 *h) {
    u32 style=h[GRKIT_HOST_GUIDE/4];
    return style<=GUIDES_STYLE_MAX && native_display()<=DISPLAY_SIMPLE ? style : 0;
}
/* The native grid guide (GridView) writes palette entry 16, black at 50%
   alpha, in 2-pixel lines. The value is the same with straight or
   premultiplied alpha. */
#define GUIDE_COLOR 0x80000000U
#define GUIDE_ARM 20U
static void guide_pixel(u32 data,u32 stride,u32 x,u32 y) { OVERLAY_PIXELS(data+y*stride*4)[x]=GUIDE_COLOR; }
/* A 2-pixel vertical line on columns x - 1 and x, or a horizontal line on rows
   y - 1 and y, as the native grid draws on each side of a division. */
static void guide_column(u32 data,u32 stride,u32 x,u32 top,u32 bottom) {
    u32 y;
    for(y=top;y<bottom;++y) { guide_pixel(data,stride,x-1,y); guide_pixel(data,stride,x,y); }
}
static void guide_row(u32 data,u32 stride,u32 y,u32 left,u32 right) {
    u32 x;
    for(x=left;x<right;++x) { guide_pixel(data,stride,x,y-1); guide_pixel(data,stride,x,y); }
}
/* A position at `fraction`/65536 of the span, rounded. The link has no division helpers. */
static u32 guide_at(u32 start,u32 end,u32 fraction) { return start+(((end-start)*fraction+0x8000)>>16); }
/* The native thirds: the divisions are at multiples of floor(span / 3).
   21846/65536 gives the exact floor for each span up to 720. */
static void guide_thirds(u32 data,u32 stride,const u16 *box) {
    u32 i, column=((box[2]-box[0])*21846U)>>16, row=((box[3]-box[1])*21846U)>>16;
    for(i=1;i<3;++i) {
        guide_column(data,stride,box[0]+i*column,box[1],box[3]);
        guide_row(data,stride,box[1]+i*row,box[0],box[2]);
    }
}
/* The golden section: one vertical and one horizontal line at each side. */
static void guide_golden(u32 data,u32 stride,const u16 *box) {
    u32 i,f;
    for(i=0;i<2;++i) {
        f=i ? 65536-25033 : 25033;
        guide_column(data,stride,guide_at(box[0],box[2],f),box[1],box[3]);
        guide_row(data,stride,guide_at(box[1],box[3],f),box[0],box[2]);
    }
}
/* Corner-to-corner Bresenham lines, 2 pixels high in each column. Every photo
   area is wider than tall, so each column has one pair of each diagonal. The
   upper pixel goes from the top row to the row above the bottom one, rounded
   half up. */
static void guide_diagonals(u32 data,u32 stride,const u16 *box) {
    int run=box[2]-box[0]-1, rise=box[3]-box[1]-2, error=0;
    u32 x, y=box[1];
    for(x=0;x<=(u32)run;++x) {
        guide_pixel(data,stride,box[0]+x,y);
        guide_pixel(data,stride,box[0]+x,y+1);
        guide_pixel(data,stride,box[2]-1-x,y);
        guide_pixel(data,stride,box[2]-1-x,y+1);
        error+=rise;
        if(2*error>=run) { ++y; error-=run; }
    }
}
/* A 2-pixel cross at the center of the photo area. Each arm reaches
   GUIDE_ARM pixels past the 2 center pixels. */
static void guide_cross(u32 data,u32 stride,const u16 *box) {
    u32 x=guide_at(box[0],box[2],32768), y=guide_at(box[1],box[3],32768);
    guide_row(data,stride,y,x-1-GUIDE_ARM,x+1+GUIDE_ARM);
    guide_column(data,stride,x,y-1-GUIDE_ARM,y+1+GUIDE_ARM);
}
/* Styles: 1 thirds, 2 center, 3 golden ratio, 4 diagonals, 5 thirds and
   center. The golden section is 0.381966, 25033/65536. */
static void guides_paint(const Surface *s,u32 style) {
    const u16 *box=s->box;
    u32 data=s->data,stride=s->stride;
    if(style==1 || style==5) guide_thirds(data,stride,box);
    if(style==3) guide_golden(data,stride,box);
    if(style==4) guide_diagonals(data,stride,box);
    if(style==2 || style==5) guide_cross(data,stride,box);
}

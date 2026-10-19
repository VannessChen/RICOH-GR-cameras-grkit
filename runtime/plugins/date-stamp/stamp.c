/* Date Stamp: the picture.encode handler. It prints the local time on each
   recorded picture, as a film data back exposes it. */
#include "../../plugin.h"

/* Longest blurred line of a date stamp mask. The line copy lives on the stack,
   because the code region is read-only. */
#define BLUR_WIDTH 640U

/* Spread a mask line in place: three box passes, radius r. */
static void blur_line(u8 *line,u32 step,u32 n,u32 r) {
    u16 copy[BLUR_WIDTH];
    u32 i,pass,sum,window=2*r+1,reciprocal=divide(65536+window/2,window);
    for(pass=0;pass<3;++pass) {
        for(i=0;i<n;++i) copy[i]=line[i*step];
        sum=copy[0]*(r+1);
        for(i=1;i<=r;++i) sum+=copy[i<n ? i : n-1];
        for(i=0;i<n;++i) {
            line[i*step]=(u8)((sum*reciprocal)>>16);
            sum+=copy[i+r+1<n ? i+r+1 : n-1];
            sum-=copy[i>=r ? i-r : 0];
        }
    }
}

/* Date stamp in the style of a film data back: italic seven-segment digits
   that light the picture like an LED exposure, an amber core in a red-orange
   halo. The digits render into a small coverage mask (at most STAMP_CELLS
   rows), which two box blurs turn into the core and the halo, then the mask is
   scaled up and added to the picture with a screen blend.
   Segments: a top, b upper right, c lower right, d bottom, e lower left,
   f upper left, g middle. */
/* Mask digit height: up to 48 cells, or 32 when the text would exceed the
   mask width. Higher masks keep the strokes sharper after scaling. */
#define STAMP_CELLS 48U
#define STAMP_CELLS_LONG 32U
static const u8 segments[10]={0x3f,0x06,0x5b,0x4f,0x66,0x6d,0x7d,0x07,0x7f,0x6f};
/* Mask advance of one character for digit height m. */
static u32 advance(char c,u32 m) {
    if(c>='0' && c<='9') return (m*54395)>>16;
    return c==' ' ? (m*22938)>>16 : (m*19661)>>16;
}
static u32 text_width(const char *text,u32 n,u32 m) {
    u32 i,total=0;
    for(i=0;i<n;++i) total+=advance(text[i],m);
    /* Every style ends in a digit: drop its trailing gap. */
    return total-((m*18000)>>16);
}
/* Cover mask row y from x0 to x1 (Q8 cells), keeping the larger coverage. */
static void span(u8 *row,u32 width,int x0,int x1) {
    int i;
    if(x0<0) x0=0;
    if(x1>(int)width<<8) x1=(int)width<<8;
    for(i=x0>>8;i<<8<x1;++i) {
        int a=i<<8,b=a+256,from=x0>a ? x0 : a,to=x1<b ? x1 : b;
        u32 cover=(u32)(to-from)*255>>8;
        if(to>from && cover>row[i]) row[i]=(u8)cover;
    }
}
/* A sheared bar: rows y0..y1 of the digit, x0..x1 (Q8) at its baseline. Upper
   rows lean right by 0.12 cell per row. */
static void bar(u8 *mask,u32 width,int x,u32 top,u32 m,int x0,int x1,u32 y0,u32 y1) {
    u32 y;
    for(y=y0;y<y1;++y) {
        int lean=(int)((m-y)*31);
        span(mask+(top+y)*width,width,x+x0+lean,x+x1+lean);
    }
}
static void glyph(u8 *mask,u32 width,char c,int x,u32 top,u32 m) {
    int w=(int)((m*36045)>>8), t=(int)((m*8520)>>8), g=(int)((m*1966)>>8);
    u32 rows=(u32)t>>8, half=m>>1, bits;
    if(!rows) rows=1;
    if(c=='\'') { bar(mask,width,x,top,m,(int)(m*13),(int)(m*13)+t,0,(m*19661)>>16); return; }
    if(c==':') {
        bar(mask,width,x,top,m,0,t,(m*19661)>>16,((m*19661)>>16)+rows);
        bar(mask,width,x,top,m,0,t,(m*45875)>>16,((m*45875)>>16)+rows);
        return;
    }
    if(c<'0' || c>'9') return;
    bits=segments[c-'0'];
    if(bits&1) bar(mask,width,x,top,m,t+g,w-t-g,0,rows);
    if(bits&64) bar(mask,width,x,top,m,t+g,w-t-g,half-(rows>>1),half-(rows>>1)+rows);
    if(bits&8) bar(mask,width,x,top,m,t+g,w-t-g,m-rows,m);
    if(bits&32) bar(mask,width,x,top,m,0,t,(rows>>1)+1,half-1);
    if(bits&2) bar(mask,width,x,top,m,w-t,w,(rows>>1)+1,half-1);
    if(bits&16) bar(mask,width,x,top,m,0,t,half+1,m-(rows>>1)-1);
    if(bits&4) bar(mask,width,x,top,m,w-t,w,half+1,m-(rows>>1)-1);
}
/* Digits without leading zeros, as a data back prints them. */
static char *number(char *o,u32 v) {
    char d[4];
    u32 n=0;
    do { u32 q=0; while(v>=10) { v-=10; ++q; } d[n++]=(char)('0'+v); v=q; } while(v && n<4);
    while(n) *o++=d[--n];
    return o;
}
static char *two(char *o,u32 v) {
    u32 tens=0;
    while(v>=10) { v-=10; ++tens; }
    *o++=(char)('0'+tens); *o++=(char)('0'+v);
    return o;
}
/* Format one style: 1 'YY M D, 2 'YY M D H:MM, 3 M D 'YY, 4 D M 'YY,
   5 D M 'YY H:MM. Two spaces come before the time. */
static u32 date_text(char *o,const u8 *dt,u32 style) {
    char *start=o;
    u32 year=dt[0]|dt[1]<<8, short_year=year>=2000 ? year-2000 : 0;
    while(short_year>=100) short_year-=100;
    if(style<=2) {
        *o++='\''; o=two(o,short_year); *o++=' '; o=number(o,dt[2]); *o++=' '; o=number(o,dt[3]);
    } else {
        o=number(o,style==3 ? dt[2] : dt[3]); *o++=' '; o=number(o,style==3 ? dt[3] : dt[2]);
        *o++=' '; *o++='\''; o=two(o,short_year);
    }
    if(style==2 || style==5) { *o++=' '; *o++=' '; o=number(o,dt[4]); *o++=':'; o=two(o,dt[5]); }
    return (u32)(o-start);
}
static u32 clamp255(int v) { return v<0 ? 0 : v>255 ? 255 : (u32)v; }
/* Screen blend of light l over value v: v + (255 - v) * l / 255. */
static u32 screen(u32 v,u32 l) { return v+((((255-v)*l)*257+32768)>>16); }
/* Add the stamp light to one pixel pair that shares Cb and Cr. */
static void light_pair(const Plane *p,u32 x,u32 y,const u32 *core,const u32 *halo,u32 mono,u32 chroma) {
    u32 i,yv[2],crow=chroma_row(p,y);
    int cb=(int)BYTE(crow,x)-128, cr=(int)BYTE(crow,x+1)-128, sb=0, sr=0;
    for(i=0;i<2;++i) {
        u32 lr=core[i]+((halo[i]*180)>>8), lg=((core[i]*150)>>8)+((halo[i]*40)>>8), lb=(core[i]*40)>>8;
        u32 luma=BYTE(p->y,y*p->pitch+x+i), r, g, b;
        if(lr>255) lr=255;
        if(mono) { yv[i]=screen(luma,(19595*lr+38470*lg+7471*lb+32768)>>16); continue; }
        r=screen(clamp255((int)luma+((91881*cr)>>16)),lr);
        g=screen(clamp255((int)luma-((22554*cb+46802*cr)>>16)),lg);
        b=screen(clamp255((int)luma+((116130*cb)>>16)),lb);
        yv[i]=(19595*r+38470*g+7471*b+32768)>>16;
        sb+=(int)(-11059*(int)r-21709*(int)g+32768*(int)b);
        sr+=(int)(32768*(int)r-27439*(int)g-5329*(int)b);
    }
    BYTE(p->y,y*p->pitch+x)=(u8)yv[0];
    BYTE(p->y,y*p->pitch+x+1)=(u8)yv[1];
    if(!mono && chroma) {
        BYTE(crow,x)=(u8)clamp255(128+((sb+65536)>>17));
        BYTE(crow,x+1)=(u8)clamp255(128+((sr+65536)>>17));
    }
}
/* A rendered stamp mask and where it lies on its picture: one cell spans
   f / ss pixels, and cell (0,0) is at pixel (ox, oy). */
typedef struct { const u8 *core,*halo; u32 mw,mh,f,ss; int ox,oy; } Stamp;
/* Bottom right of the visible area, digits at 4.5% of its short side. Small
   pictures (the screennail that playback shows first, and the thumbnail) render
   the mask at twice their resolution and average 2 x 2 cells per pixel, so their
   strokes stay as sharp as on the main picture: there, one mask cell spans
   several pixels instead. Returns 1 and describes the mask in `out` when it drew. */
static u32 stamp(u8 *scratch,const Plane *p,const char *text,u32 n,u32 mono,Stamp *out) {
    u32 width=p->right-p->left, height=p->bottom-p->top, size, f, ss=1, m, pad, cw, mw, mh, margin;
    u32 i, x, y, sx, sy, dx, dy, step, total;
    int caret;
    u8 *core=scratch, *halo;
    size=((width<height ? width : height)*2949)>>16;
    if(size<8) size=8;
    for(i=0;i<2;++i) {
        u32 cells=i ? STAMP_CELLS_LONG : STAMP_CELLS;
        f=size<=cells ? 1 : divide(size+cells-1,cells);
        ss=f==1 && 2*size<=cells ? 2 : 1;
        m=ss==2 ? 2*size : divide(size+f/2,f);
        pad=((m*5)>>5)*2+2;
        total=text_width(text,n,m);
        cw=total+((m*31)>>8)+1;
        mw=cw+2*pad; mh=m+2*pad;
        if(mw<=BLUR_WIDTH && mw*mh*2<=GRKIT_PICTURE_MASKS) break;
    }
    if(mw>BLUR_WIDTH || mh>BLUR_WIDTH || mw*mh*2>GRKIT_PICTURE_MASKS) return 0;
    margin=(size*45875)>>16;
    if((((mw-pad)*f)>>(ss-1))+margin>width || (((mh-pad)*f)>>(ss-1))+margin>height) return 0;
    halo=core+mw*mh;
    for(i=0;i<mw*mh;++i) core[i]=0;
    for(i=0,caret=(int)pad<<8;i<n;++i) { glyph(core,mw,text[i],caret,pad,m); caret+=(int)advance(text[i],m)<<8; }
    for(i=0;i<mw*mh;++i) halo[i]=core[i];
    for(y=0;y<mh;++y) { blur_line(core+y*mw,1,mw,1); blur_line(halo+y*mw,1,mw,(m*5)>>5 ? (m*5)>>5 : 1); }
    for(x=0;x<mw;++x) { blur_line(core+x,mw,mh,1); blur_line(halo+x,mw,mh,(m*5)>>5 ? (m*5)>>5 : 1); }
    if(ss==2) {
        /* Pixel origin of the mask, on an even column so CbCr pairs line up. */
        int ox=(int)p->right-(int)margin-(int)((mw-pad)>>1), oy=(int)p->bottom-(int)margin-(int)((mh-pad)>>1);
        ox&=~1;
        *out=(Stamp){core,halo,mw,mh,f,ss,ox,oy};
        for(y=0;2*y+1<mh;++y) {
            int py=oy+(int)y;
            u32 chroma=!p->half || !(py&1);
            if(py<(int)p->top || py>=(int)p->bottom) continue;
            for(x=0;2*x+3<mw;x+=2) {
                u32 c[2],l[2],k,any=0;
                for(k=0;k<2;++k) {
                    u32 at=2*y*mw+2*(x+k);
                    c[k]=(core[at]+core[at+1]+core[at+mw]+core[at+mw+1]+2)>>2;
                    c[k]=(c[k]*320)>>8; if(c[k]>255) c[k]=255;
                    l[k]=(halo[at]+halo[at+1]+halo[at+mw]+halo[at+mw+1]+2)>>2;
                    any|=c[k]|l[k];
                }
                {
                    int px=ox+(int)x;
                    if(any && px>=(int)p->left && px+1<(int)p->right) light_pair(p,(u32)px,(u32)py,c,l,mono,chroma);
                }
            }
        }
        return 1;
    }
    /* Pixel origin of the mask, on an even column so CbCr pairs line up. */
    {
        int ox=(int)p->right-(int)margin-(int)((mw-pad)*f), oy=(int)p->bottom-(int)margin-(int)((mh-pad)*f);
        ox&=~1;
        *out=(Stamp){core,halo,mw,mh,f,ss,ox,oy};
        step=divide(65536,f);
        for(y=0,sy=0,dy=0;y<mh*f;++y) {
            int py=oy+(int)y;
            u32 ty=dy*step, sy1=sy+1<mh ? sy+1 : sy, chroma=!p->half || !(py&1);
            if(py>=(int)p->top && py<(int)p->bottom)
                for(x=0,sx=0,dx=0;x<mw*f;x+=2) {
                    u32 c[2],l[2],k,any=0;
                    for(k=0;k<2;++k) {
                        u32 ux=x+k, cx=sx, fx=dx+k, cx1;
                        if(fx>=f) { fx-=f; ++cx; }
                        if(cx>=mw) cx=mw-1;
                        cx1=cx+1<mw ? cx+1 : cx;
                        {
                            u32 tx=fx*step;
                            u32 a=core[sy*mw+cx],b=core[sy*mw+cx1],cc=core[sy1*mw+cx],d=core[sy1*mw+cx1];
                            u32 e=halo[sy*mw+cx],fh=halo[sy*mw+cx1],gg=halo[sy1*mw+cx],hh=halo[sy1*mw+cx1];
                            c[k]=(((a*(65536-tx)+b*tx)>>16)*(65536-ty)+((cc*(65536-tx)+d*tx)>>16)*ty)>>16;
                            /* The blur lowers the peak of thin strokes: an LED core saturates. */
                            c[k]=(c[k]*320)>>8; if(c[k]>255) c[k]=255;
                            l[k]=(((e*(65536-tx)+fh*tx)>>16)*(65536-ty)+((gg*(65536-tx)+hh*tx)>>16)*ty)>>16;
                        }
                        any|=c[k]|l[k];
                        (void)ux;
                    }
                    {
                        int px=ox+(int)x;
                        if(any && px>=(int)p->left && px+1<(int)p->right) light_pair(p,(u32)px,(u32)py,c,l,mono,chroma);
                    }
                    dx+=2; while(dx>=f) { dx-=f; ++sx; }
                }
            if(++dy==f) { dy=0; ++sy; }
        }
    }
    return 1;
}
/* Bilinear mask value at cell coordinates (Q16, cell centers at +0.5), or 0 outside. */
static u32 mask_at(const u8 *mask,u32 mw,u32 mh,int cx,int cy) {
    int x=cx-32768, y=cy-32768, i=x>>16, j=y>>16;
    u32 tx=(u32)x&0xffff, ty=(u32)y&0xffff, a, b, c, d;
    if(i<-1 || j<-1 || i>=(int)mw || j>=(int)mh) return 0;
    a=i>=0 && j>=0 ? mask[j*mw+i] : 0;
    b=i+1<(int)mw && j>=0 ? mask[j*mw+i+1] : 0;
    c=i>=0 && j+1<(int)mh ? mask[(j+1)*mw+i] : 0;
    d=i+1<(int)mw && j+1<(int)mh ? mask[(j+1)*mw+i+1] : 0;
    return (((a*(65536-tx)+b*tx)>>16)*(65536-ty)+((c*(65536-tx)+d*tx)>>16)*ty)>>16;
}
/* The stamp of `src` scaled onto the smaller picture `dst`: each pixel of dst
   samples the source mask at four points, so the stamp keeps the same place
   and the same total width as on the main picture, only softer. */
static void stamp_scaled(const Plane *dst,const Plane *src,const Stamp *st,u32 mono) {
    u32 sw=src->right-src->left, sh=src->bottom-src->top, dw=dst->right-dst->left, dh=dst->bottom-dst->top;
    u32 kx=divide(sw<<16,dw), ky=divide(sh<<16,dh), rx=divide(dw<<16,sw), ry=divide(dh<<16,sh);
    u32 rf=divide(65536,st->f), x, y, k;
    /* Q16 cell coordinate of a Q16 source pixel coordinate. */
    #define CELL(v,o) ((int)(((long long)((v)-((long long)(o)<<16))*(long long)(st->ss*rf))>>16))
    int ext=(int)((st->mw*st->f)>>(st->ss-1)), eyt=(int)((st->mh*st->f)>>(st->ss-1));
    int x0=(int)dst->left+(int)((((long long)(st->ox-(int)src->left))*rx)>>16)-2;
    int x1=(int)dst->left+(int)((((long long)(st->ox+ext-(int)src->left))*rx)>>16)+2;
    int y0=(int)dst->top+(int)((((long long)(st->oy-(int)src->top))*ry)>>16)-2;
    int y1=(int)dst->top+(int)((((long long)(st->oy+eyt-(int)src->top))*ry)>>16)+2;
    if(x0<(int)dst->left) x0=(int)dst->left;
    if(y0<(int)dst->top) y0=(int)dst->top;
    if(x1>(int)dst->right) x1=(int)dst->right;
    if(y1>(int)dst->bottom) y1=(int)dst->bottom;
    x0&=~1;
    for(y=(u32)y0;(int)y<y1;++y) {
        u32 chroma=!dst->half || !(y&1);
        for(x=(u32)x0;(int)x+1<x1;x+=2) {
            u32 c[2],l[2],any=0;
            for(k=0;k<2;++k) {
                long long sx=((long long)src->left<<16)+(long long)(x+k-dst->left)*kx+(kx>>1);
                long long sy=((long long)src->top<<16)+(long long)(y-dst->top)*ky+(ky>>1);
                u32 core=0, halo=0, q;
                for(q=0;q<4;++q) {
                    long long px=sx+(q&1 ? (long long)(kx>>2) : -(long long)(kx>>2));
                    long long py=sy+(q&2 ? (long long)(ky>>2) : -(long long)(ky>>2));
                    int cx=CELL(px,st->ox), cy=CELL(py,st->oy);
                    core+=mask_at(st->core,st->mw,st->mh,cx,cy);
                    halo+=mask_at(st->halo,st->mw,st->mh,cx,cy);
                }
                c[k]=((core>>2)*320)>>8; if(c[k]>255) c[k]=255;
                l[k]=halo>>2;
                any|=c[k]|l[k];
            }
            if(any && x+1<dst->right) light_pair(dst,x,y,c,l,mono,chroma);
        }
    }
    #undef CELL
}

/* Off and unknown styles print nothing. */
static u32 stamp_wanted(volatile u32 *h,const PictureSet *set) {
    u32 style=h[GRKIT_HOST_STAMP/4];
    (void)set;
    return style>=1 && style<=DATE_STAMP_STYLE_MAX;
}
/* The largest picture draws the stamp. The smaller ones scale its mask, so
   playback keeps the stamp's place and width when it switches from the
   screennail to the main picture. */
static void stamp_encode(volatile u32 *h,PictureSet *set) {
    u32 date[3], n, i, main=0;
    u8 *dt=(u8 *)date;
    char text[20];
    Stamp st;
    local_time(dt);
    n=date_text(text,dt,h[GRKIT_HOST_STAMP/4]);
    for(i=1;i<set->count;++i)
        if(set->planes[i].width*set->planes[i].height>set->planes[main].width*set->planes[main].height) main=i;
    if(stamp(set->scratch,&set->planes[main],text,n,set->mono,&st)) {
        for(i=0;i<set->count;++i)
            if(i!=main) stamp_scaled(&set->planes[i],&set->planes[main],&st,set->mono);
    }
}

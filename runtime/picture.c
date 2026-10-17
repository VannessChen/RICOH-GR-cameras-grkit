#include "firmware.h"
#include "abi.h"
#include "plugin.h"
/* The core of picture.encode, and the post-curve matrix of tuning. The JPEG
   planes, the cache, and the shot ring are firmware and core details, so they
   stay here. The plugins change the checked planes (runtime/plugins). */
#ifndef WORD
#define WORD(p,o) (*(volatile u32 *)((p)+(o)))
#endif
#ifndef HALF
#define HALF(p,o) (*(volatile u16 *)((p)+(o)))
#endif
#ifndef BYTE
#define BYTE(p,o) (*(volatile u8 *)((p)+(o)))
#endif
#ifndef PICTURE_YCBCR_CALC
/* YCbCrMatrix calc: object = hue rotation x BT.601, 9-bit coefficients, 256 = 1.0. */
#define PICTURE_YCBCR_CALC(object,context) (((void (*)(u32,u32))FW_YCBCR_CALC)(object,context))
#endif
extern volatile u32 *framing_header(void);

static u32 owned(volatile u32 *h,u32 record) {
    u32 i,base=h[GRKIT_HOST_RECORDS/4];
    for(i=0;i<h[GRKIT_HOST_COUNT/4];++i) if(record==base+i*GRKIT_RECORD_SIZE) return 1;
    return 0;
}
#ifdef FW_YCBCR_CALC
/* Features of an owned record, or 0 for a native one. */
static u32 record_features(volatile u32 *h,u32 record) {
    return owned(h,record) ? WORD(record,GRKIT_RECORD_FEATURES) : 0;
}
/* The tuning context byte of a build, from the Make's third argument (settings +
   0x120): 1 for ImageTuningLiveview, 0 for a still build. module_parameters reads
   the same byte at settings + 0x82ac. */
#define PICTURE_LIVE_CONTEXT 0x818c
/* Replaces the YCbCrMatrix Make call to its calc (0x536e4774). The matrix acts
   on gamma-encoded RGB, so CSP x P applies P after the tone curves, and the hue
   rotation still follows it. The calc stores each coefficient as the low 9 bits
   of a word, and the register writer packs those bits without saturation.
   Selective color changes only recorded pictures, so the live view of such an
   effect drops its chroma rows: the viewfinder shows the luma that the removed
   hues keep in the picture. */
void picture_post(u32 object,u32 context,u32 settings) {
    volatile u32 *h=framing_header();
    u32 record=WORD(context,8),features,i,j,k;
    int csp[9],sum;
    PICTURE_YCBCR_CALC(object,context);
    features=record_features(h,record);
    if(features&GRKIT_FEATURE_POST) {
        for(i=0;i<9;++i) csp[i]=(int)(WORD(object,i*4)<<23)>>23;
        for(i=0;i<3;++i) for(j=0;j<3;++j) {
            sum=0;
            for(k=0;k<3;++k) sum+=csp[i*3+k]*(s16)HALF(record,GRKIT_RECORD_POST+(k*3+j)*2);
            sum=(sum+4096)>>13;
            WORD(object,(i*3+j)*4)=(u32)(sum<-256 ? -256 : sum>255 ? 255 : sum);
        }
    }
    if((features&GRKIT_FEATURE_SELECTIVE) && BYTE(settings,PICTURE_LIVE_CONTEXT)==1)
        for(i=3;i<9;++i) WORD(object,i*4)=0;
}
#endif
/* ---- Recorded pictures: the encode host -----------------------------------
   JpegEncodeMacroProcess::Execute (0x535047d4) calls picture_encode before it
   submits the encode. Its image descriptors hold Y at +4, CbCr at +8, then u16
   width, height, and the shared pitch at +0x10. The CbCr plane interleaves
   Cb and Cr for each pair of pixels, in full rows (4:2:2) or half rows (4:2:0). */
#ifndef PICTURE_CACHE_FLUSH
/* Clean and invalidate the data cache for a range. */
#define PICTURE_CACHE_FLUSH(address,length) (((void (*)(u32,u32))FW_CACHE_FLUSH)(address,length))
#endif
#ifndef PICTURE_DATE
/* Local time as DateTime: u16 year, then month, day, hour, minute, second. */
#define PICTURE_DATE(out) (((void (*)(u32,u32))FW_LOCAL_TIME)(WORD(((u32 (*)(void))FW_CLOCK)(),4),(u32)(out)))
#endif
#ifndef PICTURE_NATIVE_ASPECT
#define PICTURE_NATIVE_ASPECT() (((u32 (*)(u32))FW_STILL_ASPECT_GET)(((u32 (*)(void))FW_SETTINGS)()))
#endif
#ifndef PICTURE_NATIVE_ID
/* Visible-area id of a native aspect. GR III offers only 3:2 (0) and 1:1 (3). */
#define PICTURE_NATIVE_ID(native) ((native)==3 ? 3 : 0)
#endif
/* Visible-area id of the capture: 0 3:2, 1 4:3, 2 16:9, 3 1:1, 4 XPan. */
#ifdef FRAMING_XPAN_NATIVE
/* XPan crops native FRAMING_XPAN_NATIVE photos. Other native aspects ignore it. */
#define PICTURE_AREA_ID(native,ratio) \
    ((native)==FRAMING_XPAN_NATIVE && (ratio)==1 ? 4 : PICTURE_NATIVE_ID(native))
#else
/* Framing ratios (Off, XPan, 16:9, 4:3) apply only to native 3:2 captures. */
#define PICTURE_AREA_ID(native,ratio) ((native) ? PICTURE_NATIVE_ID(native) : \
    (ratio)==1 ? 4 : (ratio)==2 ? 2 : (ratio)==3 ? 1 : 0)
#endif
#ifndef PICTURE_SCRATCH
/* Stamp masks after the header and the records, inside the loader's registry array. */
#define PICTURE_SCRATCH(h) ((h)[GRKIT_HOST_RECORDS/4]+(h)[GRKIT_HOST_COUNT/4]*GRKIT_RECORD_SIZE)
#endif
#ifndef PICTURE_BARRIER
#define PICTURE_BARRIER() __asm__ volatile("dmb ish" ::: "memory")
#endif
#ifndef PICTURE_MAP
#define PICTURE_MAP(address) ((u8 *)(address))
#endif

/* Plugin services (runtime/plugin.h). */
/* Shift-subtract division: the freestanding link has no division helper. */
static u32 divide(u32 n,u32 d) {
    u32 q=0,bit=1;
    while(d<n && !(d&0x80000000U)) { d<<=1; bit<<=1; }
    while(bit) { if(n>=d) { n-=d; q|=bit; } d>>=1; bit>>=1; }
    return q;
}
static void flush(const Plane *p) {
    PICTURE_CACHE_FLUSH(p->y,p->pitch*p->height);
    PICTURE_CACHE_FLUSH(p->c,p->pitch*(p->half ? p->height>>1 : p->height));
}
static void local_time(u8 *out) { PICTURE_DATE(out); }

#include "plugins/image-control/selective.c"
#include "plugins/date-stamp/stamp.c"

/* Visible area of a preview picture: black bars depend on the capture aspect. */
static void visible_area(volatile u32 *h,Plane *p) {
    /* Screennail and thumbnail insets for ids 0 (3:2), 1 (4:3), 2 (16:9), 3 (1:1), then XPan. */
    static const u8 screennail[5][2]={{0,0},{40,0},{0,38},{120,0},{0,107}};
    static const u8 thumbnail[5][2]={{0,8},{0,0},{0,16},{20,0},{0,30}};
    /* The capture area: the Framing ratio when Framing is installed. */
    u32 ratio=PLUGIN_INSTALLED(h,PLUGIN_FRAMING) ? h[GRKIT_HOST_FRAME_RATIO/4] : 0;
    u32 native=PICTURE_NATIVE_ASPECT(), id;
    const u8 *inset=0;
    id=PICTURE_AREA_ID(native,ratio);
    if(p->width==720 && p->height==480) inset=screennail[id];
    if(p->width==160 && p->height==120) inset=thumbnail[id];
    p->left=inset ? inset[0] : 0; p->top=inset ? inset[1] : 0;
    p->right=p->width-p->left; p->bottom=p->height-p->top;
}
static u32 plane_of(u32 image,u32 code,Plane *p) {
    if(image<0x40000000U || image>0xbfffff00U || code<0x40000000U || code>0xbfffff00U) return 0;
    p->y=WORD(image,4); p->c=WORD(image,8);
    p->width=HALF(image,0x10); p->height=HALF(image,0x12); p->pitch=HALF(image,0x14);
    p->half=BYTE(code,0x10)==2;
    return p->width>=16 && p->height>=16 && p->width<=8192 && p->height<=8192 &&
           p->pitch>=p->width && !(p->width&1) &&
           p->y>=0x40000000U && p->y<0xc0000000U-p->pitch*p->height &&
           p->c>=0x40000000U && p->c<0xc0000000U-p->pitch*p->height;
}
/* The record that the still build of a shot applied, from the shot ring
   (runtime/abi.h), or 0. Newer entries come first. The key is read again after
   the record, because the build writes the ring from another task. */
static u32 shot_record(volatile u32 *h,u32 shot) {
    u32 ring=PICTURE_SCRATCH(h)+GRKIT_SHOT_RING, n=WORD(ring,0), i, entry, record;
    for(i=1;shot && i<=GRKIT_SHOT_ENTRIES;++i) {
        entry=ring+8+((n-i)&(GRKIT_SHOT_ENTRIES-1))*8;
        if(WORD(entry,0)!=shot) continue;
        record=WORD(entry,4);
        PICTURE_BARRIER();
        if(WORD(entry,0)==shot) return record;
    }
    return 0;
}
/* For picture_capture_tone: the record of a shot, or 0. */
u32 picture_capture_record(u32 shot) { return shot_record(framing_header(),shot); }
/* The encode handlers in order: selective color, then the stamp, which keeps
   its own color. */
static const Encoder encoders[]={
    {PLUGIN_IMAGE_CONTROL,selective_wanted,selective_encode},
    {PLUGIN_DATE_STAMP,stamp_wanted,stamp_encode},
};
#define ENCODER_COUNT (sizeof(encoders)/sizeof(encoders[0]))
/* Once per plane set. The second full-size image (descriptor 3) shares the
   main planes. */
void picture_encode(u32 process,u32 message) {
    volatile u32 *h=framing_header();
    /* Configuration +0x34 holds the shot settings of the encoded pictures. */
    u32 config=WORD(message,8), record=shot_record(h,WORD(config,0x34));
    u32 done[4], wanted[ENCODER_COUNT], any=0, i, j;
    PictureSet set;
    Plane p;
    (void)process;
    set.count=0;
    set.record=0;
    set.mono=0;
    set.scratch=PICTURE_MAP(PICTURE_SCRATCH(h));
    if(owned(h,record)) {
        set.record=record;
        set.mono=WORD(record,GRKIT_RECORD_MONOCHROME)&1;
    }
    for(i=0;i<ENCODER_COUNT;++i) {
        wanted[i]=PLUGIN_INSTALLED(h,encoders[i].plugin) && encoders[i].wanted(h,&set);
        any|=wanted[i];
    }
    if(!any) return;
    for(i=0;i<4;++i) {
        u32 seen=0;
        if(!plane_of(WORD(config,0x3c+i*4),WORD(config,0x4c+i*4),&p)) continue;
        for(j=0;j<set.count;++j) seen|=done[j]==p.y;
        if(seen) continue;
        done[set.count]=p.y;
        visible_area(h,&p);
        set.planes[set.count++]=p;
    }
    if(!set.count) return;
    for(i=0;i<set.count;++i) flush(&set.planes[i]);
    for(i=0;i<ENCODER_COUNT;++i) if(wanted[i]) encoders[i].encode(h,&set);
    for(i=0;i<set.count;++i) flush(&set.planes[i]);
}

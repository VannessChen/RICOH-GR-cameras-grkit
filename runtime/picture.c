#include "firmware.h"
#include "abi.h"
#include "core.h"
#include "profile.h"
/* The core of picture.encode, and the post-curve matrix of tuning. The JPEG
   planes, the cache, and the shot ring are firmware and core details, so they
   stay here. Selective color of the effects and the plane filters of the
   modules change the checked planes. */
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

#include "tone.h"
static u32 owned(volatile u32 *h,u32 record) { return record_index(h,record)<GRKIT_TEXT_RECORDS; }
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
static void post_matrix(volatile u32 *h,u32 object,u32 context,u32 settings) {
    u32 record=WORD(context,8),features,i,j,k;
    int csp[9],post[9],sum;
    PICTURE_YCBCR_CALC(object,context);
    features=record_features(h,record);
    if(features&GRKIT_FEATURE_POST) {
        /* The live view builds this matrix each frame: read each word once. */
        for(i=0;i<9;++i) {
            csp[i]=(int)(WORD(object,i*4)<<23)>>23;
            post[i]=(s16)HALF(record,GRKIT_RECORD_POST+i*2);
        }
        for(i=0;i<3;++i) for(j=0;j<3;++j) {
            sum=0;
            for(k=0;k<3;++k) sum+=csp[i*3+k]*post[k*3+j];
            sum=(sum+4096)>>13;
            WORD(object,(i*3+j)*4)=(u32)(sum<-256 ? -256 : sum>255 ? 255 : sum);
        }
    }
    if((features&GRKIT_FEATURE_SELECTIVE) && BYTE(settings,PICTURE_LIVE_CONTEXT)==1)
        for(i=3;i<9;++i) WORD(object,i*4)=0;
}
void picture_post(u32 object,u32 context,u32 settings) {
    volatile u32 *h=framing_header();
    u32 start=profile_begin(h);
    post_matrix(h,object,context,settings);
    profile_end(h,start,GRKIT_PROFILE_PICTURE_POST);
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
/* The family header maps a native aspect to its visible-area id. */
#error "The family header defines PICTURE_NATIVE_ID"
#endif
/* Visible-area id of the capture: 0 3:2, 1 4:3, 2 16:9, 3 1:1, 4 XPan,
   5 2.35:1, 6 5:4. */
#ifdef FRAMING_XPAN_NATIVE
/* XPan and 2.35:1 apply to native 3:2 and native FRAMING_XPAN_NATIVE photos,
   and 5:4 to native 3:2 photos. Other native aspects ignore them. */
#define PICTURE_AREA_ID(native,ratio) \
    (((native)==0 || (native)==FRAMING_XPAN_NATIVE) && ((ratio)==1 || (ratio)==4) ? \
     ((ratio)==1 ? 4 : 5) : (native)==0 && (ratio)==6 ? 6 : PICTURE_NATIVE_ID(native))
#else
/* Framing ratio values (runtime/firmware.h) apply only to native 3:2 captures. */
#define PICTURE_AREA_ID(native,ratio) ((native) ? PICTURE_NATIVE_ID(native) : \
    (ratio)<7 ? picture_area_ids[ratio] : 0)
/* By crop value: Off, XPan, 16:9, 4:3, 2.35:1, 1:1 (native on each camera,
   so never a crop), 5:4. */
static const u8 picture_area_ids[7]={0,4,2,1,5,0,6};
#endif
#ifndef PICTURE_SCRATCH
/* Stamp masks after the header and the records, inside the loader's registry array. */
#define PICTURE_SCRATCH(h) ((h)[GRKIT_HOST_TAIL/4])
#endif
#ifndef PICTURE_BARRIER
#define PICTURE_BARRIER() __asm__ volatile("dmb ish" ::: "memory")
#endif
#ifndef PICTURE_MAP
#define PICTURE_MAP(address) ((u8 *)(address))
#endif

/* Core services (runtime/core.h). */
/* Shift-subtract division: the freestanding link has no division helper. */
static u32 core_divide(u32 n,u32 d) {
    u32 q=0,bit=1;
    while(d<n && !(d&0x80000000U)) { d<<=1; bit<<=1; }
    while(bit) { if(n>=d) { n-=d; q|=bit; } d>>=1; bit>>=1; }
    return q;
}
/* The core exports these services to modules (runtime/sdk/grkit.h). */
__attribute__((used)) void grkit_export_local_time(u8 *out) { PICTURE_DATE(out); }
__attribute__((used)) u32 grkit_export_divide(u32 n,u32 d) { return core_divide(n,d); }
static void flush(const Plane *p) {
    PICTURE_CACHE_FLUSH(p->y,p->pitch*p->height);
    PICTURE_CACHE_FLUSH(p->c,p->pitch*(p->half ? p->height>>1 : p->height));
}

#include "aspect.h"
#include "settings.h"
/* The module plane filters that INSTALL registered (GRKIT_HOST_MODULE_FILTERS),
   or 0 for a count out of range. Each has two words: its apply function, and
   its setting handle with its stage in the high half. */
#define MODULE_FILTER_MAX (GRKIT_HOST_MODULE_FILTERS_WORDS/2)
static u32 module_filters(volatile u32 *h) {
    u32 n=h[GRKIT_HOST_MODULE_FILTER_COUNT/4];
    return n<=MODULE_FILTER_MAX ? n : 0;
}
#define MODULE_FILTER(h,i) ((h)+GRKIT_HOST_MODULE_FILTERS/4+(i)*2)
/* Apply a module filter. A host test maps the function address. */
#ifndef MODULE_APPLY
#define MODULE_APPLY(address,set,value) ((void (*)(PictureSet *,u32))(address))(set,value)
#endif
#include "selective.h"

/* Visible area of a preview picture: black bars depend on the capture aspect. */
static void visible_area(volatile u32 *h,Plane *p) {
    /* Screennail and thumbnail insets for ids 0 (3:2), 1 (4:3), 2 (16:9), 3 (1:1), then XPan,
       2.35:1, and 5:4 (runtime/framing.S). */
    static const u8 screennail[7][2]={{0,0},{40,0},{0,38},{120,0},{0,107},{0,87},{60,0}};
    static const u8 thumbnail[7][2]={{0,8},{0,0},{0,16},{20,0},{0,30},{0,26},{6,0}};
    /* The capture area: the crop of CORE.ASPECT when a module uses it. */
    u32 ratio=CAPABILITY_USED(h,GRKIT_CORE_ASPECT) ? h[GRKIT_HOST_FRAME_RATIO/4] : 0;
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
        entry=ring+GRKIT_SHOT_ENTRY+((n-i)&(GRKIT_SHOT_ENTRIES-1))*GRKIT_SHOT_STRIDE;
        if(WORD(entry,GRKIT_SHOT_KEY)!=shot) continue;
        record=WORD(entry,GRKIT_SHOT_RECORD);
        PICTURE_BARRIER();
        if(WORD(entry,GRKIT_SHOT_KEY)==shot) return record;
    }
    return 0;
}
/* For picture_capture_tone: the record of a shot, or 0. */
u32 picture_capture_record(u32 shot) { return shot_record(framing_header(),shot); }
#ifndef PICTURE_IFD_SET_SHORT
#define PICTURE_IFD_SET_SHORT(entry,index,value) (((void (*)(u32,u32,u32))FW_IFD_SET_SHORT)(entry,index,value))
#endif
/* For picture_capture_parameters: the maker-note adjustments (tag 0x247) of a
   GRKit shot. The native fill reads the settings of the native Image Control
   type. The record's Fn adjustments replace the nine Standard elements:
   saturation, hue, key, contrast, highlight, shadow, sharpness, then shading
   and clarity. An element holds the shown value, -4 to 4, as camera photos
   show. A stored byte (GRKIT_RECORD_PARAMETERS) is the shown value mod 9, and
   a byte above 8 counts as 0. */
void picture_record_parameters(u32 entry,u32 shot) {
    static const u8 elements[9]={0,1,2,3,4,5,6,9,10};
    u32 record=picture_capture_record(shot), i, stored;
    if(!record) return;
    for(i=0;i<sizeof(elements);++i) {
        stored=BYTE(record,GRKIT_RECORD_PARAMETERS+i);
        PICTURE_IFD_SET_SHORT(entry,elements[i],stored>8 ? 0 : stored<=4 ? stored : (stored-9)&0xffffU);
    }
}
/* Whether the native Image Control type of a shot makes a monochrome picture,
   such as Monotone or a type of GR IV Monochrome. */
static u32 native_monochrome(u32 shot) {
#ifdef NATIVE_MONOCHROME_TYPES
    u32 type;
    if(shot<0x40000000U || shot>0xbfffff00U) return 0;
    type=BYTE(shot,SETTINGS_IC_TYPE);
#ifdef NATIVE_MONOCHROME_TYPES_HIGH
    if(type>=32) return type<64 && (NATIVE_MONOCHROME_TYPES_HIGH>>(type-32)&1);
#endif
    return type<32 && (NATIVE_MONOCHROME_TYPES>>type&1);
#else
    (void)shot;
    return 0;
#endif
}
/* Once per plane set. The second full-size image (descriptor 3) shares the
   main planes. */
static void encode_shot(volatile u32 *h,u32 message) {
    /* Configuration +0x34 holds the shot settings of the encoded pictures. */
    u32 config=WORD(message,8), record=shot_record(h,WORD(config,0x34));
    u32 done[4], selective, values[MODULE_FILTER_MAX], filters=module_filters(h), any, i, j;
    PictureSet set;
    Plane p;
    set.count=0;
    set.record=0;
    set.mono=0;
    set.scratch=PICTURE_MAP(PICTURE_SCRATCH(h));
    if(owned(h,record)) {
        set.record=record;
        set.mono=WORD(record,GRKIT_RECORD_MONOCHROME)&1;
    }
    else set.mono=native_monochrome(WORD(config,0x34));
    /* The color of the effect first: selective color of its record. */
    selective=selective_wanted(&set);
    any=selective;
    /* The value of each filter. A filter does not run at 0, or while a soft
       Off stops its module. */
    for(i=0;i<filters;++i) {
        values[i]=services_suspended(h,MODULE_FILTER(h,i)[0]) ? 0 : setting_value(h,MODULE_FILTER(h,i)[1]&0xffffU);
        any|=values[i];
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
    if(selective) selective_encode(&set);
    /* The module filters follow the color of the effect, in stage order. */
    for(i=0;i<filters;++i) if(values[i]) {
        u32 start=profile_begin(h);
        MODULE_APPLY(MODULE_FILTER(h,i)[0],&set,values[i]);
        profile_end_module(h,start,MODULE_FILTER(h,i)[0],GRKIT_PROFILE_FILTER);
    }
    for(i=0;i<set.count;++i) flush(&set.planes[i]);
}
/* The encode of one shot, the module filters included. */
void picture_encode(u32 process,u32 message) {
    volatile u32 *h=framing_header();
    u32 start=profile_begin(h);
    (void)process;
    encode_shot(h,message);
    profile_end(h,start,GRKIT_PROFILE_PICTURE_ENCODE);
}

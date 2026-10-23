/* The core of viewfinder.overlay and the C part of capture.aspect. The live
   view canvas, its bitmap, and the native aspect are firmware details, so they
   stay here. The plugins paint layers on a checked surface. */
#include "firmware.h"
#include "abi.h"
#ifndef FRAMING_PIXELS
#define FRAMING_PIXELS(address) ((volatile u32 *)(address))
#endif
#define OVERLAY_PIXELS(address) FRAMING_PIXELS(address)
#include "plugin.h"

#define FRAMING_SETTINGS() (((u32 (*)(void))FW_SETTINGS)())
#ifndef FRAMING_NATIVE_ASPECT
#define FRAMING_NATIVE_ASPECT() (((u32 (*)(u32))FW_STILL_ASPECT_GET)(FRAMING_SETTINGS()))
#endif
/* The family header reads the requested capture aspect and sends the request
   again (FRAMING_CAPTURE_ASPECT, FRAMING_REQUEST_ASPECT). */
#if defined(FRAMING_CAPTURE_ASPECT) != defined(FRAMING_REQUEST_ASPECT)
#error "The family header defines FRAMING_CAPTURE_ASPECT and FRAMING_REQUEST_ASPECT together"
#endif
#ifndef FRAMING_INVALIDATE
/* View::Invalidate: request a redraw unless one is pending (+0xa0). */
#define FRAMING_INVALIDATE(view) (((void (*)(u32))FW_VIEW_INVALIDATE)(view))
#endif
extern volatile u32 *framing_header(void);

#ifndef FRAMING_DISPLAY_MODE
/* The native display mode that DISP cycles (runtime/targets), or 0 (full)
   before the settings exist and on a target without it. */
static u32 framing_display_mode(void) {
#if defined(UI_SETTINGS_SLOT) && defined(DISPLAY_MODE_SETTING)
    u32 settings=WORD(UI_SETTINGS_SLOT,0);
    return settings ? BYTE(settings,DISPLAY_MODE_SETTING) : 0;
#elif defined(DISPLAY_MODE_BYTE)
    return BYTE(DISPLAY_MODE_BYTE,0);
#else
    return 0;
#endif
}
#define FRAMING_DISPLAY_MODE() framing_display_mode()
#endif

/* Plugin services (runtime/plugin.h). */
static u32 framing_ratios(void) { return FRAMING_RATIO_COUNT; }
static u32 native_display(void) { return FRAMING_DISPLAY_MODE(); }
#ifndef LABEL_MAP
#define LABEL_MAP(address) ((const volatile u8 *)(address))
#endif
/* A u16 or u32 field of the label table (runtime/abi.h). */
static u32 label_field(const volatile u8 *table,u32 at,u32 bytes) {
    u32 value=0,i;
    for(i=0;i<bytes;++i) value|=(u32)table[at+i]<<(8*i);
    return value;
}
static u32 label_width(u32 label) {
    return label<GRKIT_LABEL_COUNT ? label_field(LABEL_MAP(GRKIT_LABELS(framing_header())),label*8,2) : 0;
}
/* White text with a black outline and no background, as the native live-view
   text has it: each drawn pixel is opaque, and text edges blend toward the
   outline in RGB. A transparent pixel keeps the canvas, so a label does not
   cut the Framing mask. Canvas::Draw clears the bitmap before each draw
   (framing_prepare), which removes the previous label. */
static void overlay_label(const Surface *s,u32 label,u32 left,u32 top) {
    const volatile u8 *table=LABEL_MAP(GRKIT_LABELS(framing_header())),*pixels;
    u32 width,height,x,y,value;
    volatile u32 *row;
    if(label>=GRKIT_LABEL_COUNT) return;
    width=label_field(table,label*8,2);
    height=label_field(table,label*8+2,2);
    if(!width || left+width>720 || top+height>480) return;
    pixels=table+label_field(table,label*8+4,4);
    for(y=0;y<height;++y) {
        row=OVERLAY_PIXELS(s->data+(top+y)*s->stride*4)+left;
        for(x=0;x<width;++x) {
            value=pixels[y*width+x];
            if(value) row[x]=0xff000000U|value*0x010101U;
        }
    }
}

/* The plugin handler files and PLUGIN_LAYERS. */
#include "generated/layers.h"

/* The layers in painting order (Layer.order in the registry). */
static const Layer layers[]={PLUGIN_LAYERS};
#define LAYER_COUNT (sizeof(layers)/sizeof(layers[0]))
/* The committed value of a layer, or 0 for Off and for a plugin that
   PLUGINS.TXT does not install. */
static u32 layer_value(volatile u32 *h,const Layer *layer) {
    return PLUGIN_INSTALLED(h,layer->plugin) ? layer->value(h) : 0;
}
/* ---- Load cue -----------------------------------------------------------
   INSTALL runs a few seconds after power-on, and the GRKit entry exists only
   after it. The first still live view after INSTALL therefore shows the
   GRKit label for CUE_SECONDS - 1 to CUE_SECONDS seconds. GRKIT_HOST_LOAD_CUE holds the
   state (runtime/abi.h). */
#define CUE_PENDING 1U
#define CUE_SECONDS 4U
#ifndef FRAMING_SECOND
/* The local second of the day. DateTime: u16 year, then u8 month, day, hour,
   minute, and second. */
static u32 framing_second(void) {
    u32 date[3];
    const u8 *dt=(const u8 *)date;
    ((void (*)(u32,u32))FW_LOCAL_TIME)(WORD(((u32 (*)(void))FW_CLOCK)(),4),(u32)date);
    return dt[4]*3600U+dt[5]*60U+dt[6];
}
#define FRAMING_SECOND() framing_second()
#endif
static u32 cue_shown(volatile u32 *h) { return h[GRKIT_HOST_LOAD_CUE/4]>CUE_PENDING; }
/* Start the cue in a still live view, and end it after CUE_SECONDS. Returns 1
   when the cue starts or ends, so the canvas needs a redraw. */
static u32 cue_update(volatile u32 *h,u32 controller) {
    u32 state=h[GRKIT_HOST_LOAD_CUE/4],now,start;
    if(!state) return 0;
    if(state==CUE_PENDING) {
        if(!h[GRKIT_HOST_FRAME_CANVAS/4] || BYTE(controller,LIVEVIEW_MOVIE)) return 0;
        h[GRKIT_HOST_LOAD_CUE/4]=FRAMING_SECOND()+2;
        return 1;
    }
    now=FRAMING_SECOND();
    start=state-2;
    /* The cue started before midnight. */
    if(now<start) now+=24U*3600U;
    if(now-start<CUE_SECONDS) return 0;
    h[GRKIT_HOST_LOAD_CUE/4]=0;
    return 1;
}
static u32 layers_active(volatile u32 *h) {
    u32 i;
    for(i=0;i<LAYER_COUNT;++i) if(layer_value(h,&layers[i])) return 1;
    return 0;
}
/* Whether the live view has something to draw: a layer or the cue. */
static u32 overlay_active(volatile u32 *h) { return cue_shown(h) || layers_active(h); }
/* The capture area: the Framing ratio when Framing is installed. */
static u32 capture_area(volatile u32 *h) {
    return PLUGIN_INSTALLED(h,PLUGIN_FRAMING) ? framing_value(h) : 0;
}

#ifdef FRAMING_XPAN_NATIVE
/* The live view shows the Framing ratio in native 3:2. XPan also crops the
   native aspect FRAMING_XPAN_NATIVE, whose live view shows only XPan. In other
   native aspects nothing shows: clear `data`. */
#define SHOWN_AREA(data,area) do { \
    u32 native=FRAMING_NATIVE_ASPECT(); \
    if(native!=0 && (native!=FRAMING_XPAN_NATIVE || (area)!=1)) (data)=0; \
} while(0)
#else
/* canvas_pixels() accepts only the native 3:2 aspect, where each ratio shows. */
#define SHOWN_AREA(data,area) ((void)0)
#endif

#ifdef FRAMING_CAPTURE_ASPECT
/* The still aspect property handler calls this for each requested value. The
   native menu requests 0 for 3:2, so 3:2 records the Framing ratio. Other
   native ratios pass unchanged. Only an installed Framing hooks the handler. */
u32 framing_capture_request(u32 request) {
    return request ? request : framing_capture_id(framing_header());
}
/* The live view keeps 3:2. Resend the native 3:2 request when the capture
   aspect differs from the Framing ratio, for example after a menu change. */
static void capture_sync(u32 controller) {
    volatile u32 *h=framing_header();
    if(PLUGIN_INSTALLED(h,PLUGIN_FRAMING) && BYTE(controller,LIVEVIEW_MOVIE)==0 &&
       FRAMING_NATIVE_ASPECT()==0 && FRAMING_CAPTURE_ASPECT()!=framing_capture_id(h))
        FRAMING_REQUEST_ASPECT();
}
#ifdef FW_CAPTURE_ASPECT_GET
/* The live view follows the capture aspect where the native firmware has a
   live-view mode for each aspect (GR IV). A capture id 2 with the native 3:2
   UI aspect comes from the Framing request above, so the live view keeps
   mode 3:2. The live-view mode function and the mode check call this instead
   of the capture aspect getter. */
u32 framing_live_aspect(u32 property) {
    u32 id=((u32 (*)(u32))FW_CAPTURE_ASPECT_GET)(property);
    return id==2 && FRAMING_NATIVE_ASPECT()==0 ? 0 : id;
}
#endif
#else
/* Without the capture hooks a Framing ratio cannot be selected. */
#define capture_sync(controller) ((void)(controller))
#endif
/* Only the native still-photo LCD canvas owns these transient references. */
void framing_bind(u32 controller) {
    volatile u32 *h=framing_header();
    h[GRKIT_HOST_FRAME_OWNER/4]=controller;
    h[GRKIT_HOST_FRAME_CANVAS/4]=WORD(controller,LIVEVIEW_CANVAS);
    /* Shooting: id 2 geometry follows the Framing ratio again. */
    ((volatile u8 *)h)[GRKIT_HOST_FRAME_FILE]=0;
    capture_sync(controller);
    /* A selection that has not drawn on this canvas yet, such as one restored
       at power-on, needs a redraw: nothing else invalidates a still live view.
       A cue that starts or ends needs one too. */
    if(h[GRKIT_HOST_FRAME_CANVAS/4] &&
       (cue_update(h,controller) || (!h[GRKIT_HOST_FRAME_BITMAP/4] && overlay_active(h))))
        FRAMING_INVALIDATE(h[GRKIT_HOST_FRAME_CANVAS/4]);
}
void framing_leave(u32 controller) {
    volatile u32 *h=framing_header();
    if(h[GRKIT_HOST_FRAME_OWNER/4]==controller) {
        h[GRKIT_HOST_FRAME_OWNER/4]=0;
        h[GRKIT_HOST_FRAME_CANVAS/4]=0;
    }
}
/* Pixel data of a full 720x480 32-bit canvas at the origin, or 0. */
static u32 canvas_bitmap(u32 canvas,u32 bitmap) {
    u32 stride,data;
    if(!BYTE(canvas,0x9e) || WORD(canvas,0x20) || WORD(canvas,0x24) ||
       WORD(canvas,0x28)!=720 || WORD(canvas,0x2c)!=480 ||
       BYTE(bitmap,2)!=5 || HALF(bitmap,0x1c)!=720 || HALF(bitmap,0x1e)!=480) return 0;
    stride=HALF(bitmap,0x18);
    data=WORD(bitmap,8);
    /* The OSD buffer moves between boots (0x5f83df80 and 0x619e1200 seen). */
    if(stride<720 || stride>1024 || HALF(bitmap,0x1a)!=480 ||
       data<0x40000000 || data>0x80000000-stride*480*4 || (data&3) ||
       WORD(bitmap,4)<stride*480*4) return 0;
    return data;
}
/* Pixel data of a supported canvas bitmap, or 0. The layers are defined only
   for a full canvas and the native 3:2 aspect. */
static u32 canvas_pixels(u32 canvas,u32 bitmap) {
#ifndef FRAMING_XPAN_NATIVE
    /* Native StillAspectRatio getter: 0 = 3:2, 3 = 1:1. Native square crop is untouched. */
    if(FRAMING_NATIVE_ASPECT()!=0) return 0;
#endif
    return canvas_bitmap(canvas,bitmap);
}
/* Photo area {left,top,right,bottom} on the 720x480 canvas for each capture area. */
static const u16 photo_boxes[4][4]={{0,0,720,480},{0,107,720,373},{0,37,720,442},{40,0,680,480}};
void framing_prepare(u32 canvas) {
    volatile u32 *h=framing_header();
    /* Canvas::Draw clears the full native bitmap before drawing its children.
       Use this path even for Off to remove the previous drawing on the next draw. */
    if((h[GRKIT_HOST_FRAME_CANVAS/4]==canvas &&
        (overlay_active(h) || h[GRKIT_HOST_FRAME_BITMAP/4])) ||
       (canvas && h[GRKIT_HOST_FRAME_PREVIEW_CANVAS/4]==canvas))
        BYTE(canvas,0xa0)=1;
}
/* The ADJ Framing and Guides pages preview their highlighted row on the ADJ
   canvas, below the menu controls (runtime/modules.h: frame_preview). After
   the page ends, one more full draw erases the drawing and releases the
   canvas. */
static void preview_paint(volatile u32 *h,u32 canvas,u32 bitmap) {
    u32 preview=h[GRKIT_HOST_FRAME_PREVIEW/4],style=h[GRKIT_HOST_GUIDE_PREVIEW/4],data,area,i;
    Surface surface;
    if(!preview) { h[GRKIT_HOST_FRAME_PREVIEW_CANVAS/4]=0; return; }
    if(preview>FRAMING_RATIO_MAX+1U || style>GUIDES_STYLE_MAX || (preview<2 && !style)) return;
    data=canvas_pixels(canvas,bitmap);
    area=preview-1;
    SHOWN_AREA(data,area);
    if(!data) return;
    surface.data=data; surface.stride=HALF(bitmap,0x18); surface.area=area; surface.box=photo_boxes[area];
    for(i=0;i<LAYER_COUNT;++i) {
        const Layer *layer=&layers[i];
        u32 value=layer->plugin==PLUGIN_FRAMING ? area : style;
        if(!layer->live_only && value) layer->paint(&surface,value);
    }
}
void framing_paint(u32 canvas,u32 bitmap) {
    volatile u32 *h=framing_header();
    u32 owner=h[GRKIT_HOST_FRAME_OWNER/4],full,data,cue,area,i;
    Surface surface;
    if(canvas && canvas==h[GRKIT_HOST_FRAME_PREVIEW_CANVAS/4]) { preview_paint(h,canvas,bitmap); return; }
    if(canvas!=h[GRKIT_HOST_FRAME_CANVAS/4] || !owner) return;
    if(!overlay_active(h)) { h[GRKIT_HOST_FRAME_BITMAP/4]=0; return; }
    h[GRKIT_HOST_FRAME_BITMAP/4]=bitmap;
    /* The LCD still canvas is registered in native slot zero. Unsupported
       canvas sizes and bitmap formats leave the native image untouched. */
    full=WORD(owner,4)==canvas && BYTE(owner,LIVEVIEW_MOVIE)==0 ? canvas_bitmap(canvas,bitmap) : 0;
    data=full && layers_active(h) ? canvas_pixels(canvas,bitmap) : 0;
    /* The cue does not depend on the photo area, so it shows in each native
       aspect. It hides only while DISP turns the live view off. */
    cue=full && cue_shown(h) && native_display()!=DISPLAY_OFF ? full : 0;
    area=capture_area(h);
    SHOWN_AREA(data,area);
    if(!data && !cue) { ++h[GRKIT_HOST_FRAME_SKIPPED/4]; return; }
    if(data) {
        surface.data=data; surface.stride=HALF(bitmap,0x18); surface.area=area; surface.box=photo_boxes[area];
        for(i=0;i<LAYER_COUNT;++i) {
            u32 value=layer_value(h,&layers[i]);
            if(value) layers[i].paint(&surface,value);
        }
    }
    if(cue) {
        /* Centered, on the baseline of the native focus distance. */
        surface.data=cue; surface.stride=HALF(bitmap,0x18); surface.area=area; surface.box=photo_boxes[area];
        overlay_label(&surface,LABEL_GRKIT,360-label_width(LABEL_GRKIT)/2,LABEL_TOP);
    }
    ++h[GRKIT_HOST_FRAME_DRAWS/4];
}

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
#ifdef FW_CAPTURE_PROPERTIES
#ifndef FRAMING_CAPTURE_ASPECT
/* Requested capture StillAspectRatio (CommonProperty +0x7b6). The property
   setter writes the next copy at [property+4]; readers see the current copy
   at [property] only after a later commit. Comparing the next copy sends
   each change once. */
#define FRAMING_CAPTURE_ASPECT() BYTE(WORD(WORD(WORD(((u32 (*)(void))FW_CAPTURE_PROPERTIES)(),0),0x518),4),0x7b6)
#endif
#ifndef FRAMING_REQUEST_ASPECT
/* Native StillAspectRatio setter with the unchanged 3:2 UI value. It sends
   capture property 0xd4 again and posts no UI change event. */
#define FRAMING_REQUEST_ASPECT() (((u32 (*)(u32,u32))FW_STILL_ASPECT_SET)(FRAMING_SETTINGS(),0))
#endif
#endif
#ifndef FRAMING_INVALIDATE
/* View::Invalidate: request a redraw unless one is pending (+0xa0). */
#define FRAMING_INVALIDATE(view) (((void (*)(u32))FW_VIEW_INVALIDATE)(view))
#endif
extern volatile u32 *framing_header(void);

/* Plugin services (runtime/plugin.h). */
static u32 framing_ratios(void) { return FRAMING_RATIO_COUNT; }

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
static u32 overlay_active(volatile u32 *h) {
    u32 i;
    for(i=0;i<LAYER_COUNT;++i) if(layer_value(h,&layers[i])) return 1;
    return 0;
}
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
       at power-on, needs a redraw: nothing else invalidates a still live view. */
    if(h[GRKIT_HOST_FRAME_CANVAS/4] && !h[GRKIT_HOST_FRAME_BITMAP/4] && overlay_active(h))
        FRAMING_INVALIDATE(h[GRKIT_HOST_FRAME_CANVAS/4]);
}
void framing_leave(u32 controller) {
    volatile u32 *h=framing_header();
    if(h[GRKIT_HOST_FRAME_OWNER/4]==controller) {
        h[GRKIT_HOST_FRAME_OWNER/4]=0;
        h[GRKIT_HOST_FRAME_CANVAS/4]=0;
    }
}
/* Pixel data of a supported canvas bitmap, or 0. The layers are defined only
   for a full 720x480 32-bit canvas at the origin and the native 3:2 aspect. */
static u32 canvas_pixels(u32 canvas,u32 bitmap) {
    u32 stride,data;
    if(!BYTE(canvas,0x9e) || WORD(canvas,0x20) || WORD(canvas,0x24) ||
       WORD(canvas,0x28)!=720 || WORD(canvas,0x2c)!=480 ||
       BYTE(bitmap,2)!=5 || HALF(bitmap,0x1c)!=720 || HALF(bitmap,0x1e)!=480) return 0;
#ifndef FRAMING_XPAN_NATIVE
    /* Native StillAspectRatio getter: 0 = 3:2, 3 = 1:1. Native square crop is untouched. */
    if(FRAMING_NATIVE_ASPECT()!=0) return 0;
#endif
    stride=HALF(bitmap,0x18);
    data=WORD(bitmap,8);
    /* The OSD buffer moves between boots (0x5f83df80 and 0x619e1200 seen). */
    if(stride<720 || stride>1024 || HALF(bitmap,0x1a)!=480 ||
       data<0x40000000 || data>0x80000000-stride*480*4 || (data&3) ||
       WORD(bitmap,4)<stride*480*4) return 0;
    return data;
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
    u32 owner=h[GRKIT_HOST_FRAME_OWNER/4],data,area,i;
    Surface surface;
    if(canvas && canvas==h[GRKIT_HOST_FRAME_PREVIEW_CANVAS/4]) { preview_paint(h,canvas,bitmap); return; }
    if(canvas!=h[GRKIT_HOST_FRAME_CANVAS/4] || !owner) return;
    if(!overlay_active(h)) { h[GRKIT_HOST_FRAME_BITMAP/4]=0; return; }
    h[GRKIT_HOST_FRAME_BITMAP/4]=bitmap;
    /* The LCD still canvas is registered in native slot zero. Unsupported
       canvas sizes and bitmap formats leave the native image untouched. */
    data=WORD(owner,4)==canvas && BYTE(owner,LIVEVIEW_MOVIE)==0 ? canvas_pixels(canvas,bitmap) : 0;
    area=capture_area(h);
    SHOWN_AREA(data,area);
    if(!data) { ++h[GRKIT_HOST_FRAME_SKIPPED/4]; return; }
    surface.data=data; surface.stride=HALF(bitmap,0x18); surface.area=area; surface.box=photo_boxes[area];
    for(i=0;i<LAYER_COUNT;++i) {
        u32 value=layer_value(h,&layers[i]);
        if(value) layers[i].paint(&surface,value);
    }
    ++h[GRKIT_HOST_FRAME_DRAWS/4];
}

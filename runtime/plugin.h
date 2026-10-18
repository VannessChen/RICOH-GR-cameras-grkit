#ifndef GRKIT_PLUGIN_H
#define GRKIT_PLUGIN_H
/* The interface between the GRKit core (runtime/) and its plugins
   (runtime/plugins/<id>/). A plugin includes only this file. It holds no
   firmware address: the core and its target headers keep each firmware detail.

   Each plugin has a menu page (page.h, part of the menu component) and can
   have handlers for the extension points that it uses (part of the Framing
   component). The core calls a handler only when PLUGINS.TXT installs the
   plugin. A plugin keeps its state in its own words of the private header
   (runtime/abi.h).

   The registry in crates/grkit-core/src/plugins.rs lists the plugins, their
   extension points, state keys, tiles, and handlers. runtime/generated/ holds
   that list for C. */
#include "abi.h"
#include "generated/plugins.h"

typedef unsigned int u32;
typedef unsigned short u16;
typedef unsigned char u8;
typedef short s16;
/* A service or handler that a component or target may not use. */
#define GRKIT_MAYBE_UNUSED __attribute__((unused))

/* Memory access. Host tests map these to their own buffers. */
#ifndef WORD
#define WORD(p,o) (*(volatile u32 *)((p)+(o)))
#endif
#ifndef HALF
#define HALF(p,o) (*(volatile u16 *)((p)+(o)))
#endif
#ifndef BYTE
#define BYTE(p,o) (*(volatile u8 *)((p)+(o)))
#endif

/* True when PLUGINS.TXT installs the plugin (PLUGIN_<ID>, generated/plugins.h). */
#define PLUGIN_INSTALLED(h,plugin) ((h)[GRKIT_HOST_PLUGINS/4]>>(plugin)&1U)

/* ---- menu.page ------------------------------------------------------------ */

/* Private menu levels (GRKIT_HOST_MENU_LEVEL). */
#define MENU_ROOT 1U
#define MENU_MODULE 2U
/* Icon results of a page: Off, the GRKit tile of the ADJ toolbar, the icon of
   effect n, or tile n of the plugin's own artwork (tile 0 is its root tile). */
#define GRKIT_ICON_OFF 0x7fffU
#define GRKIT_ICON_GRKIT 0x7ffeU
#define GRKIT_ICON_EFFECT(n) (0x8000U+(n))
#define GRKIT_ICON_TILE(n) (0x10000U+(n))

typedef struct {
    const u16 *(*title)(volatile u32 *);
    /* The root row text. */
    const u16 *(*summary)(volatile u32 *);
    u32 (*rows)(volatile u32 *);
    const u16 *(*label)(volatile u32 *,u32);
    u32 (*icon)(volatile u32 *,u32);
    u32 (*checked)(volatile u32 *,u32);
    /* OK on a row. */
    void (*apply)(volatile u32 *,u32);
    /* The committed row, where the page opens. */
    u32 (*committed)(volatile u32 *);
    /* Optional: the highlighted row changed. */
    void (*highlight)(volatile u32 *);
    /* Optional: Fn on the highlighted row applies it and opens the native
       parameter page. */
    u32 (*parameters)(volatile u32 *);
    /* Artwork tiles: <ID>_TILES (generated/plugins.h), from the registry. */
    u32 tiles;
} Module;

/* Core services for pages. */
/* One string of the menu text table in the menu language (runtime/abi.h). */
GRKIT_MAYBE_UNUSED static const u16 *text(volatile u32 *h,u32 id);
/* The Image Control summary of effect n in the menu language. */
GRKIT_MAYBE_UNUSED static const u16 *effect_summary(volatile u32 *h,u32 n);
/* The Framing rows of the target, Off included. */
GRKIT_MAYBE_UNUSED static u32 framing_ratios(void);
/* Resend the native still aspect, as a native menu change does, so the
   capture.aspect hooks apply the Framing ratio. */
GRKIT_MAYBE_UNUSED static void request_native_aspect(void);

/* ---- viewfinder.overlay --------------------------------------------------- */

/* A 32-bit ARGB drawing surface of 720 x 480 pixels. `area` is the capture
   area that the surface shows: 0 for the full 3:2 area, else a Framing row.
   `box` is its photo area {left, top, right, bottom}. */
typedef struct {
    u32 data;
    u32 stride;
    u32 area;
    const u16 *box;
} Surface;
/* One drawing layer. `value` gives the plugin's committed value, 0 for Off.
   `paint` gets that value in the live view, or the highlighted value on the
   ADJ canvas while the Framing or Guides page is open. The core does not paint
   a layer at Off, and it paints the layers in table order. */
typedef struct {
    u32 plugin;
    u32 (*value)(volatile u32 *);
    void (*paint)(const Surface *,u32 value);
    /* Paint in the live view only, not on the ADJ canvas. */
    u32 live_only;
} Layer;
#ifndef OVERLAY_PIXELS
#define OVERLAY_PIXELS(address) ((volatile u32 *)(address))
#endif

/* ---- picture.encode ------------------------------------------------------- */

/* One YCbCr plane of a recorded picture, with its visible area. Luma rows are
   `pitch` bytes apart. Interleaved CbCr rows follow, one for every second luma
   row when `half` is set. */
typedef struct { u32 y,c,width,height,pitch,half,left,top,right,bottom; } Plane;
/* The address of the chroma row of luma row y. */
static inline u32 chroma_row(const Plane *p,u32 y) { return p->c+(p->half ? y>>1 : y)*p->pitch; }
/* The deduplicated planes of one encode. `record` is the processing record of
   the shot, or 0 for native processing, and `mono` is set for a monochrome
   record. `scratch` holds GRKIT_PICTURE_MASKS bytes for this call. */
typedef struct {
    Plane planes[4];
    u32 count;
    u32 record;
    u32 mono;
    u8 *scratch;
} PictureSet;
/* One encode handler. The core calls `encode` only when `wanted` is nonzero,
   in table order, between two cache flushes of the planes. */
typedef struct {
    u32 plugin;
    u32 (*wanted)(volatile u32 *,const PictureSet *);
    void (*encode)(volatile u32 *,PictureSet *);
} Encoder;

/* Core services for encoders. */
/* Local time as DateTime: u16 year, then month, day, hour, minute, second. */
GRKIT_MAYBE_UNUSED static void local_time(u8 *out);
/* Unsigned division: the freestanding link has no division helper. */
GRKIT_MAYBE_UNUSED static u32 divide(u32 n,u32 d);
/* The address of the scratch for direct access. Host tests remap it. */
#ifndef PICTURE_MAP
#define PICTURE_MAP(address) ((u8 *)(address))
#endif
#endif

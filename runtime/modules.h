#ifndef GRKIT_MODULES_H
#define GRKIT_MODULES_H
/* The menu modules: the page of each plugin (generated/pages.h), in plugin
   order, so module n is plugin n (PLUGIN_<ID>). The core About row
   (runtime/about.h) follows the plugin rows. */
#include "generated/pages.h"
#include "about.h"

#define ABOUT_MODULE PLUGIN_COUNT
#define ARRAY_COUNT(a) (sizeof(a)/sizeof((a)[0]))

static const Module modules[] = {PLUGIN_PAGES, ABOUT_PAGE};

/* Plugin artwork is contiguous and ends below the Version tile: the tiles of
   each plugin in plugin order, from PLUGIN_ICON. The installer copies
   grkit_core::plugins::tiles in the same order. */
#define PLUGIN_ICON (GRKIT_ICON_VERSION-PLUGIN_TILES)
/* The tiles of the pages. A host test compares it with PLUGIN_TILES. */
GRKIT_MAYBE_UNUSED static u32 module_icon_count(void) {
    u32 i, n=0;
    for(i=0;i<ARRAY_COUNT(modules);++i) n+=modules[i].tiles;
    return n;
}
/* The global icon ID of a page's icon result. */
static u32 module_icon(u32 module,u32 icon) {
    u32 i, first=PLUGIN_ICON;
    if(icon<GRKIT_ICON_TILE(0)) return icon;
    for(i=0;i<module;++i) first+=modules[i].tiles;
    return first+icon-GRKIT_ICON_TILE(0);
}

/* ADJ covers the live-view drawing, so ADJ draws the mask and guides on its
   own canvas. The Framing page shows its highlighted ratio and the Guides page
   its highlighted style, each with the other module's committed value. Off
   ends the drawing, and the canvas stays until a full draw has erased it. */
static void frame_preview(volatile u32 *h,u32 canvas,u32 movie) {
    u32 module=h[GRKIT_HOST_MENU_LEVEL/4]==MENU_MODULE ? h[GRKIT_HOST_MODULE/4] : 0;
    u32 page=module==PLUGIN_FRAMING || module==PLUGIN_GUIDES;
    u32 ratio=module==PLUGIN_FRAMING ? h[GRKIT_HOST_MENU_CURSOR/4] : frame_ratio(h);
    u32 style=module==PLUGIN_GUIDES ? h[GRKIT_HOST_MENU_CURSOR/4] : guide_style(h);
    if(movie) { h[GRKIT_HOST_FRAME_PREVIEW/4]=0; return; }
    h[GRKIT_HOST_GUIDE_PREVIEW/4]=style;
    /* A highlighted Off keeps the canvas for the next row. */
    h[GRKIT_HOST_FRAME_PREVIEW/4]=ratio || style || page ? ratio+1 : 0;
    h[GRKIT_HOST_FRAME_PREVIEW_CANVAS/4]=canvas;
}

/* Visible modules as a bit mask in plugin order: the installed plugins
   (PLUGINS.TXT), then About, which always shows. A plugin that needs effects
   shows only with effects. With no plugin installed, the installer installs no
   menu hook, so only an inconsistent header shows About alone. */
static u32 visible_modules(volatile u32 *h) {
    u32 mask=h[GRKIT_HOST_PLUGINS/4]&((1U<<PLUGIN_COUNT)-1);
    if(!h[GRKIT_HOST_COUNT/4]) mask&=~PLUGIN_NEEDS_EFFECTS;
    return mask | 1U<<ABOUT_MODULE;
}
/* The module of a root row, or of the last row for a row past the end. */
static u32 root_module(volatile u32 *h,u32 row) {
    u32 mask=visible_modules(h), module=0, last=0;
    for(;module<ARRAY_COUNT(modules);++module) {
        if(!(mask>>module&1)) continue;
        if(!row--) return module;
        last=module;
    }
    return last;
}
static u32 root_row(volatile u32 *h,u32 module) {
    u32 mask=visible_modules(h), row=0, i;
    for(i=0;i<module && i<ARRAY_COUNT(modules);++i) row+=mask>>i&1;
    return row;
}
static u32 root_count(volatile u32 *h) { return root_row(h,ARRAY_COUNT(modules)); }
/* The module of the current row: the row's module at the root, else the open page. */
static u32 current_index(volatile u32 *h) {
    return h[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT ? root_module(h,h[GRKIT_HOST_MENU_CURSOR/4]) : h[GRKIT_HOST_MODULE/4];
}
static const Module *current_module(volatile u32 *h) { return &modules[current_index(h)]; }
static u32 count(volatile u32 *h) {
    return h[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT ? root_count(h) : current_module(h)->rows(h);
}
/* The page tells a plugin that its highlighted row changed, so Image Control
   can preview the row in the live view. */
static void highlight(volatile u32 *h) {
    const Module *module;
    if(h[GRKIT_HOST_MENU_LEVEL/4]!=MENU_MODULE || h[GRKIT_HOST_MODULE/4]>=ARRAY_COUNT(modules)) return;
    module=&modules[h[GRKIT_HOST_MODULE/4]];
    if(module->highlight) module->highlight(h);
}
/* Fn applies to the highlighted row of a page that has native parameters,
   and to the root row of such a page for its committed row. */
static u32 parameters_row(volatile u32 *h) {
    const Module *module;
    u32 row=h[GRKIT_HOST_MENU_CURSOR/4];
    if(h[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT) {
        module=&modules[root_module(h,row)];
        row=module->committed(h);
    }
    else if(h[GRKIT_HOST_MENU_LEVEL/4]!=MENU_MODULE || h[GRKIT_HOST_MODULE/4]>=ARRAY_COUNT(modules)) return 0;
    else module=&modules[h[GRKIT_HOST_MODULE/4]];
    return module->parameters && module->parameters(h,row);
}
/* Each module opens at its committed row. Image Control previews from there. */
static void menu_open(volatile u32 *h,u32 module) {
    h[GRKIT_HOST_MODULE/4]=module;
    h[GRKIT_HOST_MENU_LEVEL/4]=MENU_MODULE;
    h[GRKIT_HOST_MENU_CURSOR/4]=modules[module].committed(h);
}
/* ADJ opens at the root. About always shows there, so the root has a row even
   when no plugin is visible. */
static void menu_start(volatile u32 *h) {
    h[GRKIT_HOST_MENU_LEVEL/4]=MENU_ROOT;
    h[GRKIT_HOST_MENU_CURSOR/4]=0;
    h[GRKIT_HOST_MODULE/4]=0;
}
/* Each ADJ session opens at the root, on the row of the module that was
   highlighted or open when the last session ended. */
static void menu_normalize(volatile u32 *h);
static void menu_reopen(volatile u32 *h) {
    u32 module=h[GRKIT_HOST_MODULE/4];
    h[GRKIT_HOST_MENU_LEVEL/4]=MENU_ROOT;
    h[GRKIT_HOST_MENU_CURSOR/4]=module<ARRAY_COUNT(modules) && visible_modules(h)>>module&1 ? root_row(h,module) : 0;
    menu_normalize(h);
}
static void menu_normalize(volatile u32 *h) {
    u32 module=h[GRKIT_HOST_MODULE/4];
    if(h[GRKIT_HOST_MENU_LEVEL/4]<MENU_ROOT || h[GRKIT_HOST_MENU_LEVEL/4]>MENU_MODULE || module>=ARRAY_COUNT(modules) ||
       (h[GRKIT_HOST_MENU_LEVEL/4]==MENU_MODULE && (!(visible_modules(h)>>module&1) || !modules[module].rows(h))))
        menu_start(h);
    if(h[GRKIT_HOST_MENU_CURSOR/4]>=count(h)) h[GRKIT_HOST_MENU_CURSOR/4]=0;
}
/* OK on a root row opens its page. A row without a page, such as About, does nothing. */
static void menu_confirm(volatile u32 *h) {
    menu_normalize(h);
    if(h[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT) {
        u32 module=root_module(h,h[GRKIT_HOST_MENU_CURSOR/4]);
        if(modules[module].rows(h)) menu_open(h,module);
    }
    else current_module(h)->apply(h,h[GRKIT_HOST_MENU_CURSOR/4]);
}
/* Return false at the top level so the caller can restore the stock ADJ page. */
static u32 menu_back(volatile u32 *h) {
    /* Leaving a level without OK restores the committed effect in the live view. */
    h[GRKIT_HOST_PREVIEW/4]=0;
    menu_normalize(h);
    if(h[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT) return 0;
    h[GRKIT_HOST_MENU_LEVEL/4]=MENU_ROOT;
    h[GRKIT_HOST_MENU_CURSOR/4]=root_row(h,h[GRKIT_HOST_MODULE/4]);
    return 1;
}
#endif

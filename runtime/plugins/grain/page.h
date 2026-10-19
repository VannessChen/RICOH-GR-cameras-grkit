/* Grain: the menu page. Off, weak, medium, and strong. The core's
   pipeline.grain hooks read the level (GRKIT_PLUGIN_GRAIN_LEVEL). */
#include "../../plugin.h"

#define GRAIN_LEVEL_COUNT (GRAIN_LEVEL_MAX+1U)
static u32 grain_level(volatile u32 *h) {
    u32 value=h[GRKIT_PLUGIN_GRAIN_LEVEL/4];
    return value<GRAIN_LEVEL_COUNT ? value : 0;
}
static const u16 *grain_title(volatile u32 *h) { return text(h,TEXT_GRAIN); }
static const u16 *grain_summary(volatile u32 *h) { return text(h,TEXT_GRAIN_SUMMARY+grain_level(h)); }
static u32 grain_rows(volatile u32 *h) { (void)h; return GRAIN_LEVEL_COUNT; }
static const u16 *grain_label(volatile u32 *h,u32 row) { return text(h,row ? TEXT_GRAIN_LEVEL+row-1 : TEXT_OFF); }
static u32 grain_icon(volatile u32 *h,u32 row) {
    if(h[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT) return GRKIT_ICON_TILE(grain_level(h));
    return row ? GRKIT_ICON_TILE(row) : GRKIT_ICON_OFF;
}
static u32 grain_checked(volatile u32 *h,u32 row) { return row==grain_level(h); }
/* Recorded pictures only: the native grain stage reads the level. */
static void grain_apply(volatile u32 *h,u32 row) { h[GRKIT_PLUGIN_GRAIN_LEVEL/4]=row; }

/* The module tile, then the three levels. */
#define GRAIN_PAGE {grain_title,grain_summary,grain_rows,grain_label,grain_icon, \
    grain_checked,grain_apply,grain_level,0,0,GRAIN_TILES}

/* Guides: the menu page. Off, thirds, center, golden ratio, and diagonals. */
#include "../../plugin.h"

#define GUIDE_STYLE_COUNT 5U
static u32 guide_style(volatile u32 *h) {
    u32 value=h[GRKIT_HOST_GUIDE/4];
    return value<GUIDE_STYLE_COUNT ? value : 0;
}
static const u16 *guide_title(volatile u32 *h) { return text(h,TEXT_GUIDES); }
static const u16 *guide_summary(volatile u32 *h) { return text(h,TEXT_GUIDES_SUMMARY+guide_style(h)); }
static u32 guide_rows(volatile u32 *h) { (void)h; return GUIDE_STYLE_COUNT; }
static const u16 *guide_label(volatile u32 *h,u32 row) { return text(h,row ? TEXT_GUIDE+row-1 : TEXT_OFF); }
/* The root tile shows the selected style, like Framing shows its ratio. */
static u32 guide_icon(volatile u32 *h,u32 row) {
    if(h[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT) return GRKIT_ICON_TILE(guide_style(h));
    return row ? GRKIT_ICON_TILE(row) : GRKIT_ICON_OFF;
}
static u32 guide_checked(volatile u32 *h,u32 row) { return row==guide_style(h); }
static void guide_apply(volatile u32 *h,u32 row) { h[GRKIT_HOST_GUIDE/4]=row; }

/* The module tile, then the four styles. */
#define GUIDES_PAGE {guide_title,guide_summary,guide_rows,guide_label,guide_icon, \
    guide_checked,guide_apply,guide_style,0,0,5}

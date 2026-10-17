/* Date Stamp: the menu page. Off and four date styles. */
#include "../../plugin.h"

#define STAMP_STYLE_COUNT 5U
static u32 stamp_style(volatile u32 *h) {
    u32 value=h[GRKIT_HOST_STAMP/4];
    return value<STAMP_STYLE_COUNT ? value : 0;
}
static const u16 *stamp_title(volatile u32 *h) { return text(h,TEXT_DATE_STAMP); }
static const u16 *stamp_summary(volatile u32 *h) { return text(h,TEXT_STAMP_SUMMARY+stamp_style(h)); }
static u32 stamp_rows(volatile u32 *h) { (void)h; return STAMP_STYLE_COUNT; }
static const u16 *stamp_label(volatile u32 *h,u32 row) { return text(h,row ? TEXT_STAMP+row-1 : TEXT_OFF); }
static u32 stamp_icon(volatile u32 *h,u32 row) {
    if(h[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT) return GRKIT_ICON_TILE(stamp_style(h));
    return row ? GRKIT_ICON_TILE(row) : GRKIT_ICON_OFF;
}
static u32 stamp_checked(volatile u32 *h,u32 row) { return row==stamp_style(h); }
/* Recorded pictures only: the encode handler prints the capture date. */
static void stamp_apply(volatile u32 *h,u32 row) { h[GRKIT_HOST_STAMP/4]=row; }

/* The module tile, then the four styles. */
#define DATE_STAMP_PAGE {stamp_title,stamp_summary,stamp_rows,stamp_label,stamp_icon, \
    stamp_checked,stamp_apply,stamp_style,0,0,5}

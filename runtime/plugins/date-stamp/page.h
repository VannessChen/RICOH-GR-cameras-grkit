/* Date Stamp: the menu page. Off and five date styles. */
#include "../../plugin.h"

#define STAMP_STYLE_COUNT (DATE_STAMP_STYLE_MAX+1U)
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

/* The settings page: the date style, then the look of the digits. */
static const u16 *style_label(volatile u32 *h) { return text(h,TEXT_DATE_STYLE); }
static u32 style_values(volatile u32 *h) { return stamp_rows(h); }
#define LOOK_COUNT (DATE_STAMP_LOOK_MAX+1U)
static const u16 *look_label(volatile u32 *h) { return text(h,TEXT_DATE_LOOK); }
static u32 look_values(volatile u32 *h) { (void)h; return LOOK_COUNT; }
static const u16 *look_text(volatile u32 *h,u32 value) { return text(h,TEXT_LOOK+value); }
static u32 stamp_look(volatile u32 *h) {
    u32 value=h[GRKIT_PLUGIN_DATE_STAMP_LOOK/4];
    return value<LOOK_COUNT ? value : 0;
}
static void look_apply(volatile u32 *h,u32 value) { h[GRKIT_PLUGIN_DATE_STAMP_LOOK/4]=value; }
static const Setting stamp_settings[SETTING_ROWS]={
    {style_label,style_values,stamp_label,stamp_style,stamp_apply},
    {look_label,look_values,look_text,stamp_look,look_apply},
};

/* The module tile, then the five styles. */
#define DATE_STAMP_PAGE {stamp_title,stamp_summary,stamp_rows,stamp_label,stamp_icon, \
    stamp_checked,stamp_apply,stamp_style,0,0,DATE_STAMP_TILES,stamp_settings}

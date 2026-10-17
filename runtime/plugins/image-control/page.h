/* Image Control: the menu page. Row 0 is Off, and row n is effect n - 1. The
   installer writes one record for each effect after the header. */
#include "../../plugin.h"

/* The row of the active record, or 0 for Off or for a pointer that is not a record. */
static u32 image_active_row(volatile u32 *h) {
    u32 i;
    for(i=0;i<h[GRKIT_HOST_COUNT/4];++i)
        if(h[GRKIT_HOST_ACTIVE/4]==h[GRKIT_HOST_RECORDS/4]+i*GRKIT_RECORD_SIZE)
            return i+1;
    return 0;
}
static const u16 *image_title(volatile u32 *h) { return text(h,TEXT_IMAGE_CONTROL); }
static const u16 *image_summary(volatile u32 *h) {
    u32 row=image_active_row(h);
    return row ? effect_summary(h,row-1) : text(h,TEXT_IMAGE_OFF_SUMMARY);
}
static u32 image_rows(volatile u32 *h) { return h[GRKIT_HOST_COUNT/4]+1; }
/* Row 0 is Off and has no record. */
static u32 image_record(volatile u32 *h,u32 row) {
    return row ? h[GRKIT_HOST_RECORDS/4]+(row-1)*GRKIT_RECORD_SIZE : 0;
}
static const u16 *image_label(volatile u32 *h,u32 row) {
    return row ? (const u16 *)(h[GRKIT_HOST_RECORDS/4]+(row-1)*GRKIT_RECORD_SIZE+GRKIT_RECORD_NAME) :
        text(h,TEXT_OFF);
}
/* The root tile shows the selected effect. */
static u32 image_icon(volatile u32 *h,u32 row) {
    if(h[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT) row=image_active_row(h);
    return row ? GRKIT_ICON_EFFECT(row-1) : GRKIT_ICON_OFF;
}
static u32 image_checked(volatile u32 *h,u32 row) {
    return h[GRKIT_HOST_ACTIVE/4]==image_record(h,row);
}
/* OK commits the highlighted row and ends its live-view preview. */
static void image_apply(volatile u32 *h,u32 row) {
    h[GRKIT_HOST_ACTIVE/4]=image_record(h,row);
    h[GRKIT_HOST_PREVIEW/4]=0;
    ++h[GRKIT_HOST_MENU_APPLIES/4];
}
/* Highlighting a row previews it in the live view. Off previews native processing. */
static void image_preview(volatile u32 *h) {
    u32 record=image_record(h,h[GRKIT_HOST_MENU_CURSOR/4]);
    h[GRKIT_HOST_PREVIEW/4]=record ? record : 1;
}
/* Fn adjusts the native Standard parameters of an effect row. Off has none. */
static u32 image_parameters(volatile u32 *h) { return h[GRKIT_HOST_MENU_CURSOR/4]!=0; }

#define IMAGE_CONTROL_PAGE {image_title,image_summary,image_rows,image_label,image_icon, \
    image_checked,image_apply,image_active_row,image_preview,image_parameters,0}

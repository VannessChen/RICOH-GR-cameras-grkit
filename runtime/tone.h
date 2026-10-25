#ifndef GRKIT_TONE_H
#define GRKIT_TONE_H
/* The capability CORE.TONE (grkit_core::capabilities): the effect of still
   pictures and the live view. Its value is 0 for Off, or n + 1 for effect n.
   The installer writes one record for each effect after the header, and the
   active-record pointer (GRKIT_HOST_ACTIVE) selects one. The installer
   converts the effects only when a loaded module uses CORE.TONE.

   A module page of CORE.TONE, such as that of Image Control, gets its rows
   from the core: the effect names, their icons, the live-view preview of the
   highlighted row, and the Fn adjustments of each effect. With a listed
   effect, a last row opens the Effects page (runtime/effects.h). The page
   gives only its title. The includer defines u32, u16, the header fields,
   and MENU_TEXT (runtime/menu.c), and includes runtime/effects.h first. */

/* The value of CORE.TONE: the row of the active record, or 0 for Off or for a
   pointer that is not a record. */
static __attribute__((unused)) u32 tone_value(volatile u32 *h) {
    u32 i;
    for(i=0;i<h[GRKIT_HOST_COUNT/4];++i)
        if(h[GRKIT_HOST_ACTIVE/4]==h[GRKIT_HOST_RECORDS/4]+i*GRKIT_RECORD_SIZE)
            return i+1;
    return 0;
}
/* The number of values: Off and each effect. */
static __attribute__((unused)) u32 tone_values(volatile u32 *h) {
    u32 n=h[GRKIT_HOST_COUNT/4];
    return n<=GRKIT_TEXT_RECORDS ? n+1 : 1;
}
/* The record of a value, or 0 for Off. */
static __attribute__((unused)) u32 tone_record(volatile u32 *h,u32 value) {
    return value ? h[GRKIT_HOST_RECORDS/4]+(value-1)*GRKIT_RECORD_SIZE : 0;
}
/* Commit a value and end its live-view preview. */
static __attribute__((unused)) void tone_store(volatile u32 *h,u32 value) {
    h[GRKIT_HOST_ACTIVE/4]=tone_record(h,value);
    h[GRKIT_HOST_PREVIEW/4]=0;
    ++h[GRKIT_HOST_MENU_APPLIES/4];
}

#endif

/* The page rows, for the menu after runtime/effects.h. runtime/settings.h
   includes this file first for the value of CORE.TONE. */
#if defined(MENU_TEXT) && defined(GRKIT_EFFECTS_H) && !defined(GRKIT_TONE_MENU_H)
#define GRKIT_TONE_MENU_H
/* Whether `row` is the last row, which opens the Effects page. */
static u32 tone_effects_row(volatile u32 *h,u32 row) { return effects_count(h) && row==tone_values(h); }
/* The rows of the page. Row n is value n, then the Effects row. */
static const u16 *tone_summary(volatile u32 *h) {
    u32 row=tone_value(h);
    return row ? effect_summary(h,row-1) : text(h,TEXT_IMAGE_OFF_SUMMARY);
}
static u32 tone_rows(volatile u32 *h) { return tone_values(h)+(effects_count(h)!=0); }
static const u16 *tone_label(volatile u32 *h,u32 row) {
    if(tone_effects_row(h,row)) return effects_row_label(h);
    return row ? (const u16 *)(tone_record(h,row)+GRKIT_RECORD_NAME) : text(h,TEXT_OFF);
}
/* The root tile shows the selected effect. */
static u32 tone_icon(volatile u32 *h,u32 row) {
    if(h[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT) row=tone_value(h);
    else if(tone_effects_row(h,row)) return GRKIT_ICON_MODULES;
    return row ? GRKIT_ICON_EFFECT(row-1) : GRKIT_ICON_OFF;
}
static u32 tone_checked(volatile u32 *h,u32 row) {
    return !tone_effects_row(h,row) && h[GRKIT_HOST_ACTIVE/4]==tone_record(h,row);
}
/* OK on the Effects row opens that page (runtime/menu.c). */
static void tone_apply(volatile u32 *h,u32 row) { if(!tone_effects_row(h,row)) tone_store(h,row); }
/* Highlighting a row previews it in the live view. Off previews native
   processing. The Effects row shows the committed effect. */
static void tone_preview(volatile u32 *h) {
    u32 row=h[GRKIT_HOST_MENU_CURSOR/4], record;
    if(tone_effects_row(h,row)) { h[GRKIT_HOST_PREVIEW/4]=0; return; }
    record=tone_record(h,row);
    h[GRKIT_HOST_PREVIEW/4]=record ? record : 1;
}
/* Fn adjusts the native Standard parameters of an effect row. Off and the
   Effects row have none. */
static u32 tone_parameters(volatile u32 *h,u32 row) { return row!=0 && row<tone_values(h); }
#endif

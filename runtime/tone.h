#ifndef GRKIT_TONE_H
#define GRKIT_TONE_H
/* The capability CORE.TONE (grkit_core::capabilities): the effect of still
   pictures and the live view. Its value is 0 for Off, or n + 1 for effect n.
   The installer writes one record for each effect after the header, and the
   active-record pointer (GRKIT_HOST_ACTIVE) selects one. The installer
   converts the effects only when a loaded module uses CORE.TONE.

   A module page of CORE.TONE, such as that of Image Control, gets its rows
   from the core: the effect names, their icons, the live-view preview of the
   highlighted row, and the Fn adjustments of each effect. The options rows
   of the GRKit section of MENU turn each effect on or off
   (runtime/effects.h). The page gives only its title. The includer defines u32, u16, the header fields,
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
/* The page shows Off, then the installed effects that are On, in the order
   of their choices. The order and the choices apply at once
   (runtime/effects.h). The record index of each shown effect goes
   to `records`; the count returns. Without a listed effect, each record
   shows in its order. */
static u32 tone_shown(volatile u32 *h,u8 *records) {
    u8 order[EFFECTS_LISTED];
    u32 n=effects_count(h), i, k=0, record;
    if(!n) {
        for(i=0;i<h[GRKIT_HOST_COUNT/4] && i<GRKIT_TEXT_RECORDS;++i) records[k++]=(u8)i;
        return k;
    }
    effects_order(h,order,n);
    for(i=0;i<n;++i) {
        if(effects_off(h,order[i])) continue;
        record=effects_record(h,order[i]);
        if(record!=EFFECTS_NO_RECORD) records[k++]=(u8)record;
    }
    return k;
}
/* The record index of row `row` (1 or more), or EFFECTS_NO_RECORD. */
static u32 tone_row_index(volatile u32 *h,u32 row) {
    u8 records[GRKIT_TEXT_RECORDS];
    u32 k=tone_shown(h,records);
    return row && row<=k ? records[row-1] : EFFECTS_NO_RECORD;
}
/* Off and the shown effects. */
static u32 tone_page_values(volatile u32 *h) {
    u8 records[GRKIT_TEXT_RECORDS];
    return tone_shown(h,records)+1;
}
/* The record of row `row`, or 0 for Off. */
static u32 tone_row_record(volatile u32 *h,u32 row) {
    u32 index=tone_row_index(h,row);
    return index!=EFFECTS_NO_RECORD ? tone_record(h,index+1) : 0;
}
/* The row of the committed effect, or 0 for Off or a hidden effect. */
static u32 tone_committed(volatile u32 *h) {
    u8 records[GRKIT_TEXT_RECORDS];
    u32 k=tone_shown(h,records), value=tone_value(h), i;
    for(i=0;value && i<k;++i) if(records[i]==value-1) return i+1;
    return 0;
}
static const u16 *tone_summary(volatile u32 *h) {
    u32 row=tone_value(h);
    return row ? effect_summary(h,row-1) : text(h,TEXT_IMAGE_OFF_SUMMARY);
}
static u32 tone_rows(volatile u32 *h) { return tone_page_values(h); }
static const u16 *tone_label(volatile u32 *h,u32 row) {
    u32 record=tone_row_record(h,row);
    return record ? (const u16 *)(record+GRKIT_RECORD_NAME) : text(h,TEXT_OFF);
}
/* The root tile shows the selected effect. */
static u32 tone_icon(volatile u32 *h,u32 row) {
    u32 index;
    if(h[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT) {
        row=tone_value(h);
        return row ? GRKIT_ICON_EFFECT(row-1) : GRKIT_ICON_OFF;
    }
    index=tone_row_index(h,row);
    return index!=EFFECTS_NO_RECORD ? GRKIT_ICON_EFFECT(index) : GRKIT_ICON_OFF;
}
static u32 tone_checked(volatile u32 *h,u32 row) {
    return h[GRKIT_HOST_ACTIVE/4]==tone_row_record(h,row);
}
static void tone_apply(volatile u32 *h,u32 row) {
    u32 index=tone_row_index(h,row);
    tone_store(h,index!=EFFECTS_NO_RECORD ? index+1 : 0);
}
/* Highlighting a row previews it in the live view. Off previews native
   processing. */
static void tone_preview(volatile u32 *h) {
    u32 record=tone_row_record(h,h[GRKIT_HOST_MENU_CURSOR/4]);
    h[GRKIT_HOST_PREVIEW/4]=record ? record : 1;
}
/* Fn adjusts the native Standard parameters of an effect row. Off has
   none. */
static u32 tone_parameters(volatile u32 *h,u32 row) { return row!=0 && row<tone_page_values(h); }
#endif

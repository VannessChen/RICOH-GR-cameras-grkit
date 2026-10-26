#ifndef GRKIT_SETTINGS_H
#define GRKIT_SETTINGS_H
/* The settings of the modules (runtime/sdk/grkit.h). INSTALL writes the table
   at the start of the module script array, and GRKIT_HOST_MODULE_SETTINGS
   holds its address. Word 0 is the number of settings and word 1 counts the
   changes. Then each setting has three words: its state file ID (0 when it is
   not saved), its value, and its number of values. The resident loader
   restores the saved values at boot and saves them after the counter changes
   (crates/grkit-core/src/runtime.rs). The includer defines u32, u16, and
   the header fields (runtime/abi.h). A component that reads or sets
   CORE.ASPECT includes runtime/aspect.h first. */
#define SETTINGS_ENTRY(handle) (2U + (handle) * 3U)
/* CORE.TONE: its value is the active record (runtime/tone.h). */
#include "tone.h"
/* A change sends SETTING_CHANGED (runtime/services.h). */
#include "services.h"
#ifndef MODULE_SETTINGS_MAP
#define MODULE_SETTINGS_MAP(address) ((volatile u32 *)(address))
#endif
/* The table, or 0 without one or with a count out of range. */
static __attribute__((unused)) volatile u32 *settings_table(volatile u32 *h) {
    u32 address=h[GRKIT_HOST_MODULE_SETTINGS/4];
    volatile u32 *t=address ? MODULE_SETTINGS_MAP(address) : 0;
    return t && t[0]<=GRKIT_MODULE_SETTINGS_MAX ? t : 0;
}
/* The capabilities of the core (GRKIT_CORE_<KEY>): each has a header word
   and a number of values. The resident loader saves the words. The value of
   CORE.ASPECT comes from the native aspect and the crop (runtime/aspect.h). */
#define CORE_SETTING(handle) ((handle)-GRKIT_CORE_SETTING_BASE)
#define IS_CORE_SETTING(handle) (CORE_SETTING(handle)<GRKIT_CORE_SETTING_COUNT)
static __attribute__((unused)) const u16 core_setting_words[]={GRKIT_CORE_SETTING_WORDS};
static __attribute__((unused)) const u16 core_setting_values[]={GRKIT_CORE_SETTING_VALUES};
/* The number of values of a setting, or 0 for an unknown handle. */
static __attribute__((unused)) u32 setting_limit(volatile u32 *h,u32 handle) {
    volatile u32 *t;
    if(handle==GRKIT_CORE_TONE) return tone_values(h);
    if(IS_CORE_SETTING(handle)) return core_setting_values[CORE_SETTING(handle)];
    t=settings_table(h);
    return t && handle<t[0] ? t[SETTINGS_ENTRY(handle)+2] : 0;
}
/* Whether the camera offers a value of a setting: each value of a module
   setting, and the ratios of the target for CORE.ASPECT. */
static __attribute__((unused)) u32 setting_offered(volatile u32 *h,u32 handle,u32 value) {
    if(value>=setting_limit(h,handle)) return 0;
#ifdef GRKIT_ASPECT_H
    if(handle==GRKIT_CORE_ASPECT) return aspect_offered(value);
#endif
    return 1;
}
#ifdef GRKIT_ASPECT_H
/* The rows of a choice page of a setting: the values that the camera
   offers, in the order of the page. A module setting lists its values in
   order, and CORE.ASPECT the rows of the target (FRAMING_RATIO_ROWS). */
static __attribute__((unused)) u32 setting_rows(volatile u32 *h,u32 handle) {
    return handle==GRKIT_CORE_ASPECT ? framing_ratios() : setting_limit(h,handle);
}
/* The value of a row. */
static __attribute__((unused)) u32 setting_row_value(volatile u32 *h,u32 handle,u32 row) {
    (void)h;
    return handle==GRKIT_CORE_ASPECT ? framing_ratio(row) : row;
}
/* The row of a value, or row 0 for a value that the camera does not offer. */
static __attribute__((unused)) u32 setting_value_row(volatile u32 *h,u32 handle,u32 value) {
    if(handle==GRKIT_CORE_ASPECT) return framing_row(value);
    return value<setting_limit(h,handle) ? value : 0;
}
#endif
/* The value of a setting, or 0 for an unknown handle or a value out of its
   range. */
static __attribute__((unused)) u32 setting_value(volatile u32 *h,u32 handle) {
    volatile u32 *t;
    u32 value;
    if(handle==GRKIT_CORE_TONE) return tone_value(h);
    if(IS_CORE_SETTING(handle)) {
#ifdef ASPECT_NATIVE
        if(handle==GRKIT_CORE_ASPECT) return aspect_value(h);
#endif
        value=h[core_setting_words[CORE_SETTING(handle)]/4];
        return value<core_setting_values[CORE_SETTING(handle)] ? value : 0;
    }
    t=settings_table(h);
    if(!t || handle>=t[0]) return 0;
    value=t[SETTINGS_ENTRY(handle)+1];
    return value<t[SETTINGS_ENTRY(handle)+2] ? value : 0;
}
/* Set a setting. A value that the setting does not offer does nothing. A
   change of a module setting counts in the table. Only a component that
   sets the native aspect (ASPECT_SET_NATIVE) sets CORE.ASPECT. */
static __attribute__((unused)) void setting_store(volatile u32 *h,u32 handle,u32 value) {
    volatile u32 *t;
    if(IS_CORE_SETTING(handle)) {
        u32 old=setting_value(h,handle);
        if(!setting_offered(h,handle,value)) return;
        if(handle==GRKIT_CORE_TONE) tone_store(h,value);
        else if(handle==GRKIT_CORE_ASPECT) {
#ifdef ASPECT_SET_NATIVE
            aspect_store(h,value);
#else
            return;
#endif
        }
        else h[core_setting_words[CORE_SETTING(handle)]/4]=value;
        if(old!=value) {
            GRKIT_STATE_CHANGED(h);
            services_emit(h,SERVICE_EVENT_SETTING_CHANGED,handle,value);
        }
        return;
    }
    t=settings_table(h);
    if(!t || handle>=t[0] || value>=t[SETTINGS_ENTRY(handle)+2] || t[SETTINGS_ENTRY(handle)+1]==value) return;
    t[SETTINGS_ENTRY(handle)+1]=value;
    ++t[1];
    GRKIT_STATE_CHANGED(h);
    services_emit(h,SERVICE_EVENT_SETTING_CHANGED,handle,value);
}
#endif

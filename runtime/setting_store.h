#ifndef GRKIT_SETTING_STORE_H
#define GRKIT_SETTING_STORE_H
/* Set the settings of runtime/settings.h. The menu component sets them, and
   the other components only read them. The includer defines the three
   functions that follow. */
#include "settings.h"
/* A change sends SETTING_CHANGED (runtime/services.h). */
#include "services.h"
/* Set the native still aspect as the native menu does. */
static void aspect_set_native(u32 id);
/* Send the native aspect request again, so the capture hook applies the
   crop. */
static void aspect_request(void);
/* Act on a committed effect. */
static void tone_stored(volatile u32 *h);

/* Commit a value of CORE.TONE and end its live-view preview. */
static __attribute__((unused)) void tone_store(volatile u32 *h,u32 value) {
    h[GRKIT_HOST_ACTIVE/4]=tone_record(h,value);
    h[GRKIT_HOST_PREVIEW/4]=0;
    GRKIT_STATE_CHANGED(h);
    tone_stored(h);
}

/* Set CORE.ASPECT to a value that the target offers. Each value needs the
   native 3:2, which the native setter requests. A crop then sends the
   request again, so the capture hook applies the new crop. */
static __attribute__((unused)) void aspect_store(volatile u32 *h,u32 value) {
    h[GRKIT_HOST_FRAME_RATIO/4]=value;
    if(aspect_native()) {
        aspect_set_native(0);
        if(!value) return;
    }
    aspect_request();
}

/* Set a setting. A value that the setting does not offer does nothing. A
   change of a module setting counts in the table. */
static __attribute__((unused)) void setting_store(volatile u32 *h,u32 handle,u32 value) {
    volatile u32 *t;
    if(IS_CORE_SETTING(handle)) {
        u32 old=setting_value(h,handle);
        if(!setting_offered(h,handle,value)) return;
        if(handle==GRKIT_CORE_TONE) tone_store(h,value);
        else if(handle==GRKIT_CORE_ASPECT) aspect_store(h,value);
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

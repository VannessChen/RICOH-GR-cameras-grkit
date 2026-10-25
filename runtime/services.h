#ifndef GRKIT_SERVICES_H
#define GRKIT_SERVICES_H
/* The services of the core for modules (runtime/sdk/grkit.h): events, the
   timer, and the UI. Their state is a block of the module array, which
   INSTALL writes, and GRKIT_HOST_SERVICES holds its address. The includer
   defines u8, u16, u32, and the header fields.

   The block: the number of subscribers, then for each its handler and its
   events; the ready flag, the event depth, and the App call depth; a slot
   for each timer; then the UI state.

   The timer and UI services are valid only in the App task. The core calls
   module code in the App task only through services_call1, 2, or 3: an
   event handler, a timer callback, or a UI callback. The App call depth is
   above 0 only in such a call, so the services refuse a call from module
   code in another task, such as a layer or a plane filter. */
#define SERVICES_SUBSCRIBER_COUNT 0U
#define SERVICES_SUBSCRIBERS 4U
#define SERVICES_SUBSCRIBERS_MAX 8U
#define SERVICES_READY (SERVICES_SUBSCRIBERS + 8U * SERVICES_SUBSCRIBERS_MAX)
#define SERVICES_DEPTH (SERVICES_READY + 4U)
#define SERVICES_APP (SERVICES_DEPTH + 4U)
/* A timer slot: the id of the firmware timer (0 when free), the callback,
   and its argument. */
#define SERVICES_TIMERS (SERVICES_APP + 4U)
#define SERVICES_TIMER_SLOTS 8U
#define SERVICES_TIMER_SIZE 12U
#define SERVICES_UI (SERVICES_TIMERS + SERVICES_TIMER_SLOTS * SERVICES_TIMER_SIZE)
/* The UI: its mode, the message box and the progress bar, their parent, the
   firmware timer of the box, the callback and its argument, the texts, the
   text descriptor of the box (two words); then the native screen that waits
   to open and its firmware timer, and the callback for the return from a
   native screen and its argument. */
#define UI_MODE 0U
#define UI_VIEW 4U
#define UI_BAR 8U
#define UI_PARENT 12U
#define UI_TIMER 16U
#define UI_DONE 20U
#define UI_ARG 24U
#define UI_TEXT 28U
#define UI_DESCRIPTOR 32U
#define UI_NATIVE 40U
#define UI_NATIVE_TIMER 44U
#define UI_BACK 48U
#define UI_BACK_ARG 52U
/* A slider box (ui_slider): its range, its value and the value when it
   opened, and the callback for each change. The slider view is UI_BAR. */
#define UI_MIN 56U
#define UI_MAX 60U
#define UI_VALUE 64U
#define UI_START 68U
#define UI_CHANGED 72U
#define UI_SIZE 76U
/* The rows of modules in the native MENU (runtime/menu.c): whether the
   Customize tab has been patched and the number of rows; for each row its
   module, its label and value views, and their text descriptors; then the
   private copy of the Customize tab descriptor. */
#define SERVICES_MENU (SERVICES_UI + UI_SIZE)
#define MENU_ROWS_DONE 0U
#define MENU_ROWS_COUNT 4U
#define MENU_ROWS_SLOTS 8U
#define MENU_ROW_MODULE 0U
#define MENU_ROW_LABEL 4U
#define MENU_ROW_VALUE 8U
#define MENU_ROW_LABEL_TEXT_AT 12U
#define MENU_ROW_VALUE_TEXT_AT 20U
#define MENU_ROW_SIZE 28U
#define MENU_ROWS_TAB (8U + MENU_ROWS_SLOTS * MENU_ROW_SIZE)
#define MENU_ROWS_TAB_SIZE 0x384U
#define SERVICES_SIZE (SERVICES_MENU + MENU_ROWS_TAB + MENU_ROWS_TAB_SIZE)
#define SERVICES_WORDS (SERVICES_SIZE / 4U)
/* The events (GRKIT_EVENT_*). */
#define SERVICE_EVENT_SYSTEM_READY 1U
#define SERVICE_EVENT_SETTING_CHANGED 2U
/* The native screens of ui_open_native (GRKIT_NATIVE_*). */
#define SERVICE_NATIVE_WHITE_BALANCE 1U
#define SERVICE_NATIVE_LCD 2U

/* The block at a camera address. A host test maps it. */
#ifndef SERVICES_MAP
#define SERVICES_MAP(address) ((volatile u32 *)(address))
#endif
static __attribute__((unused)) volatile u32 *services_word(volatile u32 *h,u32 offset) {
    return SERVICES_MAP(h[GRKIT_HOST_SERVICES/4]+offset);
}

/* Whether module code that the core called in the App task runs now. */
static __attribute__((unused)) u32 services_in_app(volatile u32 *h) {
    return h[GRKIT_HOST_SERVICES/4] && services_word(h,SERVICES_APP)[0];
}
/* Call module code in the App task with one, two, or three arguments. */
static __attribute__((unused)) void services_call1(volatile u32 *h,u32 fn,u32 a) {
    ++services_word(h,SERVICES_APP)[0];
    ((void (*)(u32))fn)(a);
    --services_word(h,SERVICES_APP)[0];
}
static __attribute__((unused)) void services_call2(volatile u32 *h,u32 fn,u32 a,u32 b) {
    ++services_word(h,SERVICES_APP)[0];
    ((void (*)(u32,u32))fn)(a,b);
    --services_word(h,SERVICES_APP)[0];
}
static __attribute__((unused)) void services_call3(volatile u32 *h,u32 fn,u32 a,u32 b,u32 c) {
    ++services_word(h,SERVICES_APP)[0];
    ((void (*)(u32,u32,u32))fn)(a,b,c);
    --services_word(h,SERVICES_APP)[0];
}

/* Call the handler of each subscriber to `event`. A handler that changes a
   setting sends no second event, so a handler cannot loop. */
static __attribute__((unused)) void services_emit(volatile u32 *h,u32 event,u32 a,u32 b) {
    u32 n, i;
    if(!h[GRKIT_HOST_SERVICES/4] || services_word(h,SERVICES_DEPTH)[0]) return;
    n=services_word(h,SERVICES_SUBSCRIBER_COUNT)[0];
    if(n>SERVICES_SUBSCRIBERS_MAX) return;
    services_word(h,SERVICES_DEPTH)[0]=1;
    for(i=0;i<n;++i) {
        u32 handler=services_word(h,SERVICES_SUBSCRIBERS+8*i)[0];
        if(handler && services_word(h,SERVICES_SUBSCRIBERS+8*i+4)[0]&event)
            services_call3(h,handler,event,a,b);
    }
    services_word(h,SERVICES_DEPTH)[0]=0;
}
/* SYSTEM_READY, once after INSTALL: the core calls it when the shooting
   screen first shows (runtime/framing.c). */
static __attribute__((unused)) void services_ready(volatile u32 *h) {
    if(!h[GRKIT_HOST_SERVICES/4] || services_word(h,SERVICES_READY)[0]) return;
    services_word(h,SERVICES_READY)[0]=1;
    services_emit(h,SERVICE_EVENT_SYSTEM_READY,0,0);
}
#endif

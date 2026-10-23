/* The target header, then the shared layout. */
#include "firmware.h"
#include "abi.h"
/* Private ADJ menu. All state belongs to the module host. */
typedef unsigned int u32;
typedef unsigned short u16;
typedef unsigned char u8;
extern volatile u32 *menu_header(void);
extern u32 menu_event_original(u32, const u16 *);
extern void menu_exit_original(u32);
extern void menu_changed_original(u32);
extern void menu_refresh_original(u32);
extern u32 menu_destroy_original(u32);
#define CALL0(a) ((u32 (*)(void))(a))
#define CALL1(a) ((u32 (*)(u32))(a))
#define CALL2(a) ((u32 (*)(u32,u32))(a))
#define CALL4(a) ((u32 (*)(u32,u32,u32,u32))(a))
#define CALL5(a) ((u32 (*)(u32,u32,u32,u32,u32))(a))
#define CALL3(a) ((u32 (*)(u32,u32,u32))(a))
#define WORD(a,o) (*(u32 *)((a)+(o)))
#define BYTE(a,o) (*(u8 *)((a)+(o)))
/* The plugin interface, before the services that it declares. */
#include "plugin.h"
#define PAGE 6U
/* GRKIT_HOST_RETURN: a native page (the parameter page or a settings page)
   is opening (1) or open (2). ADJ exited while GRKit was open (3), so the next
   ADJ opens GRKit. MENU or OK on GRKit passes the event that closes ADJ to the
   native list (4); the exit then sets 3. Only an exit sets 3, because leaving
   GRKit refreshes ADJ. RETURN_ROOT marks a page that opened from the root, so
   the page returns to the root instead of the page of the module. */
#define RETURN_OPENING 1U
#define RETURN_PARAMETERS 2U
#define RETURN_GRKIT 3U
#define RETURN_CLOSING 4U
#define RETURN_ROOT 0x10U
#define RETURN_STATE(value) ((value)&~RETURN_ROOT)
/* The return state after an ADJ exit. `in_grkit` is set for an exit while
   GRKit was open. */
static u32 exit_return(u32 value,u32 in_grkit) {
    if(RETURN_STATE(value)==RETURN_OPENING) return RETURN_PARAMETERS|(value&RETURN_ROOT);
    return in_grkit || value==RETURN_CLOSING ? RETURN_GRKIT : 0;
}
/* Menu C is Thumb-2 for size. Firmware hooks and thunks enter with A32 B/BL,
   which cannot change state, so their targets stay A32. */
#ifdef __arm__
#define A32_ENTRY __attribute__((target("arm")))
#else
#define A32_ENTRY
#endif
static void plugin_touch(u32 controller);
static void render(u32 controller);
static void render_step(u32 controller,u32 step);
static void enter(u32 controller,u32 edge);
static const u16 plugins[] = {'G','R','K','i','t',0};
#ifndef MENU_TEXT
/* The menu text at a camera address. Host tests map the address. */
#define MENU_TEXT(address) ((const u16 *)(address))
#endif
#ifndef MENU_LANGUAGE
/* The camera's UI language byte (runtime/targets): 0 is English, 18
   Traditional Chinese, 19 Simplified Chinese, and 20 Japanese. */
static u32 camera_language(void) {
#if defined(UI_SETTINGS_SLOT)
    u32 settings=WORD(UI_SETTINGS_SLOT,0);
    return settings ? BYTE(settings,LANGUAGE_SETTING) : 0;
#elif defined(LANGUAGE_SETTING_BYTE)
    return BYTE(LANGUAGE_SETTING_BYTE,0);
#else
    return 0;
#endif
}
/* The camera language as a menu language (runtime/abi.h), or English. */
static u32 camera_menu_language(void) {
    switch (camera_language()) {
    case 19: return 1;
    case 18: return 2;
    case 20: return 3;
    default: return 0;
    }
}
#define MENU_LANGUAGE() camera_menu_language()
#endif
static u32 menu_language(void) {
    u32 language=MENU_LANGUAGE();
    return language<GRKIT_TEXT_LANGUAGES ? language : 0;
}
/* The language table index that the text hooks of menu.S pass on for GRKit
   text. It selects the font: a Chinese or Japanese font draws that text, and
   the Latin table index 1 draws English. Chinese or Japanese text with index 1
   shows "?" glyphs. The firmware maps the UI language byte to this index:
   Chinese keeps 18 and 19, and Japanese (byte 20) uses index 0. */
A32_ENTRY u32 menu_table_index(void) {
    switch (menu_language()) {
    case 1: return 19;
    case 2: return 18;
    case 3: return 0;
    default: return 1;
    }
}
/* One string of the label table in the menu language. */
static const u16 *text(volatile u32 *h,u32 id) {
    u32 base=GRKIT_TEXT(h);
    return MENU_TEXT(base+MENU_TEXT(base)[menu_language()*GRKIT_TEXT_STRINGS+id]);
}

/* A wrapped stock navigation result places the private entry across that edge. */
static u32 wrapped_edge(u32 kind,u32 old,u32 next,u32 movie) {
    if(movie==1) return 0;
    if(kind==EV_RIGHT && next<=old) return 2;
    if(kind==EV_LEFT && next>=old) return 1;
    return 0;
}
static u32 advance_from_private(u32 kind,u32 edge) {
    return (kind==EV_RIGHT && edge==1) || (kind==EV_LEFT && edge==2);
}
/* Match the native ActionModel: only the initial press starts an action. */
static u32 menu_press(u32 phase,u32 repeats) { return phase==2 && repeats==0; }
/* MENU emits a switch event followed by the same confirm used by OK. */
static u32 back_event(volatile u32 *h,u32 kind,u32 enabled) {
    if(kind==EV_SWITCH_END) h[GRKIT_HOST_BACK_PENDING/4]=0;
    if(kind==EV_SWITCH && enabled) {
        if(h[GRKIT_HOST_BACK_PENDING/4]) return 2;
        h[GRKIT_HOST_BACK_PENDING/4]=1;
        return 1;
    }
    if(kind==EV_CONFIRM && h[GRKIT_HOST_BACK_PENDING/4]) {
        h[GRKIT_HOST_BACK_PENDING/4]=0;
        return 2;
    }
    return 0;
}
/* MENU closes native ADJ with its switch event (5) followed by the OK confirm. */
static u32 after_menu_switch(volatile u32 *h) {
    return (h[GRKIT_HOST_MENU_EVENT_RING/4+((h[GRKIT_HOST_MENU_EVENTS/4]-2)&(GRKIT_HOST_MENU_EVENT_RING_WORDS-1))]&0xffff)==EV_SWITCH;
}
#ifdef EV_ADJ_PRESS
/* An ADJ press emits EV_ADJ_PRESS followed by the confirm that OK also sends. */
static u32 after_adj_press(volatile u32 *h) {
    return (h[GRKIT_HOST_MENU_EVENT_RING/4+((h[GRKIT_HOST_MENU_EVENTS/4]-2)&(GRKIT_HOST_MENU_EVENT_RING_WORDS-1))]&0xffff)==EV_ADJ_PRESS;
}
#endif
static u32 native_action_release(u32 action,u32 active) {
    return action==10 && active;
}
static u32 selected_icon(volatile u32 *h) {
    u32 i;
    for(i=0;i<h[GRKIT_HOST_COUNT/4];++i)
        if(h[GRKIT_HOST_ACTIVE/4]==h[GRKIT_HOST_RECORDS/4]+i*GRKIT_RECORD_SIZE)
            return 0x8000+i;
    return 0x7fff;
}
/* Override only the native still-viewfinder icon result. Off retains stock UI. */
A32_ENTRY u32 menu_viewfinder_icon_select(u32 native_icon,u32 movie) {
    u32 icon=selected_icon(menu_header());
    return !movie && icon!=0x7fff ? icon : native_icon;
}
#ifndef MENU_FRAMING_REQUEST
/* The family header sends the native aspect request again after a Framing
   change, so the capture hook maps it to the new ratio. */
#error "The family header defines MENU_FRAMING_REQUEST"
#endif
#ifndef TOOLBAR_CREATED
/* The first full ADJ draw follows the toolbar creation. */
#define TOOLBAR_CREATED(widget,icon) ((void)0)
#endif
/* Plugin services (runtime/plugin.h). */
/* The Image Control summary of effect n. The installer writes one for each
   record in each language (runtime/abi.h). */
static const u16 *effect_summary(volatile u32 *h,u32 n) {
    return MENU_TEXT(GRKIT_TEXT(h)+GRKIT_TEXT_SUMMARY+
        (menu_language()*GRKIT_TEXT_RECORDS+n)*GRKIT_SUMMARY_UNITS*2);
}
static u32 framing_ratios(void) { return FRAMING_RATIO_COUNT; }
static void request_native_aspect(void) { MENU_FRAMING_REQUEST(); }
#include "modules.h"
static u32 private_mode(u32 controller) {
    volatile u32 *h=menu_header();
    return h[GRKIT_HOST_MENU_CONTROLLER/4] == controller && h[GRKIT_HOST_MENU_LEVEL/4] != 0;
}
/* A native Image Control preview lasts while its ADJ slot has focus. */
static void native_focus(u32 controller) {
    if(CALL2(FW_ADJ_SLOT_TYPE)(controller,BYTE(controller,ADJ_SLOT))!=ADJ_IC_SLOT_TYPE)
        menu_header()[GRKIT_HOST_PREVIEW/4]=0;
}
/* Fn opens the native parameter page from an effect row in the still
   context. The native Fn hint shows only where this works. */
static u32 parameters_available(u32 controller) {
#if defined(PARAMETER_KINDS)
    /* The page edits the adjustments of the effect (menu_parameter_*), so the
       native Image Control type does not matter. */
    return parameters_row(menu_header()) && ADJ_MOVIE(controller)!=1;
#elif defined(FW_SETTINGS_DOMAIN)
    /* The page edits the native settings of the active type, which the
       effects apply. */
    u32 settings=WORD(WORD(WORD(FW_SETTINGS_DOMAIN,0),0x518),0);
    return parameters_row(menu_header()) &&
           ADJ_MOVIE(controller)!=1 && BYTE(settings,SETTINGS_IC_TYPE)==11;
#else
    /* The target has no native Standard parameter page for Fn yet. */
    (void)controller;
    return 0;
#endif
}
/* Native slot indices stay in 0..4. Only the transient toolbar has six cells. */
static u32 toolbar_x(u32 slot) { return 415 + 50 * slot; }
static void toolbar_destroy(u32 widget) {
    /* Both sibling objects share an allocation; only LineView releases it. */
    CALL1(FW_ICON_VIEW_DESTROY)(widget+0xf0);
    CALL1(FW_LINE_VIEW_DESTROY)(widget);
}
static void toolbar_visible(u32 widget,u32 visible) {
    CALL2(FW_VIEW_SET_VISIBLE)(widget,visible);
    CALL2(FW_VIEW_SET_VISIBLE)(widget+0xf0,visible);
}
static void toolbar_layout(u32 controller,u32 expanded) {
    u32 i, origin=expanded ? 415 : 425, pitch=expanded ? 50 : 60;
    for(i=0;i<5;++i) {
        CALL2(FW_VIEW_SET_X)(WORD(controller,ADJ_CELLS+i*4),origin+pitch*i);
        CALL2(FW_VIEW_SET_X)(WORD(controller,ADJ_ICONS+i*4),origin+pitch*i+5);
    }
    CALL2(FW_VIEW_SET_X)(WORD(controller,ADJ_FRAME),expanded ?
        toolbar_x(private_mode(controller) ? 5 : BYTE(controller,ADJ_SLOT)) :
        425+60*BYTE(controller,ADJ_SLOT));
}
static void toolbar(u32 controller) {
    volatile u32 *h=menu_header();
    u32 widget=h[GRKIT_HOST_TOOLBAR_WIDGET/4];
    if(widget && h[GRKIT_HOST_TOOLBAR_OWNER/4]!=controller) return;
    if(ADJ_MOVIE(controller)==1) {
        if(widget) toolbar_visible(widget,0);
        toolbar_layout(controller,0);
        return;
    }
    if(!widget) {
        u32 parent=WORD(WORD(controller,ADJ_ICONS),4);
        u32 position=WORD(parent,0xc), end=WORD(parent,0x10), icon, offset;
        u32 binding[3]={(u32)plugin_touch,0,controller};
        u32 callback[4]={(u32)binding,0,FW_ADJ_TOUCH_ENTER,FW_ADJ_TOUCH_LEAVE};
        /* Draw the native background before the shared selection frame. */
        while(position!=end && WORD(position,0)!=WORD(controller,ADJ_FRAME)) position+=4;
        if(position==end) return;
        offset=position-WORD(parent,0xc);
        widget=CALL1(FW_ALLOCATE)(0x1e8);
        if(!widget) return;
        icon=widget+0xf0;
        CALL5(FW_LINE_VIEW_INIT)(widget,toolbar_x(5),5,50,50);
        CALL2(FW_LINE_VIEW_SET_STYLE)(widget,0x10);
        CALL5(FW_ICON_VIEW_INIT)(icon,toolbar_x(5)+5,10,40,40);
        CALL2(FW_ICON_VIEW_SET_ICON)(icon,GRKIT_ICON_GRKIT);
        CALL2(FW_VIEW_SET_TOUCH)(icon,(u32)callback);
        CALL2(FW_VIEW_SET_TOUCHABLE)(icon,1);
        /* Use the same vector insertion as View::AddChild, at the marker. */
        if(!CALL3(FW_VECTOR_INSERT)(parent+8,position,(u32)&widget)) {
            toolbar_destroy(widget);
            return;
        }
        /* The icon must draw after the frame, which fills the entire cell. */
        if(!CALL3(FW_VECTOR_INSERT)(parent+8,WORD(parent,0x10),(u32)&icon)) {
            CALL2(FW_VECTOR_ERASE)(parent+8,WORD(parent,0xc)+offset);
            toolbar_destroy(widget);
            return;
        }
        CALL2(FW_VIEW_SET_PARENT)(widget,parent);
        CALL2(FW_VIEW_SET_PARENT)(icon,parent);
        TOOLBAR_CREATED(widget,icon);
        h[GRKIT_HOST_TOOLBAR_WIDGET/4]=widget;
        h[GRKIT_HOST_TOOLBAR_OWNER/4]=controller;
    }
    toolbar_visible(widget,1);
    toolbar_layout(controller,1);
    CALL2(FW_VIEW_SET_VISIBLE)(WORD(controller,ADJ_FRAME),1);
}
#ifdef PARAMETER_KINDS
/* The parameter page title of a GRKit session (GRKIT_TEXT_PARAMETER_TITLE). */
static volatile u32 *parameter_title(volatile u32 *h) {
    return (volatile u32 *)(GRKIT_TEXT(h)+GRKIT_TEXT_PARAMETER_TITLE);
}
#endif
A32_ENTRY void menu_refresh(u32 controller) {
    volatile u32 *h=menu_header();
#ifdef PARAMETER_KINDS
    /* ADJ runs, so no parameter page shows its title. */
    parameter_title(h)[0]=0;
#endif
    if(private_mode(controller)) { render(controller); return; }
    if(RETURN_STATE(h[GRKIT_HOST_RETURN/4])==RETURN_PARAMETERS && ADJ_MOVIE(controller)!=1) {
        u32 root=h[GRKIT_HOST_RETURN/4]&RETURN_ROOT;
        h[GRKIT_HOST_RETURN/4]=0;
#ifdef PARAMETER_KINDS
        /* The page has ended. A record that no page started stays unused. */
        if(h[GRKIT_HOST_PARAMETERS/4]&1U) h[GRKIT_HOST_PARAMETERS/4]=0;
#endif
        /* GRKit reopens at the root row of the module (menu_reopen). Fn on a
           page returns to that page. Fn starts only from Image Control, so it
           is visible. A settings page returns to the root. */
        enter(controller,3);
        if(!root) {
            menu_open(h,0);
            render(controller);
        }
        return;
    }
    if(h[GRKIT_HOST_RETURN/4]==RETURN_GRKIT) {
        h[GRKIT_HOST_RETURN/4]=0;
        /* ADJ opens on GRKit, as a native ADJ opens on its last entry. The
           native refresh first sets the list of the native slot, so that
           leaving GRKit restores the value of that slot. */
        if(ADJ_MOVIE(controller)!=1) {
            menu_refresh_original(controller);
            enter(controller,h[GRKIT_HOST_MENU_EDGE/4]);
            return;
        }
    }
    menu_refresh_original(controller);
    toolbar(controller);
    frame_preview(h,WORD(controller,ADJ_CANVAS),ADJ_MOVIE(controller)==1);
}
/* First row of the cursor's page. The reciprocal equals row/6 for rows below 2^15
   and avoids a division helper, which the freestanding link does not provide. */
static u32 page_start(u32 row) { return (row*0xaaabU>>18)*PAGE; }
/* The list has as many rows as the longest native slot of the controller
   (32 on GR III). It holds all rows of the root or page when they fit, and
   scrolls them as a native list does. Otherwise it holds the page of six rows
   around the cursor. Each native ADJ list holds more than six rows. */
static u32 list_capacity(u32 view) { return (WORD(view,LIST_ROWS+4)-WORD(view,LIST_ROWS))>>2; }
static u32 list_start(volatile u32 *h,u32 view) {
    return count(h)>list_capacity(view) ? page_start(h[GRKIT_HOST_MENU_CURSOR/4]) : 0;
}
static void descriptor(volatile u32 *at, const u16 *name) {
    u32 length=0;
    while (length<GRKIT_SUMMARY_UNITS-1 && name[length]) ++length;
    at[0]=(length+1)|0x100; at[1]=(u32)name;
}
static void label(u32 object) {
    CALL3(FW_STRING_VIEW_RESET)(object,0,0);
    BYTE(object,0xf3)=1;
    CALL1(FW_VIEW_INVALIDATE)(object);
    CALL2(FW_VIEW_SET_VISIBLE)(object,1);
}
static __attribute__((noinline)) void detail_width(u32 controller,u32 width) {
    u32 object=WORD(controller,ADJ_DETAIL), rectangle[4]={0,50,width,40};
    CALL2(WORD(WORD(object,0),0x18))(object,(u32)rectangle);
}
/* Size the list for `rows` rows, as the native refresh does for the values of
   a slot: rows are 60 pixels high, a short list ends at y=420, and six or more
   rows fill y=60..420 and scroll. The private menu replaces the native
   refresh, so without this the list keeps the size of the last native slot. */
static void list_frame(u32 view,u32 rows) {
    u32 width=CALL1(FW_LIST_WIDTH)(view), rectangle[4];
    if(rows>PAGE) rows=PAGE;
    rectangle[0]=720-width; rectangle[1]=60+60*(PAGE-rows);
    rectangle[2]=width; rectangle[3]=60*rows;
    CALL2(WORD(WORD(view,0),0x18))(view,(u32)rectangle);
}
/* Show the Fn hint where Fn opens the parameter page. The native refresh
   fills the hint only on a native slot that offers Fn, so a hint that GRKit
   shows could be empty. Fill it as the native Image Control slot does. */
static void fn_guide(u32 controller) {
    u32 guide=WORD(controller,ADJ_FN_GUIDE), shown=parameters_available(controller);
#ifdef FW_GUIDE_SET_CONTENT
    if(shown) {
        CALL4(FW_GUIDE_SET_CONTENT)(guide,FN_GUIDE_TEXT,10,FN_GUIDE_ICON);
#ifdef FN_GUIDE_X
        CALL2(FW_VIEW_SET_X)(guide,FN_GUIDE_X);
#endif
    }
#endif
    CALL2(FW_VIEW_SET_VISIBLE)(guide,shown);
}
/* Put the list cursor on the cursor row. After one Up or Down step (`step`
   is 1 or -1), move it as the native list does: the list keeps one row of
   context, animates the scroll, and wraps at the ends. If the list keeps its
   rows and is already on the row, as after a touch or a refresh, it keeps its
   scroll. Otherwise a jump scrolls the row into view without animation. */
static void list_cursor(u32 view,u32 row,u32 rows,u32 step,u32 kept) {
    u32 at=CALL1(FW_LIST_CURSOR)(view);
    u32 before=step==1 ? (row ? row-1 : rows-1) : (row+1==rows ? 0 : row+1);
    if(kept && step && at==before) CALL3(FW_LIST_MOVE)(view,step,1);
    else if(!kept || at!=row) CALL2(FW_LIST_SET_CURSOR)(view,row);
}
static void render(u32 controller) { render_step(controller,0); }
static void render_step(u32 controller,u32 step) {
    volatile u32 *h=menu_header();
    u32 cursor, start, n, i, total, kept;
    u32 view=WORD(controller,ADJ_LIST);
    const Module *module;
    menu_normalize(h);
    cursor=h[GRKIT_HOST_MENU_CURSOR/4]; total=count(h);
    start=list_start(h,view); n=total-start;
    module=current_module(h);
    if(total>list_capacity(view)) {
        /* Paged rows move one page at a time. */
        step=0;
        if(n>PAGE) n=PAGE;
    }
    kept=BYTE(view,LIST_COUNT)==n;
    /* Suppress the native list callback while replacing private rows. */
    h[GRKIT_HOST_MENU_REBUILDING/4]=1;
    CALL2(FW_LIST_SET_COUNT)(view,n);
    for(i=0;i<n;++i) {
        u32 row=CALL2(FW_LIST_ROW)(view,i);
        u32 index=start+i;
        /* Private icon IDs are resolved by the host's guarded icon adapter. */
        u32 owner=h[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT ? root_module(h,index) : h[GRKIT_HOST_MODULE/4];
        u32 icon=module_icon(owner,modules[owner].icon(h,index));
        CALL2(FW_ROW_SET_ICON)(row,icon);
        CALL2(FW_ROW_SET_CHECK)(row,h[GRKIT_HOST_MENU_LEVEL/4]!=MENU_ROOT && module->checked(h,index));
    }
    /* The size comes from all rows, so that a short last page keeps the
       position of the full pages. The cursor needs the new size. */
    list_frame(view,total);
    list_cursor(view,cursor-start,n,step,kept);
    descriptor(h+GRKIT_HOST_MENU_TITLE/4,h[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT ? plugins : module->title(h));
    descriptor(h+GRKIT_HOST_MENU_DETAIL/4,h[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT ? module->summary(h) : module->label(h,cursor));
    label(WORD(controller,ADJ_TITLE));
    /* The summary ends before the list at x=620; native labels use 400 pixels. */
    detail_width(controller,h[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT ? 620 : 400);
    label(WORD(controller,ADJ_DETAIL));
    /* Native detail controls do not describe private modules, except the Fn
       hint (+0x1ac) where Fn opens the native parameters. */
    CALL2(FW_VIEW_SET_VISIBLE)(WORD(controller,ADJ_DETAIL_VALUE),0);
    fn_guide(controller);
    toolbar(controller);
    /* Slot zero's canvas is at +4, as in the live view. */
    frame_preview(h,WORD(controller,ADJ_CANVAS),ADJ_MOVIE(controller)==1);
    h[GRKIT_HOST_MENU_REBUILDING/4]=0;
}
static void leave(u32 controller) {
    volatile u32 *h=menu_header();
    if(!private_mode(controller)) return;
    /* The next ADJ session opens at the row of this module (menu_reopen). */
    if(h[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT) h[GRKIT_HOST_MODULE/4]=root_module(h,h[GRKIT_HOST_MENU_CURSOR/4]);
    h[GRKIT_HOST_MENU_LEVEL/4]=0; h[GRKIT_HOST_MENU_REBUILDING/4]=1;
    h[GRKIT_HOST_PREVIEW/4]=0;
    detail_width(controller,400);
    /* Rebuild the original list before its exit callback saves a cursor. */
    CALL1(FW_ADJ_REFRESH)(controller);
    CALL2(FW_VIEW_SET_VISIBLE)(WORD(controller,ADJ_FRAME),1);
    CALL3(FW_LIST_RESTORE)(WORD(controller,ADJ_LIST),h[GRKIT_HOST_MENU_LIST/4],h[GRKIT_HOST_MENU_LIST/4+1]);
    h[GRKIT_HOST_MENU_CONTROLLER/4]=0; h[GRKIT_HOST_MENU_REBUILDING/4]=0;
}
static void enter(u32 controller,u32 edge) {
    volatile u32 *h=menu_header();
    u32 view=WORD(controller,ADJ_LIST);
    h[GRKIT_HOST_MENU_CONTROLLER/4]=controller; menu_reopen(h);
    h[GRKIT_HOST_MENU_SLOT/4]=BYTE(controller,ADJ_SLOT); h[GRKIT_HOST_MENU_EDGE/4]=edge;
    h[GRKIT_HOST_PREVIEW/4]=0;
    CALL3(FW_LIST_SAVE)(view,(u32)(h+GRKIT_HOST_MENU_LIST/4),(u32)(h+GRKIT_HOST_MENU_LIST/4+1));
    render(controller);
}
static void plugin_touch(u32 controller) {
    if(!private_mode(controller) && ADJ_MOVIE(controller)!=1) enter(controller,3);
}
#ifdef PARAMETER_KINDS
/* Fn adjustments. While GRKIT_HOST_PARAMETERS holds a record, the native
   parameter page reads and writes the adjustments of that record
   (GRKIT_RECORD_PARAMETERS) instead of the native settings, and shows the
   Standard rows for any native type. The page snapshot holds the record's
   values, so a cancel writes them back to the record. Native settings do not
   change. Only the page calls the hooked model getters and setters. */
static const u8 parameter_kinds[]=PARAMETER_KINDS;
_Static_assert(sizeof(parameter_kinds)==9, "GRKIT_RECORD_PARAMETERS holds nine adjustments");
/* The record of a started page, or 0. */
A32_ENTRY u32 menu_parameter_session(void) {
    u32 record=menu_header()[GRKIT_HOST_PARAMETERS/4];
    return record&1U ? 0 : record;
}
/* The native value 0..8 of a stored byte: (native - 4) mod 9. */
static u32 parameter_native(u32 record,u32 index) {
    u32 stored=BYTE(record,GRKIT_RECORD_PARAMETERS+index);
    return stored>8 ? 4 : stored<5 ? stored+4 : stored-5;
}
/* The native value plus 1, or 0 to run the native getter. */
A32_ENTRY u32 menu_parameter_get(u32 index) {
    u32 record=menu_parameter_session();
    return record ? parameter_native(record,index)+1 : 0;
}
/* Nonzero when the value went to the record. */
A32_ENTRY u32 menu_parameter_set(u32 index,u32 value) {
    volatile u32 *h=menu_header();
    u32 record=menu_parameter_session();
    if(!record) return 0;
    if(value<=8) {
        BYTE(record,GRKIT_RECORD_PARAMETERS+index)=(u8)(value>=4 ? value-4 : value+5);
        ++h[GRKIT_HOST_PARAMETER_CHANGES/4];
    }
    return 1;
}
/* A page that GRKit opened starts its session. Any other page ends it. */
A32_ENTRY void menu_parameter_page_start(void) {
    volatile u32 *h=menu_header();
    u32 record=h[GRKIT_HOST_PARAMETERS/4];
    h[GRKIT_HOST_PARAMETERS/4]=record&1U ? record&~1U : 0;
    parameter_title(h)[0]=0;
}
/* Show the Standard rows of the base set. The type is Standard, so the
   native title would read Standard. Keep the page and the effect name, so
   that the text hooks draw the name in the title (menu_parameter_title). */
A32_ENTRY void menu_parameter_page_kinds(u32 page) {
    u32 record=menu_parameter_session();
    volatile u32 *title;
    /* The title view keeps its measured width while its text ID stays the
       same, which clipped a longer effect name. Clear the text, so that the
       title measures the text that the page sets next. */
    CALL3(FW_STRING_VIEW_RESET)(WORD(page,DETAIL_TITLE),0,0);
    if(!record) return;
    BYTE(page,DETAIL_TYPE)=11;
    BYTE(page,DETAIL_SET_KIND)=0;
    BYTE(page,DETAIL_BASE_KIND)=0;
    title=parameter_title(menu_header());
    descriptor(title+2,(const u16 *)(record+GRKIT_RECORD_NAME));
    title[1]=WORD(page,0);
    title[0]=page;
}
/* Replace the snapshot of the Standard adjustments with the record's values. */
A32_ENTRY void menu_parameter_page_snapshot(u32 model) {
    u32 record=menu_parameter_session(),i;
    if(!record) return;
    for(i=0;i<sizeof(parameter_kinds);++i)
        BYTE(model,MODEL_SNAPSHOT+parameter_kinds[i])=(u8)parameter_native(record,i);
}
#endif
/* The text descriptor for `view`: the effect name for the title of a page
   that GRKit started, or 0 for native text. A page whose first word is no
   longer its vtable has ended, and its memory may hold another object. */
static u32 menu_parameter_title(u32 view) {
#ifdef PARAMETER_KINDS
    volatile u32 *title;
    u32 page;
    if(!menu_parameter_session()) return 0;
    title=parameter_title(menu_header());
    page=title[0];
    if(page && WORD(page,0)==title[1] && WORD(page,DETAIL_TITLE)==view) return (u32)(title+2);
#else
    (void)view;
#endif
    return 0;
}
#ifdef SETTINGS_SUBMENU
/* The native settings page of a root row (Module::settings). GRKit opens the
   MENU submenu SETTINGS_SUBMENU from ADJ, as the Fn page opens. While
   GRKIT_HOST_SETTINGS holds the module + 1, the submenu reads a private copy
   of its kind table entry with SETTING_ROWS rows, and the value pages of the
   rows show and set the settings of the module. The text hooks show the
   module title, the row labels, and the values. A value view shows the text
   ID SETTINGS_TITLE_TEXT, so that the text hooks find the view.

   The settings block (GRKIT_TEXT_SETTINGS): the entry, the descriptors of the
   title and of the row labels, then for each value view the view and its
   descriptor: the submenu value of each row, then the value page rows.

   The return to ADJ is a controller change, which ends the page. ADJ
   refreshes also while GRKit opens the page (leave), so its refresh must not
   end the page. */
#ifndef SETTINGS_MAP
#define SETTINGS_MAP(address) ((volatile u32 *)MENU_TEXT(address))
#endif
#define SETTINGS_LABELS 0x80
#define SETTINGS_VIEWS 0x98
#define SETTINGS_VIEW_SLOTS (SETTING_ROWS+SETTING_VALUES)
_Static_assert(SUBMENU_KIND_SIZE<=SETTINGS_LABELS && SETTINGS_LABELS+(1+SETTING_ROWS)*8<=SETTINGS_VIEWS &&
               SETTINGS_VIEWS+SETTINGS_VIEW_SLOTS*12<=GRKIT_TEXT_PARAMETER_TITLE-GRKIT_TEXT_SETTINGS,
               "The settings page fits its block");
static const u32 settings_pages[SETTING_ROWS]=SETTINGS_PAGES;
static const u16 settings_row_texts[SETTING_ROWS]=SETTINGS_ROW_TEXTS;
static volatile u32 *settings_block(volatile u32 *h) { return SETTINGS_MAP(GRKIT_TEXT(h)+GRKIT_TEXT_SETTINGS); }
/* The settings of the open page, or 0. */
static const Setting *settings_page(volatile u32 *h) {
    u32 module=h[GRKIT_HOST_SETTINGS/4]-1;
    return module<ARRAY_COUNT(modules) ? modules[module].settings : 0;
}
/* A controller change. The page ends at any controller outside it. */
A32_ENTRY void menu_settings_controller(u32 id) {
    u32 i;
    if(id==SETTINGS_SUBMENU) return;
    for(i=0;i<SETTING_ROWS;++i) if(id==settings_pages[i]) return;
    menu_header()[GRKIT_HOST_SETTINGS/4]=0;
}
/* The kind table that the submenu reads: the private entry at the index of
   the borrowed entry, or the native table. */
A32_ENTRY u32 menu_settings_kinds(void) {
    volatile u32 *h=menu_header();
    if(!settings_page(h)) return FW_SUBMENU_KINDS;
    return (u32)settings_block(h)-SETTINGS_SUBMENU_KIND*SUBMENU_KIND_SIZE;
}
/* Show `name` in `view` of value slot `slot`. Reset the text ID first: the
   view measures its text only when its ID changes. */
static void settings_view(volatile u32 *h,u32 slot,u32 view,const u16 *name) {
    volatile u32 *at=settings_block(h)+SETTINGS_VIEWS/4+slot*3;
    at[0]=view;
    descriptor(at+1,name);
    CALL3(FW_STRING_VIEW_RESET)(view,0,0);
}
/* The text descriptor of a settings page text, with bit 0 set (the text hooks
   then select the font of the menu language), or 0. */
static u32 settings_text(volatile u32 *h,u32 view,u32 id) {
    volatile u32 *block=settings_block(h);
    u32 i;
    if(!settings_page(h)) return 0;
    if(id==SETTINGS_TITLE_TEXT)
        for(i=0;i<SETTINGS_VIEW_SLOTS;++i)
            if(block[SETTINGS_VIEWS/4+i*3]==view) return (u32)(block+SETTINGS_VIEWS/4+i*3+1)|1U;
    if(id==SETTINGS_TITLE_TEXT) return (u32)(block+SETTINGS_LABELS/4)|1U;
    for(i=0;i<SETTING_ROWS;++i)
        if(id==settings_row_texts[i]) return (u32)(block+SETTINGS_LABELS/4+(i+1)*2)|1U;
    return 0;
}
/* The value method of submenu row `setting`, from the private entry. */
static void settings_row(u32 row,u32 setting) {
    volatile u32 *h=menu_header();
    const Setting *page=settings_page(h);
    if(!page) return;
    settings_view(h,setting,WORD(row,ROW_VALUE_VIEW),page[setting].value_text(h,page[setting].get(h)));
    CALL2(FW_ROW_VALUE)(row,SETTINGS_TITLE_TEXT);
}
A32_ENTRY void menu_settings_value_0(u32 submenu,u32 row) { (void)submenu; settings_row(row,0); }
A32_ENTRY void menu_settings_value_1(u32 submenu,u32 row) { (void)submenu; settings_row(row,1); }
static u32 settings_values(volatile u32 *h,const Setting *setting) {
    u32 n=setting->values(h);
    return n<SETTING_VALUES ? n : SETTING_VALUES;
}
/* The methods of the value page of row `setting`. Each returns 0 for the
   native method, else its result + 1. The row count: */
A32_ENTRY u32 menu_settings_count(u32 unused,u32 unused2,u32 setting) {
    volatile u32 *h=menu_header();
    const Setting *page=settings_page(h);
    (void)unused; (void)unused2;
    return page ? settings_values(h,&page[setting])+1 : 0;
}
/* The text of value row `row`, which shows value `value`: */
A32_ENTRY u32 menu_settings_draw(u32 row,u32 value,u32 setting) {
    volatile u32 *h=menu_header();
    const Setting *page=settings_page(h);
    if(!page) return 0;
    if(value<settings_values(h,&page[setting]))
        settings_view(h,SETTING_ROWS+value,WORD(row,ROW_TEXT_VIEW),page[setting].value_text(h,value));
    CALL2(FW_ROW_TEXT)(row,SETTINGS_TITLE_TEXT);
    return 1;
}
/* OK on value `value`: */
A32_ENTRY u32 menu_settings_apply(u32 value,u32 unused,u32 setting) {
    volatile u32 *h=menu_header();
    const Setting *page=settings_page(h);
    (void)unused;
    if(!page) return 0;
    if(value<settings_values(h,&page[setting])) page[setting].set(h,value);
    return 1;
}
/* Whether value `value` is the stored value: */
A32_ENTRY u32 menu_settings_current(u32 value,u32 unused,u32 setting) {
    volatile u32 *h=menu_header();
    const Setting *page=settings_page(h);
    (void)unused;
    return page ? (page[setting].get(h)==value)+1 : 0;
}
/* Start a page for `module`: copy the borrowed entry with SETTING_ROWS rows
   and the private value methods, and fill the descriptors of the labels. The
   menu language does not change while the page is open. */
static void settings_start(volatile u32 *h,u32 module) {
    static void (*const values[SETTING_ROWS])(u32,u32)={menu_settings_value_0,menu_settings_value_1};
    volatile u32 *block=settings_block(h);
    const Setting *page=modules[module].settings;
    u32 stock=FW_SUBMENU_KINDS+SETTINGS_SUBMENU_KIND*SUBMENU_KIND_SIZE, i;
    for(i=0;i<SUBMENU_KIND_SIZE/4;++i) block[i]=WORD(stock,i*4);
    ((volatile u8 *)block)[SUBMENU_KIND_ROWS]=SETTING_ROWS;
    for(i=0;i<SETTING_ROWS;++i) {
        /* A C++ member pointer: the function, then the adjustment of `this`. */
        block[SUBMENU_KIND_VALUES/4+i*2]=(u32)values[i];
        block[SUBMENU_KIND_VALUES/4+i*2+1]=0;
        descriptor(block+SETTINGS_LABELS/4+(i+1)*2,page[i].label(h));
    }
    descriptor(block+SETTINGS_LABELS/4,modules[module].title(h));
    for(i=0;i<SETTINGS_VIEW_SLOTS*3;++i) block[SETTINGS_VIEWS/4+i]=0;
    h[GRKIT_HOST_SETTINGS/4]=module+1;
}
#endif
/* The text descriptor for `view` and text ID `id`, or 0 for the native text.
   Bit 0 selects the font of the menu language. */
A32_ENTRY u32 menu_text_override(u32 view,u32 id) {
    u32 title=menu_parameter_title(view);
    if(title) return title;
#ifdef SETTINGS_SUBMENU
    return settings_text(menu_header(),view,id);
#else
    (void)id;
    return 0;
#endif
}
/* OK on a root row with settings opens the native settings page where the
   target has one. The page returns to the root (menu_refresh). */
static u32 open_settings(u32 controller) {
#ifdef SETTINGS_SUBMENU
    volatile u32 *h=menu_header();
    u32 module;
    if(h[GRKIT_HOST_MENU_LEVEL/4]!=MENU_ROOT || ADJ_MOVIE(controller)==1) return 0;
    module=root_module(h,h[GRKIT_HOST_MENU_CURSOR/4]);
    if(!modules[module].settings) return 0;
    leave(controller);
    settings_start(h,module);
    h[GRKIT_HOST_RETURN/4]=RETURN_OPENING|RETURN_ROOT;
    CALL3(FW_CHANGE_CONTROLLER)(controller,SETTINGS_SUBMENU,0x2000000);
    return 1;
#else
    (void)controller;
    return 0;
#endif
}
/* Open the native parameter page, as the native ADJ Image Control slot does
   for event 0x1b with argument 2. Refresh the model snapshot first: a cancel
   restores its type persistently. */
static u32 open_parameters(u32 controller) {
    if(!parameters_available(controller)) return 0;
#ifdef FW_SETTINGS_DOMAIN
    /* leave() clears the level, so read it first. */
    u32 root=menu_header()[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT ? RETURN_ROOT : 0;
    CALL1(FW_ADJ_SNAPSHOT)(controller);
#ifdef PARAMETER_KINDS
    /* The page edits the adjustments of the committed effect. Bit 0 marks a
       page that has not started yet. */
    menu_header()[GRKIT_HOST_PARAMETERS/4]=menu_header()[GRKIT_HOST_ACTIVE/4]|1U;
#endif
    leave(controller);
    /* The page returns to ADJ (0x2000000) with the native Image Control slot
       focused, where OK would confirm Standard. Come back to GRKit instead. */
    menu_header()[GRKIT_HOST_RETURN/4]=RETURN_OPENING|root;
    CALL3(FW_CHANGE_CONTROLLER)(controller,IC_PARAMETERS_CONTROLLER,0x2000000);
#endif
    return 1;
}
A32_ENTRY void menu_native_touch(u32 controller,u32 slot) {
    leave(controller);
    if(BYTE(controller,ADJ_SLOT)!=slot) CALL2(FW_ADJ_SELECT_SLOT)(controller,slot);
    native_focus(controller);
}
A32_ENTRY void menu_exit(u32 controller) {
    volatile u32 *h=menu_header();
    u32 in_grkit=private_mode(controller);
    leave(controller);
    h[GRKIT_HOST_BACK_PENDING/4]=0;
    h[GRKIT_HOST_PREVIEW/4]=0;
    h[GRKIT_HOST_FRAME_PREVIEW/4]=0;
    /* The next ADJ returns to the parameter page's GRKit page, opens on
       GRKit, or opens as native ADJ does (exit_return). */
    h[GRKIT_HOST_RETURN/4]=exit_return(h[GRKIT_HOST_RETURN/4],in_grkit);
#ifdef PARAMETER_KINDS
    if(RETURN_STATE(h[GRKIT_HOST_RETURN/4])!=RETURN_PARAMETERS && (h[GRKIT_HOST_PARAMETERS/4]&1U)) h[GRKIT_HOST_PARAMETERS/4]=0;
    parameter_title(h)[0]=0;
#endif
    menu_exit_original(controller);
    if(menu_header()[GRKIT_HOST_TOOLBAR_OWNER/4]==controller) {
        toolbar_visible(menu_header()[GRKIT_HOST_TOOLBAR_WIDGET/4],0);
        toolbar_layout(controller,0);
    }
}
A32_ENTRY u32 menu_destroy(u32 controller) {
    volatile u32 *h=menu_header();
    u32 widget=0, result;
    if(h[GRKIT_HOST_TOOLBAR_OWNER/4]==controller) {
        widget=h[GRKIT_HOST_TOOLBAR_WIDGET/4];
        h[GRKIT_HOST_TOOLBAR_WIDGET/4]=0;
        h[GRKIT_HOST_TOOLBAR_OWNER/4]=0;
        /* The root clears its child vector without deleting its children. */
        CALL2(WORD(WORD(widget,0),0x2c))(widget,0);
        CALL2(FW_VIEW_SET_PARENT)(widget+0xf0,0);
    }
    if(h[GRKIT_HOST_MENU_CONTROLLER/4]==controller) { h[GRKIT_HOST_MENU_CONTROLLER/4]=0; h[GRKIT_HOST_MENU_LEVEL/4]=0; }
    h[GRKIT_HOST_PREVIEW/4]=0;
    h[GRKIT_HOST_FRAME_PREVIEW/4]=0;
    h[GRKIT_HOST_FRAME_PREVIEW_CANVAS/4]=0;
    result=menu_destroy_original(controller);
    if(widget) toolbar_destroy(widget);
    return result;
}
A32_ENTRY void menu_changed(u32 controller) {
    volatile u32 *h=menu_header();
    /* Restoration sets the level to native before rebuilding the list. */
    if(h[GRKIT_HOST_MENU_REBUILDING/4]) return;
    if(h[GRKIT_HOST_MENU_CONTROLLER/4]==controller && h[GRKIT_HOST_MENU_LEVEL/4]) {
        u32 view=WORD(controller,ADJ_LIST);
        u32 next=list_start(h,view)+CALL1(FW_LIST_CURSOR)(view);
        if(next<count(h) && next!=h[GRKIT_HOST_MENU_CURSOR/4]) { h[GRKIT_HOST_MENU_CURSOR/4]=next; highlight(h); render(controller); }
        return;
    }
    menu_changed_original(controller);
}
A32_ENTRY u32 menu_event(u32 controller,const u16 *event) {
    volatile u32 *h=menu_header();
    u32 kind=*event, back;
    h[GRKIT_HOST_MENU_EVENT_RING/4+(h[GRKIT_HOST_MENU_EVENTS/4]&(GRKIT_HOST_MENU_EVENT_RING_WORDS-1))]=kind | ((kind==EV_FN || kind==EV_SWITCH || kind==EV_SWITCH_END) ? ((u32)*(u8 *)WORD((u32)event,4)<<16) :
        kind==EV_TOUCH ? (u32)BYTE(WORD((u32)event,4),0x50)<<16 : 0);
    ++h[GRKIT_HOST_MENU_EVENTS/4];
    if(kind==EV_SWITCH && !menu_press(BYTE(WORD((u32)event,4),0),WORD(WORD((u32)event,4),4))) return 0;
    back=back_event(h,kind,private_mode(controller));
    if(back==2) return 0;
    if(back==1) {
        if(menu_back(h)) { render(controller); return 0; }
        /* MENU on the root closes ADJ, as on a native entry: the restored
           native list gets this switch event and the confirm after it. */
        h[GRKIT_HOST_BACK_PENDING/4]=0;
        leave(controller);
        h[GRKIT_HOST_RETURN/4]=RETURN_CLOSING;
        return menu_event_original(controller,event);
    }
#ifdef EV_FN_ACTION
    /* The Fn press that opened a GRKit parameter page also sends its native
       action. The native page must not open for it. */
    if(kind==EV_FN_ACTION && (RETURN_STATE(h[GRKIT_HOST_RETURN/4])==RETURN_OPENING ||
                              RETURN_STATE(h[GRKIT_HOST_RETURN/4])==RETURN_PARAMETERS)) return 0;
#endif
    if(!private_mode(controller)) {
        /* OK or ADJ press on the native Image Control slot confirms a native value.
           Confirming the existing native type can omit its setter entirely. */
        if(kind==EV_CONFIRM && !after_menu_switch(h) &&
           CALL2(FW_ADJ_SLOT_TYPE)(controller,BYTE(controller,ADJ_SLOT))==ADJ_IC_SLOT_TYPE)
            h[GRKIT_HOST_ACTIVE/4]=0;
        if(ADJ_MOVIE(controller)!=1 &&
           ((kind==EV_RIGHT && BYTE(controller,ADJ_SLOT)==4) ||
            (kind==EV_LEFT && BYTE(controller,ADJ_SLOT)==0))) {
            enter(controller,kind==EV_RIGHT ? 1 : 2); return 0;
        }
        {
            u32 old=BYTE(controller,ADJ_SLOT);
            u32 result=menu_event_original(controller,event);
            u32 next=BYTE(controller,ADJ_SLOT);
            u32 edge=wrapped_edge(kind,old,next,ADJ_MOVIE(controller));
            if(edge) enter(controller,edge);
            else if(next!=old) native_focus(controller);
            return result;
        }
    }
    menu_normalize(h);
    /* On an effect row, commit the highlighted row, then adjust its parameters.
       On the root row, adjust the committed effect. Fn on Off does nothing. */
    if(kind==EV_FN && *(u8 *)WORD((u32)event,4)==FN_PRESS_ARG && parameters_row(h)) {
        if(h[GRKIT_HOST_MENU_LEVEL/4]==MENU_MODULE) current_module(h)->apply(h,h[GRKIT_HOST_MENU_CURSOR/4]);
        if(!open_parameters(controller)) render(controller);
        return 0;
    }
    /* Left and Right move to the next ADJ entry from the root and from a
       page, as on a native entry. MENU returns from a page to the root. */
    if(kind==EV_LEFT || kind==EV_RIGHT) {
        u32 edge=h[GRKIT_HOST_MENU_EDGE/4], advance=advance_from_private(kind,edge);
        leave(controller);
        if(edge==3) {
            u32 i;
            for(i=0;i<5;++i) {
                u32 slot=kind==EV_LEFT ? 4-i : i;
                if(!CALL2(FW_ADJ_SLOT_TYPE)(controller,slot)) continue;
                menu_native_touch(controller,slot);
                if(BYTE(controller,ADJ_SLOT)==slot) break;
            }
            return 0;
        }
        return advance ? menu_event_original(controller,event) : 0;
    }
    if(kind==EV_UP || kind==EV_DIAL_UP || kind==EV_DOWN || kind==EV_DIAL_DOWN) {
        u32 n=count(h);
        if(kind==EV_UP || kind==EV_DIAL_UP) h[GRKIT_HOST_MENU_CURSOR/4]=h[GRKIT_HOST_MENU_CURSOR/4] ? h[GRKIT_HOST_MENU_CURSOR/4]-1 : n-1;
        else { ++h[GRKIT_HOST_MENU_CURSOR/4]; if(h[GRKIT_HOST_MENU_CURSOR/4]==n) h[GRKIT_HOST_MENU_CURSOR/4]=0; }
        highlight(h);
        render_step(controller,kind==EV_UP || kind==EV_DIAL_UP ? (u32)-1 : 1); return 0;
    }
    /* Confirm is 0xd8. Press and release events do not activate a profile.
       OK on a root row opens its page. OK on a page row commits it and closes
       ADJ, as OK on a native value does: the restored native list gets the
       confirm, as after MENU on the root. An ADJ press closes ADJ from the
       root too, as on a native entry; on a page it commits the row first. */
    if(kind==EV_CONFIRM) {
        u32 page=h[GRKIT_HOST_MENU_LEVEL/4]==MENU_MODULE, close=page;
#ifdef EV_ADJ_PRESS
        if(after_adj_press(h)) close=1;
#endif
        if(!close && open_settings(controller)) return 0;
        if(page || !close) menu_confirm(h);
        if(close) {
            leave(controller);
            h[GRKIT_HOST_RETURN/4]=RETURN_CLOSING;
            return menu_event_original(controller,event);
        }
        render(controller); return 0;
    }
    /* Ordinary touch notifications must not dismiss a private page. */
    if(kind==EV_TOUCH && native_action_release(BYTE(WORD((u32)event,4),0x50),BYTE(controller,ADJ_ACTION))) {
        leave(controller);
        return menu_event_original(controller,event);
    }
    return 0;
}
/* Playback keeps a non-native Image Control tag as 0x10000 | (tag - 0x100) in
   metadata key 0x25. Return the private icon for a recorded effect code, or 0. */
static u32 playback_icon(volatile u32 *h,u32 value) {
    u32 code=(value+0x100)&0xffff, i;
    if(value>>16!=1 || !(code&0x8000)) return 0;
    for(i=0;i<h[GRKIT_HOST_COUNT/4];++i)
        if(*(volatile u16 *)(h[GRKIT_HOST_RECORDS/4]+i*GRKIT_RECORD_SIZE+GRKIT_RECORD_CODE)==code)
            return 0x8000+i;
    return 0;
}
#ifdef METADATA_TONE
/* The playback metadata singleton holds key 0x25 at METADATA_TONE. */
u32 menu_playback_private(void) {
    return playback_icon(menu_header(),WORD(CALL0(FW_PLAYBACK_METADATA)(),METADATA_TONE));
}
#endif
/* IDs stay in transient private widgets and never enter native settings. */
A32_ENTRY u32 menu_icon(u32 id) {
    volatile u32 *h=menu_header();
    u32 index=id-0x8000;
    /* Plugin tiles are contiguous from the first one. */
    if(id-PLUGIN_ICON<PLUGIN_TILES)
        return h[GRKIT_HOST_TILE_ICON/4]+(id-PLUGIN_ICON)*GRKIT_ICON_STRIDE;
    if(id==GRKIT_ICON_GRKIT) return h[GRKIT_HOST_PLUGINS_ICON/4];
    /* The installer puts the Version icon after the GRKit icon. */
    if(id==GRKIT_ICON_VERSION) return h[GRKIT_HOST_PLUGINS_ICON/4] ? h[GRKIT_HOST_PLUGINS_ICON/4]+GRKIT_ICON_STRIDE : 0;
    if(id==0x7fff) return h[GRKIT_HOST_OFF_ICON/4] ? h[GRKIT_HOST_OFF_ICON/4] : NATIVE_OFF_ICON();
    if(index>=h[GRKIT_HOST_COUNT/4]) return 0;
    return WORD(h[GRKIT_HOST_RECORDS/4]+index*GRKIT_RECORD_SIZE,GRKIT_RECORD_ICON);
}

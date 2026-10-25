#ifndef GRKIT_MODULES_H
#define GRKIT_MODULES_H
/* The menu modules, in root order: GRKIT_MODULE_SLOTS slots for the pages of
   modules (GRKIT_HOST_MODULE_PAGES), then the core Modules row
   (runtime/manage.h) and the core About row (runtime/about.h). */
#include "about.h"
#include "settings.h"
#include "effects.h"
#include "tone.h"

#define MODULE_PAGE(slot) (slot)
#define MANAGE_MODULE MODULE_PAGE(GRKIT_MODULE_SLOTS)
/* The Effects page opens from the last row of a page of CORE.TONE. It has
   no root row. */
#define EFFECTS_MODULE (MANAGE_MODULE+1U)
#define ABOUT_MODULE (EFFECTS_MODULE+1U)
#define MODULE_TOTAL (ABOUT_MODULE+1U)

static const Module about_module = ABOUT_PAGE;

/* ---- Module pages ----------------------------------------------------------
   One generic choice page serves each module page (runtime/sdk/grkit.h).
   module_at() selects the slot in GRKIT_HOST_MODULE_CHOICE before it returns
   the page, so get a page again after you get another one. */
#define CHOICE_ENTRY(h) ((h)+GRKIT_HOST_MODULE_PAGES/4+(h)[GRKIT_HOST_MODULE_CHOICE/4]*3)
/* GrkitPage fields, and GRKIT_CHOICE_OFF_ROW. */
#define PAGE_SETTING 8U
#define PAGE_FLAGS 12U
#define PAGE_TITLE 16U
#define PAGE_SUMMARIES 20U
#define PAGE_LABELS 24U
#define PAGE_SETTING_ROWS 32U
#define PAGE_SETTING_LIST 36U
#define CHOICE_OFF_ROW 1U
#define CHOICE_MENU_ROW 4U
/* GrkitSettingRow fields. */
#define ROW_SETTING 4U
#define ROW_LABEL 8U
#define ROW_VALUES 12U
#define ROW_SIZE 16U
/* A word of a module page in the module code. A host test maps it. */
#ifndef PAGE_WORD
#define PAGE_WORD(p,o) WORD(p,o)
#endif
#include "manage.h"
static const Module manage_module_page = MANAGE_PAGE;
static const Module effects_module_page = EFFECTS_PAGE;
static u32 module_pages(volatile u32 *h) {
    u32 n=h[GRKIT_HOST_MODULE_PAGE_COUNT/4];
    return n<=GRKIT_MODULE_SLOTS ? n : 0;
}
static u32 choice_value(volatile u32 *h) { return setting_value(h,CHOICE_ENTRY(h)[1]); }
/* The rows of a choice page are the values that the camera offers, in the
   order of the setting (runtime/settings.h: setting_rows). */
static u32 choice_rows(volatile u32 *h) { return setting_rows(h,CHOICE_ENTRY(h)[1]); }
static u32 choice_row_value(volatile u32 *h,u32 row) { return setting_row_value(h,CHOICE_ENTRY(h)[1],row); }
static u32 choice_value_row(volatile u32 *h,u32 value) { return setting_value_row(h,CHOICE_ENTRY(h)[1],value); }
/* Text `index` of a text list of the page, in the menu language. */
static const u16 *choice_text(volatile u32 *h,u32 field,u32 index) {
    return MENU_TEXT(PAGE_WORD(PAGE_WORD(CHOICE_ENTRY(h)[0],field),(index*GRKIT_TEXT_LANGUAGES+menu_language())*4));
}
static u32 choice_tile(volatile u32 *h,u32 n) { return GRKIT_ICON_MODULE_TILE(CHOICE_ENTRY(h)[2]+n); }
static const u16 *choice_title(volatile u32 *h) { return choice_text(h,PAGE_TITLE,0); }
static const u16 *choice_summary(volatile u32 *h) { return choice_text(h,PAGE_SUMMARIES,choice_value(h)); }
static const u16 *choice_label(volatile u32 *h,u32 row) { return choice_text(h,PAGE_LABELS,choice_row_value(h,row)); }
/* The root row shows the tile of the value; a row shows the tile of its
   value, or Off for value 0. */
static u32 choice_icon(volatile u32 *h,u32 row) {
    u32 value;
    if(h[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT) return choice_tile(h,choice_value(h));
    value=choice_row_value(h,row);
    return !value && PAGE_WORD(CHOICE_ENTRY(h)[0],PAGE_FLAGS)&CHOICE_OFF_ROW ? GRKIT_ICON_OFF : choice_tile(h,value);
}
static u32 choice_checked(volatile u32 *h,u32 row) { return choice_row_value(h,row)==choice_value(h); }
static void choice_apply(volatile u32 *h,u32 row) { setting_store(h,CHOICE_ENTRY(h)[1],choice_row_value(h,row)); }
static u32 choice_committed(volatile u32 *h) { return choice_value_row(h,choice_value(h)); }
static const Module choice_module={choice_title,choice_summary,choice_rows,choice_label,choice_icon,
    choice_checked,choice_apply,choice_committed,0,0,0};

/* The rows of the native settings page of a module page. Each row is a
   setting of the module, so its handle follows from the handle of the page
   setting, which INSTALL gives. */
static u32 choice_row(volatile u32 *h,u32 row) {
    return PAGE_WORD(CHOICE_ENTRY(h)[0],PAGE_SETTING_LIST)+row*ROW_SIZE;
}
static u32 choice_row_handle(volatile u32 *h,u32 row) {
    return CHOICE_ENTRY(h)[1]-PAGE_WORD(CHOICE_ENTRY(h)[0],PAGE_SETTING)+PAGE_WORD(choice_row(h,row),ROW_SETTING);
}
static const u16 *choice_row_text(volatile u32 *h,u32 row,u32 field,u32 index) {
    return MENU_TEXT(PAGE_WORD(PAGE_WORD(choice_row(h,row),field),(index*GRKIT_TEXT_LANGUAGES+menu_language())*4));
}
#define CHOICE_SETTING(row) \
    static const u16 *choice_label_##row(volatile u32 *h) { return choice_row_text(h,row,ROW_LABEL,0); } \
    static u32 choice_values_##row(volatile u32 *h) { return setting_limit(h,choice_row_handle(h,row)); } \
    static const u16 *choice_value_##row(volatile u32 *h,u32 value) { return choice_row_text(h,row,ROW_VALUES,value); } \
    static u32 choice_get_##row(volatile u32 *h) { return setting_value(h,choice_row_handle(h,row)); } \
    static void choice_set_##row(volatile u32 *h,u32 value) { setting_store(h,choice_row_handle(h,row),value); }
CHOICE_SETTING(0)
CHOICE_SETTING(1)
CHOICE_SETTING(2)
CHOICE_SETTING(3)
CHOICE_SETTING(4)
static const Setting choice_settings[SETTING_ROWS]={
    {choice_label_0,choice_values_0,choice_value_0,choice_get_0,choice_set_0},
    {choice_label_1,choice_values_1,choice_value_1,choice_get_1,choice_set_1},
    {choice_label_2,choice_values_2,choice_value_2,choice_get_2,choice_set_2},
    {choice_label_3,choice_values_3,choice_value_3,choice_get_3,choice_set_3},
    {choice_label_4,choice_values_4,choice_value_4,choice_get_4,choice_set_4},
};
static const Module choice_settings_module={choice_title,choice_summary,choice_rows,choice_label,choice_icon,
    choice_checked,choice_apply,choice_committed,0,0,choice_settings};

/* A page of CORE.TONE gets its rows from the core (runtime/tone.h), and its
   title from the module. */
static const Module tone_module={choice_title,tone_summary,tone_rows,tone_label,tone_icon,
    tone_checked,tone_apply,tone_value,tone_preview,tone_parameters,0};

/* The page of module `index`. */
static const Module *module_at(volatile u32 *h,u32 index) {
    if(index>=ABOUT_MODULE) return &about_module;
    if(index==MANAGE_MODULE) return &manage_module_page;
    if(index==EFFECTS_MODULE) return &effects_module_page;
    h[GRKIT_HOST_MODULE_CHOICE/4]=index;
    /* Only a registered page has a GrkitPage to read. */
    if(index>=module_pages(h)) return &choice_module;
    if(CHOICE_ENTRY(h)[1]==GRKIT_CORE_TONE) return &tone_module;
    if(PAGE_WORD(CHOICE_ENTRY(h)[0],PAGE_SETTING_ROWS)) return &choice_settings_module;
    return &choice_module;
}

/* The global icon ID of a page's icon result. Module pages give global IDs. */
static u32 module_icon(u32 module,u32 icon) { (void)module; return icon; }
/* Whether a module layer paints its committed value on the ADJ canvas. */
static u32 module_layers_on(volatile u32 *h) {
    u32 i, n=h[GRKIT_HOST_MODULE_LAYER_COUNT/4];
    for(i=0;i<n && i<GRKIT_MODULE_SLOTS;++i) {
        u32 handle=h[GRKIT_HOST_MODULE_LAYERS/4+i*3+1];
        if(handle==0xffffffffU || setting_value(h,handle)) return 1;
    }
    return 0;
}

/* ADJ covers the live-view drawing, so ADJ draws the module layers on its
   own canvas. A module page shows its highlighted value with the committed
   values of the others. The page of CORE.ASPECT shows the area of its
   highlighted ratio, and other pages show the crop that the live view draws.
   Off ends the drawing, and the canvas stays until a full draw has erased
   it. */
static void frame_preview(volatile u32 *h,u32 canvas,u32 movie) {
    u32 module=h[GRKIT_HOST_MENU_LEVEL/4]==MENU_MODULE ? h[GRKIT_HOST_MODULE/4] : 0;
    u32 slot=module, choice=h[GRKIT_HOST_MENU_LEVEL/4]==MENU_MODULE && slot<module_pages(h);
    u32 ratio=framing_crop(h), handle, value;
    h[GRKIT_HOST_MODULE_PREVIEW/4]=0;
    if(movie) { h[GRKIT_HOST_FRAME_PREVIEW/4]=0; return; }
    if(choice) {
        module_at(h,module);
        handle=CHOICE_ENTRY(h)[1];
        value=choice_row_value(h,h[GRKIT_HOST_MENU_CURSOR/4]);
        h[GRKIT_HOST_MODULE_PREVIEW/4]=handle+1;
        h[GRKIT_HOST_MODULE_PREVIEW/4+1]=value;
        if(handle==GRKIT_CORE_ASPECT) ratio=value;
    }
    /* A highlighted Off keeps the canvas for the next row. */
    h[GRKIT_HOST_FRAME_PREVIEW/4]=ratio || choice || module_layers_on(h) ? ratio+1 : 0;
    h[GRKIT_HOST_FRAME_PREVIEW_CANVAS/4]=canvas;
}

/* Visible modules as a bit mask in root order: each module page with two
   rows or more, Modules with a listed module, then About, which always
   shows. A page of CORE.TONE has rows only with effects. Without a listed
   module, the installer installs no menu hook, so only an inconsistent
   header shows About alone. */
static u32 visible_modules(volatile u32 *h) {
    u32 choice=h[GRKIT_HOST_MODULE_CHOICE/4], mask=1U<<ABOUT_MODULE, slot;
    for(slot=0;slot<module_pages(h);++slot)
        if(module_at(h,MODULE_PAGE(slot))->rows(h)>=2) mask|=1U<<MODULE_PAGE(slot);
    if(manage_count(h)) mask|=1U<<MANAGE_MODULE;
    /* module_at selects a page: keep the page of the caller. */
    h[GRKIT_HOST_MODULE_CHOICE/4]=choice;
    return mask;
}
/* The module of a root row, or of the last row for a row past the end. */
static u32 root_module(volatile u32 *h,u32 row) {
    u32 mask=visible_modules(h), module=0, last=0;
    for(;module<MODULE_TOTAL;++module) {
        if(!(mask>>module&1)) continue;
        if(!row--) return module;
        last=module;
    }
    return last;
}
static u32 root_row(volatile u32 *h,u32 module) {
    u32 mask=visible_modules(h), row=0, i;
    for(i=0;i<module && i<MODULE_TOTAL;++i) row+=mask>>i&1;
    return row;
}
static u32 root_count(volatile u32 *h) { return root_row(h,MODULE_TOTAL); }
/* The module of the current row: the row's module at the root, else the open page. */
static u32 current_index(volatile u32 *h) {
    return h[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT ? root_module(h,h[GRKIT_HOST_MENU_CURSOR/4]) : h[GRKIT_HOST_MODULE/4];
}
static const Module *current_module(volatile u32 *h) { return module_at(h,current_index(h)); }
static u32 count(volatile u32 *h) {
    return h[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT ? root_count(h) : current_module(h)->rows(h);
}
/* The page learns that its highlighted row changed, so the page of CORE.TONE
   can preview the row in the live view. */
static void highlight(volatile u32 *h) {
    const Module *module;
    if(h[GRKIT_HOST_MENU_LEVEL/4]!=MENU_MODULE || h[GRKIT_HOST_MODULE/4]>=MODULE_TOTAL) return;
    module=module_at(h,h[GRKIT_HOST_MODULE/4]);
    if(module->highlight) module->highlight(h);
}
/* Fn applies to the highlighted row of a page that has native parameters,
   and to the root row of such a page for its committed row. */
static u32 parameters_row(volatile u32 *h) {
    const Module *module;
    u32 row=h[GRKIT_HOST_MENU_CURSOR/4];
    if(h[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT) {
        module=module_at(h,root_module(h,row));
        row=module->committed(h);
    }
    else if(h[GRKIT_HOST_MENU_LEVEL/4]!=MENU_MODULE || h[GRKIT_HOST_MODULE/4]>=MODULE_TOTAL) return 0;
    else module=module_at(h,h[GRKIT_HOST_MODULE/4]);
    return module->parameters && module->parameters(h,row);
}
/* Each module opens at its committed row. Image Control previews from there. */
static void menu_open(volatile u32 *h,u32 module) {
    h[GRKIT_HOST_MODULE/4]=module;
    h[GRKIT_HOST_MENU_LEVEL/4]=MENU_MODULE;
    h[GRKIT_HOST_MENU_CURSOR/4]=module_at(h,module)->committed(h);
}
/* ADJ opens at the root. About always shows there, so the root has a row even
   when no module page is visible. */
static void menu_start(volatile u32 *h) {
    h[GRKIT_HOST_MENU_LEVEL/4]=MENU_ROOT;
    h[GRKIT_HOST_MENU_CURSOR/4]=0;
    h[GRKIT_HOST_MODULE/4]=0;
}
/* Each ADJ session opens at the root, on the row of the module that was
   highlighted or open when the last session ended. */
static void menu_normalize(volatile u32 *h);
static void menu_reopen(volatile u32 *h) {
    u32 module=h[GRKIT_HOST_MODULE/4];
    h[GRKIT_HOST_MENU_LEVEL/4]=MENU_ROOT;
    h[GRKIT_HOST_MENU_CURSOR/4]=module<MODULE_TOTAL && visible_modules(h)>>module&1 ? root_row(h,module) : 0;
    menu_normalize(h);
}
/* Whether a page can be open: a visible module, or the Effects page with an
   effect. */
static u32 page_reachable(volatile u32 *h,u32 module) {
    return module==EFFECTS_MODULE ? effects_count(h)!=0 : visible_modules(h)>>module&1;
}
static void menu_normalize(volatile u32 *h) {
    u32 module=h[GRKIT_HOST_MODULE/4];
    if(h[GRKIT_HOST_MENU_LEVEL/4]<MENU_ROOT || h[GRKIT_HOST_MENU_LEVEL/4]>MENU_MODULE || module>=MODULE_TOTAL ||
       (h[GRKIT_HOST_MENU_LEVEL/4]==MENU_MODULE && (!page_reachable(h,module) || !module_at(h,module)->rows(h))))
        menu_start(h);
    if(h[GRKIT_HOST_MENU_CURSOR/4]>=count(h)) h[GRKIT_HOST_MENU_CURSOR/4]=0;
}
/* The page slot of CORE.TONE, or GRKIT_MODULE_SLOTS without one. */
static u32 tone_slot(volatile u32 *h) {
    u32 slot, choice=h[GRKIT_HOST_MODULE_CHOICE/4];
    for(slot=0;slot<module_pages(h);++slot) {
        h[GRKIT_HOST_MODULE_CHOICE/4]=slot;
        if(CHOICE_ENTRY(h)[1]==GRKIT_CORE_TONE) break;
    }
    h[GRKIT_HOST_MODULE_CHOICE/4]=choice;
    return slot<module_pages(h) ? slot : GRKIT_MODULE_SLOTS;
}
/* Whether the cursor is on the Effects row of the open page. */
static u32 effects_row_open(volatile u32 *h) {
    u32 module=h[GRKIT_HOST_MODULE/4];
    return h[GRKIT_HOST_MENU_LEVEL/4]==MENU_MODULE && module<module_pages(h) && module_at(h,module)==&tone_module &&
           tone_effects_row(h,h[GRKIT_HOST_MENU_CURSOR/4]);
}
/* OK on a root row opens its page. A row without a page, such as About, does nothing. */
static void menu_confirm(volatile u32 *h) {
    menu_normalize(h);
    if(h[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT) {
        u32 module=root_module(h,h[GRKIT_HOST_MENU_CURSOR/4]);
        if(module_at(h,module)->rows(h)) menu_open(h,module);
    }
    else current_module(h)->apply(h,h[GRKIT_HOST_MENU_CURSOR/4]);
}
/* Return false at the top level so the caller can restore the stock ADJ page. */
static u32 menu_back(volatile u32 *h) {
    /* Leaving a level without OK restores the committed effect in the live view. */
    h[GRKIT_HOST_PREVIEW/4]=0;
    menu_normalize(h);
    if(h[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT) return 0;
    /* The Effects page returns to its row of the page of CORE.TONE. */
    if(h[GRKIT_HOST_MODULE/4]==EFFECTS_MODULE && tone_slot(h)<GRKIT_MODULE_SLOTS) {
        menu_open(h,tone_slot(h));
        h[GRKIT_HOST_MENU_CURSOR/4]=tone_values(h);
        return 1;
    }
    h[GRKIT_HOST_MENU_LEVEL/4]=MENU_ROOT;
    h[GRKIT_HOST_MENU_CURSOR/4]=root_row(h,h[GRKIT_HOST_MODULE/4]);
    return 1;
}
#endif

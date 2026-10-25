#ifndef GRKIT_MANAGE_H
#define GRKIT_MANAGE_H
/* The Modules page: the root row before About lists each module file of
   GRKIT/MODULES, also one that did not load. OK on a module opens a native
   settings page with two rows: Load (On or Off) and Position (its place in
   the GRKit root). A change applies at the next start, and the root row says
   so until then.

   Two settings of the settings table keep the choices of each listed module
   (runtime/settings.h): CORE.OFF.<ID>, 1 for Off, and CORE.POS.<ID>, its
   position + 1, or 0 for the default place. The resident loader saves them
   as other settings, and SCAN reads them to select and order the modules
   (runtime/installer.c).

   INSTALL writes the module list (GRKIT_HOST_MANAGE): MANAGE_HEADER words,
   then MANAGE_STRIDE bytes for each listed module, in the order of the last
   start. The includer defines u16, u32, and the header fields. */
#define MANAGE_COUNT 0U
/* The handle of CORE.OFF of module 0. Module i has CORE.OFF at handle
   base + 2i and CORE.POS at base + 2i + 1. */
#define MANAGE_SETTINGS 4U
#define MANAGE_FLAGS 8U
#define MANAGE_SAFE_MODE 1U
/* The listed module whose native settings page the Modules page opened. */
#define MANAGE_OPEN 12U
#define MANAGE_HEADER 16U
/* Each module: its status at this start, the address of the title texts of
   its page or 0, the global index of its tile 0 or MANAGE_NO_TILE, then its
   name: the file name before .GRM, with the error status after a module that
   did not link. */
#define MANAGE_STATUS 0U
#define MANAGE_TITLE 4U
#define MANAGE_TILE 8U
#define MANAGE_NAME 12U
#define MANAGE_NAME_UNITS 16U
#define MANAGE_STRIDE (MANAGE_NAME + 2U * MANAGE_NAME_UNITS)
#define MANAGE_NO_TILE 0xffffffffU
/* The status of a module that loaded, of one that safe mode kept out, and of
   one that is Off. Other values are the errors of runtime/module_link.h. */
#define MANAGE_LOADED 0U
#define MANAGE_SAFE 1U
#define MANAGE_OFF 2U
/* The most listed modules (MODULES_MAX of the installer). */
#define MANAGE_MODULES 8U
/* A position without a CORE.POS value sorts after each position. */
#define MANAGE_DEFAULT_KEY 1000U

#ifdef MENU_TEXT
/* The module list at a camera address. A host test maps it. */
#ifndef MANAGE_MAP
#define MANAGE_MAP(address) ((volatile u32 *)(address))
#endif

static u32 manage_count(volatile u32 *h) {
    u32 list=h[GRKIT_HOST_MANAGE/4];
    u32 n=list ? MANAGE_MAP(list+MANAGE_COUNT)[0] : 0;
    return n<=MANAGE_MODULES ? n : 0;
}
static u32 manage_word(volatile u32 *h,u32 offset) { return MANAGE_MAP(h[GRKIT_HOST_MANAGE/4]+offset)[0]; }
static u32 manage_entry(volatile u32 *h,u32 module) { return h[GRKIT_HOST_MANAGE/4]+MANAGE_HEADER+module*MANAGE_STRIDE; }
static u32 manage_off_handle(volatile u32 *h,u32 module) { return manage_word(h,MANAGE_SETTINGS)+2*module; }
static u32 manage_off(volatile u32 *h,u32 module) { return setting_value(h,manage_off_handle(h,module)); }
/* The sort key of a module: its CORE.POS, or the default place after them. */
static u32 manage_key(volatile u32 *h,u32 module) {
    u32 position=setting_value(h,manage_off_handle(h,module)+1);
    return position ? position : MANAGE_DEFAULT_KEY+module;
}
/* The module of a row: the modules in the order of their keys. Equal keys
   keep the order of this start. */
static u32 manage_module(volatile u32 *h,u32 row) {
    u32 n=manage_count(h), i, j, before;
    for(i=0;i<n;++i) {
        before=0;
        for(j=0;j<n;++j) {
            u32 a=manage_key(h,j), b=manage_key(h,i);
            if(a<b || (a==b && j<i)) ++before;
        }
        if(before==row) return i;
    }
    return 0;
}
static u32 manage_row(volatile u32 *h,u32 module) {
    u32 row;
    for(row=0;row<manage_count(h);++row) if(manage_module(h,row)==module) return row;
    return 0;
}
/* Whether the choices differ from this start: a module turns on or off, or
   the order changes. */
static u32 manage_changed(volatile u32 *h) {
    u32 i;
    for(i=0;i<manage_count(h);++i) {
        u32 status=MANAGE_MAP(manage_entry(h,i)+MANAGE_STATUS)[0];
        if(manage_off(h,i)!=(status==MANAGE_OFF) || manage_module(h,i)!=i) return 1;
    }
    return 0;
}
/* The name of a module: the title of its page in the menu language, else
   its file name. */
static const u16 *manage_name(volatile u32 *h,u32 module) {
    u32 entry=manage_entry(h,module), title=MANAGE_MAP(entry+MANAGE_TITLE)[0];
    if(title) return MENU_TEXT(PAGE_WORD(title,menu_language()*4));
    return (const u16 *)MANAGE_MAP(entry+MANAGE_NAME);
}

static const u16 *manage_title(volatile u32 *h) { return text(h,TEXT_MODULES); }
static const u16 *manage_summary(volatile u32 *h) {
    if(manage_word(h,MANAGE_FLAGS)&MANAGE_SAFE_MODE) return text(h,TEXT_MODULES_SUMMARY+2);
    return text(h,TEXT_MODULES_SUMMARY+manage_changed(h));
}
static u32 manage_rows(volatile u32 *h) { return manage_count(h); }
static const u16 *manage_label(volatile u32 *h,u32 row) { return manage_name(h,manage_module(h,row)); }
/* A module that is Off shows the Off icon, and a loaded module its tile. */
static u32 manage_icon(volatile u32 *h,u32 row) {
    u32 module, tile;
    if(h[GRKIT_HOST_MENU_LEVEL/4]==MENU_ROOT) return GRKIT_ICON_MODULES;
    module=manage_module(h,row);
    if(manage_off(h,module)) return GRKIT_ICON_OFF;
    tile=MANAGE_MAP(manage_entry(h,module)+MANAGE_TILE)[0];
    return tile!=MANAGE_NO_TILE ? GRKIT_ICON_MODULE_TILE(tile) : GRKIT_ICON_MODULES;
}
static u32 manage_checked(volatile u32 *h,u32 row) { return !manage_off(h,manage_module(h,row)); }
/* OK opens the settings of the row (runtime/menu.c: open_settings). */
static void manage_apply(volatile u32 *h,u32 row) { (void)h; (void)row; }
/* The page opens at the module of the last settings page. */
static u32 manage_committed(volatile u32 *h) {
    u32 module=manage_word(h,MANAGE_OPEN);
    return module<manage_count(h) ? manage_row(h,module) : 0;
}

/* The settings page of module MANAGE_OPEN. */
static u32 manage_open(volatile u32 *h) {
    u32 module=manage_word(h,MANAGE_OPEN);
    return module<manage_count(h) ? module : 0;
}
static const u16 *manage_load_label(volatile u32 *h) { return text(h,TEXT_LOAD); }
static u32 manage_load_values(volatile u32 *h) { (void)h; return 2; }
static const u16 *manage_load_text(volatile u32 *h,u32 value) { return text(h,value ? TEXT_OFF : TEXT_ON); }
static u32 manage_load_get(volatile u32 *h) { return manage_off(h,manage_open(h)); }
static void manage_load_set(volatile u32 *h,u32 value) { setting_store(h,manage_off_handle(h,manage_open(h)),value); }
static const u16 *manage_position_label(volatile u32 *h) { return text(h,TEXT_POSITION); }
static u32 manage_position_values(volatile u32 *h) { return manage_count(h); }
static const u16 manage_digits[MANAGE_MODULES][2]={{'1',0},{'2',0},{'3',0},{'4',0},{'5',0},{'6',0},{'7',0},{'8',0}};
static const u16 *manage_position_text(volatile u32 *h,u32 value) {
    (void)h;
    return manage_digits[value<MANAGE_MODULES ? value : 0];
}
static u32 manage_position_get(volatile u32 *h) { return manage_row(h,manage_open(h)); }
/* Move the module to row `value`: each module gets its row + 1 as CORE.POS. */
static void manage_position_set(volatile u32 *h,u32 value) {
    u32 n=manage_count(h), order[MANAGE_MODULES], module=manage_open(h), row, at=0;
    if(value>=n) return;
    for(row=0;row<n;++row) {
        u32 next=manage_module(h,row);
        if(next!=module) order[at++]=next;
    }
    for(row=n-1;row>value;--row) order[row]=order[row-1];
    order[value]=module;
    for(row=0;row<n;++row) setting_store(h,manage_off_handle(h,order[row])+1,row+1);
}
static const Setting manage_settings[SETTING_ROWS]={
    {manage_load_label,manage_load_values,manage_load_text,manage_load_get,manage_load_set},
    {manage_position_label,manage_position_values,manage_position_text,manage_position_get,manage_position_set},
};
#define MANAGE_PAGE {manage_title,manage_summary,manage_rows,manage_label,manage_icon, \
    manage_checked,manage_apply,manage_committed,0,0,manage_settings}
#endif
#endif

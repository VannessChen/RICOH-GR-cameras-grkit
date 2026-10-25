#ifndef GRKIT_EFFECTS_H
#define GRKIT_EFFECTS_H
/* The Effects page: the last row of a page of CORE.TONE, such as that of
   Image Control, opens a list of each effect file of GRKIT/EFFECTS, also one
   that did not install. OK on an effect opens a native settings page with
   two rows: Load (On or Off) and Order (Move up or Move down). A change
   applies at the next start, and the last row says so until then.

   One setting of the settings table keeps the choice of each listed effect
   (runtime/settings.h): CORE.EFFECT.<effect ID>, with Off in bit 0, and its
   position + 1, or 0 for the default place, above. The resident loader saves
   it as other settings, and SCAN reads it to select and order the effects
   (runtime/installer.c). GRKit installs at most MAX_COUNT effects that are
   not Off, in that order.

   INSTALL writes the effect list (GRKIT_HOST_EFFECTS): EFFECTS_HEADER bytes,
   then EFFECTS_STRIDE bytes for each listed effect, in the order of the last
   start. The includer defines u16, u32, and the header fields. */
/* The most effect files that GRKit lists, those with the smallest IDs. */
#define EFFECTS_LISTED 64U
#define EFFECTS_COUNT 0U
/* The handle of the choice of effect 0. Effect i has handle base + i. */
#define EFFECTS_SETTINGS 4U
/* The effect whose settings page is open. */
#define EFFECTS_OPEN 8U
#define EFFECTS_HEADER 12U
/* Each effect: its status at this start, its record index or EFFECTS_NO_RECORD,
   then its name, NUL-terminated. */
#define EFFECTS_STATUS 0U
#define EFFECTS_RECORD 4U
#define EFFECTS_NAME 8U
#define EFFECTS_NAME_UNITS 18U
#define EFFECTS_STRIDE (EFFECTS_NAME + 2U * EFFECTS_NAME_UNITS)
#define EFFECTS_NO_RECORD 0xffffffffU
/* The status of an effect that installed, of one that is Off, of one past
   MAX_COUNT, and of one whose file changed during the start. */
#define EFFECT_LOADED 0U
#define EFFECT_OFF 2U
#define EFFECT_OVER 3U
#define EFFECT_UNREADABLE 4U
/* A choice: Off in bit 0, the position + 1 above. */
#define EFFECT_CHOICE_VALUES (2U * (EFFECTS_LISTED + 1U))
#define EFFECTS_DEFAULT_KEY 1000U

#ifdef MENU_TEXT
#ifndef MANAGE_MAP
#define MANAGE_MAP(address) ((volatile u32 *)(address))
#endif
static u32 effects_count(volatile u32 *h) {
    u32 list=h[GRKIT_HOST_EFFECTS/4];
    u32 n=list ? MANAGE_MAP(list+EFFECTS_COUNT)[0] : 0;
    return n<=EFFECTS_LISTED ? n : 0;
}
static volatile u32 *effects_word(volatile u32 *h,u32 offset) { return MANAGE_MAP(h[GRKIT_HOST_EFFECTS/4]+offset); }
static u32 effects_entry(volatile u32 *h,u32 effect) { return h[GRKIT_HOST_EFFECTS/4]+EFFECTS_HEADER+effect*EFFECTS_STRIDE; }
static u32 effects_handle(volatile u32 *h,u32 effect) { return effects_word(h,EFFECTS_SETTINGS)[0]+effect; }
static u32 effects_choice(volatile u32 *h,u32 effect) { return setting_value(h,effects_handle(h,effect)); }
static u32 effects_off(volatile u32 *h,u32 effect) { return effects_choice(h,effect)&1U; }
static u32 effects_key(volatile u32 *h,u32 effect) {
    u32 position=effects_choice(h,effect)>>1;
    return position ? position : EFFECTS_DEFAULT_KEY+effect;
}
/* The effects in the order of their keys. Equal keys keep the order of this
   start. An insertion sort: the list is short, and the menu sorts only on a
   key press. */
/* u8 and u16 keep the frames of the menu small. */
static void effects_order(volatile u32 *h,u8 *order,u32 n) {
    u16 keys[EFFECTS_LISTED];
    u32 i, j;
    for(i=0;i<n;++i) {
        u32 key=effects_key(h,i);
        for(j=i;j && keys[j-1]>key;--j) { keys[j]=keys[j-1]; order[j]=order[j-1]; }
        keys[j]=(u16)key; order[j]=(u8)i;
    }
}
/* The effect of a row. */
static u32 effects_at(volatile u32 *h,u32 row) {
    u8 order[EFFECTS_LISTED];
    u32 n=effects_count(h);
    effects_order(h,order,n);
    return row<n ? order[row] : 0;
}
static u32 effects_row(volatile u32 *h,u32 effect) {
    u8 order[EFFECTS_LISTED];
    u32 n=effects_count(h), row;
    effects_order(h,order,n);
    for(row=0;row<n;++row) if(order[row]==effect) return row;
    return 0;
}
/* Whether the choices differ from this start. */
static u32 effects_changed(volatile u32 *h) {
    u8 order[EFFECTS_LISTED];
    u32 n=effects_count(h), i;
    effects_order(h,order,n);
    for(i=0;i<n;++i) {
        u32 status=MANAGE_MAP(effects_entry(h,i)+EFFECTS_STATUS)[0];
        if(effects_off(h,i)!=(status==EFFECT_OFF) || order[i]!=i) return 1;
    }
    return 0;
}
static const u16 *effects_name(volatile u32 *h,u32 effect) {
    return (const u16 *)MANAGE_MAP(effects_entry(h,effect)+EFFECTS_NAME);
}
/* The label of the last row of a page of CORE.TONE. */
static const u16 *effects_row_label(volatile u32 *h) { return text(h,TEXT_EFFECTS_SUMMARY+effects_changed(h)); }

static const u16 *effects_title(volatile u32 *h) { return text(h,TEXT_EFFECTS); }
static u32 effects_rows(volatile u32 *h) { return effects_count(h); }
static const u16 *effects_label(volatile u32 *h,u32 row) { return effects_name(h,effects_at(h,row)); }
/* An effect that is Off shows the Off icon, an installed effect its icon,
   and one that did not install the Modules icon. */
static u32 effects_icon(volatile u32 *h,u32 row) {
    u32 effect=effects_at(h,row), record;
    if(effects_off(h,effect)) return GRKIT_ICON_OFF;
    record=MANAGE_MAP(effects_entry(h,effect)+EFFECTS_RECORD)[0];
    return record<h[GRKIT_HOST_COUNT/4] ? GRKIT_ICON_EFFECT(record) : GRKIT_ICON_MODULES;
}
static u32 effects_checked(volatile u32 *h,u32 row) { return !effects_off(h,effects_at(h,row)); }
/* OK opens the settings of the row (runtime/menu.c: open_settings). */
static void effects_apply(volatile u32 *h,u32 row) { (void)h; (void)row; }
static u32 effects_open(volatile u32 *h) {
    u32 effect=effects_word(h,EFFECTS_OPEN)[0];
    return effect<effects_count(h) ? effect : 0;
}
static u32 effects_committed(volatile u32 *h) { return effects_row(h,effects_open(h)); }

static const u16 *effects_load_label(volatile u32 *h) { return text(h,TEXT_LOAD); }
static u32 effects_load_values(volatile u32 *h) { (void)h; return 2; }
static const u16 *effects_load_text(volatile u32 *h,u32 value) { return text(h,value ? TEXT_OFF : TEXT_ON); }
static u32 effects_load_get(volatile u32 *h) { return effects_off(h,effects_open(h)); }
static void effects_load_set(volatile u32 *h,u32 value) {
    u32 effect=effects_open(h);
    if(value<2) setting_store(h,effects_handle(h,effect),(effects_choice(h,effect)&~1U)|value);
}
static const u16 *effects_order_label(volatile u32 *h) { return text(h,TEXT_ORDER); }
static u32 effects_order_values(volatile u32 *h) { (void)h; return 2; }
static const u16 effects_no_text[1]={0};
/* The row shows no value: the values are moves. */
static const u16 *effects_order_text(volatile u32 *h,u32 value) {
    return value<2 ? text(h,TEXT_MOVE_UP+value) : effects_no_text;
}
static u32 effects_order_get(volatile u32 *h) { (void)h; return 2; }
/* Move the effect up (0) or down (1) one row: each effect gets its row + 1
   as its position. */
static void effects_order_set(volatile u32 *h,u32 value) {
    u8 order[EFFECTS_LISTED];
    u32 n=effects_count(h), effect=effects_open(h), row, at=0;
    effects_order(h,order,n);
    while(at<n && order[at]!=effect) ++at;
    if(at==n) return;
    if(value==0 && at>0) { order[at]=order[at-1]; order[at-1]=(u8)effect; }
    else if(value==1 && at+1<n) { order[at]=order[at+1]; order[at+1]=(u8)effect; }
    else return;
    for(row=0;row<n;++row)
        setting_store(h,effects_handle(h,order[row]),(effects_choice(h,order[row])&1U)|(row+1)<<1);
}
static const Setting effects_settings[SETTING_ROWS]={
    {effects_load_label,effects_load_values,effects_load_text,effects_load_get,effects_load_set},
    {effects_order_label,effects_order_values,effects_order_text,effects_order_get,effects_order_set},
};
#define EFFECTS_PAGE {effects_title,effects_title,effects_rows,effects_label,effects_icon, \
    effects_checked,effects_apply,effects_committed,0,0,effects_settings}
#endif
#endif

#include <assert.h>
#include <string.h>
/* The build's label table, which the Rust test writes for this harness. */
#include "menu_text.h"
/* The family header sends the aspect request; the harness counts requests. */
#include "firmware.h"
#undef MENU_FRAMING_REQUEST
static unsigned framing_requests;
#define MENU_FRAMING_REQUEST() (++framing_requests)
/* The native aspect that the camera applies, and the native setter calls. */
#undef MENU_NATIVE_ASPECT
#undef MENU_SET_NATIVE_ASPECT
#undef MENU_STOP_LIVE_VIEW
static unsigned native_aspect_value, native_sets, live_view_stops;
#define MENU_NATIVE_ASPECT() (native_aspect_value)
#define MENU_SET_NATIVE_ASPECT(id) (++native_sets, native_aspect_value = (id))
#define MENU_STOP_LIVE_VIEW() (++live_view_stops)
/* Camera addresses of the menu text map to text_arena, relative to the text
   start of the header, so the count of records can change during the test. */
static const unsigned short *text_at(unsigned int address);
#define MENU_TEXT(address) text_at(address)
static unsigned test_language;
#define MENU_LANGUAGE() test_language
/* Module pages in mapped module memory: the page of slot n at TEST_PAGE +
   n * TEST_PAGE_SIZE, then its text lists. Text n of all pages is at
   TEST_PAGE_TEXT + n * 128. */
#define TEST_PAGE 0x5d000000U
#define TEST_PAGE_SIZE 0x400U
#define TEST_PAGE_SLOTS 5U
#define TEST_PAGE_TEXT 0x5d800000U
static unsigned page_memory[TEST_PAGE_SLOTS * TEST_PAGE_SIZE / 4];
static unsigned short page_texts[512][64];
static unsigned *page_at(unsigned address)
{
    assert(address >= TEST_PAGE && address < TEST_PAGE + sizeof(page_memory) && !(address & 3));
    return &page_memory[(address - TEST_PAGE) / 4];
}
#define PAGE_WORD(p,o) (*page_at((p)+(o)))
#define TEST_SETTINGS 0x5f000000U
/* Three module settings, the Off and Position settings of three listed
   modules (runtime/manage.h), then the choices of three listed effects
   (runtime/effects.h). */
static unsigned settings_memory[2 + 3 * 12];
static volatile unsigned *mapped_settings(unsigned address)
{
    assert(address == TEST_SETTINGS);
    return settings_memory;
}
#define MODULE_SETTINGS_MAP(address) mapped_settings(address)
/* The module list of the Modules page, then at TEST_EFFECTS the effect list
   of the Effects page. */
#define TEST_MANAGE 0x5e000000U
#define TEST_EFFECTS (TEST_MANAGE + 0x100U)
static unsigned manage_memory[(0x100 + 12 + 3 * 44) / 4];
/* The module list: the header, then module i at word MANAGE_ENTRY(i). */
#define MANAGE_ENTRY(i) (4 + (i) * 11)
static volatile unsigned *mapped_manage(unsigned address)
{
    assert(address >= TEST_MANAGE && address < TEST_MANAGE + sizeof(manage_memory) && !(address & 3));
    return &manage_memory[(address - TEST_MANAGE) / 4];
}
#define MANAGE_MAP(address) mapped_manage(address)
/* The settings of the modules: Guides STYLE (handle 0, 6 values), then Date
   Imprint STYLE (handle 1, 6 values) and LOOK (handle 2, 3 values). */
#define GUIDE_STYLE settings_memory[3]
#define DATE_STYLE settings_memory[6]
#define DATE_LOOK settings_memory[9]
#include "menu.c"
/* The page of module i (runtime/modules.h). */
#define MODULE(i) (*module_at(test_header, (i)))
/* XPan, a crop on every camera. */
#define CROP_RATIO 1U
/* A page icon as its global ID. */
#define PAGE_ICON(module, row) module_icon((module), MODULE((module)).icon(test_header, (row)))
#define CURRENT_ICON(row) module_icon(current_index(test_header), current_module(test_header)->icon(test_header, (row)))

static volatile u32 test_header[GRKIT_HOST_HEADER_SIZE / 4];
static u16 text_arena[GRKIT_TEXT_SIZE / 2] __attribute__((aligned(4)));
static const u16 *text_at(u32 address)
{
    if (address >= TEST_PAGE_TEXT && address < TEST_PAGE_TEXT + sizeof(page_texts) / 64 * 128)
        return page_texts[(address - TEST_PAGE_TEXT) / 128];
    u32 offset = address - GRKIT_TEXT(test_header);
    assert(offset < GRKIT_TEXT_SIZE && !(offset & 1));
    return text_arena + offset / 2;
}
/* A menu string equals an ASCII string. */
static int same(const u16 *actual, const char *expected)
{
    u32 i = 0;
    for (; expected[i]; ++i) if (actual[i] != (u8)expected[i]) return 0;
    return actual[i] == 0;
}
/* Framing labels and summaries by ratio value, and the ratio value of each
   row of the target. */
static const char *const ratio_names[] = {"Off", "XPan 65:24", "16:9", "4:3", "2.35:1", "1:1", "5:4"};
static const char *const ratio_summaries[] = {"Framing: Off", "Framing: XPan", "Framing: 16:9", "Framing: 4:3",
                                              "Framing: 2.35:1", "Framing: 1:1", "Framing: 5:4"};
/* A value that no target offers. */
#define UNKNOWN_RATIO 7
static const u32 ratio_rows[] = {FRAMING_RATIO_ROWS};
#define ROW_COUNT ((u32)(sizeof(ratio_rows) / sizeof(ratio_rows[0])))
/* XPan: row 1, and a crop on every camera. */
#define CROP_ROW 1U
static const char *const guide_names[] = {"Off", "Thirds", "Center", "Golden Ratio", "Diagonals", "Thirds + Center"};
static void ascii(unsigned short *to, const char *from)
{
    while ((*to++ = (u8)*from++)) {}
}
static u32 text_count;
/* The camera address of a new page text. */
static u32 page_text(const char *text)
{
    assert(text_count < sizeof(page_texts) / sizeof(page_texts[0]));
    ascii(page_texts[text_count], text);
    return TEST_PAGE_TEXT + text_count++ * 128;
}
/* Register the choice page of module page slot `slot`, as INSTALL does: the
   page with its text in each language, its setting handle, and the global
   index of its tile 0. A page without rows (`names` 0) gets its rows from
   the core. */
static void install_page(u32 slot, const char *title, const char *const *names, u32 rows, u32 handle, u32 tile)
{
    u32 page = TEST_PAGE + slot * TEST_PAGE_SIZE, titles = page + 0x28, summaries = titles + 16,
        labels = summaries + rows * 16;
    char summary[64];
    assert(slot < TEST_PAGE_SLOTS && labels + rows * 16 <= page + 0x200);
    const unsigned fields[] = {40, 1, 0, names ? 1 : 2, titles, names ? summaries : 0, names ? labels : 0, rows, 0, 0};
    for (u32 i = 0; i < 10; ++i) *page_at(page + i * 4) = fields[i];
    for (u32 language = 0; language < 4; ++language) {
        *page_at(titles + language * 4) = page_text(title);
        for (u32 value = 0; value < rows; ++value) {
            strcpy(summary, title);
            strcat(summary, ": ");
            strcat(summary, names[value]);
            *page_at(summaries + (value * 4 + language) * 4) = page_text(summary);
            *page_at(labels + (value * 4 + language) * 4) = page_text(names[value]);
        }
    }
    if (test_header[GRKIT_HOST_MODULE_PAGE_COUNT / 4] <= slot) test_header[GRKIT_HOST_MODULE_PAGE_COUNT / 4] = slot + 1;
    test_header[GRKIT_HOST_MODULE_PAGES / 4 + slot * 3] = page;
    test_header[GRKIT_HOST_MODULE_PAGES / 4 + slot * 3 + 1] = handle;
    test_header[GRKIT_HOST_MODULE_PAGES / 4 + slot * 3 + 2] = tile;
}
static const char *const grain_names[] = {"Off", "Weak", "Medium", "Strong"};
/* The summaries of Framing, which name the ratios shorter. */
static const char *const ratio_values[] = {"Off", "XPan", "16:9", "4:3", "2.35:1", "1:1", "5:4"};
static const char *const stamp_names[] = {"Off", "'YY M D", "'YY M D H:MM", "M D 'YY", "D M 'YY", "D M 'YY H:MM"};
static const char *const look_names[] = {"Classic", "GR1s", "GR II"};
/* The rows of the native settings page of Date Imprint: STYLE, then LOOK. */
static void install_setting_rows(u32 slot)
{
    u32 page = TEST_PAGE + slot * TEST_PAGE_SIZE, rows = page + 0x200, texts = rows + 32;
    const char *const labels[2] = {"Date Style", "Date Look"};
    const char *const *values[2] = {stamp_names, look_names};
    const u32 counts[2] = {6, 3};
    *page_at(page + PAGE_SETTING_ROWS) = 2;
    *page_at(page + PAGE_SETTING_LIST) = rows;
    for (u32 r = 0; r < 2; ++r) {
        *page_at(rows + r * 16) = 16;
        *page_at(rows + r * 16 + ROW_SETTING) = r;
        *page_at(rows + r * 16 + ROW_LABEL) = texts;
        for (u32 language = 0; language < 4; ++language) *page_at(texts + language * 4) = page_text(labels[r]);
        texts += 16;
        *page_at(rows + r * 16 + ROW_VALUES) = texts;
        for (u32 value = 0; value < counts[r]; ++value)
            for (u32 language = 0; language < 4; ++language)
                *page_at(texts + (value * 4 + language) * 4) = page_text(values[r][value]);
        texts += counts[r] * 16;
    }
    assert(texts <= page + TEST_PAGE_SIZE);
}
/* The official modules (modules/), as INSTALL registers them in their root
   order: Image Control, a page of CORE.TONE with rows from the core;
   Framing, a page and layers of CORE.ASPECT; Guides, its STYLE setting and
   a layer; Date Imprint, its STYLE and LOOK settings, a page with native
   settings rows, and a filter; Grain, a page of CORE.GRAIN. */
static void install_modules(void)
{
    install_page(0, "Image Control", 0, 0, GRKIT_CORE_TONE, 0);
    install_page(1, "Framing", ratio_values, 7, GRKIT_CORE_ASPECT, 0);
    install_page(2, "Guides", guide_names, 6, 0, 7);
    install_page(3, "Date Imprint", stamp_names, 6, 1, 13);
    install_setting_rows(3);
    install_page(4, "Grain", grain_names, 4, GRKIT_CORE_GRAIN, 19);
    /* The labels of Framing name the ratios in full. */
    for (u32 value = 0; value < 7; ++value)
        for (u32 language = 0; language < 4; ++language)
            *page_at(PAGE_WORD(TEST_PAGE + TEST_PAGE_SIZE, PAGE_LABELS) + (value * 4 + language) * 4) =
                page_text(ratio_names[value]);
    memset(settings_memory, 0, sizeof settings_memory);
    settings_memory[0] = 3; settings_memory[4] = 6; settings_memory[7] = 6; settings_memory[10] = 3;
    test_header[GRKIT_HOST_MODULE_SETTINGS / 4] = TEST_SETTINGS;
    test_header[GRKIT_HOST_CAPABILITIES / 4] = 1U << (GRKIT_CORE_TONE - GRKIT_CORE_SETTING_BASE) |
                                               1U << (GRKIT_CORE_ASPECT - GRKIT_CORE_SETTING_BASE) |
                                               1U << (GRKIT_CORE_GRAIN - GRKIT_CORE_SETTING_BASE);
    test_header[GRKIT_HOST_MODULE_LAYER_COUNT / 4] = 1;
    test_header[GRKIT_HOST_MODULE_LAYERS / 4] = 0x5e000001;
    test_header[GRKIT_HOST_MODULE_LAYERS / 4 + 1] = 0;
    test_header[GRKIT_HOST_MODULE_LAYERS / 4 + 2] = 20U << 16 | 1;
}
/* The root rows of the module pages, then About. */
#define IMAGE_ROW 0U
#define IMAGE_MODULE MODULE_PAGE(0)
#define FRAMING_ROW 1U
#define FRAMING_MODULE MODULE_PAGE(1)
#define FRAMING_TILE(n) GRKIT_ICON_MODULE_TILE(n)
#define GUIDES_ROW 2U
#define GUIDES_MODULE MODULE_PAGE(2)
#define GUIDE_TILE(n) GRKIT_ICON_MODULE_TILE(7 + (n))
#define DATE_ROW 3U
#define DATE_MODULE MODULE_PAGE(3)
#define DATE_TILE(n) GRKIT_ICON_MODULE_TILE(13 + (n))
#define GRAIN_ROW 4U
#define GRAIN_MODULE MODULE_PAGE(4)
#define GRAIN_TILE(n) GRKIT_ICON_MODULE_TILE(19 + (n))
#define ALL_PAGES 5U
static u32 native_changes;
static u32 native_events;
volatile u32 *menu_header(void) { return test_header; }
u32 menu_event_original(u32 controller, const u16 *event)
{ (void)controller; (void)event; ++native_events; return 42; }
void menu_exit_original(u32 controller) { (void)controller; }
u32 menu_destroy_original(u32 controller) { return controller; }
void menu_refresh_original(u32 controller) { (void)controller; }
void menu_changed_original(u32 controller) { (void)controller; ++native_changes; }

int main(void)
{
    volatile u32 slot[2];
    assert(sizeof(menu_text_table) <= GRKIT_TEXT_SUMMARY);
    memcpy(text_arena, menu_text_table, sizeof(menu_text_table));
    descriptor(slot, plugins);
    assert(slot[0] == 0x106);
    test_header[GRKIT_HOST_COUNT / 4] = 5;
    /* The official modules are loaded. */
    install_modules();
    test_header[GRKIT_HOST_OFF_ICON / 4] = 0x59001000;
    test_header[GRKIT_HOST_PLUGINS_ICON / 4] = 0x59003000;
    test_header[GRKIT_HOST_RECORDS / 4] = 0x59005000;
    assert(selected_icon(test_header) == 0x7fff);
    descriptor(slot, MODULE(0).summary(test_header));
    assert(same(MODULE(0).summary(test_header), "Image Control: Off") && slot[0] == 0x113);
    assert(same(MODULE(0).title(test_header), "Image Control"));
    /* Each language reads its own strings, and an unknown language reads English. */
    for (u32 language = 0; language < GRKIT_TEXT_LANGUAGES + 2; ++language) {
        u32 table = language < GRKIT_TEXT_LANGUAGES ? language : 0;
        test_language = language;
        for (u32 id = 0; id < GRKIT_TEXT_STRINGS; ++id)
            assert(text(test_header, id) == text_arena + text_arena[table * GRKIT_TEXT_STRINGS + id] / 2);
    }
    test_language = 2;
    assert(!same(text(test_header, TEXT_IMAGE_OFF_SUMMARY), "Image Control: Off"));
    /* The text hooks select the font of the text language. */
    assert(menu_table_index() == 18);
    test_language = 1;
    assert(menu_table_index() == 19);
    test_language = 3;
    assert(menu_table_index() == 0);
    test_language = 7;
    assert(menu_table_index() == 1);
    test_language = 0;
    assert(menu_table_index() == 1);
    assert(menu_viewfinder_icon_select(0x166, 0) == 0x166);
    assert(menu_viewfinder_icon_select(0, 0) == 0);
    test_header[GRKIT_HOST_ACTIVE / 4] = 0x59005000 + 3 * GRKIT_RECORD_SIZE;
    assert(selected_icon(test_header) == 0x8003);
    /* The summary of record 3 is its slot in the menu language. */
    for (u32 language = 0; language < GRKIT_TEXT_LANGUAGES; ++language) {
        test_language = language;
        assert(MODULE(0).summary(test_header) ==
               text_arena + (GRKIT_TEXT_SUMMARY + (language * GRKIT_TEXT_RECORDS + 3) * GRKIT_SUMMARY_UNITS * 2) / 2);
    }
    test_language = 0;
    assert(menu_viewfinder_icon_select(0x166, 0) == 0x8003);
    assert(menu_viewfinder_icon_select(0x166, 1) == 0x166);
    test_header[GRKIT_HOST_ACTIVE / 4] += 4;
    assert(menu_viewfinder_icon_select(0x167, 0) == 0x167);
    test_header[GRKIT_HOST_ACTIVE / 4] = 0;
    /* Fn and its hint apply to effect rows of Image Control only. */
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_MODULE;
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 0;
    assert(!parameters_row(test_header));
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 2;
    assert(parameters_row(test_header));
    test_header[GRKIT_HOST_MODULE / 4] = 1;
    assert(!parameters_row(test_header));
    test_header[GRKIT_HOST_MODULE / 4] = 0;
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_ROOT;
    assert(!parameters_row(test_header));
    /* On the root, Fn applies to the Image Control row while an effect is
       committed. Off and the rows of other modules have no parameters. */
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 0;
    assert(!parameters_row(test_header));
    test_header[GRKIT_HOST_ACTIVE / 4] = 0x59005000 + 2 * GRKIT_RECORD_SIZE;
    assert(parameters_row(test_header));
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 1;
    assert(!parameters_row(test_header));
    test_header[GRKIT_HOST_ACTIVE / 4] = 0;
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = 0;
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 0;
    assert(!parameters_row(test_header));
    assert(menu_press(2, 0));
    assert(!menu_press(1, 0));
    assert(!menu_press(2, 1));
    assert(!menu_press(2, 100));
    assert(back_event(test_header, EV_SWITCH, 1) == 1);
    assert(back_event(test_header, EV_SWITCH, 1) == 2);
    assert(back_event(test_header, 0x19d, 1) == 0);
    assert(back_event(test_header, EV_CONFIRM, 0) == 2);
    assert(back_event(test_header, EV_CONFIRM, 0) == 0);
    assert(back_event(test_header, EV_SWITCH, 0) == 0);
    assert(back_event(test_header, EV_SWITCH, 1) == 1);
    assert(back_event(test_header, EV_SWITCH_END, 1) == 0);
    assert(back_event(test_header, EV_CONFIRM, 1) == 0);
    /* List restoration changes level to native before rebuilding the rows. */
    test_header[GRKIT_HOST_ACTIVE / 4] = 0x59005000;
    test_header[GRKIT_HOST_MENU_CONTROLLER / 4] = 0x1000;
    test_header[GRKIT_HOST_MENU_REBUILDING / 4] = 1;
    menu_changed(0x1000);
    assert(native_changes == 0);
    assert(test_header[GRKIT_HOST_ACTIVE / 4] == 0x59005000);
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = 1;
    menu_changed(0x1000);
    assert(native_changes == 0);
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = 0;
    test_header[GRKIT_HOST_MENU_REBUILDING / 4] = 0;
    menu_changed(0x1000);
    assert(native_changes == 1);
    {
        const u16 confirm[] = {EV_CONFIRM};
        test_header[GRKIT_HOST_MENU_LEVEL / 4] = 0;
        test_header[GRKIT_HOST_BACK_PENDING / 4] = 1;
        assert(menu_event(0x1000, confirm) == 0);
        assert(native_events == 0);
        assert(test_header[GRKIT_HOST_BACK_PENDING / 4] == 0);
        assert(test_header[GRKIT_HOST_ACTIVE / 4] == 0x59005000);
    }
    /* The event ring distinguishes MENU's confirm from OK and ADJ press confirms. */
    test_header[GRKIT_HOST_MENU_EVENTS / 4] = 18;
    test_header[GRKIT_HOST_MENU_EVENT_RING / 4] = EV_SWITCH | (2 << 16);
    test_header[GRKIT_HOST_MENU_EVENT_RING / 4 + 1] = EV_CONFIRM;
    assert(after_menu_switch(test_header));
    test_header[GRKIT_HOST_MENU_EVENT_RING / 4] = EV_SWITCH_END;
    assert(!after_menu_switch(test_header));
    test_header[GRKIT_HOST_MENU_EVENT_RING / 4] = EV_ADJ_PRESS;
    assert(!after_menu_switch(test_header));
    /* An ADJ press is EV_ADJ_PRESS, then the confirm. OK and MENU are not. */
    assert(after_adj_press(test_header));
    test_header[GRKIT_HOST_MENU_EVENT_RING / 4] = EV_SWITCH_END | (2 << 16);
    assert(!after_adj_press(test_header));
    test_header[GRKIT_HOST_MENU_EVENT_RING / 4] = EV_SWITCH | (2 << 16);
    assert(!after_adj_press(test_header));
    test_header[GRKIT_HOST_MENU_EVENTS / 4] = 0;
    test_header[GRKIT_HOST_MENU_EVENT_RING / 4] = test_header[GRKIT_HOST_MENU_EVENT_RING / 4 + 1] = 0;
#ifdef EV_FN_ACTION
    /* GR IV: the Fn press that opens a GRKit parameter page also sends the
       native Fn action, which must not open the native page. */
    {
        const u16 action[] = {EV_FN_ACTION};
        u32 events = native_events;
        test_header[GRKIT_HOST_RETURN / 4] = RETURN_OPENING;
        assert(menu_event(0x1000, action) == 0);
        test_header[GRKIT_HOST_RETURN / 4] = RETURN_PARAMETERS | RETURN_ROOT;
        assert(menu_event(0x1000, action) == 0);
        assert(native_events == events);
        test_header[GRKIT_HOST_RETURN / 4] = 0;
        test_header[GRKIT_HOST_MENU_EVENTS / 4] = 0;
        test_header[GRKIT_HOST_MENU_EVENT_RING / 4] = test_header[GRKIT_HOST_MENU_EVENT_RING / 4 + 1] = 0;
    }
#endif
    /* Playback matches only extracted non-native tags with the private code bit. */
    {
        u32 count = test_header[GRKIT_HOST_COUNT / 4];
        assert(playback_icon(test_header, 0) == 0);
        assert(playback_icon(test_header, 5) == 0);
        assert(playback_icon(test_header, 0x10000 | 0x7eff) == 0);
        assert(playback_icon(test_header, 0x20000 | 0x7f00) == 0);
        test_header[GRKIT_HOST_COUNT / 4] = 0;
        assert(playback_icon(test_header, 0x10000 | 0x7f00) == 0);
        test_header[GRKIT_HOST_COUNT / 4] = count;
    }
    assert(!native_action_release(10, 0));
    assert(!native_action_release(0, 1));
    assert(native_action_release(10, 1));
    assert(menu_icon(0x7ffe) == 0x59003000);
    /* The Version icon follows the GRKit icon in chunk 0. */
    assert(menu_icon(GRKIT_ICON_VERSION) == 0x59003000 + GRKIT_ICON_STRIDE);
    assert(toolbar_x(0) == 415 && toolbar_x(5) == 665);
    assert(menu_icon(0x7fff) == 0x59001000);
    assert(menu_icon(0x166) == 0);
    assert(menu_icon(0x8005) == 0);
    assert(menu_icon(0xffff) == 0);
    assert(wrapped_edge(EV_RIGHT, 3, 0, 0) == 2);
    assert(wrapped_edge(EV_LEFT, 1, 4, 0) == 1);
    assert(wrapped_edge(EV_RIGHT, 1, 2, 0) == 0);
    assert(wrapped_edge(EV_LEFT, 3, 2, 0) == 0);
    assert(wrapped_edge(EV_RIGHT, 4, 0, 1) == 0);
    assert(wrapped_edge(0x35, 4, 0, 0) == 0);
    assert(advance_from_private(EV_RIGHT, 1) == 1);
    assert(advance_from_private(EV_LEFT, 1) == 0);
    assert(advance_from_private(EV_RIGHT, 2) == 0);
    assert(advance_from_private(EV_LEFT, 2) == 1);
    /* ADJ exits once for the parameter page. An exit from GRKit, or MENU or OK
       that closed ADJ from GRKit, opens GRKit at the next ADJ. Any other exit
       ends the return. */
    assert(exit_return(RETURN_OPENING, 0) == RETURN_PARAMETERS);
    assert(exit_return(RETURN_CLOSING, 0) == RETURN_GRKIT);
    assert(exit_return(0, 1) == RETURN_GRKIT);
    assert(exit_return(0, 0) == 0);
    assert(exit_return(RETURN_GRKIT, 0) == 0);
    assert(exit_return(RETURN_PARAMETERS, 0) == 0);
    /* A page that Fn opened from the root keeps its mark until it returns. */
    assert(exit_return(RETURN_OPENING | RETURN_ROOT, 0) == (RETURN_PARAMETERS | RETURN_ROOT));
    assert(exit_return(RETURN_PARAMETERS | RETURN_ROOT, 0) == 0);
    assert(RETURN_STATE(RETURN_PARAMETERS | RETURN_ROOT) == RETURN_PARAMETERS);
    /* The root lists the five official modules, then About. Without effects,
       the page of Image Control does not show. */
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = 1;
    assert(count(test_header) == 6);
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = 2;
    assert(count(test_header) == 6);
    test_header[GRKIT_HOST_COUNT / 4] = 0;
    assert(count(test_header) == 1);
    /* Registry routing leaves the existing effect record untouched. */
    test_header[GRKIT_HOST_ACTIVE / 4] = 0x59005000;
    test_header[GRKIT_HOST_COUNT / 4] = 5;
    /* Module tiles follow the effect icons in the chunks: slot
       GRKIT_HOST_ICON_COUNT + effects + n. */
    test_header[GRKIT_HOST_ICON_CHUNKS / 4] = 0x59100000;
    test_header[GRKIT_HOST_ICON_CHUNKS / 4 + 1] = 0x59200000;
    {
        u32 first = GRKIT_HOST_ICON_COUNT + 5;
        assert(menu_icon(GRKIT_ICON_MODULE_TILE(0)) == 0x59100000 + first * GRKIT_ICON_STRIDE);
        assert(menu_icon(GRKIT_ICON_MODULE_TILE(40 - first)) == 0x59200000);
        assert(menu_icon(GRKIT_ICON_MODULE_TILE(41 - first)) == 0x59200000 + GRKIT_ICON_STRIDE);
        /* A chunk that the installer did not allocate. */
        assert(menu_icon(GRKIT_ICON_MODULE_TILE(80 - first)) == 0);
        assert(menu_icon(GRKIT_ICON_MODULE_TILE(GRKIT_MODULE_TILES_MAX)) == 0);
    }
    /* The Modules tile follows the Version tile, and the icon IDs below it
       are not tiles. */
    assert(menu_icon(GRKIT_ICON_MODULES) == test_header[GRKIT_HOST_PLUGINS_ICON / 4] + 2 * GRKIT_ICON_STRIDE);
    assert(menu_icon(GRKIT_ICON_MODULES - 1) == 0);
    /* The Framing page of CORE.ASPECT: its root row shows the ratio that the
       camera applies. */
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_ROOT;
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = FRAMING_ROW;
    assert(root_module(test_header, FRAMING_ROW) == FRAMING_MODULE);
    assert(same(current_module(test_header)->summary(test_header), "Framing: Off"));
    assert(PAGE_ICON(FRAMING_MODULE, FRAMING_ROW) == FRAMING_TILE(0));
    test_header[GRKIT_HOST_FRAME_RATIO / 4] = CROP_RATIO;
    assert(PAGE_ICON(FRAMING_MODULE, FRAMING_ROW) == FRAMING_TILE(CROP_RATIO));
    /* A ratio that the target does not offer reads as Off. */
    test_header[GRKIT_HOST_FRAME_RATIO / 4] = UNKNOWN_RATIO;
    assert(PAGE_ICON(FRAMING_MODULE, FRAMING_ROW) == FRAMING_TILE(0));
    /* Without a module that uses CORE.ASPECT, a stored crop is no crop. */
    test_header[GRKIT_HOST_FRAME_RATIO / 4] = CROP_RATIO;
    test_header[GRKIT_HOST_CAPABILITIES / 4] &= ~(1U << (GRKIT_CORE_ASPECT - GRKIT_CORE_SETTING_BASE));
    assert(setting_value(test_header, GRKIT_CORE_ASPECT) == 0 && framing_crop(test_header) == 0);
    test_header[GRKIT_HOST_CAPABILITIES / 4] |= 1U << (GRKIT_CORE_ASPECT - GRKIT_CORE_SETTING_BASE);
    /* Framing lists the ratios of the target in its row order and opens at
       the row of the selected one. */
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_MODULE && test_header[GRKIT_HOST_MODULE / 4] == FRAMING_MODULE);
    assert(test_header[GRKIT_HOST_MENU_CURSOR / 4] == CROP_ROW);
    assert(count(test_header) == ROW_COUNT && ROW_COUNT == 7);
    assert(same(current_module(test_header)->title(test_header), "Framing"));
    for (u32 row = 0; row < ROW_COUNT; ++row) {
        u32 value = ratio_rows[row], native = framing_native(value);
        u32 requests = framing_requests, sets = native_sets, stops = live_view_stops;
        test_header[GRKIT_HOST_MENU_CURSOR / 4] = row;
        menu_confirm(test_header);
        assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_MODULE);
        if (native != FRAMING_CROP) {
            /* A ratio that the camera has sets its native aspect and no crop. */
            assert(test_header[GRKIT_HOST_FRAME_RATIO / 4] == 0 && native_aspect_value == native);
            assert(framing_requests - requests + native_sets - sets == 1);
        } else {
            /* A crop needs native 3:2, then resends the request. */
            assert(test_header[GRKIT_HOST_FRAME_RATIO / 4] == value && native_aspect_value == 0);
            assert(framing_requests == requests + 1);
        }
        /* Each native aspect change stops the live view first. */
        assert(live_view_stops - stops == native_sets - sets);
        assert(setting_value(test_header, GRKIT_CORE_ASPECT) == value);
        assert(current_module(test_header)->checked(test_header, row));
        assert(!current_module(test_header)->checked(test_header, (row + 1) % ROW_COUNT));
        assert(same(current_module(test_header)->label(test_header, row), ratio_names[value]));
        assert(CURRENT_ICON(row) == (row ? FRAMING_TILE(value) : 0x7fff));
        assert(same(current_module(test_header)->summary(test_header), ratio_summaries[value]));
        assert(test_header[GRKIT_HOST_ACTIVE / 4] == 0x59005000);
    }
    /* 1:1 is native on each camera, and the last row. */
    assert(framing_native(ratio_rows[ROW_COUNT - 1]) == 3 && native_aspect_value == 3);
    /* A native aspect from the camera menu shows on the page. A crop applies
       only in native 3:2, except XPan and 2.35:1 in GR IV native 16:9. */
    test_header[GRKIT_HOST_FRAME_RATIO / 4] = CROP_RATIO;
    assert(current_module(test_header)->committed(test_header) == ROW_COUNT - 1);
#ifdef FRAMING_XPAN_NATIVE
    native_aspect_value = FRAMING_XPAN_NATIVE;
    assert(current_module(test_header)->committed(test_header) == CROP_ROW);
    test_header[GRKIT_HOST_FRAME_RATIO / 4] = 6;
    assert(current_module(test_header)->committed(test_header) == framing_row(2));
    native_aspect_value = 1;
    assert(current_module(test_header)->committed(test_header) == framing_row(3));
#endif
    native_aspect_value = 0;
    /* A saved value that the target does not offer reads as Off. */
    test_header[GRKIT_HOST_FRAME_RATIO / 4] = UNKNOWN_RATIO;
    assert(current_module(test_header)->committed(test_header) == 0);
    /* A saved native value is not a crop. */
    test_header[GRKIT_HOST_FRAME_RATIO / 4] = 5;
    assert(current_module(test_header)->committed(test_header) == 0);
    /* A module sets only a value that the target offers. */
    setting_store(test_header, GRKIT_CORE_ASPECT, UNKNOWN_RATIO);
    assert(test_header[GRKIT_HOST_FRAME_RATIO / 4] == 5);
    test_header[GRKIT_HOST_FRAME_RATIO / 4] = CROP_RATIO;
    /* The Framing page shows its highlighted row on the ADJ canvas: the area
       of the ratio, and the ratio as the value of the CORE.ASPECT layers. */
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 1;
    frame_preview(test_header, 0x5a000000, 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == 2);
    assert(test_header[GRKIT_HOST_MODULE_PREVIEW / 4] == GRKIT_CORE_ASPECT + 1 && test_header[GRKIT_HOST_MODULE_PREVIEW / 4 + 1] == 1);
    /* The preview holds the value of the row: row 2 is 2.35:1 (value 4). */
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 2;
    frame_preview(test_header, 0x5a000000, 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == 5 && test_header[GRKIT_HOST_MODULE_PREVIEW / 4 + 1] == 4);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW_CANVAS / 4] == 0x5a000000);
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 0;
    frame_preview(test_header, 0x5a000000, 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == 1);
    frame_preview(test_header, 0x5a000000, 1);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == 0);
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = CROP_ROW;
    assert(menu_back(test_header));
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT && test_header[GRKIT_HOST_MENU_CURSOR / 4] == FRAMING_ROW);
    /* Other ADJ pages show the committed crop. */
    frame_preview(test_header, 0x5b000000, 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == CROP_RATIO + 1);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW_CANVAS / 4] == 0x5b000000);
    /* A committed Off ends the mask but keeps the canvas for one erasing
       draw. The Guides layer is off too. */
    test_header[GRKIT_HOST_FRAME_RATIO / 4] = 0;
    frame_preview(test_header, 0x5b000000, 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW_CANVAS / 4] == 0x5b000000);
    test_header[GRKIT_HOST_FRAME_RATIO / 4] = CROP_RATIO;
    test_header[GRKIT_HOST_FRAME_PREVIEW_CANVAS / 4] = 0;
    assert(!menu_back(test_header));
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = FRAMING_ROW;
    assert(same(current_module(test_header)->summary(test_header), ratio_summaries[CROP_RATIO]));
    /* The Guides module page opens at the committed style and commits only
       its own setting. */
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_ROOT;
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = GUIDES_ROW;
    assert(root_module(test_header, GUIDES_ROW) == GUIDES_MODULE);
    assert(same(current_module(test_header)->summary(test_header), "Guides: Off"));
    assert(PAGE_ICON(GUIDES_MODULE, GUIDES_ROW) == GUIDE_TILE(0));
    GUIDE_STYLE = 3;
    assert(PAGE_ICON(GUIDES_MODULE, GUIDES_ROW) == GUIDE_TILE(3));
    {
        u32 requests = framing_requests, changes = settings_memory[1];
        menu_confirm(test_header);
        assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_MODULE && test_header[GRKIT_HOST_MODULE / 4] == GUIDES_MODULE);
        assert(test_header[GRKIT_HOST_MENU_CURSOR / 4] == 3);
        assert(count(test_header) == 6);
        assert(same(current_module(test_header)->title(test_header), "Guides"));
        for (u32 row = 0; row < 6; ++row) {
            test_header[GRKIT_HOST_MENU_CURSOR / 4] = row;
            menu_confirm(test_header);
            assert(GUIDE_STYLE == row);
            assert(current_module(test_header)->checked(test_header, row));
            assert(!current_module(test_header)->checked(test_header, (row + 1) % 6));
            assert(same(current_module(test_header)->label(test_header, row), guide_names[row]));
            assert(CURRENT_ICON(row) == (row ? GUIDE_TILE(row) : 0x7fff));
            assert(test_header[GRKIT_HOST_FRAME_RATIO / 4] == CROP_RATIO);
            assert(test_header[GRKIT_HOST_ACTIVE / 4] == 0x59005000);
        }
        assert(framing_requests == requests);
        /* Each change counts once, for the resident loader. */
        assert(settings_memory[1] == changes + 6);
        /* OK on the committed row changes nothing. */
        menu_confirm(test_header);
        assert(settings_memory[1] == changes + 6);
    }
    /* The page previews its highlighted style inside the committed ratio. */
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 2;
    frame_preview(test_header, 0x5a000000, 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == CROP_RATIO + 1);
    assert(test_header[GRKIT_HOST_MODULE_PREVIEW / 4] == 1 && test_header[GRKIT_HOST_MODULE_PREVIEW / 4 + 1] == 2);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW_CANVAS / 4] == 0x5a000000);
    /* A highlighted Off with no ratio keeps the canvas for the next row. */
    test_header[GRKIT_HOST_FRAME_RATIO / 4] = 0;
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 0;
    GUIDE_STYLE = 0;
    frame_preview(test_header, 0x5a000000, 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == 1);
    assert(test_header[GRKIT_HOST_MODULE_PREVIEW / 4] == 1 && test_header[GRKIT_HOST_MODULE_PREVIEW / 4 + 1] == 0);
    /* The movie live view previews nothing. */
    frame_preview(test_header, 0x5a000000, 1);
    assert(!test_header[GRKIT_HOST_FRAME_PREVIEW / 4] && !test_header[GRKIT_HOST_MODULE_PREVIEW / 4]);
    assert(menu_back(test_header));
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT && test_header[GRKIT_HOST_MENU_CURSOR / 4] == GUIDES_ROW);
    /* Other ADJ pages keep the committed style on the canvas, even without a
       ratio, and preview no module value. */
    GUIDE_STYLE = 4;
    frame_preview(test_header, 0x5b000000, 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == 1 && !test_header[GRKIT_HOST_MODULE_PREVIEW / 4]);
    /* The Framing page keeps the committed style with its highlighted ratio
       (row 1, XPan). */
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = FRAMING_ROW;
    menu_confirm(test_header);
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 1;
    frame_preview(test_header, 0x5b000000, 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == 2 && test_header[GRKIT_HOST_MODULE_PREVIEW / 4] == GRKIT_CORE_ASPECT + 1);
    /* Committed Off for both ends the drawing. */
    assert(menu_back(test_header));
    GUIDE_STYLE = 0;
    frame_preview(test_header, 0x5b000000, 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == 0);
    /* A value out of range of its setting reads as 0. */
    GUIDE_STYLE = 999;
    assert(setting_value(test_header, 0) == 0);
    GUIDE_STYLE = 0;
    test_header[GRKIT_HOST_FRAME_RATIO / 4] = CROP_RATIO;
    test_header[GRKIT_HOST_FRAME_PREVIEW_CANVAS / 4] = 0;
    /* Date Imprint lists its styles, opens at the committed one, and changes
       nothing else. */
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_ROOT;
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = DATE_ROW;
    assert(root_module(test_header, DATE_ROW) == DATE_MODULE);
    assert(same(current_module(test_header)->summary(test_header), "Date Imprint: Off"));
    assert(PAGE_ICON(DATE_MODULE, DATE_ROW) == DATE_TILE(0));
    DATE_STYLE = 2;
    assert(PAGE_ICON(DATE_MODULE, DATE_ROW) == DATE_TILE(2));
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_MODULE && test_header[GRKIT_HOST_MODULE / 4] == DATE_MODULE);
    assert(test_header[GRKIT_HOST_MENU_CURSOR / 4] == 2 && count(test_header) == 6);
    assert(same(current_module(test_header)->title(test_header), "Date Imprint"));
    for (u32 row = 0; row < 6; ++row) {
        test_header[GRKIT_HOST_MENU_CURSOR / 4] = row;
        menu_confirm(test_header);
        assert(DATE_STYLE == row);
        assert(current_module(test_header)->checked(test_header, row));
        assert(same(current_module(test_header)->label(test_header, row), stamp_names[row]));
        assert(CURRENT_ICON(row) == (row ? DATE_TILE(row) : 0x7fff));
        assert(test_header[GRKIT_HOST_FRAME_RATIO / 4] == CROP_RATIO && GUIDE_STYLE == 0 && DATE_LOOK == 0);
        assert(test_header[GRKIT_HOST_ACTIVE / 4] == 0x59005000);
    }
    /* The Date Imprint page keeps the committed mask on the ADJ canvas, and
       previews its highlighted style for its layers, which it has none of. */
    frame_preview(test_header, 0x5c000000, 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == CROP_RATIO + 1 && test_header[GRKIT_HOST_MODULE_PREVIEW / 4] == 2);
    assert(menu_back(test_header) && test_header[GRKIT_HOST_MENU_CURSOR / 4] == DATE_ROW);
    assert(same(current_module(test_header)->summary(test_header), "Date Imprint: D M 'YY H:MM"));
    DATE_STYLE = 999;
    assert(setting_value(test_header, 1) == 0);
    DATE_STYLE = 0;
    test_header[GRKIT_HOST_FRAME_PREVIEW_CANVAS / 4] = 0;
    /* The Grain page of the capability CORE.GRAIN lists its levels, opens at
       the committed one, and changes only the Grain word of the core. */
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_ROOT;
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = GRAIN_ROW;
    assert(root_module(test_header, GRAIN_ROW) == GRAIN_MODULE);
    assert(same(current_module(test_header)->summary(test_header), "Grain: Off"));
    assert(PAGE_ICON(GRAIN_MODULE, GRAIN_ROW) == GRAIN_TILE(0));
    test_header[GRKIT_HOST_GRAIN / 4] = 2;
    assert(PAGE_ICON(GRAIN_MODULE, GRAIN_ROW) == GRAIN_TILE(2));
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_MODULE && test_header[GRKIT_HOST_MODULE / 4] == GRAIN_MODULE);
    assert(test_header[GRKIT_HOST_MENU_CURSOR / 4] == 2 && count(test_header) == 4);
    assert(same(current_module(test_header)->title(test_header), "Grain"));
    const u32 guide_style = GUIDE_STYLE, module_changes = settings_memory[1];
    for (u32 row = 0; row < 4; ++row) {
        test_header[GRKIT_HOST_MENU_CURSOR / 4] = row;
        menu_confirm(test_header);
        assert(test_header[GRKIT_HOST_GRAIN / 4] == row);
        assert(current_module(test_header)->checked(test_header, row));
        assert(same(current_module(test_header)->label(test_header, row), grain_names[row]));
        assert(CURRENT_ICON(row) == (row ? GRAIN_TILE(row) : 0x7fff));
        assert(test_header[GRKIT_HOST_FRAME_RATIO / 4] == CROP_RATIO && DATE_STYLE == 0);
        assert(test_header[GRKIT_HOST_ACTIVE / 4] == 0x59005000);
        assert(GUIDE_STYLE == guide_style && settings_memory[1] == module_changes);
    }
    assert(menu_back(test_header) && test_header[GRKIT_HOST_MENU_CURSOR / 4] == GRAIN_ROW);
    assert(same(current_module(test_header)->summary(test_header), "Grain: Strong"));
    /* A value out of range reads as Off, and a store out of range does nothing. */
    test_header[GRKIT_HOST_GRAIN / 4] = 999;
    assert(setting_value(test_header, GRKIT_CORE_GRAIN) == 0 && setting_limit(test_header, GRKIT_CORE_GRAIN) == 4);
    setting_store(test_header, GRKIT_CORE_GRAIN, 4);
    assert(test_header[GRKIT_HOST_GRAIN / 4] == 999);
    setting_store(test_header, GRKIT_CORE_GRAIN, 1);
    assert(test_header[GRKIT_HOST_GRAIN / 4] == 1);
    /* A handle after the capabilities is no setting. */
    assert(setting_limit(test_header, GRKIT_CORE_SETTING_BASE + GRKIT_CORE_SETTING_COUNT) == 0);
    test_header[GRKIT_HOST_GRAIN / 4] = 0;
    test_header[GRKIT_HOST_FRAME_PREVIEW_CANVAS / 4] = 0;
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_ROOT;
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 1;
    /* Invalid public navigation cursors cannot form an effect pointer. */
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 999;
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_MODULE && test_header[GRKIT_HOST_MODULE / 4] == 0);
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 999;
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_ACTIVE / 4] == 0);
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 5;
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_ACTIVE / 4] == 0x59005000 + 4 * GRKIT_RECORD_SIZE);
    assert(test_header[GRKIT_HOST_FRAME_RATIO / 4] == CROP_RATIO);
    assert(menu_back(test_header));
    assert(test_header[GRKIT_HOST_MENU_CURSOR / 4] == 0);
    test_header[GRKIT_HOST_MODULE / 4] = 999;
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_MODULE + 1;
    menu_normalize(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT && test_header[GRKIT_HOST_MODULE / 4] == 0);
    test_header[GRKIT_HOST_FRAME_RATIO / 4] = 999;
    assert(setting_value(test_header, GRKIT_CORE_ASPECT) == 0);
    /* Pages start at multiples of PAGE for every supported catalog row. */
    for (u32 row = 0; row <= 146; ++row) assert(page_start(row) == row / PAGE * PAGE);

    /* Image Control opens at the committed row. */
    test_header[GRKIT_HOST_PREVIEW / 4] = 0;
    test_header[GRKIT_HOST_ACTIVE / 4] = 0x59005000 + 3 * GRKIT_RECORD_SIZE;
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_ROOT;
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 0;
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_MODULE && test_header[GRKIT_HOST_MODULE / 4] == 0);
    assert(test_header[GRKIT_HOST_MENU_CURSOR / 4] == 4);
    assert(test_header[GRKIT_HOST_PREVIEW / 4] == 0);
    /* Highlighting previews in the live view without committing. Off previews native. */
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 2;
    highlight(test_header);
    assert(test_header[GRKIT_HOST_PREVIEW / 4] == 0x59005000 + GRKIT_RECORD_SIZE);
    assert(test_header[GRKIT_HOST_ACTIVE / 4] == 0x59005000 + 3 * GRKIT_RECORD_SIZE);
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 0;
    highlight(test_header);
    assert(test_header[GRKIT_HOST_PREVIEW / 4] == 1);
    assert(test_header[GRKIT_HOST_ACTIVE / 4] == 0x59005000 + 3 * GRKIT_RECORD_SIZE);
    /* MENU restores the committed effect. */
    assert(menu_back(test_header));
    assert(test_header[GRKIT_HOST_PREVIEW / 4] == 0);
    assert(test_header[GRKIT_HOST_ACTIVE / 4] == 0x59005000 + 3 * GRKIT_RECORD_SIZE);
    /* OK commits the highlighted row and ends the preview. */
    menu_confirm(test_header);
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 1;
    highlight(test_header);
    assert(test_header[GRKIT_HOST_PREVIEW / 4] == 0x59005000);
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_ACTIVE / 4] == 0x59005000);
    assert(test_header[GRKIT_HOST_PREVIEW / 4] == 0);
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 0;
    highlight(test_header);
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_ACTIVE / 4] == 0);
    assert(test_header[GRKIT_HOST_PREVIEW / 4] == 0);
    /* Off opens Image Control at the Off row. */
    assert(menu_back(test_header));
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_MENU_CURSOR / 4] == 0);
    /* Other levels and modules do not preview effects. */
    assert(menu_back(test_header));
    highlight(test_header);
    assert(test_header[GRKIT_HOST_PREVIEW / 4] == 0);
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 1;
    menu_confirm(test_header);
    highlight(test_header);
    assert(test_header[GRKIT_HOST_PREVIEW / 4] == 0);

    /* About is the last root row. Its summary gives the version, and OK on it
       opens no page and changes nothing. */
    test_header[GRKIT_HOST_COUNT / 4] = 5;
    test_header[GRKIT_HOST_PREVIEW / 4] = 0;
    menu_start(test_header);
    assert(count(test_header) == 6 && root_module(test_header, 5) == ABOUT_MODULE);
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 5;
    assert(same(current_module(test_header)->summary(test_header), "Version 0.0.0"));
    assert(PAGE_ICON(ABOUT_MODULE, 5) == GRKIT_ICON_VERSION);
    {
        u32 active = test_header[GRKIT_HOST_ACTIVE / 4], requests = framing_requests;
        highlight(test_header);
        menu_confirm(test_header);
        assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT && test_header[GRKIT_HOST_MENU_CURSOR / 4] == 5);
        assert(test_header[GRKIT_HOST_ACTIVE / 4] == active && framing_requests == requests);
        assert(test_header[GRKIT_HOST_PREVIEW / 4] == 0 && !parameters_row(test_header));
        assert(!menu_back(test_header));
    }
    /* An About page cannot stay open: it has no rows. */
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_MODULE;
    test_header[GRKIT_HOST_MODULE / 4] = ABOUT_MODULE;
    menu_normalize(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT && test_header[GRKIT_HOST_MENU_CURSOR / 4] == 0);

    /* A module page after GRKIT_HOST_MODULE_PAGE_COUNT has no root row. The
       others keep their order. */
    test_header[GRKIT_HOST_PREVIEW / 4] = 0;
    test_header[GRKIT_HOST_FRAME_RATIO / 4] = 0;
    test_header[GRKIT_HOST_MODULE_PAGE_COUNT / 4] = 3;
    menu_start(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT && test_header[GRKIT_HOST_MENU_CURSOR / 4] == 0 && count(test_header) == 4);
    assert(current_module(test_header) == &MODULE(IMAGE_MODULE));
    assert(root_module(test_header, 1) == FRAMING_MODULE);
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 1;
    assert(same(current_module(test_header)->summary(test_header), "Framing: Off"));
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_MODULE && test_header[GRKIT_HOST_MODULE / 4] == FRAMING_MODULE);
    assert(menu_back(test_header));
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT && test_header[GRKIT_HOST_MENU_CURSOR / 4] == 1);
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 2;
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_MODULE / 4] == GUIDES_MODULE);
    assert(menu_back(test_header) && test_header[GRKIT_HOST_MENU_CURSOR / 4] == 2);
    assert(root_module(test_header, 3) == ABOUT_MODULE);
    /* The page of a module page that is gone does not stay open. */
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_MODULE;
    test_header[GRKIT_HOST_MODULE / 4] = DATE_MODULE;
    menu_normalize(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT && test_header[GRKIT_HOST_MENU_CURSOR / 4] == 0);
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_MODULE;
    test_header[GRKIT_HOST_MODULE / 4] = GRAIN_MODULE;
    menu_normalize(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT && test_header[GRKIT_HOST_MENU_CURSOR / 4] == 0);
    test_header[GRKIT_HOST_MODULE_PAGE_COUNT / 4] = ALL_PAGES;
    /* Without effects, the page of Image Control does not show. */
    test_header[GRKIT_HOST_COUNT / 4] = 0;
    assert(visible_modules(test_header) == (1U << FRAMING_MODULE | 1U << GUIDES_MODULE | 1U << DATE_MODULE |
                                            1U << GRAIN_MODULE | 1U << ABOUT_MODULE));
    menu_start(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT && count(test_header) == 5);
    assert(root_module(test_header, 0) == FRAMING_MODULE);
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 2;
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_MODULE / 4] == DATE_MODULE);
    assert(menu_back(test_header) && test_header[GRKIT_HOST_MENU_CURSOR / 4] == 2);
    /* visible_modules keeps the page that the caller selected. */
    module_at(test_header, GUIDES_MODULE);
    visible_modules(test_header);
    assert(test_header[GRKIT_HOST_MODULE_CHOICE / 4] == GUIDES_MODULE);
    /* A module page without its slot in GRKIT_HOST_MODULE_PAGE_COUNT does not
       show. Without a module page, the root shows About alone. The installer
       installs no menu hook then, so only an inconsistent header gets here. */
    test_header[GRKIT_HOST_MODULE_PAGE_COUNT / 4] = 0;
    assert(!(visible_modules(test_header) >> GUIDES_MODULE & 1));
    menu_start(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT && count(test_header) == 1);
    assert(current_module(test_header) == &MODULE(ABOUT_MODULE));
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT && !menu_back(test_header));
    /* Image Control alone keeps the root level. Its page opens at the committed
       effect, where Fn applies. */
    test_header[GRKIT_HOST_MODULE_PAGE_COUNT / 4] = 1;
    test_header[GRKIT_HOST_COUNT / 4] = 5;
    test_header[GRKIT_HOST_ACTIVE / 4] = 0x59005000 + 2 * GRKIT_RECORD_SIZE;
    menu_start(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT && count(test_header) == 2);
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_MODULE && test_header[GRKIT_HOST_MODULE / 4] == IMAGE_MODULE);
    assert(test_header[GRKIT_HOST_MENU_CURSOR / 4] == 3 && parameters_row(test_header));
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 1;
    highlight(test_header);
    assert(test_header[GRKIT_HOST_PREVIEW / 4] == 0x59005000);
    assert(menu_back(test_header));
    assert(test_header[GRKIT_HOST_PREVIEW / 4] == 0 && test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT);
    test_header[GRKIT_HOST_MODULE_PAGE_COUNT / 4] = ALL_PAGES;

    /* ADJ reopens at the root, on the row of the last module. A module without
       a row, or a module that is not valid, reopens at the first row. */
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_MODULE;
    test_header[GRKIT_HOST_MODULE / 4] = GRAIN_MODULE;
    menu_reopen(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT && test_header[GRKIT_HOST_MENU_CURSOR / 4] == GRAIN_ROW);
    test_header[GRKIT_HOST_MODULE / 4] = ABOUT_MODULE;
    menu_reopen(test_header);
    assert(test_header[GRKIT_HOST_MENU_CURSOR / 4] == count(test_header) - 1);
    test_header[GRKIT_HOST_MODULE / 4] = GUIDES_MODULE;
    menu_reopen(test_header);
    assert(test_header[GRKIT_HOST_MENU_CURSOR / 4] == GUIDES_ROW);
    test_header[GRKIT_HOST_MODULE_PAGE_COUNT / 4] = 3;
    test_header[GRKIT_HOST_MODULE / 4] = DATE_MODULE;
    menu_reopen(test_header);
    assert(test_header[GRKIT_HOST_MENU_CURSOR / 4] == 0);
    test_header[GRKIT_HOST_MODULE_PAGE_COUNT / 4] = ALL_PAGES;
    test_header[GRKIT_HOST_MODULE / 4] = 999;
    menu_reopen(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT && test_header[GRKIT_HOST_MENU_CURSOR / 4] == 0);

    /* The settings page of Date Imprint: the date style, then the look. Only
       Date Imprint has one. */
    for (u32 module = 0; module < MODULE_TOTAL; ++module)
        assert(!MODULE(module).settings == (module != DATE_MODULE && module != MANAGE_MODULE && module != EFFECTS_MODULE));
    {
        const Setting *page = MODULE(DATE_MODULE).settings;
        volatile u32 *block = settings_block(test_header);
        const u32 view = 0x59100000;
        static const u32 row_texts[] = SETTINGS_ROW_TEXTS, pages[] = SETTINGS_PAGES;
        assert(same(page[0].label(test_header), "Date Style") && same(page[1].label(test_header), "Date Look"));
        assert(page[0].values(test_header) == 6 && page[1].values(test_header) == 3);
        for (u32 value = 0; value < 6; ++value)
            assert(same(page[0].value_text(test_header, value), stamp_names[value]));
        assert(same(page[1].value_text(test_header, 0), "Classic") && same(page[1].value_text(test_header, 2), "GR II"));
        /* Without a page the native table, methods, and text stay. */
        test_header[GRKIT_HOST_SETTINGS / 4] = 0;
        assert(menu_settings_kinds() == FW_SUBMENU_KINDS);
        assert(!menu_settings_count(0, 0, 1) && !menu_settings_apply(1, 0, 0) && !menu_settings_current(0, 0, 0));
        assert(!menu_text_override(view, SETTINGS_TITLE_TEXT));
        /* A page reads the private entry at the index of the borrowed one.
           Date Imprint has two rows, which settings_start writes as the row
           count of the entry. */
        test_header[GRKIT_HOST_SETTINGS / 4] = DATE_MODULE + 1;
        assert(settings_count(test_header, page) == 2);
        ((volatile u8 *)block)[SUBMENU_KIND_ROWS] = 2;
        assert(settings_page(test_header) == page);
        assert(menu_settings_kinds() + SETTINGS_SUBMENU_KIND * SUBMENU_KIND_SIZE == (u32)block);
        assert(menu_settings_count(0, 0, 0) == 7 && menu_settings_count(0, 0, 1) == 4);
        /* OK stores a value of the row's setting, and the page marks it. */
        DATE_STYLE = 0;
        DATE_LOOK = 0;
        assert(menu_settings_apply(4, 0, 0) == 1 && DATE_STYLE == 4);
        assert(menu_settings_apply(2, 0, 1) == 1 && DATE_LOOK == 2);
        assert(menu_settings_apply(3, 0, 1) == 1 && DATE_LOOK == 2);
        assert(menu_settings_current(4, 0, 0) == 2 && menu_settings_current(3, 0, 0) == 1);
        assert(menu_settings_current(2, 0, 1) == 2 && menu_settings_current(0, 0, 1) == 1);
        /* A stored look out of range reads as classic. */
        DATE_LOOK = 7;
        assert(menu_settings_current(0, 0, 1) == 2);
        /* The title and the row labels replace their text IDs, in the font of
           the menu language. A value view shows its value for the title ID. */
        for (u32 i = 0; i < SETTINGS_VIEW_SLOTS * 3; ++i) block[SETTINGS_VIEWS / 4 + i] = 0;
        block[SETTINGS_VIEWS / 4 + 3] = view;
        block[SETTINGS_VIEWS / 4 + 4] = 0x104;
        assert(menu_text_override(view, SETTINGS_TITLE_TEXT) == ((u32)(block + SETTINGS_VIEWS / 4 + 4) | 1U));
        assert(menu_text_override(view + 4, SETTINGS_TITLE_TEXT) == ((u32)(block + SETTINGS_LABELS / 4) | 1U));
        assert(menu_text_override(view, row_texts[0]) == ((u32)(block + SETTINGS_LABELS / 4 + 2) | 1U));
        assert(menu_text_override(view, row_texts[1]) == ((u32)(block + SETTINGS_LABELS / 4 + 4) | 1U));
        /* The label of a row past the rows of the page keeps its text. */
        assert(!menu_text_override(view, row_texts[2]));
        /* A change to the submenu or to its value pages keeps the page. Any
           other controller, such as ADJ (0x2000000), ends it. */
        menu_settings_controller(SETTINGS_SUBMENU);
        menu_settings_controller(pages[0]);
        menu_settings_controller(pages[1]);
        assert(test_header[GRKIT_HOST_SETTINGS / 4] == DATE_MODULE + 1);
        menu_settings_controller(0x2000000);
        assert(!test_header[GRKIT_HOST_SETTINGS / 4] && !menu_text_override(view, SETTINGS_TITLE_TEXT));
        /* A stored module without settings, or out of range, has no page. */
        test_header[GRKIT_HOST_SETTINGS / 4] = FRAMING_MODULE + 1;
        assert(!settings_page(test_header));
        test_header[GRKIT_HOST_SETTINGS / 4] = 99;
        assert(!settings_page(test_header) && menu_settings_kinds() == FW_SUBMENU_KINDS);
        test_header[GRKIT_HOST_SETTINGS / 4] = 0;
        DATE_STYLE = 0;
        DATE_LOOK = 0;
    }

    /* The Modules page lists Guides and Date Imprint, which loaded, and a
       module that did not link. Its settings follow the three module
       settings. */
    {
        _Static_assert(MANAGE_STRIDE == 44 && MANAGE_HEADER == 16, "The test list matches manage.h");
        const u32 roots = count(test_header);
        const Setting *page;
        u32 entry;
        test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_ROOT;
        assert(!(visible_modules(test_header) >> MANAGE_MODULE & 1));
        settings_memory[0] = 9;
        for (u32 i = 0; i < 3; ++i) {
            settings_memory[2 + (3 + 2 * i) * 3 + 1] = 0;
            settings_memory[2 + (3 + 2 * i) * 3 + 2] = 2;
            settings_memory[2 + (4 + 2 * i) * 3 + 1] = 0;
            settings_memory[2 + (4 + 2 * i) * 3 + 2] = MANAGE_MODULES + 1;
        }
        memset(manage_memory, 0, sizeof(manage_memory));
        manage_memory[0] = 3;
        manage_memory[1] = 3;
        entry = MANAGE_ENTRY(0);
        manage_memory[entry] = MANAGE_LOADED;
        manage_memory[entry + 1] = TEST_PAGE + 2 * TEST_PAGE_SIZE + 0x28;
        manage_memory[entry + 2] = 7;
        entry += 11;
        manage_memory[entry] = MANAGE_LOADED;
        manage_memory[entry + 1] = TEST_PAGE + 3 * TEST_PAGE_SIZE + 0x28;
        manage_memory[entry + 2] = MANAGE_NO_TILE;
        entry += 11;
        manage_memory[entry] = 52;
        manage_memory[entry + 2] = MANAGE_NO_TILE;
        ascii((unsigned short *)&manage_memory[entry + 3], "BROKEN (52)");
        test_header[GRKIT_HOST_MANAGE / 4] = TEST_MANAGE;
        /* The root gets the Modules row before About. */
        assert(count(test_header) == roots + 1);
        assert(root_module(test_header, roots - 1) == MANAGE_MODULE && root_module(test_header, roots) == ABOUT_MODULE);
        assert(same(MODULE(MANAGE_MODULE).summary(test_header), "Modules"));
        assert(PAGE_ICON(MANAGE_MODULE, 0) == GRKIT_ICON_MODULES);
        /* The page: the title of a loaded module, the name of another one. */
        test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_MODULE;
        test_header[GRKIT_HOST_MODULE / 4] = MANAGE_MODULE;
        assert(same(current_module(test_header)->title(test_header), "Modules"));
        assert(count(test_header) == 3);
        assert(same(MODULE(MANAGE_MODULE).label(test_header, 0), "Guides"));
        assert(same(MODULE(MANAGE_MODULE).label(test_header, 1), "Date Imprint"));
        assert(same(MODULE(MANAGE_MODULE).label(test_header, 2), "BROKEN (52)"));
        assert(PAGE_ICON(MANAGE_MODULE, 0) == GUIDE_TILE(0) && PAGE_ICON(MANAGE_MODULE, 1) == GRKIT_ICON_MODULES);
        for (u32 row = 0; row < 3; ++row) assert(MODULE(MANAGE_MODULE).checked(test_header, row));
        /* The settings of Date Imprint: Load, then Position. */
        manage_memory[3] = 1;
        test_header[GRKIT_HOST_SETTINGS / 4] = MANAGE_MODULE + 1;
        page = settings_page(test_header);
        assert(page == MODULE(MANAGE_MODULE).settings);
        assert(same(page[0].label(test_header), "Load") && same(page[1].label(test_header), "Position"));
        assert(menu_settings_count(0, 0, 0) == 3 && menu_settings_count(0, 0, 1) == 4);
        assert(same(page[0].value_text(test_header, 0), "On") && same(page[0].value_text(test_header, 1), "Off"));
        assert(same(page[1].value_text(test_header, 0), "1") && same(page[1].value_text(test_header, 2), "3"));
        assert(menu_settings_current(0, 0, 0) == 2 && menu_settings_current(1, 0, 1) == 2);
        /* Off applies at the next start: the row shows Off, and the root
           row says to restart. */
        assert(menu_settings_apply(1, 0, 0) == 1 && settings_memory[2 + 5 * 3 + 1] == 1);
        assert(!MODULE(MANAGE_MODULE).checked(test_header, 1) && PAGE_ICON(MANAGE_MODULE, 1) == GRKIT_ICON_OFF);
        assert(same(MODULE(MANAGE_MODULE).summary(test_header), "Modules: Restart to apply"));
        /* Position 1 moves Date Imprint first. Each module gets its row + 1. */
        assert(menu_settings_apply(0, 0, 1) == 1);
        assert(settings_memory[2 + 6 * 3 + 1] == 1 && settings_memory[2 + 4 * 3 + 1] == 2 && settings_memory[2 + 8 * 3 + 1] == 3);
        assert(same(MODULE(MANAGE_MODULE).label(test_header, 0), "Date Imprint"));
        assert(same(MODULE(MANAGE_MODULE).label(test_header, 1), "Guides"));
        assert(MODULE(MANAGE_MODULE).committed(test_header) == 0 && menu_settings_current(0, 0, 1) == 2);
        /* The last row, then back. A row past the end does nothing. */
        assert(menu_settings_apply(2, 0, 1) == 1 && same(MODULE(MANAGE_MODULE).label(test_header, 2), "Date Imprint"));
        assert(menu_settings_apply(3, 0, 1) == 1 && same(MODULE(MANAGE_MODULE).label(test_header, 2), "Date Imprint"));
        assert(menu_settings_apply(1, 0, 1) == 1 && menu_settings_apply(0, 0, 0) == 1);
        assert(same(MODULE(MANAGE_MODULE).label(test_header, 1), "Date Imprint"));
        /* The order and the Off choices of this start: no restart. */
        assert(same(MODULE(MANAGE_MODULE).summary(test_header), "Modules"));
        /* Safe mode shows in the root row. */
        manage_memory[2] = MANAGE_SAFE_MODE;
        assert(same(MODULE(MANAGE_MODULE).summary(test_header), "Modules: Safe mode"));
        /* A count past the list hides the row. */
        manage_memory[0] = MANAGE_MODULES + 1;
        test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_ROOT;
        assert(!(visible_modules(test_header) >> MANAGE_MODULE & 1));
        test_header[GRKIT_HOST_MANAGE / 4] = 0;
        test_header[GRKIT_HOST_SETTINGS / 4] = 0;
        settings_memory[0] = 3;
    }

    /* The Effects page: Portra 400 and Velvia 100 installed as records 0 and
       1, and Gold 200 Off. Their choices follow the settings of the Modules
       page. */
    {
        _Static_assert(EFFECTS_STRIDE == 44 && EFFECTS_HEADER == 12, "The test list matches effects.h");
        static const char *const effect_names[] = {"Portra 400", "Velvia 100", "Gold 200"};
        volatile unsigned *list = &manage_memory[0x100 / 4];
        const Setting *page;
        settings_memory[0] = 12;
        for (u32 i = 0; i < 3; ++i) {
            settings_memory[2 + (9 + i) * 3 + 1] = i == 2;
            settings_memory[2 + (9 + i) * 3 + 2] = EFFECT_CHOICE_VALUES;
            list[3 + i * 11] = i == 2 ? EFFECT_OFF : EFFECT_LOADED;
            list[3 + i * 11 + 1] = i == 2 ? EFFECTS_NO_RECORD : i;
            memset((void *)&list[3 + i * 11 + 2], 0, 36);
            ascii((unsigned short *)&list[3 + i * 11 + 2], effect_names[i]);
        }
        list[0] = 3; list[1] = 9; list[2] = 0;
        test_header[GRKIT_HOST_COUNT / 4] = 5;
        /* A module reads and sets CORE.TONE as a setting: the row of the
           active effect. */
        assert(setting_limit(test_header, GRKIT_CORE_TONE) == 6);
        setting_store(test_header, GRKIT_CORE_TONE, 2);
        assert(setting_value(test_header, GRKIT_CORE_TONE) == 2 && tone_value(test_header) == 2);
        setting_store(test_header, GRKIT_CORE_TONE, 6);
        assert(setting_value(test_header, GRKIT_CORE_TONE) == 2);
        setting_store(test_header, GRKIT_CORE_TONE, 0);
        assert(!setting_value(test_header, GRKIT_CORE_TONE) && !test_header[GRKIT_HOST_ACTIVE / 4]);
        /* Without a listed effect, Image Control has no Effects row. */
        test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_MODULE;
        test_header[GRKIT_HOST_MODULE / 4] = IMAGE_MODULE;
        assert(count(test_header) == 6);
        test_header[GRKIT_HOST_EFFECTS / 4] = TEST_EFFECTS;
        /* The last row of Image Control opens the Effects page. It shows the
           committed effect in the live view and has no Fn adjustments. */
        assert(count(test_header) == 7);
        test_header[GRKIT_HOST_MENU_CURSOR / 4] = 6;
        assert(effects_row_open(test_header));
        assert(same(MODULE(IMAGE_MODULE).label(test_header, 6), "Effects"));
        assert(PAGE_ICON(IMAGE_MODULE, 6) == GRKIT_ICON_MODULES && !MODULE(IMAGE_MODULE).checked(test_header, 6));
        assert(!MODULE(IMAGE_MODULE).parameters(test_header, 6) && MODULE(IMAGE_MODULE).parameters(test_header, 5));
        test_header[GRKIT_HOST_PREVIEW / 4] = 7;
        highlight(test_header);
        assert(test_header[GRKIT_HOST_PREVIEW / 4] == 0);
        test_header[GRKIT_HOST_MENU_CURSOR / 4] = 5;
        assert(!effects_row_open(test_header));
        /* The page: its rows, icons, and marks. It has no root row. */
        menu_open(test_header, EFFECTS_MODULE);
        menu_normalize(test_header);
        assert(test_header[GRKIT_HOST_MODULE / 4] == EFFECTS_MODULE && test_header[GRKIT_HOST_MENU_CURSOR / 4] == 0);
        assert(!(visible_modules(test_header) >> EFFECTS_MODULE & 1));
        assert(same(current_module(test_header)->title(test_header), "Effects") && count(test_header) == 3);
        for (u32 row = 0; row < 3; ++row) {
            assert(same(MODULE(EFFECTS_MODULE).label(test_header, row), effect_names[row]));
            assert(MODULE(EFFECTS_MODULE).checked(test_header, row) == (row != 2));
        }
        assert(PAGE_ICON(EFFECTS_MODULE, 0) == GRKIT_ICON_EFFECT(0) && PAGE_ICON(EFFECTS_MODULE, 2) == GRKIT_ICON_OFF);
        /* The settings of Portra 400: Load, then Order, whose values are moves. */
        test_header[GRKIT_HOST_SETTINGS / 4] = EFFECTS_MODULE + 1;
        page = settings_page(test_header);
        assert(page == MODULE(EFFECTS_MODULE).settings);
        assert(same(page[0].label(test_header), "Load") && same(page[1].label(test_header), "Order"));
        assert(same(page[1].value_text(test_header, 0), "Move up") && same(page[1].value_text(test_header, 1), "Move down"));
        assert(menu_settings_current(0, 0, 1) == 1 && menu_settings_current(1, 0, 1) == 1);
        /* Move down: Velvia 100 comes first, and each effect gets its row + 1. */
        assert(menu_settings_apply(1, 0, 1) == 1);
        assert(settings_memory[2 + 10 * 3 + 1] == 2 && settings_memory[2 + 9 * 3 + 1] == 4 && settings_memory[2 + 11 * 3 + 1] == 7);
        assert(same(MODULE(EFFECTS_MODULE).label(test_header, 0), "Velvia 100") && MODULE(EFFECTS_MODULE).committed(test_header) == 1);
        assert(same(MODULE(IMAGE_MODULE).label(test_header, 6), "Effects: Restart to apply"));
        /* The first row does not move up; back up restores the order. */
        assert(menu_settings_apply(0, 0, 1) == 1 && settings_memory[2 + 9 * 3 + 1] == 2);
        assert(menu_settings_apply(0, 0, 1) == 1 && settings_memory[2 + 9 * 3 + 1] == 2);
        assert(same(MODULE(IMAGE_MODULE).label(test_header, 6), "Effects"));
        /* Gold 200 On keeps its position. */
        list[2] = 2;
        assert(menu_settings_current(1, 0, 0) == 2 && menu_settings_apply(0, 0, 0) == 1);
        assert(settings_memory[2 + 11 * 3 + 1] == 6 && MODULE(EFFECTS_MODULE).checked(test_header, 2));
        assert(same(MODULE(IMAGE_MODULE).label(test_header, 6), "Effects: Restart to apply"));
        /* MENU returns to the Effects row of Image Control. */
        test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_MODULE;
        test_header[GRKIT_HOST_MODULE / 4] = EFFECTS_MODULE;
        assert(menu_back(test_header));
        assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_MODULE && test_header[GRKIT_HOST_MODULE / 4] == IMAGE_MODULE);
        assert(test_header[GRKIT_HOST_MENU_CURSOR / 4] == 6);
        /* Without listed effects, an open Effects page returns to the root. */
        test_header[GRKIT_HOST_MODULE / 4] = EFFECTS_MODULE;
        test_header[GRKIT_HOST_EFFECTS / 4] = 0;
        menu_normalize(test_header);
        assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT);
        test_header[GRKIT_HOST_SETTINGS / 4] = 0;
        settings_memory[0] = 3;
    }
    return 0;
}

#include <assert.h>
#include <string.h>
/* The build's label table, which the Rust test writes for this harness. */
#include "menu_text.h"
static unsigned framing_requests;
#define MENU_FRAMING_REQUEST() (++framing_requests)
/* Camera addresses of the menu text map to text_arena, relative to the text
   start of the header, so the count of records can change during the test. */
static const unsigned short *text_at(unsigned int address);
#define MENU_TEXT(address) text_at(address)
static unsigned test_language;
#define MENU_LANGUAGE() test_language
#include "menu.c"
#define ALL_PLUGINS ((1U << PLUGIN_COUNT) - 1)
/* The global icon layout of the plugin tiles, from PLUGIN_ICON. */
#define FRAMING_RATIO_ICON(ratio) (PLUGIN_ICON + (ratio))
#define GUIDES_ICON (PLUGIN_ICON + FRAMING_TILES)
#define GUIDE_STYLE_ICON(style) (GUIDES_ICON + (style))
#define STAMP_ICON (GUIDES_ICON + GUIDES_TILES)
#define STAMP_STYLE_ICON(style) (STAMP_ICON + (style))
#define GRAIN_ICON (STAMP_ICON + DATE_STAMP_TILES)
#define GRAIN_LEVEL_ICON(level) (GRAIN_ICON + (level))
/* A page icon as its global ID. */
#define PAGE_ICON(module, row) module_icon((module), modules[(module)].icon(test_header, (row)))
#define CURRENT_ICON(row) module_icon(current_index(test_header), current_module(test_header)->icon(test_header, (row)))

static volatile u32 test_header[GRKIT_HOST_HEADER_SIZE / 4];
static u16 text_arena[GRKIT_TEXT_SIZE / 2];
static const u16 *text_at(u32 address)
{
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
static const char *const ratio_names[] = {"Off", "XPan 65:24", "16:9", "4:3"};
static const char *const guide_names[] = {"Off", "Thirds", "Center", "Golden Ratio", "Diagonals", "Thirds + Center"};
static const char *const stamp_names[] = {"Off", "'YY M D", "'YY M D H:MM", "M D 'YY", "D M 'YY", "D M 'YY H:MM"};
static const char *const grain_names[] = {"Off", "Weak", "Medium", "Strong"};
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
    /* Every plugin is installed (PLUGINS.TXT is missing). */
    test_header[GRKIT_HOST_PLUGINS / 4] = ALL_PLUGINS;
    test_header[GRKIT_HOST_OFF_ICON / 4] = 0x59001000;
    test_header[GRKIT_HOST_PLUGINS_ICON / 4] = 0x59003000;
    test_header[GRKIT_HOST_RECORDS / 4] = 0x59005000;
    assert(selected_icon(test_header) == 0x7fff);
    descriptor(slot, modules[0].summary(test_header));
    assert(same(modules[0].summary(test_header), "Image Control: Off") && slot[0] == 0x113);
    /* Each language reads its own strings, and an unknown language reads English. */
    for (u32 language = 0; language < GRKIT_TEXT_LANGUAGES + 2; ++language) {
        u32 table = language < GRKIT_TEXT_LANGUAGES ? language : 0;
        test_language = language;
        for (u32 id = 0; id < GRKIT_TEXT_STRINGS; ++id)
            assert(text(test_header, id) == text_arena + text_arena[table * GRKIT_TEXT_STRINGS + id] / 2);
    }
    test_language = 2;
    assert(!same(text(test_header, TEXT_FRAMING), "Framing"));
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
        assert(modules[0].summary(test_header) ==
               text_arena + (GRKIT_TEXT_SUMMARY + (language * GRKIT_TEXT_RECORDS + 3) * GRKIT_SUMMARY_UNITS * 2) / 2);
    }
    test_language = 0;
    /* x * 2260 >> 16 equals x / 29 for each record offset / 64. */
    for (u32 index = 0; index < GRKIT_TEXT_RECORDS; ++index)
        assert(((index * GRKIT_RECORD_SIZE) >> 6) * 2260U >> 16 == index);
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
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = 0;
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 0;
    assert(menu_press(2, 0));
    assert(!menu_press(1, 0));
    assert(!menu_press(2, 1));
    assert(!menu_press(2, 100));
    assert(back_event(test_header, 5, 1) == 1);
    assert(back_event(test_header, 5, 1) == 2);
    assert(back_event(test_header, 0x19d, 1) == 0);
    assert(back_event(test_header, 0xd8, 0) == 2);
    assert(back_event(test_header, 0xd8, 0) == 0);
    assert(back_event(test_header, 5, 0) == 0);
    assert(back_event(test_header, 5, 1) == 1);
    assert(back_event(test_header, 8, 1) == 0);
    assert(back_event(test_header, 0xd8, 1) == 0);
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
        const u16 confirm[] = {0xd8};
        test_header[GRKIT_HOST_MENU_LEVEL / 4] = 0;
        test_header[GRKIT_HOST_BACK_PENDING / 4] = 1;
        assert(menu_event(0x1000, confirm) == 0);
        assert(native_events == 0);
        assert(test_header[GRKIT_HOST_BACK_PENDING / 4] == 0);
        assert(test_header[GRKIT_HOST_ACTIVE / 4] == 0x59005000);
    }
    /* The event ring distinguishes MENU's confirm from OK and ADJ press confirms. */
    test_header[GRKIT_HOST_MENU_EVENTS / 4] = 18;
    test_header[GRKIT_HOST_MENU_EVENT_RING / 4] = 5 | (2 << 16);
    test_header[GRKIT_HOST_MENU_EVENT_RING / 4 + 1] = 0xd8;
    assert(after_menu_switch(test_header));
    test_header[GRKIT_HOST_MENU_EVENT_RING / 4] = 8;
    assert(!after_menu_switch(test_header));
    test_header[GRKIT_HOST_MENU_EVENT_RING / 4] = 0x1e;
    assert(!after_menu_switch(test_header));
    test_header[GRKIT_HOST_MENU_EVENTS / 4] = 0;
    test_header[GRKIT_HOST_MENU_EVENT_RING / 4] = test_header[GRKIT_HOST_MENU_EVENT_RING / 4 + 1] = 0;
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
    assert(toolbar_x(0) == 415 && toolbar_x(5) == 665);
    assert(menu_icon(0x7fff) == 0x59001000);
    assert(menu_icon(0x166) == 0);
    assert(menu_icon(0x8005) == 0);
    assert(menu_icon(0xffff) == 0);
    assert(wrapped_edge(0xcf, 3, 0, 0) == 2);
    assert(wrapped_edge(0xce, 1, 4, 0) == 1);
    assert(wrapped_edge(0xcf, 1, 2, 0) == 0);
    assert(wrapped_edge(0xce, 3, 2, 0) == 0);
    assert(wrapped_edge(0xcf, 4, 0, 1) == 0);
    assert(wrapped_edge(0x35, 4, 0, 0) == 0);
    assert(advance_from_private(0xcf, 1) == 1);
    assert(advance_from_private(0xce, 1) == 0);
    assert(advance_from_private(0xcf, 2) == 0);
    assert(advance_from_private(0xce, 2) == 1);
    /* The root lists the five plugins, then About. */
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = 1;
    assert(count(test_header) == 6);
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = 2;
    assert(count(test_header) == 6);
    test_header[GRKIT_HOST_COUNT / 4] = 0;
    assert(count(test_header) == 1);
    /* Registry routing leaves the existing effect record untouched. */
    test_header[GRKIT_HOST_ACTIVE / 4] = 0x59005000;
    test_header[GRKIT_HOST_COUNT / 4] = 5;
    test_header[GRKIT_HOST_TILE_ICON / 4] = 0x59007000;
    /* Framing tiles are contiguous after the module tile. */
    assert(menu_icon(PLUGIN_ICON) == 0x59007000);
    assert(menu_icon(FRAMING_RATIO_ICON(3)) == 0x59007000 + 3 * GRKIT_ICON_STRIDE);
    /* Guides tiles follow the Framing tiles. */
    assert(menu_icon(GUIDES_ICON) == 0x59007000 + FRAMING_TILES * GRKIT_ICON_STRIDE);
    assert(menu_icon(GUIDE_STYLE_ICON(5)) == 0x59007000 + 9 * GRKIT_ICON_STRIDE);
    /* Module tiles fill 0x7fea..0x7ffd; the GRKit tile (0x7ffe) follows them. */
    assert(menu_icon(PLUGIN_ICON - 1) == 0);
    assert(PLUGIN_ICON + PLUGIN_TILES <= 0x7ffe);
    assert(module_icon_count() == PLUGIN_TILES);
    /* Date Stamp tiles follow the Guides tiles. */
    assert(menu_icon(STAMP_ICON) == 0x59007000 + 10 * GRKIT_ICON_STRIDE);
    assert(menu_icon(STAMP_STYLE_ICON(5)) == 0x59007000 + 15 * GRKIT_ICON_STRIDE);
    /* Grain tiles follow the Date Stamp tiles and end below the GRKit tile. */
    assert(menu_icon(GRAIN_ICON) == 0x59007000 + 16 * GRKIT_ICON_STRIDE);
    assert(menu_icon(GRAIN_LEVEL_ICON(3)) == 0x59007000 + 19 * GRKIT_ICON_STRIDE);
    assert(GRAIN_LEVEL_ICON(3) == 0x7ffd);
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_ROOT;
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 1;
    assert(current_module(test_header) == &modules[1]);
    assert(same(current_module(test_header)->summary(test_header), "Framing: Off"));
    assert(PAGE_ICON(1, 1) == PLUGIN_ICON);
    test_header[GRKIT_HOST_FRAME_RATIO / 4] = 2;
    assert(PAGE_ICON(1, 1) == FRAMING_RATIO_ICON(2));
    /* Framing lists the ratios directly and opens at the selected one. */
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_MODULE && test_header[GRKIT_HOST_MENU_CURSOR / 4] == 2);
    assert(count(test_header) == 4);
    assert(same(current_module(test_header)->title(test_header), "Framing"));
    for (u32 row = 0; row < 4; ++row) {
        test_header[GRKIT_HOST_MENU_CURSOR / 4] = row;
        menu_confirm(test_header);
        assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_MODULE);
        assert(test_header[GRKIT_HOST_FRAME_RATIO / 4] == row);
        assert(framing_requests == row + 1);
        assert(current_module(test_header)->checked(test_header, row));
        assert(!current_module(test_header)->checked(test_header, (row + 1) % 4));
        assert(same(current_module(test_header)->label(test_header, row), ratio_names[row]));
        assert(CURRENT_ICON(row) == (row ? FRAMING_RATIO_ICON(row) : 0x7fff));
        assert(test_header[GRKIT_HOST_ACTIVE / 4] == 0x59005000);
    }
    /* The Framing page shows its highlighted row on the ADJ canvas. */
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 1;
    frame_preview(test_header, 0x5a000000, 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == 2);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW_CANVAS / 4] == 0x5a000000);
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 0;
    frame_preview(test_header, 0x5a000000, 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == 1);
    frame_preview(test_header, 0x5a000000, 1);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == 0);
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 3;
    assert(menu_back(test_header));
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT && test_header[GRKIT_HOST_MENU_CURSOR / 4] == 1);
    /* Other ADJ pages show the committed ratio, here 4:3. */
    frame_preview(test_header, 0x5b000000, 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == 4);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW_CANVAS / 4] == 0x5b000000);
    /* A committed Off ends the mask but keeps the canvas for one erasing draw. */
    test_header[GRKIT_HOST_FRAME_RATIO / 4] = 0;
    frame_preview(test_header, 0x5b000000, 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW_CANVAS / 4] == 0x5b000000);
    test_header[GRKIT_HOST_FRAME_RATIO / 4] = 3;
    test_header[GRKIT_HOST_FRAME_PREVIEW_CANVAS / 4] = 0;
    assert(!menu_back(test_header));
    assert(same(current_module(test_header)->summary(test_header), "Framing: 4:3"));
    /* Guides opens at the committed style and commits only its own setting. */
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_ROOT;
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 2;
    assert(same(current_module(test_header)->summary(test_header), "Guides: Off"));
    assert(PAGE_ICON(2, 2) == GUIDES_ICON);
    test_header[GRKIT_HOST_GUIDE / 4] = 3;
    assert(PAGE_ICON(2, 2) == GUIDE_STYLE_ICON(3));
    {
        u32 requests = framing_requests;
        menu_confirm(test_header);
        assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_MODULE && test_header[GRKIT_HOST_MODULE / 4] == PLUGIN_GUIDES);
        assert(test_header[GRKIT_HOST_MENU_CURSOR / 4] == 3);
        assert(count(test_header) == 6);
        assert(same(current_module(test_header)->title(test_header), "Guides"));
        for (u32 row = 0; row < 6; ++row) {
            test_header[GRKIT_HOST_MENU_CURSOR / 4] = row;
            menu_confirm(test_header);
            assert(test_header[GRKIT_HOST_GUIDE / 4] == row);
            assert(current_module(test_header)->checked(test_header, row));
            assert(!current_module(test_header)->checked(test_header, (row + 1) % 6));
            assert(same(current_module(test_header)->label(test_header, row), guide_names[row]));
            assert(CURRENT_ICON(row) == (row ? GUIDE_STYLE_ICON(row) : 0x7fff));
            assert(test_header[GRKIT_HOST_FRAME_RATIO / 4] == 3);
            assert(test_header[GRKIT_HOST_ACTIVE / 4] == 0x59005000);
        }
        assert(framing_requests == requests);
    }
    /* The Guides page previews its highlighted style inside the committed ratio. */
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 2;
    frame_preview(test_header, 0x5a000000, 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == 4);
    assert(test_header[GRKIT_HOST_GUIDE_PREVIEW / 4] == 2);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW_CANVAS / 4] == 0x5a000000);
    /* A highlighted Off with no ratio keeps the canvas for the next row. */
    test_header[GRKIT_HOST_FRAME_RATIO / 4] = 0;
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 0;
    frame_preview(test_header, 0x5a000000, 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == 1);
    assert(test_header[GRKIT_HOST_GUIDE_PREVIEW / 4] == 0);
    assert(menu_back(test_header));
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT && test_header[GRKIT_HOST_MENU_CURSOR / 4] == 2);
    /* Other ADJ pages show the committed style, even without a ratio. */
    test_header[GRKIT_HOST_GUIDE / 4] = 4;
    frame_preview(test_header, 0x5b000000, 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == 1);
    assert(test_header[GRKIT_HOST_GUIDE_PREVIEW / 4] == 4);
    /* The Framing page keeps the committed style with its highlighted ratio. */
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 1;
    menu_confirm(test_header);
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 2;
    frame_preview(test_header, 0x5b000000, 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == 3);
    assert(test_header[GRKIT_HOST_GUIDE_PREVIEW / 4] == 4);
    /* Committed Off for both ends the drawing. */
    assert(menu_back(test_header));
    test_header[GRKIT_HOST_GUIDE / 4] = 0;
    frame_preview(test_header, 0x5b000000, 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == 0);
    test_header[GRKIT_HOST_GUIDE / 4] = 999;
    assert(guide_style(test_header) == 0);
    test_header[GRKIT_HOST_GUIDE / 4] = 0;
    test_header[GRKIT_HOST_FRAME_RATIO / 4] = 3;
    test_header[GRKIT_HOST_FRAME_PREVIEW_CANVAS / 4] = 0;
    /* Date Stamp lists its styles, opens at the committed one, and changes nothing else. */
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_ROOT;
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 3;
    assert(same(current_module(test_header)->summary(test_header), "Date Stamp: Off"));
    assert(PAGE_ICON(3, 3) == STAMP_ICON);
    test_header[GRKIT_HOST_STAMP / 4] = 2;
    assert(PAGE_ICON(3, 3) == STAMP_STYLE_ICON(2));
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_MODULE && test_header[GRKIT_HOST_MODULE / 4] == PLUGIN_DATE_STAMP);
    assert(test_header[GRKIT_HOST_MENU_CURSOR / 4] == 2 && count(test_header) == 6);
    assert(same(current_module(test_header)->title(test_header), "Date Stamp"));
    for (u32 row = 0; row < 6; ++row) {
        test_header[GRKIT_HOST_MENU_CURSOR / 4] = row;
        menu_confirm(test_header);
        assert(test_header[GRKIT_HOST_STAMP / 4] == row);
        assert(current_module(test_header)->checked(test_header, row));
        assert(same(current_module(test_header)->label(test_header, row), stamp_names[row]));
        assert(CURRENT_ICON(row) == (row ? STAMP_STYLE_ICON(row) : 0x7fff));
        assert(test_header[GRKIT_HOST_FRAME_RATIO / 4] == 3 && test_header[GRKIT_HOST_GUIDE / 4] == 0);
        assert(test_header[GRKIT_HOST_ACTIVE / 4] == 0x59005000);
    }
    /* The Date Stamp page keeps the committed mask and guides on the ADJ canvas. */
    frame_preview(test_header, 0x5c000000, 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == 4 && test_header[GRKIT_HOST_GUIDE_PREVIEW / 4] == 0);
    assert(menu_back(test_header) && test_header[GRKIT_HOST_MENU_CURSOR / 4] == 3);
    assert(same(current_module(test_header)->summary(test_header), "Date Stamp: D M 'YY H:MM"));
    test_header[GRKIT_HOST_STAMP / 4] = 999;
    assert(stamp_style(test_header) == 0);
    test_header[GRKIT_HOST_STAMP / 4] = 0;
    test_header[GRKIT_HOST_FRAME_PREVIEW_CANVAS / 4] = 0;
    /* Grain lists its levels, opens at the committed one, and changes nothing else. */
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_ROOT;
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 4;
    assert(same(current_module(test_header)->summary(test_header), "Grain: Off"));
    assert(PAGE_ICON(PLUGIN_GRAIN, 4) == GRAIN_ICON);
    test_header[GRKIT_PLUGIN_GRAIN_LEVEL / 4] = 2;
    assert(PAGE_ICON(PLUGIN_GRAIN, 4) == GRAIN_LEVEL_ICON(2));
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_MODULE && test_header[GRKIT_HOST_MODULE / 4] == PLUGIN_GRAIN);
    assert(test_header[GRKIT_HOST_MENU_CURSOR / 4] == 2 && count(test_header) == 4);
    assert(same(current_module(test_header)->title(test_header), "Grain"));
    for (u32 row = 0; row < 4; ++row) {
        test_header[GRKIT_HOST_MENU_CURSOR / 4] = row;
        menu_confirm(test_header);
        assert(test_header[GRKIT_PLUGIN_GRAIN_LEVEL / 4] == row);
        assert(current_module(test_header)->checked(test_header, row));
        assert(same(current_module(test_header)->label(test_header, row), grain_names[row]));
        assert(CURRENT_ICON(row) == (row ? GRAIN_LEVEL_ICON(row) : 0x7fff));
        assert(test_header[GRKIT_HOST_FRAME_RATIO / 4] == 3 && test_header[GRKIT_HOST_STAMP / 4] == 0);
        assert(test_header[GRKIT_HOST_ACTIVE / 4] == 0x59005000);
    }
    assert(menu_back(test_header) && test_header[GRKIT_HOST_MENU_CURSOR / 4] == 4);
    assert(same(current_module(test_header)->summary(test_header), "Grain: Strong"));
    test_header[GRKIT_PLUGIN_GRAIN_LEVEL / 4] = 999;
    assert(grain_level(test_header) == 0);
    test_header[GRKIT_PLUGIN_GRAIN_LEVEL / 4] = 0;
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
    assert(test_header[GRKIT_HOST_FRAME_RATIO / 4] == 3);
    assert(menu_back(test_header));
    assert(test_header[GRKIT_HOST_MENU_CURSOR / 4] == 0);
    test_header[GRKIT_HOST_MODULE / 4] = 999;
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_MODULE + 1;
    menu_normalize(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT && test_header[GRKIT_HOST_MODULE / 4] == 0);
    test_header[GRKIT_HOST_FRAME_RATIO / 4] = 999;
    assert(frame_ratio(test_header) == 0);
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
    test_header[GRKIT_HOST_PLUGINS / 4] = ALL_PLUGINS;
    test_header[GRKIT_HOST_COUNT / 4] = 5;
    test_header[GRKIT_HOST_PREVIEW / 4] = 0;
    menu_start(test_header);
    assert(count(test_header) == 6 && root_module(test_header, 5) == ABOUT_MODULE);
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 5;
    assert(same(current_module(test_header)->summary(test_header), "Version 0.0.0"));
    assert(PAGE_ICON(ABOUT_MODULE, 5) == GRKIT_ICON_GRKIT);
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

    /* A plugin that is not installed has no root row. The others keep their order. */
    test_header[GRKIT_HOST_PREVIEW / 4] = 0;
    test_header[GRKIT_HOST_FRAME_RATIO / 4] = 0;
    test_header[GRKIT_HOST_PLUGINS / 4] = ALL_PLUGINS & ~(1U << PLUGIN_GUIDES);
    menu_start(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT && test_header[GRKIT_HOST_MENU_CURSOR / 4] == 0 && count(test_header) == 5);
    assert(current_module(test_header) == &modules[0]);
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 2;
    assert(current_module(test_header) == &modules[PLUGIN_DATE_STAMP]);
    assert(same(current_module(test_header)->summary(test_header), "Date Stamp: Off"));
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_MODULE && test_header[GRKIT_HOST_MODULE / 4] == PLUGIN_DATE_STAMP);
    assert(menu_back(test_header));
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT && test_header[GRKIT_HOST_MENU_CURSOR / 4] == 2);
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 1;
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_MODULE / 4] == PLUGIN_FRAMING);
    assert(menu_back(test_header) && test_header[GRKIT_HOST_MENU_CURSOR / 4] == 1);
    /* The page of a plugin that is not installed does not stay open. */
    test_header[GRKIT_HOST_MENU_LEVEL / 4] = MENU_MODULE;
    test_header[GRKIT_HOST_MODULE / 4] = PLUGIN_GUIDES;
    menu_normalize(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT && test_header[GRKIT_HOST_MENU_CURSOR / 4] == 0);
    /* Image Control that is not installed does not show, even with effects. */
    test_header[GRKIT_HOST_PLUGINS / 4] = ALL_PLUGINS & ~(1U | 1U << PLUGIN_FRAMING);
    assert(visible_modules(test_header) == (1U << PLUGIN_GUIDES | 1U << PLUGIN_DATE_STAMP | 1U << PLUGIN_GRAIN | 1U << ABOUT_MODULE));
    /* Without effects, Image Control does not show. */
    test_header[GRKIT_HOST_PLUGINS / 4] = ALL_PLUGINS;
    test_header[GRKIT_HOST_COUNT / 4] = 0;
    menu_start(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT && count(test_header) == 5);
    assert(current_module(test_header) == &modules[PLUGIN_FRAMING]);
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 2;
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_MODULE / 4] == PLUGIN_DATE_STAMP);
    assert(menu_back(test_header) && test_header[GRKIT_HOST_MENU_CURSOR / 4] == 2);
    /* One installed plugin keeps the root level, because About follows it. */
    test_header[GRKIT_HOST_PLUGINS / 4] = 1U | 1U << PLUGIN_DATE_STAMP;
    test_header[GRKIT_HOST_STAMP / 4] = 3;
    menu_start(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT && count(test_header) == 2);
    assert(root_module(test_header, 0) == PLUGIN_DATE_STAMP && root_module(test_header, 1) == ABOUT_MODULE);
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_MODULE && test_header[GRKIT_HOST_MODULE / 4] == PLUGIN_DATE_STAMP);
    assert(test_header[GRKIT_HOST_MENU_CURSOR / 4] == 3 && count(test_header) == 6);
    assert(menu_back(test_header) && test_header[GRKIT_HOST_MENU_CURSOR / 4] == 0);
    assert(!menu_back(test_header));
    test_header[GRKIT_HOST_STAMP / 4] = 0;
    /* With no plugin visible, the root shows About alone. The installer installs
       no menu hook then, so only an inconsistent header gets here. */
    test_header[GRKIT_HOST_PLUGINS / 4] = 1U;
    menu_start(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT && count(test_header) == 1);
    assert(current_module(test_header) == &modules[ABOUT_MODULE]);
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT && !menu_back(test_header));
    /* Image Control alone keeps the root level. Its page opens at the committed
       effect, where Fn applies. */
    test_header[GRKIT_HOST_COUNT / 4] = 5;
    test_header[GRKIT_HOST_ACTIVE / 4] = 0x59005000 + 2 * GRKIT_RECORD_SIZE;
    menu_start(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT && count(test_header) == 2);
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_MODULE && test_header[GRKIT_HOST_MODULE / 4] == 0);
    assert(test_header[GRKIT_HOST_MENU_CURSOR / 4] == 3 && parameters_row(test_header));
    test_header[GRKIT_HOST_MENU_CURSOR / 4] = 1;
    highlight(test_header);
    assert(test_header[GRKIT_HOST_PREVIEW / 4] == 0x59005000);
    assert(menu_back(test_header));
    assert(test_header[GRKIT_HOST_PREVIEW / 4] == 0 && test_header[GRKIT_HOST_MENU_LEVEL / 4] == MENU_ROOT);
    return 0;
}

/* The host test of Guides (grkit module test). The core checks the guides
   inside every Framing area (tests/runtime/framing_contract.c); this test
   checks the module alone, on the full 3:2 area. */
#include <assert.h>
#include "grkit_module.h"

void guides_paint(const Surface *s, u32 style);
void guides_paint_always(const Surface *s, u32 style);

static const u16 full[4] = {0, 0, 720, 480};

/* Whether the 2-pixel line at `line` covers `value`. */
static int on_line(u32 value, u32 line) { return value == line - 1 || value == line; }

int main(void)
{
    Surface s = grkit_test_surface(full);
    /* The descriptor: STYLE with 6 values, COLOR, OPACITY, and DISP, a page
       with their settings rows, and the two layers. */
    assert(grkit_module.setting_count == 4 && grkit_module.settings[0].values == 6);
    assert(grkit_module.registration_count == 3 && grkit_module.tile_count == 6);
    assert(SETTING_STYLE == GRKIT_SETTING(0) && SETTING_COLOR == GRKIT_SETTING(1) &&
           SETTING_OPACITY == GRKIT_SETTING(2) && SETTING_DISP == GRKIT_SETTING(3));
    /* Thirds: lines at 240, 480 and 160, 320, black at 50% alpha. */
    grkit_test_clear();
    guides_paint(&s, 1);
    for (u32 y = 0; y < 480; y += 7)
        for (u32 x = 0; x < 720; x += 5) {
            int line = on_line(x, 240) || on_line(x, 480) || on_line(y, 160) || on_line(y, 320);
            assert(grkit_test_pixel(x, y) == (line ? 0x80000000U : GRKIT_TEST_CLEAR));
        }
    /* Center: a cross of 2-pixel arms, 20 pixels past the center pair. */
    grkit_test_clear();
    guides_paint(&s, 2);
    assert(grkit_test_pixel(339, 240) == 0x80000000U && grkit_test_pixel(380, 239) == 0x80000000U);
    assert(grkit_test_pixel(338, 240) == GRKIT_TEST_CLEAR && grkit_test_pixel(381, 240) == GRKIT_TEST_CLEAR);
    assert(grkit_test_pixel(360, 219) == 0x80000000U && grkit_test_pixel(359, 260) == 0x80000000U);
    assert(grkit_test_pixel(240, 160) == GRKIT_TEST_CLEAR);
    /* Diagonals run corner to corner. */
    grkit_test_clear();
    guides_paint(&s, 4);
    assert(grkit_test_pixel(0, 0) == 0x80000000U && grkit_test_pixel(719, 478) == 0x80000000U);
    assert(grkit_test_pixel(719, 0) == 0x80000000U && grkit_test_pixel(0, 478) == 0x80000000U);
    /* White at 100%, and yellow at 50%. */
    grkit_test_settings[SETTING_COLOR] = 1;
    grkit_test_settings[SETTING_OPACITY] = 1;
    grkit_test_clear();
    guides_paint(&s, 1);
    assert(grkit_test_pixel(240, 0) == 0xffffffffU);
    grkit_test_settings[SETTING_COLOR] = 2;
    grkit_test_settings[SETTING_OPACITY] = 0;
    grkit_test_clear();
    guides_paint(&s, 1);
    /* Red in the low byte: the native yellow is red ff, green e1, blue 00. */
    assert(grkit_test_pixel(240, 0) == 0x8000e1ffU);
    grkit_test_settings[SETTING_COLOR] = 0;
    /* DISP 0: only the layer that follows DISP paints; DISP 1: only the
       other one. */
    grkit_test_clear();
    guides_paint_always(&s, 1);
    assert(grkit_test_pixel(240, 0) == GRKIT_TEST_CLEAR);
    grkit_test_settings[SETTING_DISP] = 1;
    guides_paint(&s, 1);
    assert(grkit_test_pixel(240, 0) == GRKIT_TEST_CLEAR);
    guides_paint_always(&s, 1);
    assert(grkit_test_pixel(240, 0) == 0x80000000U);
    grkit_test_settings[SETTING_DISP] = 0;
    /* An unknown style draws nothing. */
    grkit_test_clear();
    guides_paint(&s, 6);
    for (u32 y = 0; y < 480; ++y)
        for (u32 x = 0; x < 720; ++x) assert(grkit_test_pixel(x, y) == GRKIT_TEST_CLEAR);
    return 0;
}

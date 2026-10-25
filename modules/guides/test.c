/* The host test of Guides (grkit module test). The core checks the guides
   inside every Framing area (tests/runtime/framing_contract.c); this test
   checks the module alone, on the full 3:2 area. */
#include <assert.h>
#include "grkit_module.h"

void guides_paint(const Surface *s, u32 style);

static const u16 full[4] = {0, 0, 720, 480};

/* Whether the 2-pixel line at `line` covers `value`. */
static int on_line(u32 value, u32 line) { return value == line - 1 || value == line; }

int main(void)
{
    Surface s = grkit_test_surface(full);
    /* The descriptor: the STYLE setting with 6 values, a page and a layer. */
    assert(grkit_module.setting_count == 1 && grkit_module.settings[0].values == 6);
    assert(grkit_module.registration_count == 2 && grkit_module.tile_count == 6);
    assert(SETTING_STYLE == 0);
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
    /* An unknown style draws nothing. */
    grkit_test_clear();
    guides_paint(&s, 6);
    for (u32 y = 0; y < 480; ++y)
        for (u32 x = 0; x < 720; ++x) assert(grkit_test_pixel(x, y) == GRKIT_TEST_CLEAR);
    return 0;
}

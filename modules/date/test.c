/* The host test of Date Imprint (grkit module test): the stamp on a gray
   picture. tests/runtime/picture_contract.c checks the stamp in detail. */
#include <assert.h>
#include "grkit_module.h"

void date_apply(PictureSet *set, u32 style);

#define WIDTH 1200U
#define HEIGHT 800U
/* A 4:2:2 plane: luma rows, then interleaved CbCr rows of the same pitch. */
static Plane gray_plane(u32 at)
{
    Plane p = {at, at + WIDTH * HEIGHT, WIDTH, HEIGHT, WIDTH, 0, 0, 0, WIDTH, HEIGHT};
    for (u32 i = 0; i < WIDTH * HEIGHT; ++i) {
        GRKIT_BYTES(p.y + i)[0] = 64;
        GRKIT_BYTES(p.c + i)[0] = 128;
    }
    return p;
}
/* The changed pixels of a plane, and their bounds. */
static u32 changed(const Plane *p, u32 *left, u32 *top)
{
    u32 n = 0;
    *left = WIDTH; *top = HEIGHT;
    for (u32 y = 0; y < HEIGHT; ++y)
        for (u32 x = 0; x < WIDTH; ++x)
            if (GRKIT_BYTES(p->y + y * WIDTH + x)[0] != 64) {
                ++n;
                if (x < *left) *left = x;
                if (y < *top) *top = y;
            }
    return n;
}

int main(void)
{
    u32 left, top;
    /* The descriptor: STYLE (6 values) and LOOK (3), a page with its native
       settings rows, and one filter. */
    assert(grkit_module.setting_count == 2 && grkit_module.settings[0].values == 6 && grkit_module.settings[1].values == 3);
    assert(grkit_module.registration_count == 2 && grkit_module.tile_count == 6);
    assert(SETTING_STYLE == GRKIT_SETTING(0) && SETTING_LOOK == GRKIT_SETTING(1));
    /* Each look of each style lights the lower right corner of the picture
       and nothing else. */
    for (u32 look = 0; look < 3; ++look)
        for (u32 style = 1; style < 6; ++style) {
            PictureSet set = {{gray_plane(GRKIT_TEST_PLANES)}, 1, 0, 0, grkit_test_scratch};
            grkit_test_settings[SETTING_LOOK] = look;
            date_apply(&set, style);
            assert(changed(&set.planes[0], &left, &top) > 100);
            assert(left > WIDTH / 2 && top > HEIGHT / 2);
        }
    /* A monochrome picture keeps its chroma. */
    {
        PictureSet set = {{gray_plane(GRKIT_TEST_PLANES)}, 1, 0, 1, grkit_test_scratch};
        grkit_test_settings[SETTING_LOOK] = 0;
        date_apply(&set, 1);
        for (u32 i = 0; i < WIDTH * HEIGHT; ++i) assert(GRKIT_BYTES(set.planes[0].c + i)[0] == 128);
    }
    return 0;
}

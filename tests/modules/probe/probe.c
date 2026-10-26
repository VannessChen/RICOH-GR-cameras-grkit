/* The probe module: a test module, not a feature. Each draw of the live view
   counts the draws, which shows that the module code runs, draws the count
   and a green square, and draws the GRKit label of the core through an
   import.

   It also tests saved settings: the setting COUNT counts the draws of all
   boots. Each draw adds one to it, and the second row shows it. */
#include "grkit.h"

static u32 draws;

static void fill(const Surface *s, u32 left, u32 top, u32 width, u32 height, u32 color)
{
    for (u32 y = top; y < top + height && y < 480; ++y)
        for (u32 x = left; x < left + width && x < 720; ++x)
            SURFACE_PIXEL(s, x, y) = color;
}

/* Seven segments a to g of each decimal digit, bit 0 = a. */
static const u8 segments[10] = {0x3f, 0x06, 0x5b, 0x4f, 0x66, 0x6d, 0x7d, 0x07, 0x7f, 0x6f};

/* One digit of 12 x 20 pixels. */
static void digit(const Surface *s, u32 left, u32 top, u32 value)
{
    u32 bits = segments[value];
    if (bits & 1) fill(s, left + 2, top, 8, 2, 0xffffffffU);
    if (bits & 2) fill(s, left + 10, top + 2, 2, 7, 0xffffffffU);
    if (bits & 4) fill(s, left + 10, top + 11, 2, 7, 0xffffffffU);
    if (bits & 8) fill(s, left + 2, top + 18, 8, 2, 0xffffffffU);
    if (bits & 16) fill(s, left, top + 11, 2, 7, 0xffffffffU);
    if (bits & 32) fill(s, left, top + 2, 2, 7, 0xffffffffU);
    if (bits & 64) fill(s, left + 2, top + 9, 8, 2, 0xffffffffU);
}

/* A six-digit decimal number, without division (the core exports none). */
static void decimal(const Surface *s, u32 left, u32 top, u32 value)
{
    static const u32 powers[] = {100000, 10000, 1000, 100, 10, 1};
    fill(s, left - 2, top - 2, 6 * 14 + 2, 24, 0xff000000U);
    for (u32 i = 0; i < 6; ++i) {
        u32 n = 0;
        while (value >= powers[i]) { value -= powers[i]; ++n; }
        digit(s, left + i * 14, top, n > 9 ? 9 : n);
    }
}

static void probe_paint(const Surface *s, u32 value)
{
    (void)value;
    u32 total = (setting_get(GRKIT_SETTING(0)) + 1) & 0xffffU;
    ++draws;
    setting_set(GRKIT_SETTING(0), total);
    /* Equal red and blue, so the color does not depend on the channel order
       of the overlay. */
    fill(s, 16, 120, 28, 28, 0xff000000U);
    fill(s, 18, 122, 24, 24, 0xff20c020U);
    decimal(s, 56, 124, draws);
    decimal(s, 56, 152, total);
    overlay_label(s, LABEL_GRKIT, 16, 420);
}

static const GrkitLayer layer = {sizeof(GrkitLayer), 0, probe_paint, GRKIT_NO_SETTING, 50};
static const GrkitRegistration registrations[] = {{GRKIT_POINT_OVERLAY_LAYER, &layer}};
static const GrkitSetting settings[] = {{sizeof(GrkitSetting), "COUNT", 65536, GRKIT_SETTING_SAVED}};
GRKIT_MODULE(registrations, settings, 1, 0, 0, 0, 0);

/* Framing: the mask and the ratio label of the crop in the live view. The
   core sets the crop from CORE.ASPECT and gives it as the capture area of
   the surface (Surface.area). A ratio that the camera has natively has the
   native viewfinder, so its area is 0 and the layers draw nothing. */
#include "grkit.h"

/* Black at 50% alpha outside the photo area. Each full draw paints the mask,
   so the loops write only the bars: the rows above and below the photo area,
   and the columns at its sides. */
#define MASK_COLOR 0x80000000U
/* Four pixels for each loop test, which the compiler joins into one NEON
   store. The native clear in the same Canvas::Draw also uses NEON stores. */
__attribute__((noinline)) static void mask_span(volatile u32 *row, u32 from, u32 to)
{
    u32 *pixel = (u32 *)row + from, *end = (u32 *)row + to;
    for (; pixel + 4 <= end; pixel += 4) {
        pixel[0] = MASK_COLOR; pixel[1] = MASK_COLOR;
        pixel[2] = MASK_COLOR; pixel[3] = MASK_COLOR;
    }
    for (; pixel < end; ++pixel) *pixel = MASK_COLOR;
}

void framing_mask(const Surface *s, u32 value)
{
    (void)value;
    if (!s->area) return;
    for (u32 y = 0; y < 480; ++y) {
        volatile u32 *row = &SURFACE_PIXEL(s, 0, y);
        if (y < s->box[1] || y >= s->box[3]) { mask_span(row, 0, 720); continue; }
        mask_span(row, 0, s->box[0]);
        mask_span(row, s->box[2], 720);
    }
}

/* The ratio label at the left of the second row. */
void framing_badge(const Surface *s, u32 value)
{
    (void)value;
    if (s->area) overlay_label(s, LABEL_RATIO(s->area), LABEL_LEFT, LABEL_TOP);
}

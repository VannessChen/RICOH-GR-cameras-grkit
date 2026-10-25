/* Framing: the mask and the ratio label of the crop in the live view. The
   core sets the crop from CORE.ASPECT and gives it as the capture area of
   the surface (Surface.area). A ratio that the camera has natively has the
   native viewfinder, so its area is 0 and the layers draw nothing. */
#include "grkit.h"
#include "grkit_module.h"

/* MASK: black at 50% alpha (0) or at 100% (1) outside the photo area, or
   frame lines only (2). Each full draw paints the mask, so the loops write
   only the bars: the rows above and below the photo area, and the columns
   at its sides. */
static const u32 mask_colors[2] = {0x80000000U, 0xff000000U};
/* The frame lines: 2 white pixels inside each edge of the photo area. */
#define FRAME_COLOR 0xffffffffU
#define FRAME_WIDTH 2U
/* Four pixels for each loop test, which the compiler joins into one NEON
   store. The native clear in the same Canvas::Draw also uses NEON stores. */
__attribute__((noinline)) static void mask_span(volatile u32 *row, u32 from, u32 to, u32 color)
{
    u32 *pixel = (u32 *)row + from, *end = (u32 *)row + to;
    for (; pixel + 4 <= end; pixel += 4) {
        pixel[0] = color; pixel[1] = color;
        pixel[2] = color; pixel[3] = color;
    }
    for (; pixel < end; ++pixel) *pixel = color;
}

static void frame_lines(const Surface *s)
{
    const u16 *box = s->box;
    for (u32 y = box[1]; y < box[3]; ++y) {
        volatile u32 *row = &SURFACE_PIXEL(s, 0, y);
        if (y < box[1] + FRAME_WIDTH || y + FRAME_WIDTH >= box[3]) { mask_span(row, box[0], box[2], FRAME_COLOR); continue; }
        mask_span(row, box[0], box[0] + FRAME_WIDTH, FRAME_COLOR);
        mask_span(row, box[2] - FRAME_WIDTH, box[2], FRAME_COLOR);
    }
}

void framing_mask(const Surface *s, u32 value)
{
    u32 style = setting_get(SETTING_MASK), color;
    (void)value;
    if (!s->area) return;
    if (style == 2) { frame_lines(s); return; }
    color = mask_colors[style == 1];
    for (u32 y = 0; y < 480; ++y) {
        volatile u32 *row = &SURFACE_PIXEL(s, 0, y);
        if (y < s->box[1] || y >= s->box[3]) { mask_span(row, 0, 720, color); continue; }
        mask_span(row, 0, s->box[0], color);
        mask_span(row, s->box[2], 720, color);
    }
}

/* The ratio label at the left of the second row, unless LABEL is Off. */
void framing_badge(const Surface *s, u32 value)
{
    (void)value;
    if (s->area && !setting_get(SETTING_LABEL))
        overlay_label(s, LABEL_RATIO(s->area), LABEL_LEFT, LABEL_TOP);
}

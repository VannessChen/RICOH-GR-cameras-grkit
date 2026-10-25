//! The GR III family: GR III and GR IIIx, which run the same RTOS code. Its
//! hooks, in installation order. Each build gives the site of each hook.

use super::Family;
use crate::components::{
    b, bl, HookSpec,
    Part::{Framing, Menu, Processing},
    Point::{CaptureAspect, MenuPage, PictureEncode, PipelineGrain, Tuning, ViewfinderOverlay},
};

pub const FAMILY: Family = Family {
    name: "gr3",
    hooks: HOOKS,
    processing_source: Some(include_str!("../../../../runtime/processing.S")),
};

const HOOKS: &[HookSpec] = &[
    // Image Control processing (`runtime/processing.S`): the BL to the
    // parameter normalizer, the type-table loads, the last packet field
    // store, the custom-curve flag, and the BL to the curve getter.
    bl("module_parameters", Processing, Tuning),
    b("module_processing", Processing, Tuning),
    b("module_packet", Processing, Tuning),
    b("module_gamma_flag", Processing, Tuning),
    bl("module_gamma_curves", Processing, Tuning),
    // AdjModeController: the touch thunks of the five slots, destroy,
    // refresh, event, exit, and changed.
    b("menu_native_touch_0", Menu, MenuPage),
    b("menu_native_touch_1", Menu, MenuPage),
    b("menu_native_touch_2", Menu, MenuPage),
    b("menu_native_touch_3", Menu, MenuPage),
    b("menu_native_touch_4", Menu, MenuPage),
    b("menu_destroy", Menu, MenuPage),
    b("menu_refresh", Menu, MenuPage),
    b("menu_event", Menu, MenuPage),
    b("menu_exit", Menu, MenuPage),
    b("menu_changed", Menu, MenuPage),
    // The table loads of the text measure and draw.
    b("menu_text_measure", Menu, MenuPage),
    b("menu_text_draw", Menu, MenuPage),
    // The icon width, height, and limit checks, and the descriptor load.
    b("menu_icon_width", Menu, MenuPage),
    b("menu_icon_height", Menu, MenuPage),
    b("menu_icon_guard", Menu, MenuPage),
    b("menu_icon_descriptor", Menu, MenuPage),
    // The ADJ Image Control item apply, then the ImageControlController
    // (MENU page) apply, event, and exit.
    b("menu_native_adj_apply", Menu, Tuning),
    b("menu_native_dialog_apply", Menu, Tuning),
    b("menu_native_page_event", Menu, Tuning),
    b("menu_native_page_exit", Menu, Tuning),
    // LiveViewController's Image Control widget: the BL to the icon mapper.
    bl("menu_viewfinder_icon", Menu, Tuning),
    // Image Control tag for GRKit photos: the playback DISP page icon, and the
    // playback parameter detail, which renders GRKit photos as Standard.
    bl("menu_playback_icon", Menu, Tuning),
    bl("menu_playback_detail", Menu, Tuning),
    // The native parameter page edits the Fn adjustments of a GRKit effect
    // (`runtime/menu.c`). ImageControlDetailController: its start method, the
    // BL to its kind reader, the two BLs to the snapshot loader, and its event
    // handler. Then the Image Control model getters and setters of the
    // Standard adjustments (saturation, hue, key, contrast, highlight, shadow,
    // sharpness, shading, clarity), and the BL to the settings getter in the
    // row text of each adjustment.
    b("menu_parameter_start", Menu, Tuning),
    bl("menu_parameter_kinds", Menu, Tuning),
    bl("menu_parameter_snapshot_0", Menu, Tuning),
    bl("menu_parameter_snapshot_1", Menu, Tuning),
    b("menu_parameter_event", Menu, Tuning),
    b("menu_parameter_get_0", Menu, Tuning),
    b("menu_parameter_get_1", Menu, Tuning),
    b("menu_parameter_get_2", Menu, Tuning),
    b("menu_parameter_get_3", Menu, Tuning),
    b("menu_parameter_get_4", Menu, Tuning),
    b("menu_parameter_get_5", Menu, Tuning),
    b("menu_parameter_get_6", Menu, Tuning),
    b("menu_parameter_get_7", Menu, Tuning),
    b("menu_parameter_get_8", Menu, Tuning),
    b("menu_parameter_set_0", Menu, Tuning),
    b("menu_parameter_set_1", Menu, Tuning),
    b("menu_parameter_set_2", Menu, Tuning),
    b("menu_parameter_set_3", Menu, Tuning),
    b("menu_parameter_set_4", Menu, Tuning),
    b("menu_parameter_set_5", Menu, Tuning),
    b("menu_parameter_set_6", Menu, Tuning),
    b("menu_parameter_set_7", Menu, Tuning),
    b("menu_parameter_set_8", Menu, Tuning),
    bl("menu_parameter_row_0", Menu, Tuning),
    bl("menu_parameter_row_1", Menu, Tuning),
    bl("menu_parameter_row_2", Menu, Tuning),
    bl("menu_parameter_row_3", Menu, Tuning),
    bl("menu_parameter_row_4", Menu, Tuning),
    bl("menu_parameter_row_5", Menu, Tuning),
    bl("menu_parameter_row_6", Menu, Tuning),
    bl("menu_parameter_row_7", Menu, Tuning),
    bl("menu_parameter_row_8", Menu, Tuning),
    // A GRKit root row opens a borrowed native MENU submenu as its settings
    // page (`runtime/menu_settings.h`, `Module.settings`). The Instant Review submenu
    // shows the rows, and the Display Time and Zoom Review value pages show
    // the values of the two settings. ChangeController (its entry) ends the
    // session at any other controller. The five MOVT of the submenu kind
    // table base load a private copy of the Instant Review entry instead.
    // Then the row count, row text, apply, and current-value methods of the
    // two value pages.
    b("menu_settings_change", Menu, MenuPage),
    // The update of the camera information page, which About GRKit shows.
    b("menu_info_update", Menu, MenuPage),
    bl("menu_settings_kinds_r3_0", Menu, MenuPage),
    bl("menu_settings_kinds_r2", Menu, MenuPage),
    bl("menu_settings_kinds_r8", Menu, MenuPage),
    bl("menu_settings_kinds_r5", Menu, MenuPage),
    bl("menu_settings_kinds_r3_1", Menu, MenuPage),
    // The submenu shows six rows (`runtime/menu.S`): the store of the fifth
    // row type in its constructor, the two compares of the rows to hide, and
    // the load of the row kind in its draw.
    bl("menu_settings_build", Menu, MenuPage),
    bl("menu_settings_hide_first", Menu, MenuPage),
    bl("menu_settings_hide_next", Menu, MenuPage),
    bl("menu_settings_row_kind", Menu, MenuPage),
    b("menu_settings_count_0", Menu, MenuPage),
    b("menu_settings_draw_0", Menu, MenuPage),
    b("menu_settings_apply_0", Menu, MenuPage),
    b("menu_settings_current_0", Menu, MenuPage),
    b("menu_settings_count_1", Menu, MenuPage),
    b("menu_settings_draw_1", Menu, MenuPage),
    b("menu_settings_apply_1", Menu, MenuPage),
    b("menu_settings_current_1", Menu, MenuPage),
    b("menu_settings_count_2", Menu, MenuPage),
    b("menu_settings_draw_2", Menu, MenuPage),
    b("menu_settings_apply_2", Menu, MenuPage),
    b("menu_settings_current_2", Menu, MenuPage),
    b("menu_settings_count_3", Menu, MenuPage),
    b("menu_settings_draw_3", Menu, MenuPage),
    b("menu_settings_apply_3", Menu, MenuPage),
    b("menu_settings_current_3", Menu, MenuPage),
    b("menu_settings_count_4", Menu, MenuPage),
    b("menu_settings_draw_4", Menu, MenuPage),
    b("menu_settings_apply_4", Menu, MenuPage),
    b("menu_settings_current_4", Menu, MenuPage),
    // Framing and Guides: LiveViewController update, exit, and destroy, then
    // Canvas::Draw entry and its child-draw BL.
    b("framing_update", Framing, ViewfinderOverlay),
    b("framing_exit", Framing, ViewfinderOverlay),
    b("framing_destroy", Framing, ViewfinderOverlay),
    b("framing_canvas", Framing, ViewfinderOverlay),
    bl("framing_children", Framing, ViewfinderOverlay),
    // Still aspect property handler: replaces `ldrb r1, [r4]`, the requested
    // capture aspect, so a native 3:2 request records the Framing ratio.
    bl("framing_capture_request_hook", Framing, CaptureAspect),
    // GetResolution id 2 heights (L, then each smaller size or crop step),
    // then the id 2 top insets of the screennail and the thumbnail.
    bl("framing_height_0", Framing, CaptureAspect),
    bl("framing_height_1", Framing, CaptureAspect),
    bl("framing_height_2", Framing, CaptureAspect),
    bl("framing_height_3", Framing, CaptureAspect),
    bl("framing_screennail_inset", Framing, CaptureAspect),
    bl("framing_thumbnail_inset", Framing, CaptureAspect),
    // GetResolution id 1 widths (L, M, S, XS), then the id 1 left inset of
    // the screennail and the second store of the thumbnail insets: 5:4
    // narrows the 4:3 capture aspect.
    bl("framing_width_0", Framing, CaptureAspect),
    bl("framing_width_1", Framing, CaptureAspect),
    bl("framing_width_2", Framing, CaptureAspect),
    bl("framing_width_3", Framing, CaptureAspect),
    bl("framing_narrow_screennail", Framing, CaptureAspect),
    bl("framing_narrow_thumbnail", Framing, CaptureAspect),
    // Playback size classifier and image aspect getter.
    bl("framing_classify", Framing, CaptureAspect),
    b("framing_image_aspect", Framing, CaptureAspect),
    // Effect extension and picture processing (`runtime/picture.*`), linked
    // into the Framing region. The live-view diff before the gamma stage
    // flag; the YCbCrMatrix Make call to its calc, which applies the
    // post-curve matrix; the JpegEncodeMacroProcess::Execute entry, for the
    // date stamp before the encode; and the maker-note setter of tag 0x4f
    // (ImageTone, the effect code of the shot) and tag 0x247 (the Fn
    // adjustments of the shot).
    b("module_gamma_dirty", Framing, Tuning),
    bl("picture_post", Framing, Tuning),
    b("picture_encode_entry", Framing, PictureEncode),
    bl("picture_capture_tone", Framing, Tuning),
    bl("picture_capture_parameters", Framing, Tuning),
    // DigitalFilterProcess SetEffectParameter: the Particle load, then the BL
    // to the stage gate.
    bl("grain_particle", Framing, PipelineGrain),
    bl("grain_gate", Framing, PipelineGrain),
];

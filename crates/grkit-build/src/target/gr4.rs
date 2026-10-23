//! The GR IV family: each GR IV build, including GR IV HDF and GR IV
//! Monochrome. Its hooks, in installation order. Each build gives the site of
//! each hook. The hooks follow those of GR III (`super::gr3`) where the
//! firmware code is alike.

use super::Family;
use crate::components::{
    b, bl, HookSpec,
    Part::{Framing, Menu, Processing},
    Point::{CaptureAspect, MenuPage, PictureEncode, PipelineGrain, Tuning, ViewfinderOverlay},
};

pub const FAMILY: Family = Family {
    name: "gr4",
    hooks: HOOKS,
    processing_source: Some(include_str!("../../../../runtime/targets/gr4-processing.S")),
};

const HOOKS: &[HookSpec] = &[
    // Image Control processing (`runtime/targets/gr4-processing.S`): the BL
    // to the parameter normalizer, `mov r1, sl` before the type-table
    // getters, the record and MultiAxial set stores, the last packet field
    // store, the custom-curve flag, and the BL to the curve getter.
    bl("module_parameters", Processing, Tuning),
    bl("module_processing", Processing, Tuning),
    bl("module_record", Processing, Tuning),
    bl("module_multiaxial", Processing, Tuning),
    bl("module_packet", Processing, Tuning),
    bl("module_gamma_flag", Processing, Tuning),
    bl("module_gamma_curves", Processing, Tuning),
    // AdjModeController, the text table loads, and the native Image Control
    // pages, as on GR III.
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
    b("menu_text_measure", Menu, MenuPage),
    b("menu_text_draw", Menu, MenuPage),
    b("menu_native_adj_apply", Menu, Tuning),
    b("menu_native_dialog_apply", Menu, Tuning),
    b("menu_native_page_event", Menu, Tuning),
    b("menu_native_page_exit", Menu, Tuning),
    // Icon lookup entry, then the limit check and the table read of the icon
    // draw.
    b("menu_icon_lookup", Menu, MenuPage),
    b("menu_icon_limit", Menu, MenuPage),
    b("menu_icon_entry", Menu, MenuPage),
    // The viewfinder icon and the Image Control tag, as on GR III.
    bl("menu_viewfinder_icon", Menu, Tuning),
    bl("menu_playback_icon", Menu, Tuning),
    bl("menu_playback_detail", Menu, Tuning),
    // The parameter page, as on GR III. The model getters and setters and the
    // row texts of shading and clarity are those of row kinds 14 and 15.
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
    // The settings page, as on GR III.
    b("menu_settings_change", Menu, MenuPage),
    bl("menu_settings_kinds_r3_0", Menu, MenuPage),
    bl("menu_settings_kinds_r2", Menu, MenuPage),
    bl("menu_settings_kinds_r8", Menu, MenuPage),
    bl("menu_settings_kinds_r5", Menu, MenuPage),
    bl("menu_settings_kinds_r3_1", Menu, MenuPage),
    b("menu_settings_count_0", Menu, MenuPage),
    b("menu_settings_draw_0", Menu, MenuPage),
    b("menu_settings_apply_0", Menu, MenuPage),
    b("menu_settings_current_0", Menu, MenuPage),
    b("menu_settings_count_1", Menu, MenuPage),
    b("menu_settings_draw_1", Menu, MenuPage),
    b("menu_settings_apply_1", Menu, MenuPage),
    b("menu_settings_current_1", Menu, MenuPage),
    // Framing and Guides, as on GR III. GR IV has a live-view mode for each
    // capture aspect, so instead of the GR III patches the capture aspect
    // getter BLs of the live-view mode function and of the live-view mode
    // check keep the live view in 3:2 while XPan uses capture aspect id 2.
    b("framing_update", Framing, ViewfinderOverlay),
    b("framing_exit", Framing, ViewfinderOverlay),
    b("framing_destroy", Framing, ViewfinderOverlay),
    b("framing_canvas", Framing, ViewfinderOverlay),
    bl("framing_children", Framing, ViewfinderOverlay),
    bl("framing_capture_request_hook", Framing, CaptureAspect),
    bl("framing_live_mode", Framing, CaptureAspect),
    bl("framing_live_check", Framing, CaptureAspect),
    // ImageResolutionTranslatorKb636 (the GR IV sizes) GetResolution id 2
    // heights, and the insets and the playback hooks, as on GR III. The
    // Kb588 class holds the GR III sizes and is not used.
    bl("framing_height_0", Framing, CaptureAspect),
    bl("framing_height_1", Framing, CaptureAspect),
    bl("framing_height_2", Framing, CaptureAspect),
    bl("framing_height_3", Framing, CaptureAspect),
    bl("framing_screennail_inset", Framing, CaptureAspect),
    bl("framing_thumbnail_inset", Framing, CaptureAspect),
    bl("framing_classify", Framing, CaptureAspect),
    b("framing_image_aspect", Framing, CaptureAspect),
    // Picture processing, as on GR III.
    b("module_gamma_dirty", Framing, Tuning),
    bl("picture_post", Framing, Tuning),
    b("picture_encode_entry", Framing, PictureEncode),
    bl("picture_capture_tone", Framing, Tuning),
    bl("picture_capture_parameters", Framing, Tuning),
    // DigitalFilterProcess SetEffectParameter: the grain on, strength, and
    // size loads, then the BL to the stage gate.
    bl("grain_enable", Framing, PipelineGrain),
    bl("grain_strength", Framing, PipelineGrain),
    bl("grain_size", Framing, PipelineGrain),
    bl("grain_gate", Framing, PipelineGrain),
];

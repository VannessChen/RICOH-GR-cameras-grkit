/* Generated from crates/grkit-core/src/plugins.rs. Do not edit: change the
   registry there, then run GRKIT_UPDATE_GENERATED=1 cargo test -p grkit-core
   --test generated. */
/* The viewfinder.overlay layers of all plugins, in painting order. */
#ifndef GRKIT_GENERATED_LAYERS_H
#define GRKIT_GENERATED_LAYERS_H

#include "../plugins/framing/framing.c"
#include "../plugins/guides/guides.c"

#define PLUGIN_LAYERS \
    {PLUGIN_FRAMING,framing_value,framing_mask,0}, \
    {PLUGIN_GUIDES,guides_value,guides_paint,0}, \
    {PLUGIN_FRAMING,framing_badge_value,framing_badge,1},

#endif

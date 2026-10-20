/* Selects the firmware definitions of the build target. The build and the host
   tests define GRKIT_TARGET_<target> (components::Layout::define). A new target
   must add its header here. */

/* A firmware address of a relocatable target. The installer finds the address
   in the running build of the firmware version and moves each reference
   (crates/grkit-build/src/relocation.rs). The symbol fw_<address> has the
   address of the pinned build. In C the value is opaque, so the compiler keeps
   it in one literal word instead of a direct call or a derived constant. */
#if defined(GRKIT_HOST_ADDRESSES)
/* Host contract tests link the code without the firmware: plain addresses. */
#define FIRMWARE(address) ((unsigned int)(address))
#elif defined(__ASSEMBLER__)
#define FIRMWARE(address) fw_##address
#else
#define FIRMWARE(address) ({ extern char fw_##address[]; unsigned int firmware_; \
    __asm__("" : "=r"(firmware_) : "0"((unsigned int)(__UINTPTR_TYPE__)fw_##address)); firmware_; })
#endif
#if defined(GRKIT_TARGET_GR3_210)
#include "targets/gr3-2.10.h"
#elif defined(GRKIT_TARGET_GR4_111)
#include "targets/gr4-1.11.h"
#else
#error "Define GRKIT_TARGET_<target> to select runtime/targets/<target>.h"
#endif

#ifndef FRAMING_RATIO_COUNT
/* Framing ratios: Off, XPan, 16:9, 4:3. A target can offer only the first ones. */
#define FRAMING_RATIO_COUNT 4U
#endif

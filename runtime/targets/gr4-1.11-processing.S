// Private Image Control processing hooks for GR IV 1.11 (runtime/processing.S
// describes the GR III 2.10 version). The build places this
// code after the RTOS image. It has no relocations: local calls use BL words.
// The header fields are those of abi.h.
#include "abi.h"
.syntax unified
.arm
.text

// Return Z=1 only for an owned record pointer in r0. Preserve all registers.
.Lowned_record:
    push {r0-r3, lr}
    ldr r1, =GRKIT_HEADER_REFERENCE
    ldr r2, [r1, #GRKIT_HOST_COUNT]
    ldr r1, [r1, #GRKIT_HOST_RECORDS]
    cmp r2, #0
    beq owned_no
owned_loop:
    cmp r0, r1
    beq owned_done
    add r1, r1, #GRKIT_RECORD_SIZE
    subs r2, r2, #1
    bne owned_loop
owned_no:
    movs r3, #1
owned_done:
    pop {r0-r3, pc}

// Store r0, the record of a non-live-view build (0 for native processing),
// in the shot ring under the shot settings pointer r7. The live view (context
// kind 1) stores nothing. A reader in another task checks the key again after
// it reads the record, so the key is cleared while the record changes.
// Preserves all registers.
.if GRKIT_SHOT_RING != 0x22840
.error "The shot ring offset changed"
.endif
.Lshot_store:
    push {r0-r3, r12, lr}
    ldrb r2, [sb, #0x2ac]
    cmp r2, #1
    beq shot_done
    ldr r1, =GRKIT_HEADER_REFERENCE
    str r0, [r1, #GRKIT_HOST_STILL]
    ldr r2, [r1, #GRKIT_HOST_COUNT]
    ldr r3, [r1, #GRKIT_HOST_RECORDS]
    mov r12, #GRKIT_RECORD_SIZE
    mla r3, r2, r12, r3
    add r3, r3, #0x22800
    add r3, r3, #0x40
    ldr r2, [r3]
    add r12, r2, #1
    str r12, [r3]
    and r2, r2, #(GRKIT_SHOT_ENTRIES - 1)
    add r2, r3, r2, lsl #3
    mov r12, #0
    str r12, [r2, #8]
    dmb ish
    str r0, [r2, #12]
    dmb ish
    str r7, [r2, #8]
shot_done:
    pop {r0-r3, r12, pc}

// Replaces the BL to the parameter normalizer (0x539113d8) at 0x53912c70.
// sb = context (+0x8000), r4 = packet, r5 = transient object, r7 = settings.
// Snapshot activation before the packet is built. Unsupported contexts use the
// native normalizer and cannot inherit a private record.
module_parameters:
    push {r0-r3, r12, lr}
    ldr r0, =GRKIT_HEADER_REFERENCE
    ldr r2, [sb, #0x2ac]
    str r2, [r0, #GRKIT_HOST_PROCESSING_CONTEXT]
    ldr r3, [r0, #GRKIT_HOST_PROCESSING_CALLS]
    add r3, r3, #1
    str r3, [r0, #GRKIT_HOST_PROCESSING_CALLS]
    ldr r0, [r0, #GRKIT_HOST_ACTIVE]
    // Context 1 is the live view; 0 is the still path.
    ldrb r2, [sb, #0x2ac]
    cmp r2, #1
    bhi parameters_stock
    // Menu previews change only the live view. Still captures keep the committed record.
    bne parameters_committed
    ldr r3, =GRKIT_HEADER_REFERENCE
    ldr r3, [r3, #GRKIT_HOST_PREVIEW]
    cmp r3, #0
    movne r0, r3
parameters_committed:
    ldrb r2, [sb, #0x2ad]
    cmp r2, #0
    bne parameters_stock
    ldrb r2, [sb, #0x2b2]
    cmp r2, #0
    bne parameters_stock
    .word 0xeb000000 | (((.Lowned_record - . - 8) >> 2) & 0x00ffffff)
    bne parameters_stock
    str r0, [r4, #8]
    // Capture metadata and the encode read the record of this shot.
    .word 0xeb000000 | (((.Lshot_store - . - 8) >> 2) & 0x00ffffff)
    // Fill the transient object like the native Standard branch (0x53911614),
    // from the adjustments of the record (r0), not from the native settings.
    // The native branch writes one byte at +0x1f8.
    add r3, r0, #(GRKIT_RECORD_PARAMETERS & ~0xff)
    add r3, r3, #(GRKIT_RECORD_PARAMETERS & 0xff)
    ldrb r2, [r5, #0x1f9]
    push {r2}
    add r1, r5, #0x1e0
    add r0, r5, #0x200
    adr r12, parameters_sources
parameters_fill:
    ldrb r2, [r12], #1
    cmp r2, #9
    subhs r2, r2, #0x10
    bhs parameters_store
    // A stored byte is (native - 4) mod 9. Another value is neutral.
    ldrb r2, [r3, r2]
    cmp r2, #8
    movhi r2, #0
    cmp r2, #5
    addlo r2, r2, #4
    subhs r2, r2, #5
parameters_store:
    strh r2, [r1], #2
    cmp r1, r0
    bne parameters_fill
    pop {r2}
    strb r2, [r5, #0x1f9]
    pop {r0-r3, r12, pc}
// Source per halfword at +0x1e0: an index of GRKIT_RECORD_PARAMETERS, or 0x10
// plus a constant. Saturation, hue, key, contrast, highlight, shadow, 1,
// sharpness, 0 (byte), 0, shading, clarity, 0, 0, 0, 0.
parameters_sources:
    .byte 0, 1, 2, 3, 4, 5, 0x11, 6, 0x10, 0x10, 7, 8, 0x10, 0x10, 0x10, 0x10
parameters_stock:
    mov r0, #0
    str r0, [r4, #8]
    // Other non-live-view builds, such as in-camera RAW development, record no effect.
    .word 0xeb000000 | (((.Lshot_store - . - 8) >> 2) & 0x00ffffff)
    pop {r0-r3, r12, lr}
    ldr pc, =0x539113d8

// Replaces `mov r1, sl` at 0x53912d40, where every type path joins before the
// type-table getters. sl is the table index. A private record uses the
// Standard index 11 for the other tables, as GR III does.
module_processing:
    push {r0, lr}
    ldr r0, [r4, #8]
    .word 0xeb000000 | (((.Lowned_record - . - 8) >> 2) & 0x00ffffff)
    moveq sl, #11
    streq sl, [fp, #-0x34]
    mov r1, sl
    pop {r0, pc}

// Replaces `str r0, [r4, #8]` at 0x53912d4c, the record from the first getter.
// A private record set by module_parameters stays.
module_record:
    push {r0, lr}
    ldr r0, [r4, #8]
    .word 0xeb000000 | (((.Lowned_record - . - 8) >> 2) & 0x00ffffff)
    pop {r0, lr}
    strne r0, [r4, #8]
    bx lr

// Replaces `str r3, [r4, #0x90]` at 0x53913188, before the native packet tail.
// The GR IV tail has no special type path, so it continues unchanged.
module_packet:
    str r3, [r4, #0x90]
    push {r0-r3, lr}
    ldr r0, [r4, #8]
    .word 0xeb000000 | (((.Lowned_record - . - 8) >> 2) & 0x00ffffff)
    bne packet_done
    ldr r1, =GRKIT_HEADER_REFERENCE
    str r0, [r1, #GRKIT_HOST_LAST_APPLIED]
    ldr r1, [r0, #GRKIT_RECORD_MONOCHROME]
    and r1, r1, #1
    strb r1, [r4, #0x79]
    mov r2, #0
    strb r2, [r4, #0x7a]
    strb r2, [r4, #0x55]
    strb r2, [r4, #0x56]
    cmp r1, #0
    movne r1, #0x2000
    strhne r1, [r4, #0x7c]
    strhne r2, [r4, #0x7e]
    strhne r2, [r4, #0x80]
packet_done:
    pop {r0-r3, pc}

// Replaces `ldrb r1, [r1, #0x9b]` at 0x5391e360 (r1 = packet): a private
// record takes the custom-curve path.
module_gamma_flag:
    push {r0, lr}
    ldr r0, [r1, #8]
    .word 0xeb000000 | (((.Lowned_record - . - 8) >> 2) & 0x00ffffff)
    ldrbne r1, [r1, #0x9b]
    moveq r1, #1
    pop {r0, pc}

// Replaces the BL to the curve getter (0x53925000) at 0x5391e76c. r1..r3 receive
// the R, G, and B curves. The packet is at [fp, #-0x54].
module_gamma_curves:
    push {r0, r5, r12, lr}
    ldr r0, [fp, #-0x54]
    ldr r0, [r0, #8]
    .word 0xeb000000 | (((.Lowned_record - . - 8) >> 2) & 0x00ffffff)
    bne curves_stock
    add r5, r0, #0x68
    str r5, [r1]
    add r5, r5, #0x200
    str r5, [r2]
    add r5, r5, #0x200
    str r5, [r3]
    pop {r0, r5, r12, pc}
curves_stock:
    pop {r0, r5, r12, lr}
    ldr pc, =0x53925000
.ltorg

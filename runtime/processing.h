// The parts of the Image Control processing hooks that each family shares:
// the record check, the shot ring store, and the parameter build hook. The
// processing source of a family (Layout::processing_source) includes this
// file after it defines the registers of the hooked parameter build:
//   PROCESSING_CONTEXT   the context (+0x2ac holds its kind)
//   PROCESSING_OBJECT    the transient object that the build fills
//   PROCESSING_KEPT_BYTE optional: a byte of the filled halfwords that the
//                        native fill does not write, so the fill keeps it
// r4 is the packet and r7 the shot settings in each family.
#if !defined(PROCESSING_CONTEXT) || !defined(PROCESSING_OBJECT)
#error "Define the registers of the parameter build before processing.h"
#endif

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
.if GRKIT_SHOT_STRIDE != 1 << 3
.error "The shot ring index shift assumes 8-byte entries"
.endif
.Lshot_store:
    push {r0-r3, r12, lr}
    ldrb r2, [PROCESSING_CONTEXT, #0x2ac]
    cmp r2, #1
    beq shot_done
    ldr r1, =GRKIT_HEADER_REFERENCE
    str r0, [r1, #GRKIT_HOST_STILL]
    ldr r2, [r1, #GRKIT_HOST_COUNT]
    ldr r3, [r1, #GRKIT_HOST_RECORDS]
    mov r12, #GRKIT_RECORD_SIZE
    mla r3, r2, r12, r3
    add r3, r3, #(GRKIT_SHOT_RING & ~0xff)
    add r3, r3, #(GRKIT_SHOT_RING & 0xff)
    ldr r2, [r3]
    add r12, r2, #1
    str r12, [r3]
    and r2, r2, #(GRKIT_SHOT_ENTRIES - 1)
    add r2, r3, r2, lsl #3
    mov r12, #0
    str r12, [r2, #(GRKIT_SHOT_ENTRY + GRKIT_SHOT_KEY)]
    dmb ish
    str r0, [r2, #(GRKIT_SHOT_ENTRY + GRKIT_SHOT_RECORD)]
    dmb ish
    str r7, [r2, #(GRKIT_SHOT_ENTRY + GRKIT_SHOT_KEY)]
shot_done:
    pop {r0-r3, r12, pc}

// Replaces the BL to the native parameter normalizer, not its shared entry.
// Snapshot activation before constructing this task's packet. Unsupported
// contexts use the native normalizer and cannot accidentally inherit a
// private record.
module_parameters:
    push {r0-r3, r12, lr}
    ldr r0, =GRKIT_HEADER_REFERENCE
    ldr r2, [PROCESSING_CONTEXT, #0x2ac]
    str r2, [r0, #GRKIT_HOST_PROCESSING_CONTEXT]
    ldr r3, [r0, #GRKIT_HOST_PROCESSING_CALLS]
    add r3, r3, #1
    str r3, [r0, #GRKIT_HOST_PROCESSING_CALLS]
    ldr r0, [r0, #GRKIT_HOST_ACTIVE]
    // Context 1 is the live view (ImageTuningLiveview); 0 is the still path.
    ldrb r2, [PROCESSING_CONTEXT, #0x2ac]
    cmp r2, #1
    bhi parameters_stock
    // Menu previews change only the live view. Still captures keep the committed record.
    bne parameters_committed
    ldr r3, =GRKIT_HEADER_REFERENCE
    ldr r3, [r3, #GRKIT_HOST_PREVIEW]
    cmp r3, #0
    movne r0, r3
parameters_committed:
    ldrb r2, [PROCESSING_CONTEXT, #0x2ad]
    cmp r2, #0
    bne parameters_stock
    ldrb r2, [PROCESSING_CONTEXT, #0x2b2]
    cmp r2, #0
    bne parameters_stock
    .word 0xeb000000 | (((.Lowned_record - . - 8) >> 2) & 0x00ffffff)
    bne parameters_stock
    str r0, [r4, #8]
    // Capture metadata and the encode read the record of this shot.
    .word 0xeb000000 | (((.Lshot_store - . - 8) >> 2) & 0x00ffffff)
    // Fill the transient object like the native Standard branch, from the
    // adjustments of the record (r0), not from the native settings.
    add r3, r0, #(GRKIT_RECORD_PARAMETERS & ~0xff)
    add r3, r3, #(GRKIT_RECORD_PARAMETERS & 0xff)
#ifdef PROCESSING_KEPT_BYTE
    ldrb r2, [PROCESSING_OBJECT, #PROCESSING_KEPT_BYTE]
    push {r2}
#endif
    add r1, PROCESSING_OBJECT, #0x1e0
    add r0, PROCESSING_OBJECT, #0x200
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
#ifdef PROCESSING_KEPT_BYTE
    pop {r2}
    strb r2, [PROCESSING_OBJECT, #PROCESSING_KEPT_BYTE]
#endif
    pop {r0-r3, r12, pc}
// Source per halfword at +0x1e0: an index of GRKIT_RECORD_PARAMETERS, or 0x10
// plus a constant. Saturation, hue, key, contrast, highlight, shadow, 1,
// sharpness, 0, 0, shading, clarity, 0, 0, 0, 0.
parameters_sources:
    .byte 0, 1, 2, 3, 4, 5, 0x11, 6, 0x10, 0x10, 7, 8, 0x10, 0x10, 0x10, 0x10
parameters_stock:
    mov r0, #0
    str r0, [r4, #8]
    // Other non-live-view builds, such as in-camera RAW development, record no effect.
    .word 0xeb000000 | (((.Lshot_store - . - 8) >> 2) & 0x00ffffff)
    pop {r0-r3, r12, lr}
    ldr pc, =HOOK_MODULE_PARAMETERS_TARGET

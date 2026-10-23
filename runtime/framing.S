#include "firmware.h"
#include "abi.h"
#include "hooks.h"
/* framing.c syncs the capture aspect with FRAMING_CAPTURE_ASPECT and keeps the
   live view in 3:2 with FW_CAPTURE_ASPECT_GET (target headers). The hooks
   that complete each come from the site tables of the target (hooks.h). */
#if defined(FRAMING_CAPTURE_ASPECT) != defined(HOOK_FRAMING_CAPTURE_REQUEST_HOOK)
#error "FRAMING_CAPTURE_ASPECT and the framing_capture_request_hook hook go together"
#endif
#if defined(FW_CAPTURE_ASPECT_GET) != defined(HOOK_FRAMING_LIVE_MODE)
#error "FW_CAPTURE_ASPECT_GET and the framing_live_* hooks go together"
#endif
.syntax unified
.arm
.text
/* The installer relocates this literal to the private host header. */
.global framing_header
framing_header:
    ldr r0, =GRKIT_HEADER_REFERENCE
    bx lr
#ifdef HOOK_FRAMING_CAPTURE_REQUEST_HOOK
/* Replaces `ldrb r1, [r4]` in the still aspect property handler. r1 returns
   the capture request. The next native call reads r0. */
.global framing_capture_request_hook
framing_capture_request_hook:
    push {r0-r3,r12,lr}
    ldrb r0, [r4]
    bl framing_capture_request
    str r0, [sp,#4]
    pop {r0-r3,r12,pc}
#endif
#ifdef HOOK_FRAMING_LIVE_MODE
/* Replace the capture aspect getter BL in the live-view mode function and in
   the live-view mode check (GR IV). r0 = the capture property. */
.global framing_live_mode
.global framing_live_check
framing_live_mode:
framing_live_check:
    push {r4,lr}
    bl framing_live_aspect
    pop {r4,pc}
#endif
/* Each entry hook preserves the displaced prologue and all incoming arguments. */
.macro ENTRY name, helper, resume
.global \name
\name:
    push {r0-r3,r12,lr}
    bl \helper
    pop {r0-r3,r12,lr}
    mov r12,sp
    ldr pc,=\resume
.endm
ENTRY framing_update, framing_bind, HOOK_FRAMING_UPDATE_RESUME
ENTRY framing_exit, framing_leave, HOOK_FRAMING_EXIT_RESUME
ENTRY framing_destroy, framing_leave, HOOK_FRAMING_DESTROY_RESUME
ENTRY framing_canvas, framing_prepare, HOOK_FRAMING_CANVAS_RESUME
/* Replaces the Canvas::Draw BL after its native full-background clear. */
.global framing_children
framing_children:
    push {r0-r3,r12,lr}
    bl framing_paint
    pop {r0-r3,r12,lr}
    ldr pc,=HOOK_FRAMING_CHILDREN_TARGET
#ifdef HOOK_FRAMING_CLASSIFY
/* 65:24 shares capture aspect id 2 with 16:9. These hooks switch the id 2
   geometry: shooting follows the Framing ratio, playback follows the size of
   the photo that the classifier saw last. The family header gives the sizes
   (XPAN_HEIGHT_<n>: the stock 16:9 and the 65:24 height of GetResolution id 2;
   XPAN_SIZES: the 65:24 sizes, largest first) and the classifier registers at
   the aspect BL (the width, the class pointer, and the return of its frame). */
#if !defined(XPAN_HEIGHT_0) || !defined(XPAN_HEIGHT_1) || !defined(XPAN_HEIGHT_2) || \
    !defined(XPAN_HEIGHT_3) || !defined(XPAN_SIZES)
#error "The family header defines the XPan sizes"
#endif
#if !defined(CLASSIFY_WIDTH) || !defined(CLASSIFY_CLASS) || !defined(CLASSIFY_RETURN)
#error "The family header defines the classifier registers"
#endif
/* r12 = 1 when id 2 means 65:24. Clobbers only r12 and the flags. */
xpan_active:
    ldr r12, =GRKIT_HEADER_REFERENCE
    ldrb r12, [r12, #GRKIT_HOST_FRAME_FILE]
    cmp r12, #0
    beq 1f
    cmp r12, #2
    moveq r12, #1
    movne r12, #0
    bx lr
1:  ldr r12, =GRKIT_HEADER_REFERENCE
    ldr r12, [r12, #GRKIT_HOST_FRAME_RATIO]
    cmp r12, #1
    moveq r12, #1
    movne r12, #0
    bx lr
/* GetResolution id 2 heights replace `mov r3, #height` before a store and an
   unconditional branch to the epilogue, so r12 and the flags are dead. */
.macro HEIGHT name, stock, xpan
.global \name
\name:
    push {lr}
    bl xpan_active
    pop {lr}
    movw r3, #\stock
    cmp r12, #0
    movwne r3, #\xpan
    bx lr
.endm
HEIGHT framing_height_0, XPAN_HEIGHT_0
HEIGHT framing_height_1, XPAN_HEIGHT_1
HEIGHT framing_height_2, XPAN_HEIGHT_2
HEIGHT framing_height_3, XPAN_HEIGHT_3
/* Top insets of the 720x480 screennail and 160x120 thumbnail. */
.macro INSET name, stock, xpan
.global \name
\name:
    push {lr}
    bl xpan_active
    pop {lr}
    mov r1, #\stock
    cmp r12, #0
    movne r1, #\xpan
    bx lr
.endm
INSET framing_screennail_inset, 38, 107
INSET framing_thumbnail_inset, 16, 30
/* r0 = width, r1 = height. r12 = 65:24 size class 0..3, or -1.
   Clobbers r0-r3, r12 and the flags. */
xpan_size:
    cmp r0, r1
    movhs r2, r0
    movhs r3, r1
    movlo r2, r1
    movlo r3, r0
    adr r12, xpan_sizes
    push {r4}
    mov r4, #0
2:  ldrh r0, [r12], #2
    ldrh r1, [r12], #2
    cmp r0, #0
    beq 3f
    cmp r0, r2
    cmpeq r1, r3
    beq 4f
    add r4, r4, #1
    b 2b
3:  mvn r4, #0
4:  mov r12, r4
    pop {r4}
    bx lr
xpan_sizes:
    .hword XPAN_SIZES, 0, 0
/* Replaces the aspect BL in the size classifier (GR III 0x532b3988: frame
   {r4-r7,fp,ip,lr,pc}, r7 = width, r4 = height, r6 = class pointer).
   A 65:24 size returns class and success from the classifier. Other sizes
   record "not 65:24" and continue with the native aspect function. */
.global framing_classify
framing_classify:
    push {r0-r3,lr}
    bl xpan_size
    pop {r0-r3,lr}
    ldr r2, =GRKIT_HEADER_REFERENCE
    cmp r12, #0
    blt 5f
    mov r3, #2
    strb r3, [r2, #GRKIT_HOST_FRAME_FILE]
    cmp CLASSIFY_CLASS, #0
    strbne r12, [CLASSIFY_CLASS]
    mov r0, #1
    CLASSIFY_RETURN
5:  mov r3, #1
    strb r3, [r2, #GRKIT_HOST_FRAME_FILE]
    mov r0, CLASSIFY_WIDTH
    mov r1, r4
    ldr pc, =HOOK_FRAMING_CLASSIFY_TARGET
/* Replaces the tail branch of the image aspect getter (GR III 0x532b3ca0,
   r0 = width, r1 = height): 65:24 sizes report aspect id 2. */
.global framing_image_aspect
framing_image_aspect:
    push {r0-r3,lr}
    bl xpan_size
    pop {r0-r3,lr}
    cmp r12, #0
    movge r0, #2
    bxge lr
    ldr pc, =HOOK_FRAMING_IMAGE_ASPECT_TARGET
#endif
.ltorg

/* RICOH GR IV 1.11 (also GR IV HDF): firmware entry points, data, and object
   layouts that the runtime uses. The addresses are those of the pinned 1.11
   build. Cameras also run other 1.11 builds, so each address is a FIRMWARE()
   symbol that the installer relocates. Data that no firmware code loads
   directly is an offset from an address that it does load. */
#ifndef GRKIT_TARGET_H
#define GRKIT_TARGET_H

/* Icon slots hold a 12-byte descriptor, then 40 x 40 RGBA. */
#define GRKIT_ICON_STRIDE 6412

/* Camera settings and the still aspect getter. The native ids are those of
   GR III: 0 = 3:2, 1 = 4:3, 2 = 16:9, 3 = 1:1. GR IV offers all four. */
#define FW_SETTINGS FIRMWARE(0x5323f018)
#define FW_STILL_ASPECT_GET FIRMWARE(0x5329c09c)
/* Settings domain: the capture property singleton (0x5537960c) starts with a
   vtable, then the domain. Domain +0x518 -> control -> payload with the native
   Image Control type at +0x79c. Standard is type 11. */
#define FW_SETTINGS_DOMAIN (FIRMWARE(0x5537960c) + 4)
#define SETTINGS_IC_TYPE 0x79c
/* Framing: GR IV has native 4:3 and 16:9, so Framing offers Off and XPan.
   XPan uses capture aspect id 2 with the 65:24 heights. In native 3:2 the
   capture request hook maps the 3:2 request to id 2, and the live-view hooks
   keep the live view in 3:2, as on GR III. In native 16:9 XPan crops the
   native 16:9 photos. */
#define FRAMING_RATIO_COUNT 2U
#define FRAMING_XPAN_NATIVE 2
/* Capture aspect (property 0xdf, CommonProperty +0x7da) getter. The live-view
   mode function 0x5388c7b8 and the mode check 0x5363fa84 call it. */
#define FW_CAPTURE_ASPECT_GET FIRMWARE(0x5375840c)
/* Resend the still aspect: with the UI aspect (+0x49f) equal to the applied
   aspect (+0xf8b) it posts no UI event and, in still mode, sends property
   0xdf with the applied value. */
#define FW_STILL_ASPECT_RESEND FIRMWARE(0x53284454)
/* The next copy of the capture aspect: domain +0x518 -> control, +4 -> next. */
#define FRAMING_CAPTURE_ASPECT() BYTE(WORD(WORD(WORD(FW_SETTINGS_DOMAIN,0),0x518),4),0x7da)
#define FRAMING_REQUEST_ASPECT() (((void (*)(u32))FW_STILL_ASPECT_RESEND)(FRAMING_SETTINGS()))
#define MENU_FRAMING_REQUEST() (((void (*)(u32))FW_STILL_ASPECT_RESEND)(CALL0(FW_SETTINGS)()))
/* ImageResolutionTranslatorKb636 GetResolution id 2: the stock 16:9 height and
   the 65:24 height (width x 24 / 65, a multiple of 8) of each size. */
#define XPAN_HEIGHT_0 3480, 2288
#define XPAN_HEIGHT_1 2784, 1824
#define XPAN_HEIGHT_2 1968, 1296
#define XPAN_HEIGHT_3 1080, 712
#define XPAN_SIZES 6192, 2288, 4944, 1824, 3504, 1296, 1920, 712
/* Size classifier 0x533a2820 at its aspect BL: the frame is
   {r3-r6,fp,ip,lr,pc}, r6 = width, r4 = height, r5 = class pointer. */
#define CLASSIFY_WIDTH r6
#define CLASSIFY_CLASS r5
#define CLASSIFY_RETURN ldm sp, {r3,r4,r5,r6,fp,sp,pc}

/* Allocation and the vector helpers of View children. */
#define FW_ALLOCATE FIRMWARE(0x538e09b0)
#define FW_VECTOR_INSERT FIRMWARE(0x538eb348)
#define FW_VECTOR_ERASE FIRMWARE(0x538eb4cc)

/* View classes. Their vtables and object sizes equal those of GR III 2.10. */
#define FW_VIEW_SET_TOUCH FIRMWARE(0x535885a4)
#define FW_VIEW_SET_PARENT FIRMWARE(0x53588814)
#define FW_VIEW_SET_TOUCHABLE FIRMWARE(0x535888a8)
#define FW_VIEW_SET_X FIRMWARE(0x53588c5c)
#define FW_VIEW_INVALIDATE FIRMWARE(0x535899fc)
#define FW_VIEW_SET_VISIBLE FIRMWARE(0x53589ab4)
#define FW_LINE_VIEW_INIT FIRMWARE(0x535700c0)
#define FW_LINE_VIEW_SET_STYLE FIRMWARE(0x5357013c)
#define FW_LINE_VIEW_DESTROY FIRMWARE(0x53570090)
#define FW_ICON_VIEW_INIT FIRMWARE(0x53576da0)
#define FW_ICON_VIEW_SET_ICON FIRMWARE(0x53576e38)
#define FW_ICON_VIEW_DESTROY FIRMWARE(0x53576d0c)
#define FW_STRING_VIEW_RESET FIRMWARE(0x5357338c)
#define FW_LIST_SET_COUNT FIRMWARE(0x535711e4)
#define FW_LIST_ROW FIRMWARE(0x53570a9c)
#define FW_LIST_SAVE FIRMWARE(0x53570b3c)
#define FW_LIST_CURSOR FIRMWARE(0x53570b6c)
#define FW_LIST_WIDTH FIRMWARE(0x53570c10)
#define FW_LIST_RESTORE FIRMWARE(0x5357157c)
#define FW_LIST_SET_CURSOR FIRMWARE(0x53571788)
#define FW_LIST_MOVE FIRMWARE(0x535715b8)
/* The list\'s row vector (first and end pointers) and its shown row count. */
#define LIST_ROWS 0xf4
#define LIST_COUNT 0x108
#define FW_ROW_SET_ICON FIRMWARE(0x5357dd98)
#define FW_ROW_SET_CHECK FIRMWARE(0x5357e018)

/* AdjModeController: methods and the touch callback pair of a toolbar icon. */
#define FW_ADJ_TOUCH_ENTER FIRMWARE(0x531bb7c0)
#define FW_ADJ_TOUCH_LEAVE FIRMWARE(0x531bb8ec)
#define FW_ADJ_SLOT_TYPE FIRMWARE(0x531bd580)
#define FW_ADJ_SNAPSHOT FIRMWARE(0x531bcdf0)
/* The slot type of the native Image Control slot, and the controller of its
   Standard parameter page (still mode). */
#define ADJ_IC_SLOT_TYPE 15
#define IC_PARAMETERS_CONTROLLER 0x2000017
/* ImageControlDetailController, as on GR III with other fields: the type
   (+0x19c), the set kind (+0x19d), and the base kind of its row table
   (+0x19e, 0 Standard). The model snapshot holds one byte for each row kind
   from +3. GR IV has one more row kind before shading, so shading and clarity
   are kinds 14 and 15. */
/* The main title view of the page, as on GR III. */
#define DETAIL_TITLE 0x15c
#define DETAIL_TYPE 0x19c
#define DETAIL_SET_KIND 0x19d
#define DETAIL_BASE_KIND 0x19e
#define MODEL_SNAPSHOT 3
#define PARAMETER_KINDS {0,1,2,3,4,5,6,14,15}
#define DETAIL_EV_SUBPAGE 0x8c
#define FW_ADJ_REFRESH FIRMWARE(0x531bf3c8)
#define FW_ADJ_SELECT_SLOT FIRMWARE(0x531bf94c)
#define FW_CHANGE_CONTROLLER FIRMWARE(0x53182e4c)
/* AdjModeController fields. GR IV adds two FunctionGuideViews before the list,
   which moves the list and the slot byte by 8. */
#define ADJ_CANVAS 0x4
/* The UI language byte of the static settings object 0x55084dd8 (reader
   0x5329b068, +0x1497), with the GR III values. Not confirmed on a camera. */
#define LANGUAGE_SETTING_BYTE (FIRMWARE(0x55084dd8) + 0x1497)
/* The live-view display mode that DISP cycles, in the same object, with the
   GR III values: 0 full, 1 simple, 2 no information, 3 display off. */
#define DISPLAY_MODE_BYTE (FIRMWARE(0x55084dd8) + 0x1409)
#define ADJ_TITLE 0x160
#define ADJ_DETAIL 0x164
#define ADJ_DETAIL_VALUE 0x168
#define ADJ_CELLS 0x16c
#define ADJ_FRAME 0x180
#define ADJ_ICONS 0x184
#define ADJ_FN_GUIDE 0x1ac
/* The Image Control slot method of the ADJ refresh fills the Fn hint with the
   Fn label, layout 10, and the settings icon, and moves it to x 300. A new ADJ
   starts with an empty hint. */
#define FW_GUIDE_SET_CONTENT FIRMWARE(0x53577c6c)
#define FN_GUIDE_TEXT 0x1de
#define FN_GUIDE_ICON 0x240
#define FN_GUIDE_X 300
#define ADJ_LIST 0x1f8
#define ADJ_SLOT 0x1fc
#define ADJ_ACTION 0x281
/* 1 in movie mode. GR IV computes the mode from the camera settings. */
#define ADJ_MOVIE(controller) (((u32 (*)(u32))FIRMWARE(0x531bcd00))(controller))

/* AdjModeController events (the first halfword of an event). Measured, except
   the dial events, which follow the +6 offset of the direction events. The
   touch event is the one that the native handler tests like GR III 0x30. */
#define EV_SWITCH 5
#define EV_SWITCH_END 8
#define EV_FN 0x1b
/* An Fn press comes as EV_FN with argument 2, as on GR III, then EV_FN_ACTION,
   then EV_FN with argument 1 at the release. The native ADJ handler ignores
   EV_FN and acts on EV_FN_ACTION (0x531bd388). */
#define FN_PRESS_ARG 2
#define EV_FN_ACTION 0x187
/* A press of the rear dial, which opens and closes ADJ, before the confirm
   that OK also sends. */
#define EV_ADJ_PRESS 0x23
#define EV_TOUCH 0x31
#define EV_LEFT 0xd4
#define EV_RIGHT 0xd5
#define EV_UP 0xd6
#define EV_DOWN 0xd7
#define EV_DIAL_UP 0xda
#define EV_DIAL_DOWN 0xdb
#define EV_CONFIRM 0xe0

/* GR IV draws ADJ before the toolbar exists, so the new cell needs a redraw. */
#define TOOLBAR_CREATED(widget,icon) \
    (CALL1(FW_VIEW_INVALIDATE)(widget), CALL1(FW_VIEW_INVALIDATE)(icon))

/* The installer always provides the GRKit Off icon. */
#define NATIVE_OFF_ICON() 0

/* Playback metadata singleton (+0x228 holds key 0x25, the Image Control tag). */
#define FW_PLAYBACK_METADATA FIRMWARE(0x5323f67c)
#define METADATA_TONE 0x228
/* Maker-note IFD entry setter: (entry, 0, value) stores a u16 value. */
#define FW_IFD_SET_SHORT FIRMWARE(0x537e7148)

/* LiveViewController: nonzero in movie mode, and the LCD canvas. GR IV adds a
   FrameView at 0x2d0, which moves the movie flag by 4. */
#define LIVEVIEW_MOVIE 0x2d6
#define LIVEVIEW_CANVAS 0x15c

/* Picture processing. GAMMA_BUILT: the word of the gamma object that is 1
   after the native curves are built. */
#define GAMMA_BUILT 0x382c
#define FW_YCBCR_CALC FIRMWARE(0x5391a650)
/* DigitalFilterProcess plan: the grain stage flag that the stage gate sets. */
#define GRAIN_GATE_FLAG 0x25

/* Date stamp: cache maintenance and the local time. */
#define FW_CACHE_FLUSH FIRMWARE(0x53a95804)
#define FW_CLOCK FIRMWARE(0x538f21a0)
#define FW_LOCAL_TIME FIRMWARE(0x538ea9b0)
/* Each native aspect has its own visible area in a preview picture. */
#define PICTURE_NATIVE_ID(native) ((native)<=3 ? (native) : 0)

#endif

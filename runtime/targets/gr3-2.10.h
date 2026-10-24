/* RICOH GR III 2.10 (also GR IIIx 1.60): firmware entry points, data, and
   object layouts that the runtime uses. The addresses are those of the
   pinned 2.10 image. Each is a FIRMWARE() symbol that the installer
   relocates, as on GR IV. gr3.h has the values that the shared runtime takes
   from each family. */
#ifndef GRKIT_TARGET_H
#define GRKIT_TARGET_H

#include "gr3.h"

/* Camera settings and the still aspect property. */
#define FW_SETTINGS FIRMWARE(0x5320b31c)
#define FW_STILL_ASPECT_GET FIRMWARE(0x53245260)
#define FW_STILL_ASPECT_SET FIRMWARE(0x532525a4)
/* ShootingModel singleton, and the two requests with which a controller that
   needs no live view stops it (Controller live-view dispatch 0x5316d9b4,
   state 1). Each takes the model and a completion callback. */
#define FW_SHOOTING_MODEL FIRMWARE(0x5320b19c)
#define FW_LIVE_VIEW_END FIRMWARE(0x53238b40)
#define FW_LIVE_VIEW_STOP FIRMWARE(0x53238724)
#define FW_CAPTURE_PROPERTIES FIRMWARE(0x53525d5c)
/* Settings domain: +0x518 -> control -> payload with the native Image Control
   type at +0x77c. Standard is type 11. */
#define FW_SETTINGS_DOMAIN FIRMWARE(0x553458b0)
#define SETTINGS_IC_TYPE 0x77c
/* The native monochrome types, as a bit mask of the type: 13 to 16 are
   Monotone, Soft Monotone, Hard Monotone, and Hi-Contrast B&W (ImageTone 258
   to 261, table 0x53d2b6f4). The shot settings hold the type at the same
   offset as the settings payload. */
#define NATIVE_MONOCHROME_TYPES 0x0001e000U

/* Allocation and the vector helpers of View children. */
#define FW_ALLOCATE FIRMWARE(0x536af2a8)
#define FW_VECTOR_INSERT FIRMWARE(0x536b6de4)
#define FW_VECTOR_ERASE FIRMWARE(0x536b6f68)

/* View classes. */
#define FW_VIEW_SET_TOUCH FIRMWARE(0x533abeb0)
#define FW_VIEW_SET_PARENT FIRMWARE(0x533ac120)
#define FW_VIEW_SET_TOUCHABLE FIRMWARE(0x533ac1b4)
#define FW_VIEW_SET_X FIRMWARE(0x533ac568)
#define FW_VIEW_INVALIDATE FIRMWARE(0x533ad308)
#define FW_VIEW_SET_VISIBLE FIRMWARE(0x533ad3c0)
#define FW_LINE_VIEW_INIT FIRMWARE(0x53393d84)
#define FW_LINE_VIEW_SET_STYLE FIRMWARE(0x53393e00)
#define FW_LINE_VIEW_DESTROY FIRMWARE(0x53393d54)
#define FW_ICON_VIEW_INIT FIRMWARE(0x5339a908)
#define FW_ICON_VIEW_SET_ICON FIRMWARE(0x5339a9a0)
#define FW_ICON_VIEW_DESTROY FIRMWARE(0x5339a874)
#define FW_STRING_VIEW_RESET FIRMWARE(0x53396f80)
#define FW_LIST_SET_COUNT FIRMWARE(0x53394e8c)
#define FW_LIST_ROW FIRMWARE(0x53394758)
#define FW_LIST_SAVE FIRMWARE(0x533947f0)
#define FW_LIST_CURSOR FIRMWARE(0x53394820)
#define FW_LIST_WIDTH FIRMWARE(0x533948c4)
#define FW_LIST_RESTORE FIRMWARE(0x53395224)
#define FW_LIST_SET_CURSOR FIRMWARE(0x53395430)
#define FW_LIST_MOVE FIRMWARE(0x53395260)
/* The list\'s row vector (first and end pointers) and its shown row count. */
#define LIST_ROWS 0xf4
#define LIST_COUNT 0x108
#define FW_ROW_SET_ICON FIRMWARE(0x533a1734)
#define FW_ROW_SET_CHECK FIRMWARE(0x533a19b4)

/* AdjModeController: methods and the touch callback pair of a toolbar icon. */
#define FW_ADJ_SNAPSHOT FIRMWARE(0x531a0150)
#define FW_ADJ_TOUCH_ENTER FIRMWARE(0x531a16c4)
#define FW_ADJ_TOUCH_LEAVE FIRMWARE(0x531a1874)
#define FW_ADJ_SLOT_TYPE FIRMWARE(0x531a2820)
/* The slot type of the native Image Control slot, and the controller of its
   Standard parameter page (still mode). */
#define ADJ_IC_SLOT_TYPE 12
#define IC_PARAMETERS_CONTROLLER 0x2000014
/* ImageControlDetailController, the parameter page. Its start method sets the
   type (+0x198), the set kind (+0x199: 0 the base set, 1 and 2 Custom), and
   the base kind of its row table (+0x19a: 0 Standard). The Image Control
   model keeps a snapshot of the adjustments from +3, one byte for each row
   kind, and a cancel writes the snapshot back through the model setters. */
/* The main title view of the page. UpdateViewMainTitle sets its text from the type. */
#define DETAIL_TITLE 0x15c
#define DETAIL_TYPE 0x198
#define DETAIL_SET_KIND 0x199
#define DETAIL_BASE_KIND 0x19a
#define MODEL_SNAPSHOT 3
/* The row kinds of the Standard adjustments, in GRKIT_RECORD_PARAMETERS order. */
#define PARAMETER_KINDS {0,1,2,3,4,5,6,13,14}
/* The page event that opens another controller (0x2000015) from the page. */
#define DETAIL_EV_SUBPAGE 0x88
/* The native settings page of a GRKit root row (runtime/menu.c). It borrows
   the MENU submenu Instant Review (controller 0x20000be, entry 10 of the
   submenu kind table) and its value pages Display Time (0x20000c3) and Zoom
   Review (0x20000c4). A kind table entry holds the row count at +6, the row
   label IDs (u16) at +8, the row controllers (u32) at +0x14, and the row value
   methods (C++ member pointers, 8 bytes) at +0x3c. A value method gets the
   submenu and the row. */
#define FW_SUBMENU_KINDS FIRMWARE(0x53b3fb10)
#define SUBMENU_KIND_SIZE 0x64
#define SUBMENU_KIND_ROWS 6
#define SUBMENU_KIND_CONTROLLERS 0x14
#define SUBMENU_KIND_VALUES 0x3c
#define SETTINGS_SUBMENU 0x20000beU
#define SETTINGS_SUBMENU_KIND 10
#define SETTINGS_PAGES {0x20000c3U,0x20000c4U}
/* The text IDs of the submenu title and of its two rows. The value pages show
   the label of their row as title. */
#define SETTINGS_TITLE_TEXT 0x1e7
#define SETTINGS_ROW_TEXTS {0x1e9,0x1eb}
/* Row text setters: a value page row sets the string view at +0xf8, and a
   submenu row its value view at +0xfc. Each sets a text ID. */
#define FW_ROW_TEXT FIRMWARE(0x5339e7a0)
#define ROW_TEXT_VIEW 0xf8
#define FW_ROW_VALUE FIRMWARE(0x533a6ad0)
#define ROW_VALUE_VIEW 0xfc
#define FW_ADJ_REFRESH FIRMWARE(0x531a4114)
#define FW_ADJ_SELECT_SLOT FIRMWARE(0x531a4864)
#define FW_CHANGE_CONTROLLER FIRMWARE(0x5316c9d8)
/* AdjModeController fields. */
#define ADJ_CANVAS 0x4
/* The current UI settings object is at this slot, +4 in the object
   0x55063f30, which firmware code loads. LANGUAGE_SETTING is its UI language
   byte (reader 0x532465dc): 18 is Traditional Chinese and 19 Simplified
   Chinese. */
#define UI_SETTINGS_SLOT (FIRMWARE(0x55063f30) + 4)
#define LANGUAGE_SETTING 0x9ba
/* The live-view display mode that DISP cycles, in the same settings object:
   0 full, 1 simple, 2 no information, 3 display off. */
#define DISPLAY_MODE_SETTING 0x912
#define ADJ_TITLE 0x160
#define ADJ_DETAIL 0x164
#define ADJ_DETAIL_VALUE 0x168
#define ADJ_CELLS 0x16c
#define ADJ_FRAME 0x180
#define ADJ_ICONS 0x184
#define ADJ_FN_GUIDE 0x1ac
/* The ADJ refresh fills the Fn hint only on a native slot that offers Fn. For
   the Image Control slot, it uses the Fn label, layout 10, and the settings
   icon. A new ADJ starts with an empty hint. */
#define FW_GUIDE_SET_CONTENT FIRMWARE(0x5339b7d4)
#define FN_GUIDE_TEXT 0x1cf
#define FN_GUIDE_ICON 0x233
#define ADJ_LIST 0x1f0
#define ADJ_SLOT 0x1f4
/* 1 in movie mode. */
#define ADJ_MOVIE(controller) BYTE(controller,0x270)
#define ADJ_ACTION 0x279

/* AdjModeController events (the first halfword of an event). */
#define EV_SWITCH 5
#define EV_SWITCH_END 8
#define EV_FN 0x1b
/* An ADJ press, before the confirm that OK also sends. */
#define EV_ADJ_PRESS 0x1e
/* The argument of an Fn press. ADJ opens the native parameters on it. */
#define FN_PRESS_ARG 2
#define EV_TOUCH 0x30
#define EV_LEFT 0xce
#define EV_RIGHT 0xcf
#define EV_UP 0xd0
#define EV_DOWN 0xd1
#define EV_DIAL_UP 0xd4
#define EV_DIAL_DOWN 0xd5
#define EV_CONFIRM 0xd8

/* Playback metadata singleton (+0x220 holds key 0x25, the Image Control tag)
   and the native icon table. */
#define FW_PLAYBACK_METADATA FIRMWARE(0x5320b97c)
#define METADATA_TONE 0x220
#define FW_ICON_TABLE FIRMWARE(0x550020fc)
/* The native Off icon, used until the installer provides the GRKit one. */
#define NATIVE_OFF_ICON() WORD(FW_ICON_TABLE,0x166*4)
/* Maker-note IFD entry setter: (entry, 0, value) stores a u16 value. */
#define FW_IFD_SET_SHORT FIRMWARE(0x535d02c8)
/* LiveViewController: nonzero in movie mode, and the LCD canvas. */
#define LIVEVIEW_MOVIE 0x2d2
#define LIVEVIEW_CANVAS 0x15c

/* Picture processing. GAMMA_BUILT: the word of the gamma object that is 1
   after the native curves are built. */
#define GAMMA_BUILT 0x3230
/* GAMMA_UPLOAD: the byte of the gamma object that makes the table build use
   the composed curves. */
#define GAMMA_UPLOAD 0x320a
#define FW_YCBCR_CALC FIRMWARE(0x536e4720)
/* DigitalFilterProcess plan: the grain stage flag that the stage gate sets. */
#define GRAIN_GATE_FLAG 0x24
#define FW_CACHE_FLUSH FIRMWARE(0x5383c8a8)
#define FW_CLOCK FIRMWARE(0x536bd838)
#define FW_LOCAL_TIME FIRMWARE(0x536b6510)

#endif

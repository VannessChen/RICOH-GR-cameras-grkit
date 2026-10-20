/* RICOH GR IV 1.20 (also GR IV HDF): the firmware addresses and the values of
   the 1.20 build. gr4.h has the layouts and constants of each GR IV build and
   describes what each address is. The addresses are those of the pinned 1.20
   build; each address is a FIRMWARE() symbol that the installer relocates.
   Data that no firmware code loads directly is an offset from an address that
   it does load. The values differ from 1.11 where a comment names 1.20: the
   static settings object grew by 0x58 bytes before its UI fields, the capture
   property singleton by one word, 1.20 adds two monochrome Image Control
   types, and the menu text IDs move. Not tested on a camera. */
#ifndef GRKIT_TARGET_H
#define GRKIT_TARGET_H

#include "gr4.h"

/* The icon draw compares the icon ID with this native limit as an immediate
   (1.11 loads 0x296 into r2). */
#define NATIVE_ICON_LIMIT 0x2c4

/* Camera settings, the still aspect getter, and the settings domain after the
   vtable of the capture property singleton. */
#define FW_SETTINGS FIRMWARE(0x53240c38)
#define FW_STILL_ASPECT_GET FIRMWARE(0x532a03e0)
#define FW_SETTINGS_DOMAIN (FIRMWARE(0x5537b29c) + 4)
/* The native monochrome types, as bit masks of the type: 13 to 16 as on
   GR III, 25 to 30 as on 1.11, and the 1.20 types 31 (ImageTone 0x8107) and
   32 (0x8106) of the table 0x53fd7fc8. NATIVE_MONOCHROME_TYPES_HIGH holds the
   types from 32. */
#define NATIVE_MONOCHROME_TYPES 0xfe01e000U
#define NATIVE_MONOCHROME_TYPES_HIGH 0x00000001U
/* The capture aspect getter, which the live-view mode function 0x538b40c8 and
   the mode check 0x5366440c call, and the still aspect resend. */
#define FW_CAPTURE_ASPECT_GET FIRMWARE(0x5377d960)
#define FW_STILL_ASPECT_RESEND FIRMWARE(0x5328634c)
/* The size classifier is 0x533b205c. */

/* Allocation and the vector helpers of View children. */
#define FW_ALLOCATE FIRMWARE(0x539084bc)
#define FW_VECTOR_INSERT FIRMWARE(0x53912f10)
#define FW_VECTOR_ERASE FIRMWARE(0x53913094)

/* View classes. */
#define FW_VIEW_SET_TOUCH FIRMWARE(0x535ab874)
#define FW_VIEW_SET_PARENT FIRMWARE(0x535abae4)
#define FW_VIEW_SET_TOUCHABLE FIRMWARE(0x535abb78)
#define FW_VIEW_SET_X FIRMWARE(0x535abf2c)
#define FW_VIEW_INVALIDATE FIRMWARE(0x535acccc)
#define FW_VIEW_SET_VISIBLE FIRMWARE(0x535acd84)
#define FW_LINE_VIEW_INIT FIRMWARE(0x53593388)
#define FW_LINE_VIEW_SET_STYLE FIRMWARE(0x53593404)
#define FW_LINE_VIEW_DESTROY FIRMWARE(0x53593358)
#define FW_ICON_VIEW_INIT FIRMWARE(0x5359a06c)
#define FW_ICON_VIEW_SET_ICON FIRMWARE(0x5359a104)
#define FW_ICON_VIEW_DESTROY FIRMWARE(0x53599fd8)
#define FW_STRING_VIEW_RESET FIRMWARE(0x53596654)
#define FW_LIST_SET_COUNT FIRMWARE(0x535944ac)
#define FW_LIST_ROW FIRMWARE(0x53593d64)
#define FW_LIST_SAVE FIRMWARE(0x53593e04)
#define FW_LIST_CURSOR FIRMWARE(0x53593e34)
#define FW_LIST_WIDTH FIRMWARE(0x53593ed8)
#define FW_LIST_RESTORE FIRMWARE(0x53594844)
#define FW_LIST_SET_CURSOR FIRMWARE(0x53594a50)
#define FW_LIST_MOVE FIRMWARE(0x53594880)
#define FW_ROW_SET_ICON FIRMWARE(0x535a1064)
#define FW_ROW_SET_CHECK FIRMWARE(0x535a12e4)

/* AdjModeController: methods and the touch callback pair of a toolbar icon. */
#define FW_ADJ_TOUCH_ENTER FIRMWARE(0x531bccf8)
#define FW_ADJ_TOUCH_LEAVE FIRMWARE(0x531bce24)
#define FW_ADJ_SLOT_TYPE FIRMWARE(0x531beab8)
#define FW_ADJ_SNAPSHOT FIRMWARE(0x531be328)
/* The settings page: the MENU submenu kind table, the text IDs of Instant
   Review, Display Time, and Zoom Review (1.20 moves them by 0x24), and the row
   text setters. */
#define FW_SUBMENU_KINDS FIRMWARE(0x53dd0f00)
#define SETTINGS_TITLE_TEXT 0x228
#define SETTINGS_ROW_TEXTS {0x22a,0x22c}
#define FW_ROW_TEXT FIRMWARE(0x5359e0ac)
#define FW_ROW_VALUE FIRMWARE(0x535a64b4)
#define FW_ADJ_REFRESH FIRMWARE(0x531c0900)
#define FW_ADJ_SELECT_SLOT FIRMWARE(0x531c0e84)
#define FW_CHANGE_CONTROLLER FIRMWARE(0x5318408c)
/* The UI language byte of the static settings object 0x550832a8 (reader
   0x5329ef68, +0x14ef; 1.11 +0x1497), with the GR III values. Not confirmed
   on a camera. */
#define LANGUAGE_SETTING_BYTE (FIRMWARE(0x550832a8) + 0x14ef)
/* The live-view display mode that DISP cycles, in the same object (reader
   0x5329ec5c, +0x1461; 1.11 +0x1409), with the GR III values: 0 full,
   1 simple, 2 no information, 3 display off. */
#define DISPLAY_MODE_BYTE (FIRMWARE(0x550832a8) + 0x1461)
#define FW_GUIDE_SET_CONTENT FIRMWARE(0x5359af38)
/* The Fn label and the settings icon of the Fn hint, as the native ADJ
   refresh gives them to FW_GUIDE_SET_CONTENT. 1.20 moves both IDs. */
#define FN_GUIDE_TEXT 0x1fd
#define FN_GUIDE_ICON 0x26a
/* 1 in movie mode. GR IV computes the mode from the camera settings. */
#define ADJ_MOVIE(controller) (((u32 (*)(u32))FIRMWARE(0x531be238))(controller))
/* The native ADJ handler acts on EV_FN_ACTION at 0x531be8c0. */

/* Playback metadata singleton and the maker-note IFD entry setter: (entry, 0,
   value) stores a u16 value. */
#define FW_PLAYBACK_METADATA FIRMWARE(0x5324129c)
#define FW_IFD_SET_SHORT FIRMWARE(0x5380dd48)

/* Picture processing: the YCbCr matrix calculation. */
#define FW_YCBCR_CALC FIRMWARE(0x53942440)
/* The processing hooks (gr4-processing.S) keep the type table index in the
   frame slot fp-0x30 (1.11 fp-0x34). The function stores the highlight
   correction flag of the capture in fp-0x34 (1.11 fp-0x30) before the hooks. */
#define PROCESSING_INDEX_SLOT 0x30
#define PROCESSING_HIGHLIGHT_SLOT 0x34

/* Date stamp: cache maintenance and the local time. */
#define FW_CACHE_FLUSH FIRMWARE(0x53abdb48)
#define FW_CLOCK FIRMWARE(0x53919d68)
#define FW_LOCAL_TIME FIRMWARE(0x53912578)

#endif

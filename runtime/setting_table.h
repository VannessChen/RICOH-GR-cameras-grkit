#ifndef GRKIT_SETTING_TABLE_H
#define GRKIT_SETTING_TABLE_H
/* The layout of the settings (runtime/settings.h), which INSTALL also
   writes. The settings table has the number of settings in word 0 and a
   count of changes in word 1. Then each setting has three words: its state
   file ID (0 when it is not saved), its value, and its number of values.
   The includer defines u16 and the header fields (runtime/abi.h). */
#define SETTINGS_ENTRY(handle) (2U + (handle) * 3U)
/* The capabilities of the core (GRKIT_CORE_<KEY>): each has a header word
   and a number of values. The resident loader saves the words. */
#define CORE_SETTING(handle) ((handle)-GRKIT_CORE_SETTING_BASE)
#define IS_CORE_SETTING(handle) (CORE_SETTING(handle)<GRKIT_CORE_SETTING_COUNT)
static __attribute__((unused)) const u16 core_setting_words[]={GRKIT_CORE_SETTING_WORDS};
static __attribute__((unused)) const u16 core_setting_values[]={GRKIT_CORE_SETTING_VALUES};
#endif

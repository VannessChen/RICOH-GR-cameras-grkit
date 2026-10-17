#ifndef GRKIT_SELECTION_H
#define GRKIT_SELECTION_H
/* PLUGINS.TXT names the plugins to install, separated by ASCII whitespace. The
   result has bit n set for plugin n, which is module n of runtime/modules.h.
   grkit_core::modules implements the same rules. Callers define u8 and u32. */
#define SELECTION_FILE_MAX 256U
#define SELECTION_PLUGINS 5U
#define SELECTION_ALL ((1U<<SELECTION_PLUGINS)-1)
static const char *const selection_names[SELECTION_PLUGINS] = {"image-control", "framing", "guides", "date-stamp", "grain"};
static int selection_space(u8 c) { return c==' ' || c=='\t' || c=='\r' || c=='\n'; }
static int selection_name(const u8 *text, u32 n, const char *name)
{
    u32 i=0;
    while (i<n && name[i] && text[i]==(u8)name[i]) ++i;
    return i==n && !name[i];
}
/* Returns 0 for an unknown or repeated name. An empty file selects nothing. */
static int selection_parse(const u8 *text, u32 n, u32 *selected)
{
    u32 i=0, result=0;
    while (i<n) {
        u32 start, k=0;
        while (i<n && selection_space(text[i])) ++i;
        if (i==n) break;
        start=i;
        while (i<n && !selection_space(text[i])) ++i;
        while (k<SELECTION_PLUGINS && !selection_name(text+start,i-start,selection_names[k])) ++k;
        if (k==SELECTION_PLUGINS || (result & 1U<<k)) return 0;
        result |= 1U<<k;
    }
    *selected=result;
    return 1;
}
#endif

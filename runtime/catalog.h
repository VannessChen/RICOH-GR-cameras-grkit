#ifndef GRKIT_CATALOG_H
#define GRKIT_CATALOG_H
/* Effect catalog selection for the installer. The includer defines u8 and u32.
   A file that cannot be used never prevents the other effects from loading. */
#include "grf.h"

/* An 8.3 file name and its terminator. */
#define CATALOG_NAME_SIZE 13U
/* The NUL-terminated effect ID in a GRF record. */
#define CATALOG_ID GRKIT_GRF_ID
/* The effect name in a GRF record: at most CATALOG_EFFECT_NAME_MAX
   characters, then NULs. */
#define CATALOG_EFFECT_NAME GRKIT_GRF_NAME
#define CATALOG_EFFECT_NAME_MAX GRKIT_GRF_NAME_MAX

static int catalog_compare(const u8 *a, const u8 *b)
{ while (*a && *a==*b) {++a; ++b;} return (int)*a-(int)*b; }
static void catalog_copy(u8 *a, const u8 *b, u32 n)
{ for (u32 i=0; i<n; ++i) a[i] = b[i]; }
/* Copy an accepted file name through its terminator only. */
static void catalog_copy_name(char *a, const char *b)
{ u32 i=0; do a[i]=b[i]; while(b[i++]); }

/* Return one for an effect file name: NAME.GRF with 1 to 8 letters, digits, or
   underscores before the suffix. Other names, including the "._" companions
   that macOS writes on FAT cards, are not effect files. */
static int catalog_name(const char *name, u32 n)
{
    if(n<5 || n>12 || !((name[n-4]=='.') && (name[n-3]=='G'||name[n-3]=='g') &&
       (name[n-2]=='R'||name[n-2]=='r') && (name[n-1]=='F'||name[n-1]=='f'))) return 0;
    for(u32 i=0;i<n-4;++i) {
        char c=name[i];
        if(!((c>='a'&&c<='z')||(c>='A'&&c<='Z')||(c>='0'&&c<='9')||c=='_')) return 0;
    }
    return 1;
}
/* Offer a valid profile from the named file and return the new count. The
   catalog keeps the `capacity` smallest IDs in ID order: for each, the first
   `stride` bytes of its profile, which hold its ID and name, and its file
   name. For a duplicate ID, the file with the smallest name wins. The result
   does not depend on directory order. */
static u32 catalog_offer(u8 *records, u32 stride, char (*names)[CATALOG_NAME_SIZE],
                         u32 count, u32 capacity, const u8 *candidate, const char *name)
{
    u32 i=0;
    while(i<count && catalog_compare(records+i*stride+CATALOG_ID,candidate+CATALOG_ID)<0) ++i;
    if(i<count && !catalog_compare(records+i*stride+CATALOG_ID,candidate+CATALOG_ID)) {
        if(catalog_compare((const u8 *)names[i],(const u8 *)name)>0) {
            catalog_copy(records+i*stride,candidate,stride);
            catalog_copy_name(names[i],name);
        }
        return count;
    }
    if(i==capacity) return count;
    /* A full catalog drops its largest ID. */
    if(count==capacity) --count;
    for(u32 j=count;j>i;--j) {
        catalog_copy(records+j*stride,records+(j-1)*stride,stride);
        catalog_copy_name(names[j],names[j-1]);
    }
    catalog_copy(records+i*stride,candidate,stride);
    catalog_copy_name(names[i],name);
    return count+1;
}
#endif

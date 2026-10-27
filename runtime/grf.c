/* Shared GRFX0001 validation for the installer and host tools. The layout
   (GRKIT_GRF_*) comes from runtime/abi.h, which grkit_core::abi generates. */
#include "abi.h"
#include "grf.h"
#ifndef GRKIT_GRF_EMBEDDED
typedef unsigned char u8;
typedef unsigned int u32;
#endif
#ifdef GRKIT_GRF_EMBEDDED
#define grf_get32 get32
#define grf_equal equal
#else
static u32 grf_get32(const u8 *p)
{ return p[0] | p[1] << 8 | p[2] << 16 | (u32)p[3] << 24; }
static int grf_equal(volatile const u8 *a, const u8 *b, u32 n)
{ for (u32 i=0; i<n; ++i) if (a[i] != b[i]) return 0; return 1; }
#endif
static u32 crc(const u8 *p, u32 size)
{
    u32 value=~0U;
    for (u32 i=0; i<size; ++i) {
        value ^= (i>=GRKIT_GRF_CRC && i<GRKIT_GRF_CRC+GRKIT_GRF_CRC_SIZE) ? 0 : p[i];
        for (u32 bit=0; bit<8; ++bit) value=(value>>1)^((0U-(value&1))&0xedb88320U);
    }
    return ~value;
}
static int zeros(const u8 *p, u32 n) { for(u32 i=0;i<n;++i) if(p[i]) return 0; return 1; }
/* The extension features and the hue sector layout (GRKIT_FEATURE_*, GRKIT_HUE_*)
   come from abi.h, which grkit_core::abi generates. */
static u32 grf_get16(const u8 *p) { return p[0] | p[1] << 8; }
/* The bounds of grkit_core::selective: a Q12 unit direction, inner < outer <= 1024
   on each side, and c0 < c1 <= 181. */
static int valid_selective(const u8 *p)
{
    int c=(short)grf_get16(p), s=(short)grf_get16(p+2);
    u32 square=(u32)(c*c)+(u32)(s*s);
    if (square<15000000U || square>18000000U) return 0;
    for (u32 side=0; side<2; ++side) {
        u32 inner=grf_get16(p+4+side*4), outer=grf_get16(p+6+side*4);
        if (inner>=outer || outer>1024) return 0;
    }
    return p[12]<p[13] && p[13]<=181;
}
/* Each feature bit owns its field, and an unused field stays zero. An effect
   without features has zeros in the whole extension. A monochrome effect has
   no features. */
static int valid_extension(const u8 *p)
{
    u32 features=grf_get32(p+GRKIT_GRF_FEATURES);
    if ((features & ~(GRKIT_FEATURE_POST|GRKIT_FEATURE_SELECTIVE|GRKIT_FEATURE_HUE|GRKIT_FEATURE_HUE_LEVELS)) ||
        !zeros(p+GRKIT_GRF_RESERVED,GRKIT_GRF_RESERVED_SIZE)) return 0;
    if ((features&GRKIT_FEATURE_HUE_LEVELS) && !(features&GRKIT_FEATURE_HUE)) return 0;
    if (!(features&GRKIT_FEATURE_POST) && !zeros(p+GRKIT_GRF_POST,GRKIT_GRF_POST_SIZE)) return 0;
    if (!(features&GRKIT_FEATURE_SELECTIVE) && !zeros(p+GRKIT_GRF_SELECTIVE,GRKIT_GRF_SELECTIVE_SIZE)) return 0;
    if (features && grf_get32(p+GRKIT_GRF_FLAGS)==1) return 0;
    if ((features&GRKIT_FEATURE_SELECTIVE) && !valid_selective(p+GRKIT_GRF_SELECTIVE)) return 0;
    return 1;
}
/* Each row's diagonal, 128 minus its two coefficients, fits the 12-bit register
   in Q10. */
static int valid_sectors(const u8 *p, u32 n)
{
    for (u32 i=0; i<n; i+=2) {
        int a=(signed char)p[i], b=(signed char)p[i+1];
        if (a==-128 || b==-128 || a+b<-127) return 0;
    }
    return 1;
}
/* The hue sectors of feature 8 and the levels of feature 16, or zeros. */
static int valid_hue(const u8 *p, u32 size)
{
    u32 used=(grf_get32(p+GRKIT_GRF_FEATURES)&GRKIT_FEATURE_HUE) ? GRKIT_GRF_HUE_SIZE : 0;
    if (!zeros(p+GRKIT_GRF_HUE+used,GRKIT_GRF_SIZE-GRKIT_GRF_HUE-used)) return 0;
    if (size==GRKIT_GRF_MAX_SIZE && (!valid_sectors(p+GRKIT_HUE_LEVEL_OFFSET,GRKIT_HUE_LEVEL_SIZE) ||
        !zeros(p+GRKIT_HUE_LEVEL_OFFSET+GRKIT_HUE_LEVEL_SIZE,size-GRKIT_HUE_LEVEL_OFFSET-GRKIT_HUE_LEVEL_SIZE))) return 0;
    if (!used) return 1;
    if (!valid_sectors(p+GRKIT_GRF_HUE,GRKIT_HUE_MATRICES)) return 0;
    const u8 *f=p+GRKIT_GRF_HUE+GRKIT_HUE_MATRICES;
    static const unsigned short borders[3]=GRKIT_HUE_BORDERS;
    for (u32 k=0; k<4; ++k) if (grf_get16(f+k*2)>1024 || grf_get16(f+8+k*2)>0x1ff) return 0;
    for (u32 k=0; k<3; ++k) if (grf_get16(f+16+k*2)!=borders[k]) return 0;
    return 1;
}
/* The size is GRKIT_GRF_MAX_SIZE exactly with feature 16, else GRKIT_GRF_SIZE. */
static int valid_profile(const u8 *p, u32 size)
{
    const u8 *id=p+GRKIT_GRF_ID, *name=p+GRKIT_GRF_NAME;
    /* A monochrome effect has equal matrix rows and equal curves. */
    const u32 row=GRKIT_GRF_MATRIX_SIZE/3, curve=GRKIT_GRF_CURVES_SIZE/3;
    u32 levels=(grf_get32(p+GRKIT_GRF_FEATURES)&GRKIT_FEATURE_HUE_LEVELS) ? 1 : 0;
    if (size!=(levels ? GRKIT_GRF_MAX_SIZE : GRKIT_GRF_SIZE)) return 0;
    if (!grf_equal(p+GRKIT_GRF_MAGIC,(const u8 *)"GRFX0001",GRKIT_GRF_MAGIC_SIZE) ||
        grf_get32(p+GRKIT_GRF_BYTES)!=size || grf_get32(p+GRKIT_GRF_CRC)!=crc(p,size)) return 0;
    u32 n=0;
    while (n<GRKIT_GRF_ID_SIZE && id[n]) {
        u8 c=id[n];
        if (!((c>='a'&&c<='z')||(c>='0'&&c<='9')||(n&&c=='-'))) return 0;
        ++n;
    }
    if (!n || n>GRKIT_GRF_ID_MAX || !zeros(id+n,GRKIT_GRF_ID_SIZE-n)) return 0;
    n=0;
    while (n<GRKIT_GRF_NAME_SIZE && name[n]) { if(name[n]<32 || name[n]>126) return 0; ++n; }
    if (!n || n>GRKIT_GRF_NAME_MAX || name[0]==' ' || name[n-1]==' ' || !zeros(name+n,GRKIT_GRF_NAME_SIZE-n)) return 0;
    if (grf_get32(p+GRKIT_GRF_FLAGS)>1 || !valid_extension(p) ||
        !zeros(p+GRKIT_GRF_PADDING,GRKIT_GRF_PADDING_SIZE) || !valid_hue(p,size)) return 0;
    for (u32 i=GRKIT_GRF_CURVES;i<GRKIT_GRF_CURVES+GRKIT_GRF_CURVES_SIZE;i+=2) if (((u32)p[i]|p[i+1]<<8)>16383) return 0;
    if(grf_get32(p+GRKIT_GRF_FLAGS)==1 && (!grf_equal(p+GRKIT_GRF_MATRIX,p+GRKIT_GRF_MATRIX+row,row) ||
       !grf_equal(p+GRKIT_GRF_MATRIX,p+GRKIT_GRF_MATRIX+2*row,row) ||
       !grf_equal(p+GRKIT_GRF_CURVES,p+GRKIT_GRF_CURVES+curve,curve) ||
       !grf_equal(p+GRKIT_GRF_CURVES,p+GRKIT_GRF_CURVES+2*curve,curve))) return 0;
    /* The alpha of each icon pixel is 0 or 255. */
    for (u32 i=GRKIT_GRF_ICON+3;i<GRKIT_GRF_ICON+GRKIT_GRF_ICON_SIZE;i+=4) if(p[i]!=0 && p[i]!=255) return 0;
    return 1;
}

#ifndef GRKIT_GRF_EMBEDDED
int grkit_grf_validate(const unsigned char *data, unsigned long size)
{
    return data && (size == GRKIT_GRF_SIZE || size == GRKIT_GRF_MAX_SIZE) && valid_profile(data, (u32)size);
}
#endif

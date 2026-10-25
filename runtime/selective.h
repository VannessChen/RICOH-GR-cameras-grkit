#ifndef GRKIT_SELECTIVE_H
#define GRKIT_SELECTIVE_H
/* Selective color of the effects of CORE.TONE, a part of the color of a
   recorded picture (runtime/picture.c). The includer defines the memory
   access and core_divide (runtime/core.h). */

/* ---- Recorded pictures: selective color ------------------------------------
   An effect with GRKIT_FEATURE_SELECTIVE keeps the chroma of one hue range and
   removes it elsewhere. The hue is the angle of (Cb - 128, Cr - 128). Each side
   of the target hue has an inner and an outer half width: inside the inner one a
   pair keeps all of its chroma, outside the outer one none, and between them the
   weight falls linearly in tan(angle offset). Side 1 is toward larger angles. A
   chroma gate fades out pairs whose chroma along the target is low. Luma stays,
   so the rest of the picture keeps the luma of the effect's color rendering.
   grkit_core::selective computes the same table for the site preview. */
#define SELECTIVE_ALONG 182U /* the largest projection of a CbCr vector, plus one */
#define SELECTIVE_SIDE 128U  /* table cells per axis: one per 2 x 2 (Cb, Cr) block */
#define SELECTIVE_TABLE (SELECTIVE_SIDE*SELECTIVE_SIDE)
typedef struct { int hc,hs; u32 t[2][2],c0,c1; } Selective;
/* Q16 reciprocals: 256 / ((outer - inner) * along) per side, and 256 / (c1 - c0). */
typedef struct { u32 angle[2][SELECTIVE_ALONG],gate; } SelectiveFactors;

static void selective_load(u32 record,Selective *s) {
    u32 i;
    s->hc=(s16)HALF(record,GRKIT_RECORD_SELECTIVE); s->hs=(s16)HALF(record,GRKIT_RECORD_SELECTIVE+2);
    for(i=0;i<4;++i) s->t[i>>1][i&1]=HALF(record,GRKIT_RECORD_SELECTIVE+4+i*2);
    s->c0=BYTE(record,GRKIT_RECORD_SELECTIVE+12); s->c1=BYTE(record,GRKIT_RECORD_SELECTIVE+13);
}
/* Keep weight of one pair, 0..256. The installer accepted inner < outer and c0 < c1. */
static u32 selective_weight(const Selective *s,const SelectiveFactors *f,int u,int v) {
    int along=(u*s->hc+v*s->hs)>>12, across=(v*s->hc-u*s->hs)>>12;
    u32 a,b,i,limit,full,w;
    if(along<=0) return 0;
    a=(u32)along; i=across>0; b=(u32)(across<0 ? -across : across);
    if(a>=SELECTIVE_ALONG) a=SELECTIVE_ALONG-1;
    /* Compare tan(offset) = b / a with the tangents by cross-multiplying. */
    limit=s->t[i][1]*a; full=s->t[i][0]*a;
    if((b<<8)>=limit || a<=s->c0) return 0;
    w=(b<<8)<=full ? 256 : ((limit-(b<<8))*f->angle[i][a])>>16;
    if(a<s->c1) w=(w*(((a-s->c0)*f->gate)>>16))>>8;
    return w;
}
/* The weight of each 2 x 2 (Cb, Cr) block at its center, Cr rows of Cb columns. */
static void selective_table(u32 record,u8 *table,SelectiveFactors *f) {
    Selective s;
    u32 i,j,a;
    selective_load(record,&s);
    for(i=0;i<2;++i) {
        f->angle[i][0]=0;
        for(a=1;a<SELECTIVE_ALONG;++a) f->angle[i][a]=core_divide(256U<<16,(s.t[i][1]-s.t[i][0])*a);
    }
    f->gate=core_divide(256U<<16,s.c1-s.c0);
    for(j=0;j<SELECTIVE_SIDE;++j) for(i=0;i<SELECTIVE_SIDE;++i) {
        u32 w=selective_weight(&s,f,(int)(2*i+1)-128,(int)(2*j+1)-128);
        table[j*SELECTIVE_SIDE+i]=(u8)(w>255 ? 255 : w);
    }
}
/* Scale the chroma of every pair of a plane by its weight. The black bars of a
   preview picture are neutral, so they stay unchanged. */
static void selective_plane(const Plane *p,const u8 *table) {
    u32 rows=p->half ? p->height>>1 : p->height, y, x;
    for(y=0;y<rows;++y) {
        u32 row=p->c+y*p->pitch;
        for(x=0;x<p->width;x+=2) {
            u32 cb=BYTE(row,x), cr=BYTE(row,x+1), w=table[(cr>>1)*SELECTIVE_SIDE+(cb>>1)];
            w+=w==255; /* 255 keeps all chroma */
            BYTE(row,x)=(u8)(128+((((int)cb-128)*(int)w+128)>>8));
            BYTE(row,x+1)=(u8)(128+((((int)cr-128)*(int)w+128)>>8));
        }
    }
}

/* A color record with GRKIT_FEATURE_SELECTIVE. */
static u32 selective_wanted(const PictureSet *set) {
    return set->record && !set->mono &&
        (WORD(set->record,GRKIT_RECORD_FEATURES)&GRKIT_FEATURE_SELECTIVE);
}
/* The table and its factors use the scratch, before the plane filters of
   the modules do. */
static void selective_encode(PictureSet *set) {
    u8 *table=set->scratch;
    u32 i;
    selective_table(set->record,table,(SelectiveFactors *)(table+SELECTIVE_TABLE));
    for(i=0;i<set->count;++i) selective_plane(&set->planes[i],table);
}
#endif

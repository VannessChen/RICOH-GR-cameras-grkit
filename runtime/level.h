#ifndef GRKIT_LEVEL_H
#define GRKIT_LEVEL_H
/* The live-view highlight warning, part of the framing component
   (runtime/framing.c), which includes this file once.
   The live view warns where the shown luma reaches the threshold that
   ScreenDevice::OnSetLevelAlarm gives the display: 0xfe00, luma 254 of 255.
   The curves of a GRKit color effect end below that white, so the warning
   never showed. With an effect, the threshold is the luma of the effect's
   white, less 1: input 255 of its curves, after its post matrix, with the
   BT.601 luma weights. The Fn adjustments are not in it. runtime/menu.c sets
   the warning again after a change of the effect. */
static u32 level_threshold(volatile u32 *h) {
    static const int luma[3]={77,150,29};
    u32 record=h[GRKIT_HOST_ACTIVE/4], post, c, k, y8;
    int rgb[3], sum, y=0;
    if(!record || !(h[GRKIT_HOST_CAPABILITIES/4]&GRKIT_CORE_TONE_BIT)) return 0xfe00U;
    post=WORD(record,GRKIT_RECORD_FEATURES)&GRKIT_FEATURE_POST;
    for(c=0;c<3;++c) rgb[c]=(int)HALF(record,GRKIT_RECORD_CURVES+c*GRKIT_RECORD_CURVE_SIZE+255*2);
    for(k=0;k<3;++k) {
        sum=rgb[k];
        if(post) {
            sum=0;
            for(c=0;c<3;++c) sum+=(s16)HALF(record,GRKIT_RECORD_POST+(k*3+c)*2)*rgb[c];
            sum=(sum+4096)>>13;
        }
        y+=luma[k]*(sum<0 ? 0 : sum>16383 ? 16383 : sum);
    }
    /* y is the 14-bit luma in Q8. Round the 8-bit luma up. */
    y8=(((u32)y>>8)*255U+16383U)>>14;
    return y8>=255U ? 0xfe00U : y8 ? (y8-1U)<<8 : 0;
}
/* framing_level_threshold (runtime/framing.S): the threshold of a warning
   that the camera turns on. */
u32 framing_level_value(void) { return level_threshold(framing_header()); }
/* framing_histogram (runtime/framing.S): the live-view histogram.
   LiveViewController sums the 256 luma bins of the shown picture in pairs
   into 128 bins, and HistogramView draws the last bin, luma 254 and 255, as
   the red clipping line. A color effect ends below that white, so its
   histogram stopped short and the line never showed. With an effect, the
   bins below the warning luma stretch to fill bins 0 to 126, and the last bin
   counts the luma that the highlight warning marks, so the histogram and the
   warning agree. Each bin takes a bin at or below its own index, so the
   stretch runs in place from the top. */
void framing_histogram_remap(u32 *bins) {
    u32 threshold=level_threshold(framing_header()), warn=threshold>>8, j, clipped=0;
    if(threshold==0xfe00U || warn<2) return;
    for(j=warn>>1;j<128;++j) clipped+=bins[j];
    for(j=126;j>0;--j) bins[j]=bins[j*warn/254U];
    bins[127]=clipped;
}
/* framing_level_mode: the screen and the mode of each warning setting. */
void framing_level_record(u32 screen,u32 mode) {
    volatile u32 *h=framing_header();
    h[GRKIT_HOST_LEVEL_SCREEN/4]=screen;
    h[GRKIT_HOST_LEVEL_MODE/4]=mode;
}
/* The camera can set the warning before INSTALL installs the hooks, with
   the native threshold, because the live view starts first. The first live
   view update after INSTALL marks the warning as changed, as a change of
   its setting does, and sets it again with the threshold of the effect.
   UpdateLevelAlarm gets 1, as from its other caller, so that a DISP state of
   the live view does not skip the change. The screen word holds 1 until that
   setting records the screen. */
#ifndef FRAMING_LEVEL_UPDATE
#define FRAMING_LEVEL_UPDATE(controller) (((void (*)(u32,u32))FW_LIVEVIEW_LEVEL_UPDATE)(controller,1))
#endif
static void level_boot(volatile u32 *h,u32 controller) {
    if(h[GRKIT_HOST_LEVEL_SCREEN/4] || level_threshold(h)==0xfe00U) return;
    h[GRKIT_HOST_LEVEL_SCREEN/4]=1;
    BYTE(controller,LIVEVIEW_LEVEL_PENDING)=1;
    FRAMING_LEVEL_UPDATE(controller);
}
#endif

#ifndef GRKIT_MENU_ABOUT_H
#define GRKIT_MENU_ABOUT_H
/* About GRKit on the native camera information page (INFO_CONTROLLER,
   runtime/menu.c). The build header gives the lines of the page that show
   the version and the address (INFO_LINES), or the text IDs of its labels
   (INFO_LABEL_TEXTS). */
#if !defined(INFO_LINES) && !defined(INFO_LABEL_TEXTS)
#error "The build header defines INFO_LINES or INFO_LABEL_TEXTS"
#endif
static u32 about_shows(volatile u32 *h) {
    return h[GRKIT_HOST_SERVICES/4] && services_word(h,SERVICES_MENU+MENU_ROWS_ABOUT)[0];
}
/* Copy ASCII text, at most `room` - 1 units, after `at` units of `to`. */
static u32 ascii_add(volatile char *to,u32 at,const u16 *from,u32 room) {
    while(at<room-1 && *from) to[at++]=(char)*from++;
    to[at]=0;
    return at;
}
static const u16 about_prefix[]={'G','R','K','i','t',' ','V','e','r','.',0};
/* The update of the page ended (menu.S: menu_info_update): show the version
   and the project address in its lines. */
A32_ENTRY void menu_info_fill(u32 controller) {
    volatile u32 *h=menu_header();
    volatile char *version, *project;
    if(!about_shows(h)) return;
    version=(volatile char *)services_word(h,SERVICES_MENU+MENU_ROWS_ABOUT_LINES);
    project=version+MENU_ABOUT_LINE;
    ascii_add(project,0,text(h,TEXT_ABOUT+4),MENU_ABOUT_LINE);
#ifdef INFO_LINES
    {
        static const u16 lines[2]=INFO_LINES;
        ascii_add(version,ascii_add(version,0,about_prefix,MENU_ABOUT_LINE),text(h,TEXT_ABOUT+2),MENU_ABOUT_LINE);
        MENU_VIEW2(FW_STRING_VIEW_SET_ASCII,MENU_OBJECT_WORD(controller,lines[0]),(u32)version);
        MENU_VIEW2(FW_STRING_VIEW_SET_ASCII,MENU_OBJECT_WORD(controller,lines[1]),(u32)project);
    }
#else
    {
        static const u16 labels[3]=INFO_LABELS, values[3]=INFO_VALUES;
        ascii_add(version,0,text(h,TEXT_ABOUT+2),MENU_ABOUT_LINE);
        MENU_VIEW2(FW_STRING_VIEW_SET_ASCII,MENU_OBJECT_WORD(controller,values[0]),(u32)version);
        MENU_VIEW2(FW_STRING_VIEW_SET_ASCII,MENU_OBJECT_WORD(controller,values[1]),(u32)project);
        MENU_VIEW2(FW_VIEW_SET_VISIBLE,MENU_OBJECT_WORD(controller,labels[2]),0);
        MENU_VIEW2(FW_VIEW_SET_VISIBLE,MENU_OBJECT_WORD(controller,values[2]),0);
    }
#endif
}
/* The title of the page reads About GRKit, and on GR IV its labels Version
   and Project. */
static u32 info_text(volatile u32 *h,u32 id) {
    volatile u32 *at;
    u32 n=3;
#ifdef INFO_LABEL_TEXTS
    static const u16 labels[3]=INFO_LABEL_TEXTS;
#endif
    if(!about_shows(h)) return 0;
    at=services_word(h,SERVICES_MENU+MENU_ROWS_ABOUT_TEXTS);
    if(id==INFO_TITLE_TEXT) n=0;
#ifdef INFO_LABEL_TEXTS
    else if(id==labels[0]) n=1;
    else if(id==labels[1]) n=2;
#endif
    if(n==3) return 0;
    descriptor(at+n*2,text(h,TEXT_ABOUT+(n==0 ? 0 : n==1 ? 1 : 3)));
    return (u32)(at+n*2)|1U;
}
#endif /* GRKIT_MENU_ABOUT_H */

#include "abi.h"
/* GRKit installer. Native selection state is read-only. */
typedef unsigned char u8;
typedef unsigned short u16;
typedef unsigned int u32;
/* kind 1: a reservation after the decoded RTOS image. BOOT does not reload
   it, so it can keep the previous session's code after a restart. */
struct patch { u32 address, size, kind; const u8 *before; u8 *after; };
#include "relocation.h"
#include "module_payload.h"
#include "grf.h"
#include "module_link.h"
#include "settings.h"
/* The layout of the effect list (no menu functions without MENU_TEXT). */
#include "effects.h"
/* The layout of the state of the core services. */
#include "services.h"
#if !defined(GRKIT_NO_MODULES) && !defined(GRKIT_RELOCATE)
#error "The installer relocates each runtime with modules"
#endif

static long syscall6(u32 number, u32 a, u32 b, u32 c, u32 d, u32 e, u32 f)
{
    register u32 r0 __asm__("r0") = a;
    register u32 r1 __asm__("r1") = b;
    register u32 r2 __asm__("r2") = c;
    register u32 r3 __asm__("r3") = d;
    register u32 r4 __asm__("r4") = e;
    register u32 r5 __asm__("r5") = f;
    register u32 r7 __asm__("r7") = number;
    __asm__ volatile("svc 0" : "+r"(r0) : "r"(r1), "r"(r2), "r"(r3),
                     "r"(r4), "r"(r5), "r"(r7) : "memory", "cc");
    return (long)r0;
}
static volatile u8 *mapping;
/* One profile more than MAX_COUNT shows that the effects do not all fit. */
static u8 profiles[MAX_COUNT + 1][GRKIT_GRF_MAX_SIZE];
static u8 temporary[GRKIT_GRF_MAX_SIZE];
static u8 directory_entries[4096];
/* The plan: magic, effect count, header words, icon chunks, plan size, module
   words, module tiles, safe mode, the effect files past the list, the listed
   effects, then the installed effects and the listed effects
   (PLANNED_EFFECT bytes each). The loader reads the first seven words. */
#define PLAN_HEADER 40U
/* A listed effect: the head of its profile, its file name, its status, its
   choice, and its record. */
#define PLANNED_EFFECT (64U + 16U + 12U)
static u8 plan[PLAN_HEADER + MAX_COUNT * GRKIT_GRF_MAX_SIZE + EFFECTS_LISTED * PLANNED_EFFECT];
static u32 count, size, icon_chunks;
/* The package folder holds the core, the package list, the package modules,
   and the inbox of the package effects. GRKIT holds the data of the person:
   the state, the report, the effects library, and the modules that the
   person adds. A copy of a new package, by replace or by merge, therefore
   keeps the data. */
static const char library_path[] = LIBRARY_PATH;
static const char inbox_path[] = INBOX_PATH;
static const char package_path[] = PACKAGE_PATH;
static const char imported_path[] = IMPORTED_PATH;
static const char data_path[] = DATA_PATH;
static const char plan_path[] = PLAN_PATH;
static const char plan_tmp[] = PLAN_TMP_PATH;
static const char lock_path[] = LOCK_PATH;
static const char done_path[] = DONE_PATH;
static const char failed_path[] = FAILED_PATH;
static const char safe_path[] = SAFE_PATH;
static const char profile_path[] = PROFILE_PATH;
static const char state_path[] = STATE_PATH;
static const char state_tmp_path[] = STATE_TMP_PATH;
/* The folder of GRKit 1.2: INSTALL reads its state file when GRKIT has none,
   and copies its effects when GRKIT has no effects library. */
static const char legacy_state_path[] = LEGACY_STATE_PATH;
static const char legacy_state_tmp_path[] = LEGACY_STATE_TMP_PATH;
static const char legacy_effects_path[] = LEGACY_EFFECTS_PATH;
static const char report_path[] = REPORT_PATH;
static volatile u8 *at(u32 physical) { return mapping + physical - MAP_BASE; }
static u32 get32(const u8 *p) { return p[0] | p[1] << 8 | p[2] << 16 | (u32)p[3] << 24; }
static void put32(u8 *p, u32 v) { for (u32 i=0; i<4; ++i) p[i] = v >> (i*8); }
static u32 word(u32 p) { return *(volatile u32 *)at(p); }
static void store(u32 p, u32 v) { *(volatile u32 *)at(p) = v; }
static int equal(volatile const u8 *a, const u8 *b, u32 n)
{ for (u32 i=0; i<n; ++i) if (a[i] != b[i]) return 0; return 1; }
static void copy(volatile u8 *a, const u8 *b, u32 n)
{ for (u32 i=0; i<n; ++i) a[i] = b[i]; }
static void zero(volatile u8 *a, u32 n) { for (u32 i=0; i<n; ++i) a[i]=0; }
static u32 length(const char *s) { u32 n=0; while (s[n]) ++n; return n; }
static int compare(const u8 *a, const u8 *b)
{ while (*a && *a==*b) {++a; ++b;} return (int)*a-(int)*b; }
static int text_equal(const char *a, const char *b) { return compare((const u8 *)a,(const u8 *)b)==0; }
static void print(const char *s) { syscall6(4,1,(u32)s,length(s),0,0,0); }
static void number(u32 value)
{
    char digits[12]; u32 n=0;
    do { digits[n++]='0'+value%10; value/=10; } while(value);
    for (u32 i=0; i<n/2; ++i) { char x=digits[i]; digits[i]=digits[n-1-i]; digits[n-1-i]=x; }
    digits[n]=0; print(digits);
}
#define GRKIT_GRF_EMBEDDED
#include "grf.c"
#include "catalog.h"
#include "directory.h"
#include "state.h"
#include "manage.h"
/* The effect files that GRKit lists (runtime/effects.h): the head of each
   profile, which holds its ID and name, its file name, its status at this
   start, and its choice in the GRKit section of MENU: Off in bit 0, and its position
   + 1, or 0 for the default place, above. SCAN lists them in ID order, then
   orders them by their choices. */
#define LISTED_HEAD 64U
static u8 listed[EFFECTS_LISTED + 1][LISTED_HEAD];
static char names[EFFECTS_LISTED + 1][CATALOG_NAME_SIZE];
static u32 listed_count, listed_status[EFFECTS_LISTED], listed_choice[EFFECTS_LISTED];
/* The record of each listed effect, or EFFECTS_NO_RECORD. */
static u32 listed_record[EFFECTS_LISTED];
static long directory_read(long fd, u8 *buffer, u32 size)
{ return syscall6(217,fd,(u32)buffer,size,0,0,0); }
static long directory_seek(long fd, long offset, u32 whence)
{ return syscall6(19,fd,(u32)offset,whence,0,0,0); }
/* Reads an effect file of GRKIT_GRF_SIZE or GRKIT_GRF_MAX_SIZE bytes into a buffer
   of GRKIT_GRF_MAX_SIZE bytes, zeros after a shorter file. Returns the file size,
   or 0 for any other size or a read error. */
static u32 read_effect(const char *path, u8 *buffer)
{
    zero(buffer, GRKIT_GRF_MAX_SIZE);
    long fd=syscall6(5,(u32)path,0x8800,0,0,0,0);
    if(fd<0) return 0;
    u32 have=0; long got=0;
    while(have<GRKIT_GRF_MAX_SIZE) {
        got=syscall6(3,fd,(u32)(buffer+have),GRKIT_GRF_MAX_SIZE-have,0,0,0);
        if(got<=0) break;
        have+=(u32)got;
    }
    u8 extra; long more=0;
    if(have==GRKIT_GRF_MAX_SIZE) more=syscall6(3,fd,(u32)&extra,1,0,0,0);
    syscall6(6,fd,0,0,0,0,0);
    if(got<0 || more!=0) return 0;
    return (have==GRKIT_GRF_SIZE || have==GRKIT_GRF_MAX_SIZE) ? have : 0;
}
/* A path in the library: the folder, '/', an 8.3 name, and NUL. */
#define PATH_MAX_OF(a, b) (sizeof(a) > sizeof(b) ? sizeof(a) : sizeof(b))
#define EFFECT_PATH_SIZE (PATH_MAX_OF(legacy_effects_path, PATH_MAX_OF(library_path, inbox_path)) + CATALOG_NAME_SIZE)
/* Write `folder`/`name` into `path`. Return 0 if it does not fit in `size`
   bytes. */
static int join(char *path, u32 size, const char *folder, const char *name)
{
    u32 prefix=length(folder), n=length(name);
    if(prefix+n+2>size) return 0;
    copy((u8 *)path,(const u8 *)folder,prefix); path[prefix++]='/';
    copy((u8 *)path+prefix,(const u8 *)name,n+1);
    return 1;
}
/* Read `path` into `buffer`. Return its size, or -1 for a missing file, a
   read error, or a file larger than `max` bytes. */
static long read_text(const char *path, u8 *buffer, u32 max)
{
    long fd=syscall6(5,(u32)path,0x8800,0,0,0,0);
    if(fd<0) return -1;
    u32 have=0; long got=0; u8 extra;
    while(have<max && (got=syscall6(3,fd,(u32)(buffer+have),max-have,0,0,0))>0) have+=(u32)got;
    if(have==max) got=syscall6(3,fd,(u32)&extra,1,0,0,0);
    syscall6(6,fd,0,0,0,0,0);
    return got<0 || (have==max && got>0) ? -1 : (long)have;
}
/* Write `size` bytes to `path`, which the installer creates or truncates.
   Return 1 on success. */
static int write_file(const char *path, const u8 *data, u32 size)
{
    long fd=syscall6(5,(u32)path,0x241,0644,0,0,0);
    if(fd<0) return 0;
    u32 done=0;
    while(done<size) {
        long n=syscall6(4,fd,(u32)(data+done),size-done,0,0,0);
        if(n<=0) break;
        done+=(u32)n;
    }
    return syscall6(6,fd,0,0,0,0,0)>=0 && done==size;
}
/* ---- Package list and effects library -------------------------------------
   PACKAGE.TXT names the files of the package, separated by ASCII whitespace:
   its effects in the inbox and its modules. The effects of the package
   install before the other effects of the library. A package module loads
   only if the list names it, so a merge copy of a new package that leaves
   an old module behind does not load it. Without the list, each package
   module loads. */
#define NAMES_MAX 64U
#define PACKAGE_FILE_MAX 1024U
/* ASCII whitespace, which separates the names of the list. */
static int text_space(u8 c) { return c==' ' || c=='\t' || c=='\r' || c=='\n'; }
static char package_effects[NAMES_MAX][CATALOG_NAME_SIZE], package_modules[NAMES_MAX][CATALOG_NAME_SIZE];
static u32 package_effect_count, package_module_count;
static int package_listed;
static int same_name(const char *a, const char *b)
{
    for(;;) {
        char x=*a++, y=*b++;
        if(x>='a' && x<='z') x-=32;
        if(y>='a' && y<='z') y-=32;
        if(x!=y) return 0;
        if(!x) return 1;
    }
}
/* A module file name: <ID>.GRM, 5 to 12 characters. */
static int module_file_name(const char *name, u32 n)
{
    if (n < 5 || n > 12 || name[n - 4] != '.') return 0;
    for (u32 i = 0; i < 3; ++i) if ((name[n - 3 + i] & ~0x20) != "GRM"[i]) return 0;
    return 1;
}
static void read_package_list(void)
{
    static u8 text[PACKAGE_FILE_MAX];
    package_effect_count = package_module_count = 0;
    long have = read_text(package_path, text, sizeof(text));
    package_listed = have >= 0;
    for (long i = 0; i < have;) {
        while (i < have && text_space(text[i])) ++i;
        long start = i;
        while (i < have && !text_space(text[i])) ++i;
        u32 n = (u32)(i - start);
        const char *name = (const char *)text + start;
        if (!n || n >= CATALOG_NAME_SIZE) continue;
        if (catalog_name(name, n) && package_effect_count < NAMES_MAX) {
            copy((u8 *)package_effects[package_effect_count], text + start, n);
            package_effects[package_effect_count++][n] = 0;
        } else if (module_file_name(name, n) && package_module_count < NAMES_MAX) {
            copy((u8 *)package_modules[package_module_count], text + start, n);
            package_modules[package_module_count++][n] = 0;
        }
    }
}
static int package_effect(const char *name)
{
    for (u32 i = 0; i < package_effect_count; ++i) if (same_name(package_effects[i], name)) return 1;
    return 0;
}
static int package_module(const char *name)
{
    if (!package_listed) return 1;
    for (u32 i = 0; i < package_module_count; ++i) if (same_name(package_modules[i], name)) return 1;
    return 0;
}
/* The package effects that `collect` could not copy into the library at
   this power-on. They install from the inbox. */
static char pending[NAMES_MAX][CATALOG_NAME_SIZE];
static u32 pending_count;
static u8 existing[GRKIT_GRF_MAX_SIZE];
/* The names of the effect files to copy from one folder. */
struct effect_names {
    char name[NAMES_MAX][CATALOG_NAME_SIZE];
    u32 count;
};
static struct effect_names copies;
static void effect_name(const char *name, u32 n, void *context)
{
    struct effect_names *list = context;
    if (catalog_name(name, n) && list->count < NAMES_MAX) catalog_copy_name(list->name[list->count++], name);
}
/* List the effect files of `folder` in `copies`: 0, or nonzero when the
   folder does not open or does not list. */
static int list_effects(const char *folder)
{
    copies.count = 0;
    long fd = syscall6(5, (u32)folder, 0xc000, 0, 0, 0, 0);
    if (fd < 0) return 1;
    static const struct directory_calls calls = {directory_read, directory_seek};
    int status = directory_list(fd, directory_entries, sizeof(directory_entries), &calls, effect_name, &copies);
    syscall6(6, fd, 0, 0, 0, 0, 0);
    return status;
}
/* IMPORTED.TXT records the package that the library has: for each listed
   inbox file, its name and the FNV-1a hash of its bytes. */
#define IMPORTED_MAX (NAMES_MAX * 24U)
static u8 record[IMPORTED_MAX], recorded[IMPORTED_MAX];
static u32 record_line(u32 n, const char *name, u32 hash)
{
    u32 k = length(name);
    if (n + k + 11 > IMPORTED_MAX) return n;
    copy(record + n, (const u8 *)name, k); n += k; record[n++] = ' ';
    for (int shift = 28; shift >= 0; shift -= 4) record[n++] = (u8)"0123456789ABCDEF"[(hash >> shift) & 15];
    record[n++] = '\r'; record[n++] = '\n';
    return n;
}
static u32 fnv(const u8 *data, u32 size)
{
    u32 hash = 0x811c9dc5;
    for (u32 i = 0; i < size; ++i) { hash ^= data[i]; hash *= 0x01000193; }
    return hash;
}
/* Copy the effects of a new package into the library, where each replaces
   the file with the same name. The package is new when the record of its
   listed files differs from IMPORTED.TXT. The first power-on without the
   library copies every valid inbox file. The installer deletes no file and
   renames none: on the camera's Linux, a rename does not replace a file (on
   GR IV exFAT it fails with an I/O error), and on GR IV exFAT a delete made
   later lookups fail, also the request that runs INSTALL. So the inbox keeps
   its files, and a copy of the same package does not bring back an effect
   that a person deleted from the library. If the power fails during a copy,
   IMPORTED.TXT stays old, and a later power-on copies again. */
static void copy_effects(const char *folder, int package)
{
    for (u32 i = 0; i < copies.count; ++i) {
        char from[EFFECT_PATH_SIZE], to[EFFECT_PATH_SIZE];
        if (!join(from, sizeof(from), folder, copies.name[i]) || !join(to, sizeof(to), library_path, copies.name[i])) continue;
        u32 size = read_effect(from, temporary);
        if (!size || !valid_profile(temporary, size)) continue;
        if (read_effect(to, existing) == size && equal(existing, temporary, size)) continue;
        if (!write_file(to, temporary, size) && package && package_effect(copies.name[i]) && pending_count < NAMES_MAX)
            catalog_copy_name(pending[pending_count++], copies.name[i]);
    }
}
static void collect(void)
{
    pending_count = 0;
    long dir = syscall6(5, (u32)library_path, 0xc000, 0, 0, 0, 0);
    int first = dir == -2;
    if (dir >= 0) syscall6(6, dir, 0, 0, 0, 0, 0);
    else if (!first) return;
    u32 n = 0;
    for (u32 i = 0; i < package_effect_count; ++i) {
        char from[EFFECT_PATH_SIZE];
        if (!join(from, sizeof(from), inbox_path, package_effects[i])) continue;
        u32 size = read_effect(from, temporary);
        if (size && valid_profile(temporary, size)) n = record_line(n, package_effects[i], fnv(temporary, size));
    }
    long had = read_text(imported_path, recorded, sizeof(recorded));
    if (!first && had == (long)n && equal(recorded, record, n)) return;
    if (first) {
        long fd = syscall6(5, (u32)inbox_path, 0xc000, 0, 0, 0, 0);
        if (fd < 0) return;
        syscall6(6, fd, 0, 0, 0, 0, 0);
    }
    long made = syscall6(39, (u32)data_path, 0755, 0, 0, 0, 0);
    if (made < 0 && made != -17) return;
    made = syscall6(39, (u32)library_path, 0755, 0, 0, 0, 0);
    if (made < 0 && made != -17) return;
    if (first) {
        /* The effects of GRKit 1.2 come first, so a package effect with the
           same name replaces an old one. */
        if (!list_effects(legacy_effects_path)) copy_effects(legacy_effects_path, 0);
        if (list_effects(inbox_path)) return;
    } else {
        copies.count = 0;
        for (u32 i = 0; i < package_effect_count; ++i) catalog_copy_name(copies.name[copies.count++], package_effects[i]);
    }
    copy_effects(inbox_path, 1);
    if (!pending_count) write_file(imported_path, record, n);
}
/* The number of effect files past EFFECTS_LISTED, for REPORT.TXT. */
static u32 overflow_count;
/* Offer the effect in `path`, under its file name `name`. */
static void offer_path(const char *path, const char *name)
{
    u32 size=read_effect(path,temporary);
    if(!size || !valid_profile(temporary,size)) return;
    listed_count=catalog_offer(&listed[0][0],LISTED_HEAD,names,listed_count,EFFECTS_LISTED + 1,temporary,name);
    /* The catalog keeps the smallest IDs: the one past EFFECTS_LISTED is out. */
    if(listed_count>EFFECTS_LISTED) { ++overflow_count; listed_count=EFFECTS_LISTED; }
}
/* A symbolic link or an unusable file does not stop the other effects. */
static void offer_effect(const char *name, u32 n, void *context)
{
    if(!catalog_name(name,n)) return;
    char path[EFFECT_PATH_SIZE];
    if(join(path,sizeof(path),(const char *)context,name)) offer_path(path,name);
}
/* The state file ID of the choice of listed effect i: CORE.EFFECT.<its ID>. */
static u32 effect_choice_id(u32 i)
{
    char name[16 + CATALOG_NAME_SIZE + 32];
    u32 n = 0;
    for (const char *p = "CORE.EFFECT."; *p; ++p) name[n++] = *p;
    for (const u8 *p = listed[i] + CATALOG_ID; *p && n < sizeof(name) - 1; ++p) name[n++] = (char)*p;
    name[n] = 0;
    return grm_state_id(name);
}
/* Read the choices of the listed effects and order them: an effect with a
   position first, then the effects of the package, then the others, each
   group in ID order. Then install the effects that are not Off, in that
   order, then the effects that are Off while there is room, at most
   MAX_COUNT: an installed effect turns on or off at once
   (runtime/effects.h). A pending effect installs from the inbox. */
static struct state saved_state;
static void choose_effects(void)
{
    u32 keys[EFFECTS_LISTED], value;
    for (u32 i = 0; i < listed_count; ++i) {
        listed_choice[i] = state_value(&saved_state, effect_choice_id(i), &value) && value < EFFECT_CHOICE_VALUES ? value : 0;
        keys[i] = listed_choice[i] >> 1 ? listed_choice[i] >> 1
                  : (package_effect(names[i]) ? EFFECTS_DEFAULT_KEY : 2 * EFFECTS_DEFAULT_KEY) + i;
    }
    for (u32 i = 1; i < listed_count; ++i)
        for (u32 j = i; j && keys[j - 1] > keys[j]; --j) {
            u8 head[LISTED_HEAD];
            char file[CATALOG_NAME_SIZE];
            u32 t;
            copy(head, listed[j], LISTED_HEAD); copy(listed[j], listed[j - 1], LISTED_HEAD); copy(listed[j - 1], head, LISTED_HEAD);
            catalog_copy_name(file, names[j]); catalog_copy_name(names[j], names[j - 1]); catalog_copy_name(names[j - 1], file);
            t = keys[j]; keys[j] = keys[j - 1]; keys[j - 1] = t;
            t = listed_choice[j]; listed_choice[j] = listed_choice[j - 1]; listed_choice[j - 1] = t;
        }
    count = 0;
    for (u32 pass = 0; pass < 2; ++pass)
        for (u32 i = 0; i < listed_count; ++i) {
            char path[EFFECT_PATH_SIZE];
            u32 size, off = listed_choice[i] & 1;
            if (!pass) listed_record[i] = EFFECTS_NO_RECORD;
            if (off != pass) continue;
            listed_status[i] = off ? EFFECT_OFF : EFFECT_OVER;
            if (count == MAX_COUNT) continue;
            /* The file can change between the two reads: a profile with
               another ID does not install. */
            if (!join(path, sizeof(path), library_path, names[i]) || !(size = read_effect(path, profiles[count])))
                if (!join(path, sizeof(path), inbox_path, names[i]) || !(size = read_effect(path, profiles[count]))) size = 0;
            if (!size || !valid_profile(profiles[count], size) || !equal(profiles[count] + CATALOG_ID, listed[i] + CATALOG_ID, 32)) {
                listed_status[i] = EFFECT_UNREADABLE;
                continue;
            }
            if (!off) listed_status[i] = EFFECT_LOADED;
            listed_record[i] = count++;
        }
}
/* The effects of GRKIT/EFFECTS: GRKit lists at most EFFECTS_LISTED, those
   with the smallest IDs, and installs at most MAX_COUNT of them. */
static int tone_used(void);
static int discover(void)
{
    count=0; listed_count=0; overflow_count=0;
#if defined(GRKIT_NO_MODULES) || defined(GRKIT_NO_IMAGE_CONTROL)
    /* Without Image Control, the target has no use for effect files. */
    return 0;
#endif
    /* Without a module that uses CORE.TONE, no effect installs
       (plan_modules). */
    if(!tone_used()) return 0;
    long fd=syscall6(5,(u32)library_path,0xc000,0,0,0,0);
    if(fd<0 && fd!=-2) return 30;
    if(fd>=0) {
        static const struct directory_calls calls={directory_read,directory_seek};
        int status=directory_list(fd,directory_entries,sizeof(directory_entries),&calls,offer_effect,(void *)library_path);
        syscall6(6,fd,0,0,0,0,0);
        if(status) return status;
    }
    for(u32 i=0;i<pending_count;++i) {
        char path[EFFECT_PATH_SIZE];
        if(join(path,sizeof(path),inbox_path,pending[i])) offer_path(path,pending[i]);
    }
    choose_effects();
    return 0;
}
/* Photos record this code as their Image Control tag. It stays stable across
   catalogs and outside the native values 256..267. */
#ifdef GRKIT_NATIVE_TONE_PAGE
/* The target also records native values in this 256-value page. */
#define NATIVE_CODE(code) ((code) >> 8 == GRKIT_NATIVE_TONE_PAGE)
#else
#define NATIVE_CODE(code) 0
#endif
#if !defined(GRKIT_NO_MODULES) && !defined(GRKIT_NO_IMAGE_CONTROL)
/* The MultiAxial parameter set of an effect with hue sectors (GRF feature 8,
   grkit_core::hue), at RECORD_MULTIAXIAL. The firmware reads a set as six
   pointers: bounds (16 u16, four descending ones per Cb/Cr quadrant), bank
   selectors (20 u8, five segments per quadrant), a matrix header whose word
   +0x10 points to 60 3x3 i32 Q10 matrices (12 banks x 5 levels), region values,
   21 u16 control fields, and the RGB to YCbCr matrix. The bounds and selectors
   give 11 equal sectors of the hue coordinate, sector k in bank k + 1; bank 0
   stays the identity. Without hue levels, every level of a bank gets the same
   matrix and control fields 1..8 = 0xfff keep every pixel on level 0. With them
   (feature 16), levels 1 to 3 come from the GRF, level 4 repeats level 3, and
   fields 1..8 are hue::LEVEL_CONTROL. Fields 10..20, the weight curve, come from
   the GRF. The region values and the YCbCr matrix are those of Standard's set
   (type 11). */
static const u16 multiaxial_bounds[16] = GRKIT_HUE_BOUNDS;
static const u8 multiaxial_banks[20] = GRKIT_HUE_SELECTORS;
static const u16 multiaxial_levels[8] = GRKIT_HUE_LEVEL_CONTROL;
static const u16 multiaxial_levels_halved[8] = GRKIT_HUE_LEVEL_CONTROL_HALVED;
static void multiaxial(u32 record, const u8 *profile)
{
    const u8 *sectors = profile + GRKIT_HUE_OFFSET;
    int levels = (get32(profile + 84) & GRKIT_FEATURE_HUE_LEVELS) != 0;
    u32 slot = record + GRKIT_RECORD_MULTIAXIAL, table = slot + 0x98;
    u32 stock = word(TABLE_ADDRESS(1) + 11 * 4);
    store(slot, slot + 8);
    store(slot + 8, slot + 0x20);
    store(slot + 12, slot + 0x40);
    store(slot + 16, slot + 0x58);
    store(slot + 20, word(stock + 12));
    store(slot + 24, slot + 0x6c);
    store(slot + 28, word(stock + 20));
    copy(at(slot + 0x20), (const u8 *)multiaxial_bounds, sizeof multiaxial_bounds);
    copy(at(slot + 0x40), multiaxial_banks, sizeof multiaxial_banks);
    for (u32 k = 0; k < 5; ++k) store(slot + 0x58 + k * 4, table);
    volatile u16 *control = (volatile u16 *)at(slot + 0x6c);
    control[0] = 0;
    for (u32 k = 1; k <= 8; ++k) control[k] = levels ? multiaxial_levels[k - 1] : 0xfff;
    control[9] = 0;
    const u8 *fields = sectors + GRKIT_HUE_MATRICES;
    for (u32 k = 0; k < 11; ++k) control[10 + k] = (u16)(fields[k * 2] | fields[k * 2 + 1] << 8);
    for (u32 bank = 0; bank < 12; ++bank)
        for (u32 level = 0; level < 5; ++level) {
            /* Levels 1 to 3 follow GRKIT_HUE_LEVEL_OFFSET; level 4 repeats level 3. */
            u32 from = levels ? (level < 4 ? level : 3) : 0;
            const u8 *c = from ? profile + GRKIT_HUE_LEVEL_OFFSET + (from - 1) * GRKIT_HUE_MATRICES : sectors;
            int m[9] = {1024, 0, 0, 0, 1024, 0, 0, 0, 1024};
            for (u32 r = 0; bank && r < 3; ++r) {
                /* GRF validation keeps each diagonal within the 12-bit register. */
                int a = (signed char)c[(bank - 1) * 6 + r * 2] * 8;
                int b = (signed char)c[(bank - 1) * 6 + r * 2 + 1] * 8;
                m[r * 3 + (r + 1) % 3] = a;
                m[r * 3 + (r + 2) % 3] = b;
                m[r * 4] = 1024 - a - b;
            }
            for (u32 k = 0; k < 9; ++k) store(table + (bank * 5 + level) * 36 + k * 4, (u32)m[k]);
        }
    /* The set for a capture with highlight correction, which the hooks pick by the
       native flag. The camera then halves the stage's input, so these control fields
       select the levels and the weight of twice the luminance: the weight breakpoints
       halve and its slopes double (9-bit, clamped). The address at +4, the set at
       +0x908, its control fields at +0x920; the other pointers are those of the set. */
    u32 halved = slot + 0x908;
    store(slot + 4, halved);
    for (u32 k = 0; k < 6; ++k) store(halved + k * 4, word(slot + 8 + k * 4));
    store(halved + 16, slot + 0x920);
    volatile u16 *half = (volatile u16 *)at(slot + 0x920);
    half[0] = 0;
    for (u32 k = 1; k <= 8; ++k) half[k] = levels ? multiaxial_levels_halved[k - 1] : 0xfff;
    half[9] = 0;
    for (u32 k = 0; k < 4; ++k) {
        int slope = (int)(control[14 + k] & 0x1ff) - ((control[14 + k] & 0x100) ? 0x200 : 0);
        slope *= 2;
        if (slope > 255) slope = 255;
        if (slope < -256) slope = -256;
        half[10 + k] = control[10 + k];
        half[14 + k] = (u16)(slope & 0x1ff);
    }
    for (u32 k = 18; k <= 20; ++k) half[k] = (u16)(control[k] / 2);
}
#endif
static u32 effect_code(const u8 *id)
{
    u32 hash = 0x811c9dc5;
    while (*id) { hash ^= *id++; hash *= 0x01000193; }
    return 0x8000 | (hash & 0x7fff);
}
/* The artwork tiles of the usable modules (plan_modules). Their icon slots
   follow the effect icons. */
static u32 module_tiles;
static void layout(void)
{
    size = GRKIT_HOST_HEADER_SIZE + count * GRKIT_RECORD_SIZE;
    icon_chunks = (count + HOST_ICON_COUNT + module_tiles + ICONS_PER_CHUNK - 1) / ICONS_PER_CHUNK;
}
static u32 chunk_count(u32 chunk)
{
    u32 left = count + HOST_ICON_COUNT + module_tiles - chunk * ICONS_PER_CHUNK;
    return left < ICONS_PER_CHUNK ? left : ICONS_PER_CHUNK;
}
/* ---- Modules (runtime/module_link.h) ------------------------------------
   Each GRKIT/MODULES/<ID>.GRM is one module. SCAN reserves script
   memory for the modules, and INSTALL links them there and registers their
   layers. Both steps skip the same unusable modules.

   The code of each module goes after the tail, up to RTOS_EXECUTABLE_END.
   The RTOS maps that area read-only and executable, and no firmware code uses
   it, so the code stays valid even if the resident script ends. The data of
   each module goes into the module script array, which the RTOS can write. */
#define MODULES_MAX 8U
#define MODULE_FILE_MAX (GRM_HEADER + 4U * GRM_MAX_SEGMENT)
/* The largest script array (TTL intdim), which holds the module data. */
#define MODULE_ARRAY_WORDS 65536U
/* The most code bytes of all modules after the tail. The area is larger on
   each target. */
#define MODULE_CODE_LIMIT 0x100000U
#ifndef GRKIT_NO_MODULES
static const char modules_path[] = MODULES_PATH;
static const char package_modules_path[] = PACKAGE_MODULES_PATH;
_Static_assert(sizeof(package_modules_path) >= sizeof(modules_path), "The path buffer fits both folders");
#endif
static char module_names[MODULES_MAX][CATALOG_NAME_SIZE];
/* The listed modules, the usable ones (bit i), and what they need: script
   words for their data, and code bytes after the tail. */
static u32 module_count, module_usable, module_words, module_code_bytes;
static u8 module_file[MODULE_FILE_MAX];
static struct grm_module module;
/* The settings of the usable modules in module order (runtime/settings.h):
   the full name, values, and flags of each, the first setting and the count
   of each module, and the used settings of each module. */
static char setting_names[GRKIT_MODULE_SETTINGS_MAX][GRM_NAME];
static u32 setting_values[GRKIT_MODULE_SETTINGS_MAX], setting_flags[GRKIT_MODULE_SETTINGS_MAX], setting_total;
static u32 module_first[MODULES_MAX], module_settings[MODULES_MAX], module_use_count[MODULES_MAX];
/* The global index of tile 0 of each module (GRKIT_ICON_MODULE_TILE). */
static u32 module_tile_first[MODULES_MAX];
static char module_uses[MODULES_MAX][GRM_USES][GRM_NAME + 1];
/* The result of each listed module for REPORT.TXT and the Modules page: 0
   when it loads, also when it is Off, MANAGE_SAFE in safe mode, else a
   status (runtime/module_link.h), with the name of the missing import or
   setting of GRM_MISSING_IMPORT and GRM_MISSING_SETTING. */
_Static_assert(MODULES_MAX == MANAGE_MODULES, "The Modules page lists each module");
static u32 module_status[MODULES_MAX];
static char module_detail[MODULES_MAX][GRM_NAME + 1];
/* The choices of the GRKit section of MENU (runtime/manage.h): Off, the
   position + 1 or 0, and the values that are off in ADJ of each module.
   INSTALL also notes the title texts and tile 0 of each module. */
static u32 module_off[MODULES_MAX], module_position[MODULES_MAX], module_rows[MODULES_MAX];
static u32 module_title[MODULES_MAX], module_tile[MODULES_MAX];
/* The code range and the used capabilities of each loaded module, for the
   soft Off of runtime/manage.h. */
static u32 module_code_from[MODULES_MAX], module_code_to[MODULES_MAX], module_capabilities[MODULES_MAX];
/* The capabilities of the modules that run: not those of a module that is
   Off, which loads stopped (runtime/manage.h). */
static u32 capabilities_running;
/* The addresses of the module list, of the effect list, and of the state of
   the core services, or 0 (install_modules). */
static u32 module_list, effect_list, services_block;
/* The words of the module list and of the effect list. */
#define MANAGE_WORDS(n) ((MANAGE_HEADER + (n) * MANAGE_STRIDE) / 4U)
#define EFFECT_LIST_WORDS(n) ((EFFECTS_HEADER + (n) * EFFECTS_STRIDE) / 4U)
/* The words of the contexts and of the module data (plan_modules). */
static u32 module_contexts, module_data_words;
/* Saved values that no setting of this start takes, such as the settings of
   a module that is Off. The settings table keeps them, so the next save of
   the loader keeps them too. */
static u32 kept_ids[GRKIT_MODULE_SETTINGS_MAX], kept_values[GRKIT_MODULE_SETTINGS_MAX], kept_count;
/* Safe mode: 1 while the file GRKIT/SAFE exists. No module loads then
   (scan). */
static u32 safe_mode;
/* The development profile: 1 while the file GRKIT/PROFILE exists
   (runtime/profile.h). */
static u32 profile_mode;
/* The modes in the plan: bit 0 the safe mode, bit 1 the profile. */
#define PLAN_SAFE 1U
#define PLAN_PROFILE 2U
/* The capabilities that the loaded modules use (GRKIT_HOST_CAPABILITIES). */
static u32 capabilities_used;
static void read_state(void);
/* The words of the settings table and of the contexts (GrkitContext). */
#define SETTINGS_WORDS(n) (2U + 3U * (n))
#define CONTEXT_WORDS(uses) (3U + (uses))
static void name_copy(char *to, const char *from, u32 n)
{
    for (u32 i = 0; i < n; ++i) to[i] = from[i];
    to[n] = 0;
}
/* The order of each listed module, from the header of its file: its default
   place in ADJ and in MENU. A file that is not a module sorts last. */
static u32 module_order[MODULES_MAX];
#define ORDER_UNKNOWN (GRM_MAX_ORDER + 1U)
/* Whether a module of `order_a` in file `a` goes after one of `order_b` in
   file `b`: by order, then by name. */
static int module_after(u32 order_a, const char *a, u32 order_b, const char *b)
{
    return order_a != order_b ? order_a > order_b : compare((const u8 *)a, (const u8 *)b) > 0;
}
/* The order in the header of module file `name` in the package folder or in
   GRKIT/MODULES (`source`). */
static u32 module_file_order(const char *name, u32 source)
{
#ifdef GRKIT_NO_MODULES
    (void)name; (void)source;
    return ORDER_UNKNOWN;
#else
    char path[sizeof(package_modules_path) + CATALOG_NAME_SIZE];
    u8 header[GRM_HEADER];
    u32 have = 0, order;
    long got;
    if (!join(path, sizeof(path), source ? modules_path : package_modules_path, name)) return ORDER_UNKNOWN;
    long fd = syscall6(5, (u32)path, 0x8800, 0, 0, 0, 0);
    if (fd < 0) return ORDER_UNKNOWN;
    while (have < GRM_HEADER && (got = syscall6(3, fd, (u32)(header + have), GRM_HEADER - have, 0, 0, 0)) > 0)
        have += (u32)got;
    syscall6(6, fd, 0, 0, 0, 0, 0);
    if (have < GRM_HEADER || grm_word(header) != GRM_MAGIC) return ORDER_UNKNOWN;
    order = grm_word(header + GRM_ORDER);
    return order <= GRM_MAX_ORDER ? order : ORDER_UNKNOWN;
#endif
}
/* A module file is <ID>.GRM. Keep the names in their default order, so SCAN
   and INSTALL take the modules in the same order. */
/* The module files past MODULES_MAX, for REPORT.TXT. A full list keeps the
   first files in the default order, so the result does not depend on the
   directory order. */
static u32 module_overflow;
/* The folder of each listed module: 0 for the package folder, 1 for
   GRKIT/MODULES. A module file of GRKIT/MODULES replaces the package module
   with the same name. */
static u32 module_source[MODULES_MAX];
static void offer_module(const char *name, u32 n, void *context)
{
    u32 source = context != 0;
    if (!module_file_name(name, n)) return;
    char file[CATALOG_NAME_SIZE];
    name_copy(file, name, n);
    if (!source && !package_module(file)) return;
    /* A file of GRKIT/MODULES replaces the package module of its name, with
       its own order. */
    for (u32 k = 0; k < module_count; ++k)
        if (same_name(module_names[k], file)) {
            if (!source) return;
            for (u32 j = k; j + 1 < module_count; ++j) {
                name_copy(module_names[j], module_names[j + 1], length(module_names[j + 1]));
                module_source[j] = module_source[j + 1];
                module_order[j] = module_order[j + 1];
            }
            --module_count;
            break;
        }
    u32 order = module_file_order(file, source);
    if (module_count == MODULES_MAX) {
        ++module_overflow;
        if (!module_after(module_order[MODULES_MAX - 1], module_names[MODULES_MAX - 1], order, file)) return;
        --module_count;
    }
    u32 at = module_count++;
    while (at && module_after(module_order[at - 1], module_names[at - 1], order, file)) {
        name_copy(module_names[at], module_names[at - 1], length(module_names[at - 1]));
        module_source[at] = module_source[at - 1];
        module_order[at] = module_order[at - 1];
        --at;
    }
    name_copy(module_names[at], name, n);
    module_source[at] = source;
    module_order[at] = order;
}
/* Read and check module i into `module`. Return 0 or a status. */
static int read_module(u32 i)
{
#ifdef GRKIT_NO_MODULES
    (void)i;
    return GRM_BAD_FILE;
#else
    char path[sizeof(package_modules_path) + CATALOG_NAME_SIZE];
    if (!join(path, sizeof(path), module_source[i] ? modules_path : package_modules_path, module_names[i])) return GRM_BAD_FILE;
    long fd = syscall6(5, (u32)path, 0x8800, 0, 0, 0, 0);
    if (fd < 0) return GRM_BAD_FILE;
    u32 have = 0; long got = 0; u8 extra;
    while (have < sizeof(module_file)) {
        got = syscall6(3, fd, (u32)(module_file + have), sizeof(module_file) - have, 0, 0, 0);
        if (got <= 0) break;
        have += (u32)got;
    }
    if (got >= 0 && have == sizeof(module_file)) got = syscall6(3, fd, (u32)&extra, 1, 0, 0, 0) ? -1 : 0;
    syscall6(6, fd, 0, 0, 0, 0, 0);
    if (got < 0) return GRM_BAD_FILE;
    int status = grm_parse(module_file, have, &module);
    if (status) return status;
    /* The file name before the dot is the module ID. */
    const char *name = module_names[i];
    u32 k = 0;
    for (; name[k] != '.'; ++k) if ((name[k] >= 'a' && name[k] <= 'z' ? name[k] - 32 : name[k]) != module.id[k]) return GRM_BAD_FILE;
    return module.id[k] ? GRM_BAD_FILE : 0;
#endif
}
#ifndef GRKIT_NO_MODULES
/* The state file ID of CORE.<key>.<ID> for module i. */
static u32 manage_id(u32 i, const char *key)
{
    char name[GRM_NAME + 1];
    u32 n = 0;
    for (const char *p = "CORE."; *p; ++p) name[n++] = *p;
    for (const char *p = key; *p; ++p) name[n++] = *p;
    name[n++] = '.';
    for (const char *p = module_names[i]; *p && *p != '.' && n < GRM_NAME; ++p)
        name[n++] = *p >= 'a' && *p <= 'z' ? (char)(*p - 32) : *p;
    name[n] = 0;
    return grm_state_id(name);
}
/* Read the choices of the GRKit section. The list keeps the default order,
   which the GRKit section of MENU shows. The core orders the GRKit root of
   ADJ by the positions (runtime/manage.h: manage_key). */
static void order_modules(void)
{
    u32 value;
    for (u32 i = 0; i < module_count; ++i) {
        module_off[i] = state_value(&saved_state, manage_id(i, "OFF"), &value) && value == 1;
        module_position[i] = state_value(&saved_state, manage_id(i, "POS"), &value) && value <= MANAGE_MODULES ? value : 0;
        module_rows[i] = state_value(&saved_state, manage_id(i, "ROWS"), &value) && value < 1U << MANAGE_OPTION_VALUES ? value : 0;
    }
}
/* Whether a setting of this start takes the saved value of `id`: a
   capability that a usable module uses, a saved setting of a usable module,
   or a choice of the GRKit section. The Fn adjustments are not kept. */
static int state_taken(u32 id)
{
    static const u32 core_ids[] = {CORE_STATE_IDS};
    if (id >= PARAMETER_KEY_BASE) return 1;
    for (u32 k = 0; k < GRKIT_CORE_SETTING_COUNT; ++k) {
        if (core_ids[k] != id) continue;
        for (u32 i = 0; i < module_count; ++i) {
            if (!(module_usable >> i & 1)) continue;
            for (u32 u = 0; u < module_use_count[i]; ++u) {
                const char *name = module_uses[i][u];
                if (text_equal(name + (name[0] == '?'), core_setting_names[k])) return 1;
            }
        }
    }
    for (u32 k = 0; k < setting_total; ++k)
        if ((setting_flags[k] & GRM_SETTING_SAVED) && grm_state_id(setting_names[k]) == id) return 1;
    for (u32 i = 0; i < module_count; ++i)
        if (manage_id(i, "OFF") == id || manage_id(i, "POS") == id || manage_id(i, "ROWS") == id) return 1;
    for (u32 i = 0; i < listed_count; ++i)
        if (effect_choice_id(i) == id) return 1;
    return 0;
}
/* The saved values that the table keeps, at most `room`. A later pair of a
   key wins. */
static void plan_kept(u32 room)
{
    kept_count = 0;
    for (u32 p = 0; p < saved_state.pairs; ++p) {
        u32 id = saved_state.ids[p], value = saved_state.values[p], k = 0;
        if (!id || state_taken(id)) continue;
        while (k < kept_count && kept_ids[k] != id) ++k;
        if (k == kept_count) {
            if (kept_count == room) continue;
            kept_ids[kept_count++] = id;
        }
        kept_values[k] = value;
    }
    /* A key with the value 0 has no entry in the file. */
    for (u32 k = 0; k < kept_count;)
        if (kept_values[k]) ++k;
        else { kept_ids[k] = kept_ids[--kept_count]; kept_values[k] = kept_values[kept_count]; }
}
#endif
/* List the modules and find the usable ones and the memory that they need. */
static void plan_modules(void)
{
    module_count = module_usable = module_words = module_code_bytes = setting_total = module_tiles = kept_count = 0;
    module_overflow = 0;
    module_contexts = module_data_words = 0;
    for (u32 i = 0; i < MODULES_MAX; ++i) {
        module_status[i] = module_off[i] = module_position[i] = module_rows[i] = module_title[i] = 0;
        module_code_from[i] = module_code_to[i] = module_capabilities[i] = 0;
        module_detail[i][0] = 0;
        module_tile[i] = MANAGE_NO_TILE;
    }
    read_state();
    read_package_list();
#ifndef GRKIT_NO_MODULES
    /* The package modules, then the modules of the person. */
    for (u32 source = 0; source < 2; ++source) {
        long fd = syscall6(5, (u32)(source ? modules_path : package_modules_path), 0xc000, 0, 0, 0, 0);
        if (fd < 0) continue;
        static const struct directory_calls calls = {directory_read, directory_seek};
        int listed = directory_list(fd, directory_entries, sizeof(directory_entries), &calls, offer_module, (void *)source);
        syscall6(6, fd, 0, 0, 0, 0, 0);
        if (listed) { module_count = 0; return; }
    }
    if (!module_count) return;
    order_modules();
    /* The module array: the settings table with the choices of the GRKit
       section of MENU and the kept values, the contexts, the module list,
       the effect list, the services, then the data of each module. The
       effects are listed later (discover), so the modules leave room for
       each of them. plan_array counts the same parts. */
    u32 contexts = 0, data = 0, choices = MANAGE_PER_MODULE * module_count + EFFECTS_LISTED;
    u32 list = MANAGE_WORDS(module_count) + EFFECT_LIST_WORDS(EFFECTS_LISTED) + SERVICES_WORDS;
    for (u32 i = 0; i < module_count; ++i) {
        struct grm_setting settings[GRM_SETTINGS];
        u32 n = 0, uses = 0, tiles = 0;
        const u8 *tile_data;
        /* A module that is Off loads too, stopped (runtime/manage.h), so that
           Enable turns it on at once. Its code does not run while it is Off. */
        int status = safe_mode ? (int)MANAGE_SAFE : read_module(i);
        if (!status) status = grm_settings(&module, module.code, 0, settings, &n, module_uses[i], &uses);
        if (!status) status = grm_tiles(&module, module.code, 0, &tile_data, &tiles);
        if (!status && module_tiles + tiles > GRKIT_MODULE_TILES_MAX) status = GRM_NO_ROOM;
        u32 words = (module.data_size + module.bss_size) / 4;
        if (!status && (setting_total + n + choices > GRKIT_MODULE_SETTINGS_MAX ||
                        SETTINGS_WORDS(setting_total + n + choices) + contexts + CONTEXT_WORDS(uses) + list + data + words > MODULE_ARRAY_WORDS ||
                        module_code_bytes + module.code_size > MODULE_CODE_LIMIT)) status = GRM_NO_ROOM;
        if (status) {
            module_status[i] = (u32)status;
            print("GR_MODULE_SKIPPED "); print(module_names[i]); print(" "); number((u32)status); print("\n");
            continue;
        }
        module_usable |= 1U << i;
        module_tile_first[i] = module_tiles;
        module_tiles += tiles;
        module_first[i] = setting_total;
        module_settings[i] = n;
        module_use_count[i] = uses;
        for (u32 k = 0; k < n; ++k, ++setting_total) {
            u32 a = length(module.id), b = length(settings[k].key);
            name_copy(setting_names[setting_total], module.id, a);
            setting_names[setting_total][a] = '.';
            name_copy(setting_names[setting_total] + a + 1, settings[k].key, b);
            setting_values[setting_total] = settings[k].values;
            setting_flags[setting_total] = settings[k].flags;
        }
        contexts += CONTEXT_WORDS(uses);
        data += words;
        module_code_bytes += module.code_size;
    }
    module_contexts = contexts;
    module_data_words = data;
#endif
}
/* The words of the module array, after the effects are listed: the kept
   values fill the rest of the table and of the array. */
static void plan_array(void)
{
    module_words = kept_count = 0;
#ifndef GRKIT_NO_MODULES
    if (!module_count) return;
    u32 entries = setting_total + MANAGE_PER_MODULE * module_count + listed_count;
    u32 fixed = SETTINGS_WORDS(entries) + module_contexts + MANAGE_WORDS(module_count) +
                EFFECT_LIST_WORDS(listed_count) + SERVICES_WORDS + module_data_words;
    u32 room = GRKIT_MODULE_SETTINGS_MAX - entries;
    if (room > (MODULE_ARRAY_WORDS - fixed) / 3) room = (MODULE_ARRAY_WORDS - fixed) / 3;
    plan_kept(room);
    module_words = fixed + 3 * kept_count;
#endif
}
/* Whether a usable module uses CORE.TONE (plan_modules). */
static int tone_used(void)
{
    for (u32 i = 0; i < module_count; ++i) {
        if (!(module_usable >> i & 1)) continue;
        for (u32 u = 0; u < module_use_count[i]; ++u) {
            const char *name = module_uses[i][u];
            if (text_equal(name + (name[0] == '?'), "CORE.TONE")) return 1;
        }
    }
    return 0;
}
static int scan(void)
{
    /* An earlier attempt of this power-on wrote the plan: the loader deletes
       it at power-on. A second plan would replace it in a rename, which the
       camera's Linux does not do. */
    long fd = syscall6(5, (u32)plan_path, 0x8800, 0, 0, 0, 0);
    if (fd >= 0) { syscall6(6, fd, 0, 0, 0, 0, 0); return 0; }
    /* O_EXCL: the loader resends SCAN until a plan appears, so attempts can
       overlap. Only the attempt that creates the temporary plan moves the
       inbox and lists the effects. */
    fd = syscall6(5, (u32)plan_tmp, 0xc1, 0600, 0, 0, 0);
    if (fd == -17) return 3;
    if (fd < 0) return 36;
    /* Safe mode: GRKIT/SAFE exists. */
    {
        u8 mode_text[4];
        safe_mode = read_text(safe_path, mode_text, sizeof(mode_text)) >= 0;
        profile_mode = read_text(profile_path, mode_text, sizeof(mode_text)) >= 0;
    }
    /* The modules first: the effects install only for a module that uses
       CORE.TONE. */
    plan_modules();
    collect();
    int status = discover();
    if (status) {
        syscall6(6, fd, 0, 0, 0, 0, 0);
        syscall6(10, (u32)plan_tmp, 0, 0, 0, 0, 0);
        return status;
    }
    plan_array();
    layout();
    u32 effects_at = PLAN_HEADER + count * GRKIT_GRF_MAX_SIZE;
    put32(plan, 0x314d5247); put32(plan + 4, count);
    put32(plan + 8, size / 4); put32(plan + 12, icon_chunks);
    put32(plan + 16, effects_at + listed_count * PLANNED_EFFECT);
    put32(plan + 20, module_words);
    put32(plan + 24, module_tiles);
    put32(plan + 28, (safe_mode ? PLAN_SAFE : 0) | (profile_mode ? PLAN_PROFILE : 0));
    put32(plan + 32, overflow_count);
    put32(plan + 36, listed_count);
    for (u32 i = 0; i < count; ++i)
        copy(plan + PLAN_HEADER + i * GRKIT_GRF_MAX_SIZE, profiles[i], GRKIT_GRF_MAX_SIZE);
    for (u32 i = 0; i < listed_count; ++i) {
        u8 *entry = plan + effects_at + i * PLANNED_EFFECT;
        zero(entry, PLANNED_EFFECT);
        copy(entry, listed[i], LISTED_HEAD);
        copy(entry + LISTED_HEAD, (const u8 *)names[i], CATALOG_NAME_SIZE);
        put32(entry + 80, listed_status[i]);
        put32(entry + 84, listed_choice[i]);
        put32(entry + 88, listed_record[i]);
    }
    u32 done = 0, total = effects_at + listed_count * PLANNED_EFFECT;
    while (done < total) {
        long n = syscall6(4, fd, (u32)(plan + done), total - done, 0, 0, 0);
        if (n <= 0) break;
        done += n;
    }
    if (syscall6(6, fd, 0, 0, 0, 0, 0) < 0 || done < total ||
        syscall6(38, (u32)plan_tmp, (u32)plan_path, 0, 0, 0, 0) < 0) {
        syscall6(10, (u32)plan_tmp, 0, 0, 0, 0, 0);
        return 36;
    }
    print("GR_MODULE_SCANNED "); number(count); print("\n");
    return 0;
}
static int sync_alias(u32 address, u32 n)
{
    __asm__ volatile("dsb sy" ::: "memory");
    return syscall6(0xf0002, (u32)at(address), (u32)at(address + n), 0, 0, 0, 0) == 0;
}
/* A native icon descriptor, then the 40 x 40 pixels. */
#ifdef GRKIT_ICON_DESCRIPTOR_12
/* Width and a valid flag, height, and the pixel offset in words. */
#define ICON_PIXELS 12
#define ICON_HEADER(icon) do { store(icon, 0x00280001); store(icon + 4, 40); \
    store(icon + 8, (icon + ICON_PIXELS - pixels) / 4); } while (0)
#else
/* Height and width, then the pixel offset in words. */
#define ICON_PIXELS 8
#define ICON_HEADER(icon) do { store(icon, 0x00280028); \
    store(icon + 4, (icon + ICON_PIXELS - pixels) / 4); } while (0)
#endif
static int heap(u32 p, u32 n)
{
    return !(p & 3) && p >= HEAP_MIN && n <= HEAP_END - HEAP_MIN && p <= HEAP_END - n;
}
/* The extension points of the selected plugins. Only their patches change
   the firmware. */
static u32 needed_points;
/* patch_points[i]: the extension points that need patch i, or 0 for code that
   each installation publishes. */
static int applies(u32 i) { return !patch_points[i] || (patch_points[i] & needed_points); }
static void publish(struct patch *p, const u8 *bytes)
{
    if (p->size == 4) store(p->address, get32(bytes));
    else copy(at(p->address), bytes, p->size);
    __asm__ volatile("dsb sy" ::: "memory");
}
#ifdef GRKIT_RELOCATE
/* Relocation of the private code to the running build of the firmware
   version (relocation.h). The report of a failure goes to INSTALL.ERR. */
static u32 window_at[RELOCATION_WINDOW_COUNT];
static struct relocation_state relocation;
static u32 read_word(u32 address) { return word(address); }
static u32 symbol_delta(u32 symbol) { return relocation_delta(&relocation_tables, &relocation, symbol); }
static u32 pair_value(u32 address)
{
    u32 low = word(address), high = word(address + 4);
    return (((high >> 4) & 0xf000U) | (high & 0xfffU)) << 16 | ((low >> 4) & 0xf000U) | (low & 0xfffU);
}
static int relocate(void)
{
    relocation.read = read_word;
    relocation.window_at = window_at;
    relocation.symbol_at = relocated;
    if (!relocation_resolve(&relocation_tables, &relocation)) return 47;
    /* Patched firmware can keep data after the image end that the startup
       literals give, where the private code goes. */
    for (u32 i = 0; i < GUARD_PAIR_COUNT; ++i) {
        u32 value = pair_value(relocated[guard_symbols[i]]);
        if (value >= relocation.image_end && value < RELOCATION_DATA_START) return 45;
    }
    for (u32 i = 0; i < PATCH_COUNT; ++i) {
        if (patch_symbols[i] == RELOCATION_NONE) continue;
        patches[i].address += symbol_delta(patch_symbols[i]);
        if (patches[i].kind) continue;
        /* A stock branch, or a MOVW or MOVT of a firmware address, can
           change when code or data moves, so only its masked bits must be
           equal (patch_compare). Other stock words must be equal. Rollback
           restores the running bytes. */
        u8 *before = (u8 *)patches[i].before;
        u32 live = word(patches[i].address), stock = get32(before);
        if (patch_compare[i] ? relocation_mask(live) != relocation_mask(stock) : live != stock) return 16;
        put32(before, live);
    }
    for (u32 i = 0; i < RELOCATION_REFERENCE_COUNT; ++i) {
        const u32 *r = relocation_references + i * 4;
        u32 patch = r[0], site = patch_symbols[patch];
        if (r[1] + 4 > patches[patch].size ||
            !relocation_move(patches[patch].after + r[1], r[3], symbol_delta(r[2]), symbol_delta(site)))
            return 47;
    }
    return 0;
}
#endif
/* SCAN wrote the plan at this power-on, and the loader allocated the arrays
   for it. INSTALL takes the effects from the plan, so that both steps install
   the same effects. */
static int read_plan(void)
{
    u32 have = 0; long got = 0; u8 extra;
    long fd = syscall6(5, (u32)plan_path, 0x8800, 0, 0, 0, 0);
    if (fd < 0) return 37;
    while (have < sizeof(plan)) {
        got = syscall6(3, fd, (u32)(plan + have), sizeof(plan) - have, 0, 0, 0);
        if (got <= 0) break;
        have += (u32)got;
    }
    if (have == sizeof(plan)) got = syscall6(3, fd, (u32)&extra, 1, 0, 0, 0);
    syscall6(6, fd, 0, 0, 0, 0, 0);
    if (got < 0 || (have == sizeof(plan) && got) || have < PLAN_HEADER || get32(plan) != 0x314d5247) return 37;
    count = get32(plan + 4);
    safe_mode = get32(plan + 28) & PLAN_SAFE;
    profile_mode = (get32(plan + 28) & PLAN_PROFILE) != 0;
    overflow_count = get32(plan + 32);
    listed_count = get32(plan + 36);
    u32 effects_at = PLAN_HEADER + count * GRKIT_GRF_MAX_SIZE;
    if (count > MAX_COUNT || listed_count > EFFECTS_LISTED || have != effects_at + listed_count * PLANNED_EFFECT ||
        get32(plan + 16) != have || get32(plan + 28) > (PLAN_SAFE | PLAN_PROFILE)) return 37;
    for (u32 i = 0; i < listed_count; ++i) {
        const u8 *entry = plan + effects_at + i * PLANNED_EFFECT;
        copy(listed[i], entry, LISTED_HEAD);
        copy((u8 *)names[i], entry + LISTED_HEAD, CATALOG_NAME_SIZE);
        names[i][CATALOG_NAME_SIZE - 1] = 0;
        listed[i][CATALOG_ID + 31] = 0;
        listed_status[i] = get32(entry + 80);
        listed_choice[i] = get32(entry + 84);
        listed_record[i] = get32(entry + 88);
        if (listed_status[i] > EFFECT_UNREADABLE || listed_choice[i] >= EFFECT_CHOICE_VALUES ||
            (listed_record[i] != EFFECTS_NO_RECORD && listed_record[i] >= count)) return 37;
    }
    for (u32 i = 0; i < count; ++i) {
        copy(profiles[i], plan + PLAN_HEADER + i * GRKIT_GRF_MAX_SIZE, GRKIT_GRF_MAX_SIZE);
        u32 bytes = get32(profiles[i] + 8);
        if ((bytes != GRKIT_GRF_SIZE && bytes != GRKIT_GRF_MAX_SIZE) || !valid_profile(profiles[i], bytes)) return 37;
    }
    return 0;
}
#ifndef GRKIT_NO_MODULES
/* The header entries of the module pages and layers (runtime/abi.h): three
   words each. Layers stay in painting order. */
static u32 module_pages[GRKIT_MODULE_SLOTS * 3], module_page_count;
/* The plane filters in stage order (GRKIT_HOST_MODULE_FILTERS), two words
   each. */
#define MODULE_FILTERS (GRKIT_HOST_MODULE_FILTERS_WORDS / 2)
static u32 module_filters[MODULE_FILTERS * 2], module_filter_count;
static u32 module_layers[GRKIT_MODULE_SLOTS * 3], module_layer_count;
static u8 module_code[GRM_MAX_SEGMENT], module_data[GRM_MAX_SEGMENT];
/* The handle of a setting that a page or a layer of module i names
   (struct grm_layer): its own setting, or the handle of a used one. */
static u32 module_handle(u32 i, const u32 *use_handles, u32 setting)
{
    if (setting == GRM_NO_SETTING) return GRM_NO_SETTING;
    return setting & GRM_USED ? use_handles[setting & ~GRM_USED] : module_first[i] + setting;
}
/* The number of values of a setting handle. */
static u32 handle_values(u32 handle)
{
    if (IS_CORE_SETTING(handle)) return core_setting_values[CORE_SETTING(handle)];
    return handle < setting_total ? setting_values[handle] : 0;
}
/* Link and copy each usable module: its code at `code` and after, its data
   in the script array `array`. A module that does not link is skipped; its
   memory stays reserved. */
static int install_modules(u32 code, u32 array, u32 pixels, const u32 *icon_bases)
{
    struct grm_symbol symbols[CORE_EXPORT_COUNT + 1];
    u32 n = 0, code_at = code;
    for (; n < CORE_EXPORT_COUNT; ++n) {
        symbols[n].hash = core_exports[n * 2];
        symbols[n].value = relocated[TAIL_SYMBOL] + core_exports[n * 2 + 1];
    }
    /* grkit_self: the context of the module that links. */
    u32 self = n++;
    symbols[self].hash = 0x811c9dc5;
    for (const char *p = "grkit_self"; *p; ++p) { symbols[self].hash ^= (u8)*p; symbols[self].hash *= 0x01000193; }
    /* The settings table. A saved setting has its state file ID; a second
       setting with the same ID is not saved. The choices of the GRKit
       section, those of the effects, and the kept values follow the settings
       of the modules. */
    u32 choices = MANAGE_PER_MODULE * module_count + listed_count, total = setting_total + choices + kept_count;
    store(array, total);
    store(array + 4, 0);
    for (u32 k = 0; k < setting_total; ++k) {
        u32 entry = array + 4 * SETTINGS_ENTRY(k), id = 0;
        if (setting_flags[k] & GRM_SETTING_SAVED) {
            id = grm_state_id(setting_names[k]);
            for (u32 j = 0; j < k; ++j)
                if (word(array + 4 * SETTINGS_ENTRY(j)) == id) {
                    print("GR_MODULE_SETTING_UNSAVED "); print(setting_names[k]); print("\n");
                    id = 0;
                }
        }
        store(entry, id);
        store(entry + 4, 0);
        store(entry + 8, setting_values[k]);
    }
    for (u32 i = 0; i < module_count; ++i) {
        u32 entry = array + 4 * SETTINGS_ENTRY(setting_total + MANAGE_PER_MODULE * i);
        store(entry, manage_id(i, "OFF"));
        store(entry + 4, module_off[i]);
        store(entry + 8, 2);
        store(entry + 12, manage_id(i, "POS"));
        store(entry + 16, module_position[i]);
        store(entry + 20, MANAGE_MODULES + 1);
        store(entry + 24, manage_id(i, "ROWS"));
        store(entry + 28, module_rows[i]);
        store(entry + 32, 1U << MANAGE_OPTION_VALUES);
    }
    for (u32 i = 0; i < listed_count; ++i) {
        u32 entry = array + 4 * SETTINGS_ENTRY(setting_total + MANAGE_PER_MODULE * module_count + i);
        store(entry, effect_choice_id(i));
        store(entry + 4, listed_choice[i]);
        store(entry + 8, EFFECT_CHOICE_VALUES);
    }
    for (u32 k = 0; k < kept_count; ++k) {
        u32 entry = array + 4 * SETTINGS_ENTRY(setting_total + choices + k);
        store(entry, kept_ids[k]);
        store(entry + 4, kept_values[k]);
        store(entry + 8, kept_values[k] + 1);
    }
    u32 context_at = array + 4 * SETTINGS_WORDS(total), list_at = context_at;
    for (u32 i = 0; i < module_count; ++i)
        if (module_usable >> i & 1) list_at += 4 * CONTEXT_WORDS(module_use_count[i]);
    u32 effects_list = list_at + 4 * MANAGE_WORDS(module_count);
    u32 services = effects_list + 4 * EFFECT_LIST_WORDS(listed_count);
    u32 data_at = services + 4 * SERVICES_WORDS;
    u32 subscribers = 0;
    for (u32 k = 0; k < SERVICES_WORDS; ++k) store(services + 4 * k, 0);
    services_block = services;
    if (!sync_alias(array, list_at - array)) return 21;
    module_page_count = module_layer_count = module_filter_count = capabilities_used = capabilities_running = 0;
    for (u32 i = 0; i < module_count; ++i) {
        if (!(module_usable >> i & 1) || read_module(i)) continue;
        u32 context = context_at, code_base = code_at, data_base = data_at, data_bytes = module.data_size + module.bss_size;
        context_at += 4 * CONTEXT_WORDS(module_use_count[i]);
        code_at += module.code_size;
        data_at += data_bytes;
        u32 failed = 0, own_count = 0, own_uses = 0, layer_count = 0, page_count = 0, tile_count = 0;
        struct grm_setting own[GRM_SETTINGS];
        char own_names[GRM_USES][GRM_NAME + 1];
        struct grm_layer layers[GRKIT_MODULE_SLOTS];
        struct grm_page pages[GRKIT_MODULE_SLOTS];
        struct grm_filter filters[GRKIT_MODULE_SLOTS];
        u32 filter_count = 0;
        const u8 *tiles = 0;
        int status = 0;
        /* The context: its first setting, their count, reserved, then the
           handle of each used setting: a capability of the core, or a
           setting of a module. A setting of another module must be public. */
        u32 use_handles[GRM_USES], points = 0, capabilities = 0;
        store(context, module_first[i]);
        store(context + 4, module_settings[i]);
        store(context + 8, 0);
        for (u32 u = 0; u < module_use_count[i]; ++u) {
            const char *name = module_uses[i][u];
            int optional = name[0] == '?';
            u32 handle = GRM_NO_SETTING;
            for (u32 k = 0; k < GRKIT_CORE_SETTING_COUNT && handle == GRM_NO_SETTING; ++k)
                if (text_equal(core_setting_names[k], name + optional)) {
                    handle = GRKIT_CORE_SETTING_BASE + k;
                    points |= core_setting_points[k];
                    capabilities |= 1U << k;
                }
            for (u32 k = 0; k < setting_total && handle == GRM_NO_SETTING; ++k)
                if (text_equal(setting_names[k], name + optional) &&
                    ((setting_flags[k] & GRM_SETTING_PUBLIC) || (k - module_first[i] < module_settings[i]))) handle = k;
            if (handle == GRM_NO_SETTING && !optional) { status = GRM_MISSING_SETTING; failed = u; }
            use_handles[u] = handle;
            store(context + 12 + u * 4, handle);
        }
        if (!sync_alias(context, 4 * CONTEXT_WORDS(module_use_count[i]))) return 21;
        symbols[self].value = context;
        if (!status) status = grm_link(&module, module_code, module_data, code_base, data_base, symbols, n, &failed);
        if (!status) status = grm_settings(&module, module_code, code_base, own, &own_count, own_names, &own_uses);
        if (!status) status = grm_tiles(&module, module_code, code_base, &tiles, &tile_count);
        if (!status)
            status = grm_registrations(&module, module_code, code_base, own, own_count, own_uses,
                                       layers, &layer_count, pages, &page_count, filters, &filter_count,
                                       GRKIT_MODULE_SLOTS);
        /* The handles of the pages and the layers. A page or a layer of an
           optional used setting that no module has is not registered. A
           page needs a row for each value of a used setting. */
        for (u32 k = 0; !status && k < page_count; ++k) {
            pages[k].setting = module_handle(i, use_handles, pages[k].setting);
            /* The core gives the rows of a page of CORE.TONE. */
            u32 core_rows = pages[k].flags & GRM_CHOICE_CORE_ROWS;
            /* The rows do not fit the setting: the page is not valid. */
            if (pages[k].setting != GRM_NO_SETTING &&
                (core_rows ? pages[k].setting != GRKIT_CORE_TONE : handle_values(pages[k].setting) != pages[k].rows))
                status = GRM_BAD_DESCRIPTOR;
        }
        for (u32 k = 0; !status && k < layer_count; ++k)
            if (layers[k].setting != GRM_NO_SETTING) {
                layers[k].setting = module_handle(i, use_handles, layers[k].setting);
                if (layers[k].setting == GRM_NO_SETTING) layers[k].paint = 0;
            }
        for (u32 k = 0; !status && k < filter_count; ++k) {
            filters[k].setting = module_handle(i, use_handles, filters[k].setting);
            if (filters[k].setting == GRM_NO_SETTING) filters[k].apply = 0;
        }
        if (!status && (module_layer_count + layer_count > GRKIT_MODULE_SLOTS ||
                        module_page_count + page_count > GRKIT_MODULE_SLOTS ||
                        module_filter_count + filter_count > MODULE_FILTERS)) status = GRM_NO_ROOM;
        if (status) {
            module_status[i] = (u32)status;
            if (status == GRM_MISSING_IMPORT) grm_import_name(&module, failed, module_detail[i], sizeof(module_detail[i]));
            if (status == GRM_MISSING_SETTING && failed < module_use_count[i])
                name_copy(module_detail[i], module_uses[i][failed], length(module_uses[i][failed]));
            print("GR_MODULE_SKIPPED "); print(module.id); print(" "); number((u32)status);
            if (status == GRM_MISSING_IMPORT || status == GRM_MISSING_SETTING) { print(" "); number(failed); }
            print("\n");
            continue;
        }
        copy(at(code_base), module_code, module.code_size);
        /* The native settings rows of each page get absolute setting handles
           (runtime/modules.h): row 0 the page setting, the others settings of
           the module. */
        for (u32 k = 0; k < page_count; ++k) {
            u32 rows = word(pages[k].address + 32), list = word(pages[k].address + 36);
            for (u32 r = 0; r < rows && pages[k].setting != GRM_NO_SETTING; ++r)
                store(list + r * 16 + 4, r ? module_handle(i, use_handles, word(list + r * 16 + 4)) : pages[k].setting);
        }
        copy(at(data_base), module_data, module.data_size);
        zero(at(data_base + module.data_size), module.bss_size);
        if (!sync_alias(code_base, module.code_size)) return 21;
        if (data_bytes && !sync_alias(data_base, data_bytes)) return 21;
        /* The tiles, in the icon slots after the effect icons. */
        for (u32 k = 0; k < tile_count; ++k) {
            u32 slot = HOST_ICON_COUNT + count + module_tile_first[i] + k;
            u32 icon = icon_bases[slot / ICONS_PER_CHUNK] + (slot % ICONS_PER_CHUNK) * ICON_STRIDE;
            ICON_HEADER(icon);
            copy(at(icon + ICON_PIXELS), tiles + k * GRM_TILE_BYTES, GRM_TILE_BYTES);
            if (!sync_alias(icon, ICON_STRIDE)) return 21;
        }
        /* Module management shows the title of the first page. */
        if (page_count) module_title[i] = word(pages[0].address + 16);
        if (tile_count) module_tile[i] = module_tile_first[i];
        /* The pages in load order, and the layers in painting order: a
           layer goes after the layers with the same or a lower order. */
        for (u32 k = 0; k < page_count; ++k) {
            if (pages[k].setting == GRM_NO_SETTING) continue;
            module_pages[module_page_count * 3] = pages[k].address;
            module_pages[module_page_count * 3 + 1] = pages[k].setting;
            module_pages[module_page_count * 3 + 2] = module_tile_first[i];
            ++module_page_count;
            points |= POINT_MENU_PAGE;
        }
        for (u32 k = 0; k < layer_count; ++k) {
            if (!layers[k].paint) continue;
            points |= POINT_VIEWFINDER_OVERLAY;
            u32 at_slot = module_layer_count++;
            u32 key = layers[k].order << 16 | layers[k].flags;
            while (at_slot && module_layers[(at_slot - 1) * 3 + 2] >> 16 > layers[k].order) {
                for (u32 w = 0; w < 3; ++w) module_layers[at_slot * 3 + w] = module_layers[(at_slot - 1) * 3 + w];
                --at_slot;
            }
            module_layers[at_slot * 3] = layers[k].paint;
            module_layers[at_slot * 3 + 1] = layers[k].setting;
            module_layers[at_slot * 3 + 2] = key;
        }
        /* The filters in stage order: a filter goes after the filters of the
           same or a lower stage. */
        for (u32 k = 0; k < filter_count; ++k) {
            if (!filters[k].apply) continue;
            points |= POINT_PICTURE_ENCODE;
            u32 at_slot = module_filter_count++;
            while (at_slot && module_filters[(at_slot - 1) * 2 + 1] >> 16 > filters[k].stage) {
                module_filters[at_slot * 2] = module_filters[(at_slot - 1) * 2];
                module_filters[at_slot * 2 + 1] = module_filters[(at_slot - 1) * 2 + 1];
                --at_slot;
            }
            module_filters[at_slot * 2] = filters[k].apply;
            module_filters[at_slot * 2 + 1] = filters[k].stage << 16 | filters[k].setting;
        }
        /* The event subscriber: SYSTEM_READY needs the hook of the
           shooting screen. */
        struct grm_subscriber subscriber;
        if (grm_subscriber(&module, module_code, code_base, &subscriber) == 1 &&
            subscribers < SERVICES_SUBSCRIBERS_MAX) {
            store(services + SERVICES_SUBSCRIBERS + 8 * subscribers, subscriber.handle);
            store(services + SERVICES_SUBSCRIBERS + 8 * subscribers + 4, subscriber.events);
            store(services + SERVICES_SUBSCRIBER_COUNT, ++subscribers);
            if (subscriber.events & SERVICE_EVENT_SYSTEM_READY) points |= POINT_VIEWFINDER_OVERLAY;
        }
        /* The patches of the used capabilities and of the menu. */
        needed_points |= points;
        capabilities_used |= capabilities;
        module_code_from[i] = code_base;
        module_code_to[i] = code_base + module.code_size;
        module_capabilities[i] = capabilities;
        if (!module_off[i]) capabilities_running |= capabilities;
        print("GR_MODULE_LOADED "); print(module.id); print(" "); number(code_base); print("\n");
    }
    /* The module list of the Modules page. */
    module_list = list_at;
    store(list_at + MANAGE_COUNT, module_count);
    store(list_at + MANAGE_SETTINGS, setting_total);
    store(list_at + MANAGE_FLAGS, safe_mode ? MANAGE_SAFE_MODE : 0);
    store(list_at + MANAGE_OPEN, 0);
    for (u32 i = 0; i < module_count; ++i) {
        u32 entry = list_at + MANAGE_HEADER + i * MANAGE_STRIDE, n = 0;
        volatile u16 *name = (volatile u16 *)at(entry + MANAGE_NAME);
        u32 status = module_status[i];
        store(entry + MANAGE_STATUS, status);
        store(entry + MANAGE_TITLE, status ? 0 : module_title[i]);
        store(entry + MANAGE_TILE, status ? MANAGE_NO_TILE : module_tile[i]);
        store(entry + MANAGE_CODE, status ? 0 : module_code_from[i]);
        store(entry + MANAGE_CODE_END, status ? 0 : module_code_to[i]);
        store(entry + MANAGE_USES, status ? 0 : module_capabilities[i]);
        for (const char *p = module_names[i]; *p && *p != '.'; ++p) name[n++] = (u16)(*p >= 'a' && *p <= 'z' ? *p - 32 : *p);
        /* An error shows after the name: "NAME (52)". */
        if (status > MANAGE_OFF) {
            char digits[12]; u32 d = sizeof(digits);
            do { digits[--d] = (char)('0' + status % 10); status /= 10; } while (status);
            name[n++] = ' '; name[n++] = '(';
            while (d < sizeof(digits)) name[n++] = (u16)digits[d++];
            name[n++] = ')';
        }
        while (n < MANAGE_NAME_UNITS) name[n++] = 0;
    }
    if (!sync_alias(list_at, 4 * MANAGE_WORDS(module_count))) return 21;
    /* The modules that are Off start stopped: the core calls none of their
       code (SERVICES_SUSPENDED). */
    {
        u32 stopped = 0;
        for (u32 i = 0; i < module_count && stopped < SERVICES_SUSPENDED_MAX; ++i) {
            if (module_status[i] || !module_off[i]) continue;
            store(services + SERVICES_SUSPENDED + 4 + 8 * stopped, module_code_from[i]);
            store(services + SERVICES_SUSPENDED + 8 + 8 * stopped, module_code_to[i]);
            ++stopped;
        }
        store(services + SERVICES_SUSPENDED, stopped);
    }
    /* The effect list of the GRKit section of MENU, with the record of each
       installed effect. */
    effect_list = listed_count ? effects_list : 0;
    store(effects_list + EFFECTS_COUNT, listed_count);
    store(effects_list + EFFECTS_SETTINGS, setting_total + MANAGE_PER_MODULE * module_count);
    for (u32 i = 0; i < listed_count; ++i) {
        u32 entry = effects_list + EFFECTS_HEADER + i * EFFECTS_STRIDE, n = 0;
        volatile u16 *name = (volatile u16 *)at(entry + EFFECTS_NAME);
        store(entry + EFFECTS_STATUS, listed_status[i]);
        store(entry + EFFECTS_RECORD, listed_record[i]);
        for (; n < 16 && listed[i][48 + n]; ++n) name[n] = listed[i][48 + n];
        while (n < EFFECTS_NAME_UNITS) name[n++] = 0;
    }
    if (listed_count && !sync_alias(effects_list, 4 * EFFECT_LIST_WORDS(listed_count))) return 21;
    if (!sync_alias(services, 4 * SERVICES_WORDS)) return 21;
    return 0;
}
#endif
/* ---- State (runtime/state.h) ----------------------------------------------
   A missing STATE.TXT reads STATE.TMP, which a save that the power cut
   left. */
static void read_state(void)
{
    static u8 text[STATE_TEXT_MAX];
    long n = read_text(state_path, text, sizeof(text));
    if (n < 0) n = read_text(state_tmp_path, text, sizeof(text));
    if (n >= 0) {
        state_parse(&saved_state, text, n);
        return;
    }
    /* The first start after an update from GRKit 1.2: its values, under the
       current keys. The loader saves them in GRKIT at once. */
    static const u32 from[] = {LEGACY_STATE_FROM}, to[] = {LEGACY_STATE_TO};
    n = read_text(legacy_state_path, text, sizeof(text));
    if (n < 0) n = read_text(legacy_state_tmp_path, text, sizeof(text));
    state_parse(&saved_state, text, n);
    state_translate(&saved_state, from, to, LEGACY_STATE_COUNT, PARAMETER_KEY_BASE);
}
/* Restore the values of the capabilities that loaded modules use, the saved
   module settings in `table` (0 without one), and the Fn adjustments of the
   records. */
static void apply_state(u32 base, u32 table)
{
    static const u32 ids[] = {CORE_STATE_IDS};
    static const u16 words[] = {GRKIT_CORE_SETTING_WORDS}, values[] = {GRKIT_CORE_SETTING_VALUES};
    u32 value;
    for (u32 k = 0; k < GRKIT_CORE_SETTING_COUNT; ++k) {
        if (!(capabilities_used >> k & 1) || !state_value(&saved_state, ids[k], &value)) continue;
        if (k == GRKIT_CORE_TONE - GRKIT_CORE_SETTING_BASE) {
            /* The effect is its ImageTone code. */
            for (u32 i = 0; i < count; ++i) {
                u32 record = base + GRKIT_HOST_HEADER_SIZE + i * GRKIT_RECORD_SIZE;
                if (*(volatile u16 *)at(record + GRKIT_RECORD_CODE) == value) store(base + GRKIT_HOST_ACTIVE, record);
            }
        } else if (value < values[k]) store(base + words[k], value);
    }
    if (table) {
        u32 n = word(table);
        for (u32 k = 0; k < n && k < GRKIT_MODULE_SETTINGS_MAX; ++k) {
            u32 entry = table + 4 * SETTINGS_ENTRY(k), id = word(entry);
            if (id && state_value(&saved_state, id, &value) && value < word(entry + 8)) store(entry + 4, value);
        }
        sync_alias(table, 4 * SETTINGS_WORDS(n));
    }
    /* The adjustments of an effect: base-9 digits, the first lowest. */
    for (u32 i = 0; i < count; ++i) {
        u32 record = base + GRKIT_HOST_HEADER_SIZE + i * GRKIT_RECORD_SIZE;
        u32 code = *(volatile u16 *)at(record + GRKIT_RECORD_CODE) & 0x7fffU;
        if (!state_value(&saved_state, PARAMETER_KEY_BASE + code, &value) || value > PARAMETER_VALUE_MAX) continue;
        for (u32 d = 0; d < 9; ++d) { at(record + GRKIT_RECORD_PARAMETERS + d)[0] = (u8)(value % 9); value /= 9; }
    }
}
static int install(void)
{
    int status = read_plan();
    if (status) return status;
    /* The loaded modules add the points that they need (install_modules). */
    needed_points = 0;
    /* The modules of SCAN, which the loader reserved memory for. */
    plan_modules();
    plan_array();
    layout();
    if (get32(plan + 8) != size / 4 || get32(plan + 12) != icon_chunks) return 37;
    if (get32(plan + 20) != module_words || get32(plan + 24) != module_tiles) return 37;
#ifdef GRKIT_RELOCATE
    if ((status = relocate())) return status;
#endif
    /* The arrays: the registry, the icon chunks, the module array, then the
       tail (grkit_core::runtime::loader). */
    if (*(volatile u16 *)at(ARRAY_COUNT) != 2 + icon_chunks + (module_words != 0) || word(ARRAY_TABLE) != size / 4) return 10;
    u32 base = word(ARRAY_TABLE + 4), icon_bases[MAX_CHUNKS];
    if (!heap(base, size)) return 11;
    for (u32 i = 0; i < icon_chunks; ++i) {
        u32 bytes = chunk_count(i) * ICON_STRIDE;
        u32 ptr = word(ARRAY_TABLE + (i + 1) * 8 + 4);
        if (word(ARRAY_TABLE + (i + 1) * 8) != bytes / 4 || !heap(ptr, bytes) ||
            (ptr < base + size && base < ptr + bytes)) return 11;
        for (u32 j = 0; j < i; ++j)
            if (ptr < icon_bases[j] + chunk_count(j) * ICON_STRIDE && icon_bases[j] < ptr + bytes) return 11;
        icon_bases[i] = ptr;
    }
    /* The module array follows the icon chunks. */
    u32 module_array = 0;
    if (module_words) {
        u32 entry = ARRAY_TABLE + (1 + icon_chunks) * 8, bytes = module_words * 4;
        module_array = word(entry + 4);
        if (word(entry) != module_words || !heap(module_array, bytes) ||
            (module_array < base + size && base < module_array + bytes)) return 11;
        for (u32 j = 0; j < icon_chunks; ++j)
            if (module_array < icon_bases[j] + chunk_count(j) * ICON_STRIDE && icon_bases[j] < module_array + bytes) return 11;
    }
    /* The tail follows the other arrays and overlaps none of them. */
    u32 tail = word(ARRAY_TABLE + (2 + icon_chunks + (module_words != 0) - 1) * 8 + 4);
    if (word(ARRAY_TABLE + (2 + icon_chunks + (module_words != 0) - 1) * 8) != GRKIT_TAIL_SIZE / 4 ||
        !heap(tail, GRKIT_TAIL_SIZE) || (tail < base + size && base < tail + GRKIT_TAIL_SIZE) ||
        (module_array && tail < module_array + module_words * 4 && module_array < tail + GRKIT_TAIL_SIZE)) return 11;
    for (u32 j = 0; j < icon_chunks; ++j)
        if (tail < icon_bases[j] + chunk_count(j) * ICON_STRIDE && icon_bases[j] < tail + GRKIT_TAIL_SIZE) return 11;
    /* The tail after the RTOS image has no stock bytes to check. On a camera it
       holds the noise of uninitialized DRAM after power-on, or the code of an
       earlier GRKit session, so its content cannot show that it is free. The
       guards of the relocation detect a firmware that extends the image into
       it. */
#ifndef GRKIT_NO_MODULES
    /* The .data values of the running image are those at its copy source. */
#define INITIAL(address) ((address) - RELOCATION_DATA_START + relocation.data_source)
    if (word(ENGLISH_SLOT) != word(INITIAL(ENGLISH_SLOT))) return 12;
#ifndef GRKIT_NO_IMAGE_CONTROL
    /* The record is unchanged since startup and has the pinned layout. */
    if (!equal(at(STOCK_RECORD), (const u8 *)at(INITIAL(STOCK_RECORD)), 68)) return 13;
    for (u32 i = 0; i < 68; i += 4)
        if (relocation_mask(word(STOCK_RECORD + i)) != relocation_mask(get32(stock_record + i))) return 13;
    for (u32 i = 0; i < 4; ++i) {
        if (word(STOCK_RECORD + 68 + i * 4) == relocated[initialized_symbols[i]]) continue;
        return equal(at(STOCK_RECORD + 68), (const u8 *)at(INITIAL(STOCK_RECORD + 68)), 16) ? 14 : 13;
    }
#endif
    /* No firmware code refers to the RTOS tail, so earlier GRKit code there is
       overwritten. Every other patch must still find its stock bytes. */
#endif
    for (u32 i = 0; i < PATCH_COUNT; ++i)
        if (!patches[i].kind && applies(i) && !equal(at(patches[i].address), patches[i].before, patches[i].size)) return 16;
#ifndef GRKIT_NO_MODULES
#ifndef GRKIT_NO_IMAGE_CONTROL
    /* Each table is unchanged since startup and has the pinned layout, and
       entry 11 of the first one is the Standard record. */
    for (u32 i = 0; i < 3; ++i) {
        if (!equal(at(TABLE_ADDRESS(i)), (const u8 *)at(INITIAL(TABLE_ADDRESS(i))), TABLE_BYTES)) return 17;
        for (u32 j = 0; j < TABLE_BYTES; j += 4)
            if (relocation_mask(word(TABLE_ADDRESS(i) + j)) !=
                relocation_mask(get32(stock_tables + i * TABLE_BYTES + j))) return 17;
    }
    if (word(TABLE_ADDRESS(0) + 11 * 4) != STOCK_RECORD) return 17;
    /* Reject stale native types; never attempt an in-place settings repair. */
    u32 domain = word(SETTINGS_DOMAIN);
    if (!heap(domain, 0x51c)) return 39;
    u32 control = word(domain + 0x518);
    if (!heap(control, 4)) return 39;
    u32 payload = word(control);
    if (!heap(payload, SETTINGS_IC_TYPE + 1) || at(payload + SETTINGS_IC_TYPE)[0] >= IC_TYPE_COUNT) return 39;
#endif
#ifdef ICON_DRAWER_SLOT
    /* The icon drawer exists after the first icon draw. Its first word is the
       base of icon pixel offsets, which can lie outside the mapping. */
    u32 drawer = word(ICON_DRAWER_SLOT);
    if (!heap(drawer, 4)) return 26;
    u32 pixels = word(drawer);
    if ((pixels & 3) || !pixels) return 26;
#else
    u32 pixels = word(ICON_PIXEL_BASE_SLOT);
    if ((pixels & 3) || pixels < MAP_BASE || pixels >= HEAP_MIN) return 26;
#endif
#endif
    zero(at(base), size);
    store(base + GRKIT_HOST_COUNT, count); store(base + GRKIT_HOST_RECORDS, base + GRKIT_HOST_HEADER_SIZE);
    store(base + GRKIT_HOST_TAIL, tail);
    store(base + GRKIT_HOST_MAGIC, GRKIT_HOST_MAGIC_VALUE); store(base + GRKIT_HOST_VERSION, GRKIT_HOST_VERSION_VALUE);
    /* The next still live view shows that GRKit has loaded (runtime/framing.c). */
    store(base + GRKIT_HOST_LOAD_CUE, 1);
#ifndef GRKIT_NO_MODULES
    for (u32 i = 0; i < icon_chunks; ++i) {
        zero(at(icon_bases[i]), chunk_count(i) * ICON_STRIDE);
        store(base + GRKIT_HOST_ICON_CHUNKS + i * 4, icon_bases[i]);
    }
    /* Menu text after the picture scratch (runtime/abi.h). */
    _Static_assert(MAX_COUNT <= GRKIT_TEXT_RECORDS, "Menu text has no summary slot for each record");
    _Static_assert(MENU_TEXT_BYTES <= GRKIT_TEXT_SUMMARY, "Menu text table overlaps the summaries");
    u32 text = tail + GRKIT_PICTURE_SCRATCH;
    copy(at(text), menu_text, MENU_TEXT_BYTES);
    _Static_assert(LABELS_BYTES <= GRKIT_LABELS_SIZE, "Live-view labels exceed their area");
    copy(at(text + GRKIT_TEXT_SIZE), overlay_labels, LABELS_BYTES);
#ifndef GRKIT_NO_IMAGE_CONTROL
    for (u32 i = 0; i < count; ++i) {
        u32 record = base + GRKIT_HOST_HEADER_SIZE + i * GRKIT_RECORD_SIZE;
        const u8 *profile = profiles[i];
        copy(at(record), (const u8 *)at(STOCK_RECORD), 84);
        store(record + 4, record + 84);
        copy(at(record + 84), profile + 128, 1556);
        /* Extension: features, then the post matrix, selective color, and the
           reserved zero bytes. */
        copy(at(record + GRKIT_RECORD_FEATURES), profile + 84, 4);
        copy(at(record + GRKIT_RECORD_POST), profile + 88, 40);
        if (get32(profile + 84) & 8U) multiaxial(record, profile);
        /* Neutral Fn adjustments until the loader restores the saved ones. */
        zero(at(record + GRKIT_RECORD_PARAMETERS), 9);
        u32 n = length((const char *)profile + 48);
        for (u32 j = 0; j < n; ++j) at(record + GRKIT_RECORD_NAME + j * 2)[0] = profile[48 + j];
        store(record + GRKIT_RECORD_MONOCHROME, get32(profile + 80));
        copy(at(record + GRKIT_RECORD_ID), profile + 16, 32);
        /* The Image Control summary in each language: the prefix, then the name. */
        for (u32 language = 0; language < GRKIT_TEXT_LANGUAGES; ++language) {
            volatile u16 *summary = (volatile u16 *)at(text + GRKIT_TEXT_SUMMARY +
                (language * GRKIT_TEXT_RECORDS + i) * GRKIT_SUMMARY_UNITS * 2);
            const u16 *table = (const u16 *)menu_text;
            const u16 *prefix = table + table[language * GRKIT_TEXT_STRINGS + TEXT_IMAGE_PREFIX] / 2;
            u32 k = 0, m = n < 16 ? n : 16;
            for (; prefix[k]; ++k) summary[k] = prefix[k];
            for (u32 j = 0; j < m; ++j) summary[k + j] = profile[48 + j];
            summary[k + m] = 0;
        }
        u32 slot = HOST_ICON_COUNT + i;
        u32 icon = icon_bases[slot / ICONS_PER_CHUNK] + (slot % ICONS_PER_CHUNK) * ICON_STRIDE;
        ICON_HEADER(icon);
        copy(at(icon + ICON_PIXELS), profile + 1684, 6400);
        store(record + GRKIT_RECORD_ICON, icon);
        /* Resolve hash collisions in catalog order, and skip native values. */
        u32 code = effect_code(profile + 16), j = 0;
        while (j < i || NATIVE_CODE(code)) {
            if (!NATIVE_CODE(code) && *(volatile u16 *)at(base + GRKIT_HOST_HEADER_SIZE + j * GRKIT_RECORD_SIZE + GRKIT_RECORD_CODE) != code) { ++j; continue; }
            code = 0x8000 | ((code + 1) & 0x7fff);
            j = 0;
        }
        *(volatile u16 *)at(record + GRKIT_RECORD_CODE) = code;
    }
#endif
    /* Host artwork does not consume an effect record or a native settings ID.
       It comes first in chunk 0: Off, the GRKit tile, the Version tile, then
       the Modules tile. */
    _Static_assert(HOST_ICON_COUNT == 4, "The host icons are Off, GRKit, Version, and Modules");
    for (u32 i = 0; i < HOST_ICON_COUNT; ++i) {
        u32 icon = icon_bases[0] + i * ICON_STRIDE;
        static const u8 *const artwork[HOST_ICON_COUNT] = {off_pixels, plugins_pixels, version_pixels, modules_pixels};
        ICON_HEADER(icon);
        copy(at(icon + ICON_PIXELS), artwork[i], 6400);
        /* The menu derives the Version and Modules icons from the GRKit icon. */
        if (i < 2) store(base + (i ? GRKIT_HOST_PLUGINS_ICON : GRKIT_HOST_OFF_ICON), icon);
    }
#endif
#ifndef GRKIT_NO_MODULES
    /* The GRKit section of MENU lists each module, also when none loads, so
       the menu hooks install with a listed module. */
    if (module_count) {
        u32 code = relocated[TAIL_SYMBOL] + TAIL_BYTES;
        if (code + module_code_bytes > RTOS_EXECUTABLE_END) return GRM_NO_ROOM;
        if ((status = install_modules(code, module_array, pixels, icon_bases))) return status;
        needed_points |= POINT_MENU_PAGE;
        store(base + GRKIT_HOST_MANAGE, module_list);
        store(base + GRKIT_HOST_EFFECTS, effect_list);
        store(base + GRKIT_HOST_SERVICES, services_block);
        store(base + GRKIT_HOST_MODULE_SETTINGS, module_array);
        store(base + GRKIT_HOST_CAPABILITIES, capabilities_running);
        store(base + GRKIT_HOST_MODULE_PAGE_COUNT, module_page_count);
        for (u32 i = 0; i < module_page_count * 3; ++i) store(base + GRKIT_HOST_MODULE_PAGES + i * 4, module_pages[i]);
        store(base + GRKIT_HOST_MODULE_LAYER_COUNT, module_layer_count);
        for (u32 i = 0; i < module_layer_count * 3; ++i) store(base + GRKIT_HOST_MODULE_LAYERS + i * 4, module_layers[i]);
        store(base + GRKIT_HOST_MODULE_FILTER_COUNT, module_filter_count);
        for (u32 i = 0; i < module_filter_count * 2; ++i) store(base + GRKIT_HOST_MODULE_FILTERS + i * 4, module_filters[i]);
    }
#else
    (void)module_array;
#endif
    /* The profile table: the code range and the name of each module that
       loaded, and counters at 0 (the tail holds noise after power-on). */
    if (profile_mode) {
        u32 table = tail + GRKIT_PROFILE;
        zero(at(table), GRKIT_PROFILE_SIZE);
#ifndef GRKIT_NO_MODULES
        for (u32 i = 0; i < module_count && i < GRKIT_MODULE_SLOTS; ++i) {
            u32 entry = table + i * GRKIT_PROFILE_MODULE_SIZE, n = 0;
            if (module_status[i]) continue;
            store(entry, module_code_from[i]);
            store(entry + 4, module_code_to[i]);
            for (const char *p = module_names[i]; *p && *p != '.' && n < GRKIT_PROFILE_NAME_BYTES; ++p, ++n)
                at(entry + 8 + n)[0] = (u8)(*p >= 'a' && *p <= 'z' ? *p - 32 : *p);
        }
#endif
        store(base + GRKIT_HOST_PROFILE, table);
    }
    /* The saved values, before the hooks start (plan_modules read them). */
    apply_state(base, module_count ? module_array : 0);
    for (u32 i = 0; i < CODE_RELOCATION_COUNT; ++i) {
        u32 patch_index = code_relocations[i * 2], offset = code_relocations[i * 2 + 1];
        put32(patches[patch_index].after + offset, base);
    }
    if (!sync_alias(base, size)) return 21;
    if (!sync_alias(tail, GRKIT_TAIL_SIZE)) return 21;
    for (u32 i = 0; i < icon_chunks; ++i)
        if (!sync_alias(icon_bases[i], chunk_count(i) * ICON_STRIDE)) return 21;
    /* Publish payloads before entry branches. Cross-domain coherence needs device validation. */
    u32 installed = 0;
    for (; installed < PATCH_COUNT; ++installed) {
        if (!applies(installed)) continue;
        publish(&patches[installed], patches[installed].after);
        if (!sync_alias(patches[installed].address, patches[installed].size)) break;
    }
    if (installed != PATCH_COUNT) {
        int rollback_ok = 1;
        for (u32 i = installed + 1; i > 0; --i) {
            if (!applies(i - 1)) continue;
            publish(&patches[i - 1], patches[i - 1].before);
            if (!sync_alias(patches[i - 1].address, patches[i - 1].size)) rollback_ok = 0;
        }
        return rollback_ok ? 23 : 24;
    }
    print("GR_MODULE_INSTALLED "); number(count); print("\n");
    return 0;
}
#ifdef MODULE_DIAGNOSTICS
/* Diagnostic builds can select profiles independently of the menu. */
static int select_profile(const char *id)
{
#ifdef GRKIT_RELOCATE
    relocation.read = read_word;
    relocation.window_at = window_at;
    relocation.symbol_at = relocated;
    if (!relocation_resolve(&relocation_tables, &relocation)) return 47;
#endif
    u32 base = word(ARRAY_TABLE + 4);
    if (!heap(base, GRKIT_HOST_HEADER_SIZE) || word(base + GRKIT_HOST_MAGIC) != GRKIT_HOST_MAGIC_VALUE || word(base + GRKIT_HOST_VERSION) != GRKIT_HOST_VERSION_VALUE) return 41;
    u32 n = word(base + GRKIT_HOST_COUNT), records = word(base + GRKIT_HOST_RECORDS), selected = 0;
    if (n > MAX_COUNT || records != base + GRKIT_HOST_HEADER_SIZE || !heap(base, GRKIT_HOST_HEADER_SIZE + n * GRKIT_RECORD_SIZE)) return 41;
    if (!text_equal(id, "off")) {
        for (u32 i = 0; i < n; ++i) {
            u32 p = records + i * GRKIT_RECORD_SIZE;
            if (equal(at(p + GRKIT_RECORD_ID), (const u8 *)id, length(id) + 1)) { selected = p; break; }
        }
        if (!selected) return 42;
    }
    store(base + GRKIT_HOST_ACTIVE, selected);
    if (!sync_alias(base, GRKIT_HOST_HEADER_SIZE)) return 21;
    print("GR_MODULE_SELECTED "); print(id); print("\n");
    return 0;
}
#endif
/* Create an empty file only when it is absent. */
/* Create a result file. A nonzero status is written as decimal text, so a
   card from a camera shows why INSTALL failed. */
static int create(const char *path, u32 status)
{
    long fd = syscall6(5, (u32)path, 0xc1, 0600, 0, 0, 0);
    if (fd < 0) return 0;
    if (status) {
        char text[12]; u32 n = sizeof(text) - 1, code = status;
        text[n] = '\n';
        do { text[--n] = '0' + status % 10; status /= 10; } while (status);
        syscall6(4, fd, (u32)(text + n), sizeof(text) - n, 0, 0, 0);
#ifdef GRKIT_RELOCATE
        /* After the status: one line for each relocation failure, with the
           reason, the window or symbol index, the pinned address, and a
           count or value (runtime/relocation.h). */
        if (code == 47) {
            u32 shown = relocation.failures < RELOCATION_REPORT ? relocation.failures : RELOCATION_REPORT;
            for (u32 i = 0; i < shown; ++i) {
                const struct relocation_failure *f = &relocation.failure[i];
                u32 fields[4] = {f->reason, f->index, f->address, f->count};
                char line[48]; u32 m = 0;
                for (u32 k = 0; k < 4; ++k) {
                    for (int shift = 28; shift >= 0; shift -= 4)
                        line[m++] = "0123456789abcdef"[(fields[k] >> shift) & 15];
                    line[m++] = k == 3 ? '\n' : ' ';
                }
                syscall6(4, fd, (u32)line, m, 0, 0, 0);
            }
        }
#else
        (void)code;
#endif
    }
    syscall6(6, fd, 0, 0, 0, 0, 0);
    return 1;
}
/* ---- REPORT.TXT ------------------------------------------------------------
   The result of each boot for a person: the modules and why one did not
   load, the effects, and the safe mode. INSTALL writes it only when its text
   changes, in place: on GR IV exFAT a rename that replaces a file made the
   later requests of the camera fail. */
#define REPORT_MAX 8192U
static char report_text[REPORT_MAX + 1], report_old[REPORT_MAX + 1];
static u32 report_length;
static void say(const char *text)
{
    while (*text && report_length < REPORT_MAX) report_text[report_length++] = *text++;
}
static void say_number(u32 value)
{
    char digits[12]; u32 n = sizeof(digits) - 1;
    digits[n] = 0;
    do { digits[--n] = (char)('0' + value % 10); value /= 10; } while (value);
    say(digits + n);
}
/* Why a module did not load (runtime/module_link.h). */
static const char *module_reason(u32 status)
{
    switch (status) {
    case GRM_BAD_FILE: return "it is not a module file that this GRKit reads";
    case GRM_BAD_ABI: return "it needs another version of GRKit";
    case GRM_MISSING_IMPORT: return "it calls a function that GRKit does not have: ";
    case GRM_BAD_DESCRIPTOR: return "its description is not valid";
    case GRM_NO_ROOM: return "the modules need more memory than GRKit has";
    case GRM_MISSING_SETTING: return "it uses a setting that no module has: ";
    default: return "INSTALL stopped";
    }
}
static void write_report(int status)
{
    report_length = 0;
    say("GRKit "); say(GRKIT_VERSION); say(" ("); say(target_marker + sizeof("GRKIT_TARGET=") - 1); say(")\r\n");
    if (status) { say("\r\nGRKit did not install: error "); say_number((u32)status); say(".\r\n"); }
    if (safe_mode) say("\r\nSafe mode: GRKIT\\SAFE exists, so no module loads. Delete it to load the modules.\r\n");
    if (profile_mode) say("\r\nProfile: GRKIT\\PROFILE exists, so GRKit writes its CPU use to GRKIT\\PROFILE.TXT. Delete it to stop.\r\n");
    say("\r\nModules (the package, and GRKIT\\MODULES):\r\n");
    if (!module_count) say("none\r\n");
    for (u32 i = 0; i < module_count; ++i) {
        say(module_names[i]);
        if (module_source[i]) say(" (GRKIT\\MODULES)");
        u32 result = module_status[i];
        if (!result && (module_usable >> i & 1))
            say(module_off[i] ? "  loaded, stopped: Enable is Off in the GRKit section of MENU\r\n" : "  loaded\r\n");
        else if (result == MANAGE_SAFE) say("  not loaded: safe mode\r\n");
        else {
            say("  not loaded: error "); say_number(result); say(", "); say(module_reason(result));
            say(module_detail[i]); say("\r\n");
        }
    }
    if (module_overflow) {
        say_number(module_overflow);
        say(" more module files are not loaded. GRKit loads at most "); say_number(MODULES_MAX); say(" modules.\r\n");
    }
    say("\r\nEffects (GRKIT\\EFFECTS):\r\n");
    if (!listed_count) say(tone_used() ? "none\r\n" : "none: no module uses them\r\n");
    for (u32 i = 0; i < listed_count; ++i) {
        char name[17]; u32 k = 0;
        while (k < 16 && listed[i][48 + k]) { name[k] = (char)listed[i][48 + k]; ++k; }
        name[k] = 0;
        say(name); say(" ("); say(names[i]); say(")");
        if (listed_status[i] == EFFECT_LOADED) say("  loaded\r\n");
        else if (listed_status[i] == EFFECT_OFF && listed_record[i] != EFFECTS_NO_RECORD)
            say("  loaded, hidden: Off in the GRKit section of MENU\r\n");
        else if (listed_status[i] == EFFECT_OFF) say("  not loaded: Off in the GRKit section of MENU\r\n");
        else if (listed_status[i] == EFFECT_OVER) { say("  not loaded: GRKit installs at most "); say_number(MAX_COUNT); say(" effects\r\n"); }
        else say("  not loaded: the file changed during the start\r\n");
    }
    if (overflow_count) {
        say_number(overflow_count);
        say(" more effect files are not listed. GRKit lists at most "); say_number(EFFECTS_LISTED); say(" effects.\r\n");
    }
    /* A new card has no GRKIT folder until this start. */
    long made = syscall6(39, (u32)data_path, 0755, 0, 0, 0, 0);
    if (made < 0 && made != -17) return;
    long had = read_text(report_path, (u8 *)report_old, sizeof(report_old));
    if (had == (long)report_length && equal((const u8 *)report_old, (const u8 *)report_text, report_length)) return;
    write_file(report_path, (const u8 *)report_text, report_length);
}
/* Native state that the camera is still initializing. Other failures are final. */
static int not_ready(int status)
{
    return status == 12 || status == 14 || status == 26 || status == 39;
}
/* A package with a runtime for each build of a camera runs its installers in
   turn (runtime::package_runtime_files). `--known` installs on a build that
   the runtime names, and exits with 48 on another build. `--try` installs on a
   build that the relocation resolves, and exits with 47 on another build.
   On another build, both read the RTOS only and leave each file as it is:
   after Linux deletes a file in the folder, the camera does not find the next
   name that it looks up there, such as the next installer. `--install` is the
   last attempt and always leaves its result. */
enum install_mode { INSTALL_LAST, INSTALL_KNOWN, INSTALL_TRY };
static int names_build(void)
{
#if RTOS_IDENTITY_COUNT
    u32 identity = word(RTOS_IDENTITY);
    for (u32 i = 0; i < RTOS_IDENTITY_COUNT; ++i)
        if (identity == rtos_identities[i]) return 1;
#endif
    return 0;
}
static int relocation_found(void)
{
#ifdef GRKIT_RELOCATE
    relocation.read = read_word;
    relocation.window_at = window_at;
    relocation.symbol_at = relocated;
    return relocation_resolve(&relocation_tables, &relocation);
#else
    return 1;
#endif
}
/* Whether the running build is not the one of `mode`. A failure to read the
   RTOS is left to the install attempt, which reports it. */
static int other_build(enum install_mode mode)
{
    long fd = syscall6(5, (u32)"/dev/mem", 0x101002, 0, 0, 0, 0), other = 0;
    if (fd < 0) return 0;
    long mapped = syscall6(192, 0, MAP_SIZE, 1, 1, fd, MAP_BASE >> 12);
    if ((u32)mapped < 0xfffff001U) {
        mapping = (volatile u8 *)mapped;
        other = mode == INSTALL_KNOWN ? !names_build() : !relocation_found();
        syscall6(91, (u32)mapping, MAP_SIZE, 0, 0, 0, 0);
    }
    syscall6(6, fd, 0, 0, 0, 0, 0);
    return other;
}
__attribute__((used)) void start(u32 *stack)
{
    u32 argc = stack[0]; char **argv = (char **)(stack + 1); int status = 40;
    enum install_mode mode = INSTALL_LAST;
    if (argc == 2 && text_equal(argv[1], "--known")) mode = INSTALL_KNOWN;
    else if (argc == 2 && text_equal(argv[1], "--try")) mode = INSTALL_TRY;
    int installing = argc == 2 && (mode != INSTALL_LAST || text_equal(argv[1], "--install"));
    if (argc == 2 && text_equal(argv[1], "--scan")) status = scan();
    else if (installing
#ifdef MODULE_DIAGNOSTICS
             || (argc == 3 && text_equal(argv[1], "--select") && length(argv[2]) <= 31)
#endif
    ) {
        long fd = -1;
        int other = mode != INSTALL_LAST && other_build(mode);
        if (other) status = mode == INSTALL_KNOWN ? 48 : 47;
        /* The lock admits one install attempt at a time and stays after a result. */
        else if (argc == 2 && !create(lock_path, 0)) status = 3;
        else if ((fd = syscall6(5, (u32)"/dev/mem", 0x101002, 0, 0, 0, 0)) < 0) status = 1;
        else {
            long mapped = syscall6(192, 0, MAP_SIZE, 3, 1, fd, MAP_BASE >> 12);
            status = 2;
            if ((u32)mapped < 0xfffff001U) {
                mapping = (volatile u8 *)mapped;
                if (argc == 2) status = install();
#ifdef MODULE_DIAGNOSTICS
                else status = select_profile(argv[2]);
#endif
                syscall6(91, (u32)mapping, MAP_SIZE, 0, 0, 0, 0);
            }
            syscall6(6, fd, 0, 0, 0, 0, 0);
        }
        /* The loader polls these files: retry after not-ready, stop after a result.
           Another build leaves the next installer to run. */
        if (argc == 2 && status != 3 && !other) {
            if (not_ready(status)) syscall6(10, (u32)lock_path, 0, 0, 0, 0, 0);
            else {
                write_report(status);
                create(status ? failed_path : done_path, status);
            }
        }
    }
    if (status) { print("GR_MODULE_REJECTED "); number(status); print("\n"); }
    syscall6(1, status, 0, 0, 0, 0, 0); for (;;) {}
}
__attribute__((naked)) void _start(void) { __asm__ volatile("mov r0, sp\nb start"); }

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
static u8 profiles[MAX_COUNT][GRKIT_GRF_MAX_SIZE], temporary[GRKIT_GRF_MAX_SIZE];
static u8 directory_entries[4096];
static u8 plan[20 + MAX_COUNT * GRKIT_GRF_MAX_SIZE];
static u32 count, size, icon_chunks;
static const char root[] = EFFECTS_PATH;
static const char plan_path[] = PLAN_PATH;
static const char plan_tmp[] = PLAN_TMP_PATH;
static const char lock_path[] = LOCK_PATH;
static const char done_path[] = DONE_PATH;
static const char failed_path[] = FAILED_PATH;
static const char plugins_path[] = PLUGINS_PATH;
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
#include "selection.h"
static char names[MAX_COUNT][CATALOG_NAME_SIZE];
static int read_file(const char *path, u8 *buffer, u32 wanted)
{
    long fd=syscall6(5,(u32)path,0x8800,0,0,0,0);
    if(fd<0) return 0;
    u32 have=0; long got=0;
    while(have<wanted) {
        got=syscall6(3,fd,(u32)(buffer+have),wanted-have,0,0,0);
        if(got<=0) break;
        have+=(u32)got;
    }
    u8 extra; got=syscall6(3,fd,(u32)&extra,1,0,0,0);
    syscall6(6,fd,0,0,0,0,0);
    return have==wanted && got==0;
}
/* A missing PLUGINS.TXT selects every plugin. An unreadable or invalid one
   stops the installation, so a card never installs what its file does not
   describe. */
static int read_selection(u32 *selected)
{
    u8 text[SELECTION_FILE_MAX + 1];
    u32 have = 0; long got = 0;
    long fd = syscall6(5, (u32)plugins_path, 0x8800, 0, 0, 0, 0);
    *selected = SELECTION_ALL;
    if (fd == -2) return 0;
    if (fd < 0) return 44;
    while (have < sizeof(text)) {
        got = syscall6(3, fd, (u32)(text + have), sizeof(text) - have, 0, 0, 0);
        if (got <= 0) break;
        have += (u32)got;
    }
    syscall6(6, fd, 0, 0, 0, 0, 0);
    if (got < 0) return 44;
    if (have > SELECTION_FILE_MAX || !selection_parse(text, have, selected)) return 43;
    return 0;
}
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
/* A symbolic link or an unusable file does not stop the other effects. */
static void offer_effect(const char *name, u32 n, void *context)
{
    (void)context;
    if(!catalog_name(name,n)) return;
    char path[sizeof(root) + 13]; u32 prefix=length(root);
    copy((u8 *)path,(const u8 *)root,prefix); path[prefix++]='/';
    copy((u8 *)path+prefix,(const u8 *)name,n+1);
    u32 size=read_effect(path,temporary);
    if(size && valid_profile(temporary,size))
        count=catalog_offer(profiles,names,count,MAX_COUNT,temporary,name);
}
static int discover(void)
{
    count=0;
#if defined(GRKIT_NO_MODULES) || defined(GRKIT_NO_IMAGE_CONTROL)
    /* Without Image Control, the target has no use for effect files. */
    return 0;
#endif
    long fd=syscall6(5,(u32)root,0xc000,0,0,0,0);
    if(fd==-2) return 0;
    if(fd<0) return 30;
    static const struct directory_calls calls={directory_read,directory_seek};
    int status=directory_list(fd,directory_entries,sizeof(directory_entries),&calls,offer_effect,0);
    syscall6(6,fd,0,0,0,0,0);
    return status;
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
static void layout(void)
{
    size = GRKIT_HOST_HEADER_SIZE + count * GRKIT_RECORD_SIZE + GRKIT_TAIL_SIZE;
    icon_chunks = (count + HOST_ICON_COUNT + ICONS_PER_CHUNK - 1) / ICONS_PER_CHUNK;
}
static u32 chunk_count(u32 chunk)
{
    u32 left = count + HOST_ICON_COUNT - chunk * ICONS_PER_CHUNK;
    return left < ICONS_PER_CHUNK ? left : ICONS_PER_CHUNK;
}
static int scan(void)
{
    int status = discover();
    if (status) return status;
    layout();
    put32(plan, 0x314d5247); put32(plan + 4, count);
    put32(plan + 8, size / 4); put32(plan + 12, icon_chunks);
    put32(plan + 16, 20 + count * GRKIT_GRF_MAX_SIZE);
    for (u32 i = 0; i < count; ++i)
        copy(plan + 20 + i * GRKIT_GRF_MAX_SIZE, profiles[i], GRKIT_GRF_MAX_SIZE);
    /* O_EXCL: the loader resends SCAN until a plan appears, so attempts can overlap. */
    long fd = syscall6(5, (u32)plan_tmp, 0xc1, 0600, 0, 0, 0);
    if (fd == -17) return 3;
    if (fd < 0) return 36;
    u32 done = 0, total = 20 + count * GRKIT_GRF_MAX_SIZE;
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
static int install(void)
{
    u32 selected;
    int status = discover();
    if (status) return status;
    if ((status = read_selection(&selected))) return status;
    /* A plugin that needs effects has nothing to show without them. */
    if (!count) selected &= ~PLUGIN_NEEDS_EFFECTS;
    needed_points = 0;
    for (u32 i = 0; i < PLUGIN_COUNT; ++i)
        if (selected >> i & 1) needed_points |= plugin_points[i];
    layout();
    if (!read_file(plan_path, plan, 20 + count * GRKIT_GRF_MAX_SIZE) || get32(plan) != 0x314d5247 ||
        get32(plan + 4) != count || get32(plan + 8) != size / 4 ||
        get32(plan + 12) != icon_chunks || get32(plan + 16) != 20 + count * GRKIT_GRF_MAX_SIZE) return 37;
    for (u32 i = 0; i < count; ++i)
        if (!equal(plan + 20 + i * GRKIT_GRF_MAX_SIZE, profiles[i], GRKIT_GRF_MAX_SIZE)) return 37;
#ifdef GRKIT_RELOCATE
    if ((status = relocate())) return status;
#endif
    if (*(volatile u16 *)at(ARRAY_COUNT) != 1 + icon_chunks || word(ARRAY_TABLE) != size / 4) return 10;
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
    store(base + GRKIT_HOST_MAGIC, GRKIT_HOST_MAGIC_VALUE); store(base + GRKIT_HOST_VERSION, GRKIT_HOST_VERSION_VALUE);
    store(base + GRKIT_HOST_PLUGINS, selected);
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
    u32 text = base + GRKIT_HOST_HEADER_SIZE + count * GRKIT_RECORD_SIZE + GRKIT_PICTURE_SCRATCH;
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
       It comes first in chunk 0, so the plugin tiles stay contiguous. */
    for (u32 i = 0; i < HOST_ICON_COUNT; ++i) {
        u32 icon = icon_bases[0] + i * ICON_STRIDE;
        ICON_HEADER(icon);
        const u8 *artwork = i >= 3 ? module_pixels + (i - 3) * 6400 :
            i == 2 ? version_pixels : i ? plugins_pixels : off_pixels;
        copy(at(icon + ICON_PIXELS), artwork, 6400);
        /* The menu derives the Version icon from the GRKit icon, and the other
           plugin tiles from the first one. */
        if (i != 2 && i <= 3) store(base + (i == 3 ? GRKIT_HOST_TILE_ICON : i ? GRKIT_HOST_PLUGINS_ICON : GRKIT_HOST_OFF_ICON), icon);
    }
#endif
    for (u32 i = 0; i < CODE_RELOCATION_COUNT; ++i) {
        u32 patch_index = code_relocations[i * 2], offset = code_relocations[i * 2 + 1];
        put32(patches[patch_index].after + offset, base);
    }
    if (!sync_alias(base, size)) return 21;
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
    if (n > MAX_COUNT || records != base + GRKIT_HOST_HEADER_SIZE || !heap(base, GRKIT_HOST_HEADER_SIZE + n * GRKIT_RECORD_SIZE + GRKIT_TAIL_SIZE)) return 41;
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
/* Native state that the camera is still initializing. Other failures are final. */
static int not_ready(int status)
{
    return status == 12 || status == 14 || status == 26 || status == 39;
}
__attribute__((used)) void start(u32 *stack)
{
    u32 argc = stack[0]; char **argv = (char **)(stack + 1); int status = 40;
    if (argc == 2 && text_equal(argv[1], "--scan")) status = scan();
    else if ((argc == 2 && text_equal(argv[1], "--install"))
#ifdef MODULE_DIAGNOSTICS
             || (argc == 3 && text_equal(argv[1], "--select") && length(argv[2]) <= 31)
#endif
    ) {
        long fd = -1;
        /* The lock admits one install attempt at a time and stays after a result. */
        if (argc == 2 && !create(lock_path, 0)) status = 3;
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
        /* The loader polls these files: retry after not-ready, stop after a result. */
        if (argc == 2 && status != 3) {
            if (not_ready(status)) syscall6(10, (u32)lock_path, 0, 0, 0, 0, 0);
            else create(status ? failed_path : done_path, status);
        }
    }
    if (status) { print("GR_MODULE_REJECTED "); number(status); print("\n"); }
    syscall6(1, status, 0, 0, 0, 0, 0); for (;;) {}
}
__attribute__((naked)) void _start(void) { __asm__ volatile("mov r0, sp\nb start"); }

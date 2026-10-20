/* Firmware dump: copies the running RTOS image from RAM to the SD card. GR III,
   GR IIIx, and GR IV load the RTOS at the same base and keep the startup
   literals at the same address, so one program serves each camera.

   The range starts at the RTOS base and ends at the image end that the startup
   literals give (the .data copy source plus its size), rounded up to 4 KiB.
   The program maps /dev/mem with PROT_READ only and never writes camera
   memory. Output, in the folder of this program:
     RTOS.BIN   the image bytes
     INFO.TXT   the range, the startup literals, the build identity, a CRC-32,
                and the words of the boot parameter block
     DUMP.OK    written last, after a complete dump
     DUMP.ERR   a failure code instead of DUMP.OK
   GRDUMP.LCK admits one run at a time; STARTUP.TTL deletes it at power-on.

   A developer compares RTOS.BIN with the pinned firmware, or resolves the
   relocation tables of a runtime against it (tests/runtime/relocation_image.c). */
typedef unsigned char u8;
typedef unsigned int u32;

#define RTOS_BASE 0x53000000U
#define DATA_START 0x55000000U
#define STARTUP 0x53000450U
#define IDENTITY 0x53002038U
/* The parameter block that the boot code writes before the RTOS starts: the
   start handshake, the start factor, and the board version from word 0. */
#define PARAMETERS 0x50800000U
#define PARAMETER_WORDS 16
#define DIR "/media/storage1/SCRIPT/GRDUMP/"

static long sys(u32 n, u32 a, u32 b, u32 c, u32 d, u32 e, u32 f)
{
    register u32 r0 __asm__("r0") = a;
    register u32 r1 __asm__("r1") = b;
    register u32 r2 __asm__("r2") = c;
    register u32 r3 __asm__("r3") = d;
    register u32 r4 __asm__("r4") = e;
    register u32 r5 __asm__("r5") = f;
    register u32 r7 __asm__("r7") = n;
    __asm__ volatile("svc 0" : "+r"(r0) : "r"(r1), "r"(r2), "r"(r3), "r"(r4), "r"(r5), "r"(r7)
                     : "memory", "cc");
    return (long)r0;
}
#define SYS_EXIT 1
#define SYS_WRITE 4
#define SYS_OPEN 5
#define SYS_CLOSE 6
#define SYS_MUNMAP 91
#define SYS_FSYNC 118
#define SYS_MMAP2 192
#define O_RDONLY 0
#define O_WRONLY 1
#define O_CREAT 0100
#define O_EXCL 0200
#define O_TRUNC 01000
#define O_SYNC 04010000

static volatile const u32 *mem;
static u32 mem_word(u32 address) { return mem[(address - RTOS_BASE) / 4]; }

static u32 crc_table[256];
static void crc_init(void)
{
    for (u32 i = 0; i < 256; ++i) {
        u32 c = i;
        for (int k = 0; k < 8; ++k) c = c & 1 ? 0xedb88320U ^ (c >> 1) : c >> 1;
        crc_table[i] = c;
    }
}

static long create(const char *path, u32 flags)
{
    return sys(SYS_OPEN, (u32)path, O_WRONLY | O_CREAT | flags, 0644, 0, 0, 0);
}
static int write_all(long fd, const u8 *p, u32 n)
{
    while (n) {
        long w = sys(SYS_WRITE, (u32)fd, (u32)p, n, 0, 0, 0);
        if (w <= 0) return 0;
        p += w;
        n -= (u32)w;
    }
    return 1;
}

/* A small text buffer for INFO.TXT and DUMP.ERR. */
static char text[1024];
static u32 used;
static void put(const char *s) { while (*s && used < sizeof text) text[used++] = *s++; }
static void hex(u32 v)
{
    put("0x");
    for (int i = 7; i >= 0; --i) {
        char c = "0123456789abcdef"[(v >> (i * 4)) & 15];
        if (used < sizeof text) text[used++] = c;
    }
}
static void line(const char *name, u32 v) { put(name); put(" "); hex(v); put("\n"); }
static void save(const char *path)
{
    long fd = create(path, O_TRUNC);
    if (fd >= 0) {
        write_all(fd, (const u8 *)text, used);
        sys(SYS_FSYNC, (u32)fd, 0, 0, 0, 0, 0);
        sys(SYS_CLOSE, (u32)fd, 0, 0, 0, 0, 0);
    }
}
static void fail(u32 code)
{
    used = 0;
    line("error", code);
    save(DIR "DUMP.ERR");
    sys(SYS_EXIT, code, 0, 0, 0, 0, 0);
    for (;;) {}
}

static u32 buffer[16384];

__attribute__((used)) static void start(void)
{
    /* A second run while the first one copies stops here. */
    long lock = create(DIR "GRDUMP.LCK", O_EXCL);
    if (lock < 0) sys(SYS_EXIT, 0, 0, 0, 0, 0, 0);
    sys(SYS_CLOSE, (u32)lock, 0, 0, 0, 0, 0);

    long fd = sys(SYS_OPEN, (u32)"/dev/mem", O_RDONLY | O_SYNC, 0, 0, 0, 0);
    if (fd < 0) fail(1);
    const u32 span = DATA_START - RTOS_BASE;
    long mapped = sys(SYS_MMAP2, 0, span, 1 /* PROT_READ */, 1 /* MAP_SHARED */, (u32)fd,
                      RTOS_BASE >> 12);
    if ((u32)mapped >= 0xfffff001U) fail(2);
    mem = (volatile const u32 *)mapped;

    /* The startup literals: .data source, .data start, .data size, BSS start, BSS end. */
    u32 source = mem_word(STARTUP), start_data = mem_word(STARTUP + 4);
    u32 size = mem_word(STARTUP + 8), bss_start = mem_word(STARTUP + 12);
    u32 bss_end = mem_word(STARTUP + 16);
    u32 end = DATA_START, valid = 0;
    if (start_data == DATA_START && source > RTOS_BASE && source < DATA_START && size &&
        size <= DATA_START - source) {
        end = (source + size + 0xfffU) & ~0xfffU;
        valid = 1;
    }

    long out = create(DIR "RTOS.BIN", O_TRUNC);
    if (out < 0) fail(3);
    crc_init();
    u32 crc = 0xffffffffU;
    for (u32 address = RTOS_BASE; address < end;) {
        u32 n = end - address > sizeof buffer ? sizeof buffer : end - address;
        for (u32 i = 0; i < n / 4; ++i) buffer[i] = mem_word(address + i * 4);
        const u8 *bytes = (const u8 *)buffer;
        for (u32 i = 0; i < n; ++i) crc = crc_table[(crc ^ bytes[i]) & 0xff] ^ (crc >> 8);
        if (!write_all(out, bytes, n)) fail(4);
        address += n;
    }
    crc ^= 0xffffffffU;
    if (sys(SYS_FSYNC, (u32)out, 0, 0, 0, 0, 0) < 0) fail(5);
    sys(SYS_CLOSE, (u32)out, 0, 0, 0, 0, 0);

    used = 0;
    put("GRDUMP 1\n");
    line("start", RTOS_BASE);
    line("end", end);
    line("bytes", end - RTOS_BASE);
    line("crc32", crc);
    line("literals_valid", valid);
    line("data_source", source);
    line("data_start", start_data);
    line("data_size", size);
    line("bss_start", bss_start);
    line("bss_end", bss_end);
    line("identity", mem_word(IDENTITY));
    /* The parameter block is optional: without it the image is still complete. */
    long block = sys(SYS_MMAP2, 0, 0x1000, 1 /* PROT_READ */, 1 /* MAP_SHARED */, (u32)fd,
                     PARAMETERS >> 12);
    if ((u32)block < 0xfffff001U) {
        volatile const u32 *words = (volatile const u32 *)block;
        for (u32 i = 0; i < PARAMETER_WORDS; ++i) {
            put("parameter_");
            put((const char[]){"0123456789abcdef"[i], 0});
            put(" ");
            hex(words[i]);
            put("\n");
        }
        sys(SYS_MUNMAP, (u32)block, 0x1000, 0, 0, 0, 0);
    } else {
        line("parameters_unavailable", (u32)block);
    }
    save(DIR "INFO.TXT");

    sys(SYS_MUNMAP, (u32)mapped, span, 0, 0, 0, 0);
    sys(SYS_CLOSE, (u32)fd, 0, 0, 0, 0, 0);
    used = 0;
    line("bytes", end - RTOS_BASE);
    save(DIR "DUMP.OK");
    sys(SYS_EXIT, 0, 0, 0, 0, 0, 0);
    for (;;) {}
}
__attribute__((naked)) void _start(void) { __asm__ volatile("b start"); }

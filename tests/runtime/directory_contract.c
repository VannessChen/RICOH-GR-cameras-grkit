#include <assert.h>
#include <string.h>

typedef unsigned char u8;
typedef unsigned int u32;
#include "directory.h"

/* A model of a camera directory listing. Linux numbers the entries from 2 after
   "." and "..". The camera returns an empty name for a name longer than 12
   characters, and Linux then fails each read at that offset. The camera keeps
   one listing position: after a listing that stopped early, the next listing
   returns the old directory until its end. */
#define EIO_ERROR (-5L)
#define MAX_ENTRIES 64U
struct listing { const char *const *names; u32 count; };
static struct listing directory, stale;
static int stale_pending;
static long position, failed;
static u32 cursor;
static const struct listing *current;
static int fail_seek_at = -1, seeks;

static u32 put(u8 *buffer, u32 size, u32 used, const char *name, u8 type)
{
    u32 n = (u32)strlen(name), record = (DIRECTORY_NAME + n + 1 + 7) & ~7U;
    if (used + record > size) return 0;
    u8 *entry = buffer + used;
    memset(entry, 0, record);
    entry[16] = (u8)record; entry[17] = (u8)(record >> 8); entry[18] = type;
    memcpy(entry + DIRECTORY_NAME, name, n);
    return record;
}
static long model_read(long fd, u8 *buffer, u32 size)
{
    (void)fd;
    if (failed == position) return EIO_ERROR;
    u32 used = 0, record;
    if (position == 0) {
        current = stale_pending ? &stale : &directory;
        cursor = 0;
    }
    while (position < 2) {
        if (!(record = put(buffer, size, used, position ? ".." : ".", 4))) return (long)used;
        used += record; ++position;
    }
    while (cursor < current->count) {
        const char *name = current->names[cursor];
        if (strlen(name) > 12) {
            /* The camera moves past the entry, and Linux fails at this offset. */
            ++cursor; failed = position;
            stale_pending = 1;
            return used ? (long)used : EIO_ERROR;
        }
        u8 type = name[0] == '@' ? DIRECTORY_LINK : 8;
        if (!(record = put(buffer, size, used, name + (name[0] == '@'), type))) return (long)used;
        used += record; ++cursor; ++position;
    }
    /* A listing that reaches its end clears the camera's position. */
    stale_pending = 0;
    return (long)used;
}
static long model_seek(long fd, long offset, u32 whence)
{
    (void)fd;
    if (seeks++ == fail_seek_at) return EIO_ERROR;
    if (whence == 1) offset += position;
    else if (whence != 0) return EIO_ERROR;
    if (offset != position) failed = -1;
    position = offset;
    return position;
}
static const struct directory_calls calls = {model_read, model_seek};

static char seen[MAX_ENTRIES][16];
static u32 visits;
static void visit(const char *name, u32 n, void *context)
{
    assert(context == &visits);
    assert(n < sizeof(seen[0]) && strlen(name) == n);
    assert(visits < MAX_ENTRIES);
    memcpy(seen[visits++], name, n + 1);
}
static void reset(const char *const *names, u32 count)
{
    directory.names = names; directory.count = count;
    position = 0; failed = -1; cursor = 0; seeks = 0; fail_seek_at = -1;
    visits = 0;
}
static int list(u32 size)
{
    static u8 buffer[4096];
    assert(size <= sizeof(buffer));
    return directory_list(3, buffer, size, &calls, visit, &visits);
}
static void expect(const char *const *names, u32 count)
{
    assert(visits == count);
    for (u32 i = 0; i < count; ++i) assert(!strcmp(seen[i], names[i]));
}

static const char *const effects[] = {"A98CE47F.GRF", "0589146D.GRF", "E587917C.GRF"};
#define EFFECTS (sizeof(effects) / sizeof(effects[0]))
static const char *const listed[] = {".", "..", "A98CE47F.GRF", "0589146D.GRF", "E587917C.GRF"};
#define LISTED (sizeof(listed) / sizeof(listed[0]))

static void short_names_list_once(void)
{
    reset(effects, EFFECTS); stale_pending = 0;
    assert(list(4096) == 0);
    expect(listed, LISTED);
}
static void long_names_are_skipped(void)
{
    static const char *const names[] = {
        "._A98CE47F.GRF", "A98CE47F.GRF", "._0589146D.GRF", "0589146D.GRF",
        "E587917C.GRF", "._E587917C.GRF",
    };
    reset(names, sizeof(names) / sizeof(names[0])); stale_pending = 0;
    assert(list(4096) == 0);
    expect(listed, LISTED);
}
static void small_reads_skip_long_names(void)
{
    static const char *const names[] = {
        "A98CE47F.GRF", "._A98CE47F.GRF", "0589146D.GRF", "._0589146D.GRF", "E587917C.GRF",
    };
    reset(names, sizeof(names) / sizeof(names[0])); stale_pending = 0;
    /* Each read holds one record. */
    assert(list(40) == 0);
    expect(listed, LISTED);
}
static void a_stale_listing_is_not_visited(void)
{
    static const char *const scripts[] = {"PLUGINS.TTL", "SCAN.SH", "._SCAN.SH", "INSTALL.BIN"};
    stale.names = scripts; stale.count = sizeof(scripts) / sizeof(scripts[0]);
    reset(effects, EFFECTS); stale_pending = 1;
    assert(list(4096) == 0);
    expect(listed, LISTED);
}
static void links_are_not_visited(void)
{
    static const char *const names[] = {"A98CE47F.GRF", "@0589146D.GRF", "E587917C.GRF"};
    static const char *const expected[] = {".", "..", "A98CE47F.GRF", "E587917C.GRF"};
    reset(names, 3); stale_pending = 0;
    assert(list(4096) == 0);
    expect(expected, 4);
}
static long failing_read(long fd, u8 *buffer, u32 size)
{ (void)fd; (void)buffer; (void)size; return EIO_ERROR; }
static void a_lasting_error_stops_the_listing(void)
{
    static const struct directory_calls broken = {failing_read, model_seek};
    static u8 buffer[64];
    reset(effects, EFFECTS);
    assert(directory_list(3, buffer, sizeof(buffer), &broken, visit, &visits) == 30);
    /* One seek per pass start, then two per skip, up to the skip limit. */
    assert(seeks == 1 + 2 * DIRECTORY_SKIPS + 1);
    assert(visits == 0);
}
static void a_failed_seek_stops_the_listing(void)
{
    static const char *const names[] = {"._A98CE47F.GRF", "A98CE47F.GRF"};
    reset(names, 2); stale_pending = 0;
    fail_seek_at = 0;
    assert(list(4096) == 30);
    reset(names, 2); stale_pending = 0;
    /* The seek that reads the offset after the failed read. */
    fail_seek_at = 1;
    assert(list(4096) == 30);
}
static long bad_read(long fd, u8 *buffer, u32 size)
{
    (void)fd; (void)size;
    memset(buffer, 0, 32);
    buffer[16] = 64; /* longer than the read */
    return 32;
}
static void a_bad_record_stops_the_listing(void)
{
    static const struct directory_calls broken = {bad_read, model_seek};
    static u8 buffer[64];
    reset(effects, EFFECTS);
    assert(directory_list(3, buffer, sizeof(buffer), &broken, visit, &visits) == 31);
    assert(visits == 0);
}

int main(void)
{
    short_names_list_once();
    long_names_are_skipped();
    small_reads_skip_long_names();
    a_stale_listing_is_not_visited();
    links_are_not_visited();
    a_lasting_error_stops_the_listing();
    a_failed_seek_stops_the_listing();
    a_bad_record_stops_the_listing();
    return 0;
}

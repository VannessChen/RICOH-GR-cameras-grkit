#include <assert.h>
#include <string.h>

typedef unsigned char u8;
typedef unsigned int u32;
#include "state.h"

static struct state state;

static void parse(const char *text)
{
    state_parse(&state, (const u8 *)text, (long)strlen(text));
}

static u32 value_of(u32 key)
{
    u32 value = 0xdeadbeefU;
    assert(state_value(&state, key, &value));
    return value;
}

static int has(u32 key)
{
    u32 value;
    return state_value(&state, key, &value);
}

int main(void)
{
    /* CR LF and LF lines, and a last line without a newline. */
    parse("12\r\n3\r\n45\n6\n789\r\n0");
    assert(state.pairs == 3);
    assert(value_of(12) == 3 && value_of(45) == 6 && value_of(789) == 0);
    assert(!has(3));

    /* Empty lines do not shift the pairs. */
    parse("\r\n\r\n12\r\n\r\n3\r\n\r\n");
    assert(state.pairs == 1 && value_of(12) == 3);

    /* A later pair of a key wins. */
    parse("12\r\n3\r\n12\r\n4\r\n");
    assert(value_of(12) == 4);

    /* A pair with an invalid line is skipped, and the next pair still reads. */
    parse("12\r\nx\r\n13\r\n5\r\n");
    assert(!has(12) && value_of(13) == 5);
    parse("1a\r\n7\r\n13\r\n5\r\n");
    assert(state.pairs == 1 && value_of(13) == 5);
    parse("-1\r\n7\r\n");
    assert(state.pairs == 0);

    /* Nine digits read, ten are invalid. */
    parse("999999999\r\n999999999\r\n1000000000\r\n1\r\n");
    assert(state.pairs == 1 && value_of(999999999U) == 999999999U);

    /* A key without its value is no pair. */
    parse("12\r\n3\r\n14");
    assert(state.pairs == 1 && !has(14));

    /* A missing file gives no pairs, and an empty one too. */
    state_parse(&state, (const u8 *)"", -2);
    assert(state.pairs == 0);
    parse("");
    assert(state.pairs == 0);

    /* The pairs stop at STATE_PAIRS. */
    {
        static char text[STATE_PAIRS * 2 * 8 + 64];
        u32 used = 0;
        for (u32 i = 0; i <= STATE_PAIRS; ++i) {
            char line[16];
            u32 n = 0, key = 1000 + i;
            do { line[n++] = (char)('0' + key % 10); key /= 10; } while (key);
            while (n) text[used++] = line[--n];
            memcpy(text + used, "\r\n1\r\n", 5);
            used += 5;
        }
        text[used] = 0;
        parse(text);
        assert(state.pairs == STATE_PAIRS);
        assert(has(1000 + STATE_PAIRS - 1) && !has(1000 + STATE_PAIRS));
    }

    /* The boot counter. */
    assert(state_boots((const u8 *)"", -2) == 0);
    assert(state_boots((const u8 *)"2", 1) == 2);
    assert(state_boots((const u8 *)"12\r\n", 4) == 12);
    assert(state_boots((const u8 *)"99999999", 8) == 9999);
    return 0;
}

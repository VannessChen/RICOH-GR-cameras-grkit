#ifndef GRKIT_STATE_H
#define GRKIT_STATE_H
/* The state file for the installer (crates/grkit-core/src/runtime.rs). The
   includer defines u8 and u32.

   STATE.TXT holds two decimal lines for each value: the key ID, then the
   value. The resident loader writes it, and INSTALL restores it before the
   hooks start. CR and LF end a line, and empty lines are skipped. A line with
   another character or more than 9 digits is invalid, and a pair with an
   invalid line is skipped. A later pair of a key wins. */

/* The most pairs that INSTALL keeps. The largest file of the loader has fewer
   (grkit_core::runtime::worst_state_size). */
#define STATE_PAIRS 512U

struct state {
    u32 ids[STATE_PAIRS], values[STATE_PAIRS], pairs;
};

/* Parse `n` bytes of `text`. A negative `n`, a missing file, gives no pairs. */
static void state_parse(struct state *state, const u8 *text, long n)
{
    u32 token = 0, digits = 0, invalid = 0, expecting = 0, key = 0, key_invalid = 0;
    state->pairs = 0;
    for (long i = 0; i <= n; ++i) {
        u8 c = i < n ? text[i] : '\n';
        if (c == '\r' || c == '\n') {
            if (digits + invalid) {
                if (!expecting) {
                    key = token;
                    key_invalid = invalid;
                    expecting = 1;
                } else {
                    expecting = 0;
                    if (!key_invalid && !invalid && state->pairs < STATE_PAIRS) {
                        state->ids[state->pairs] = key;
                        state->values[state->pairs++] = token;
                    }
                }
            }
            token = digits = invalid = 0;
            continue;
        }
        if (c < '0' || c > '9' || digits >= 9) { invalid = 1; continue; }
        token = token * 10 + (u32)(c - '0');
        ++digits;
    }
}

/* The value of `key`: 1 and *value, or 0 when the file has no valid pair. */
static int state_value(const struct state *state, u32 key, u32 *value)
{
    for (u32 i = state->pairs; i > 0; --i)
        if (state->ids[i - 1] == key) { *value = state->values[i - 1]; return 1; }
    return 0;
}

/* The boot counter of the safe mode (BOOT.TXT): its decimal digits, at most
   9999. A missing file is 0. */
static u32 state_boots(const u8 *text, long n)
{
    u32 boots = 0;
    for (long i = 0; i < n && boots < 1000; ++i)
        if (text[i] >= '0' && text[i] <= '9') boots = boots * 10 + (u32)(text[i] - '0');
    return boots;
}
#endif

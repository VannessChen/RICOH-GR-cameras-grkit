#ifndef GRKIT_PROFILE_H
#define GRKIT_PROFILE_H
/* The development profile. While the card has GRKIT/PROFILE or PROFILE.TXT,
   INSTALL sets GRKIT_HOST_PROFILE to the profile table (runtime/abi.h: GRKIT_PROFILE), and
   the core counts the calls and the CPU cycles of its hook points and of each
   module. The resident loader writes the counts of each period to
   GRKIT/PROFILE.LOG and clears them. Without the file, each point costs one
   read of the header word.

   The cycle counter of the PMU is per core and runs only after it is enabled
   on that core, so each start enables it where it is off. The firmware lets
   every task read it (PMUSERENR) and does not use it. A call that moves to
   another core reads two counters with different offsets, so a start keeps
   its core in the low 2 bits, and a call that ends on another core is not
   counted. The includer defines u32 and the header fields. */

#ifndef PROFILE_START
#if defined(__arm__)
/* The counter after it is enabled on this core. Writing PMCR with its value
   resets nothing: the reset bits read as 0. */
static inline u32 profile_start_cycles(void)
{
    u32 enabled, control, cycles;
    __asm__ volatile("mrc p15, 0, %0, c9, c12, 1" : "=r"(enabled));
    __asm__ volatile("mrc p15, 0, %0, c9, c12, 0" : "=r"(control));
    if (!(enabled & 0x80000000U) || !(control & 1U)) {
        __asm__ volatile("mcr p15, 0, %0, c9, c12, 0" ::"r"(control | 1U));
        __asm__ volatile("mcr p15, 0, %0, c9, c12, 1" ::"r"(0x80000000U));
        __asm__ volatile("isb" ::: "memory");
    }
    __asm__ volatile("mrc p15, 0, %0, c9, c13, 0" : "=r"(cycles));
    return cycles;
}
static inline u32 profile_now_cycles(void)
{
    u32 cycles;
    __asm__ volatile("mrc p15, 0, %0, c9, c13, 0" : "=r"(cycles));
    return cycles;
}
/* The core of the caller (MPIDR affinity 0). MPIDR is readable only in a
   privileged mode, and the RTOS tasks run in one. In User mode, the core
   reads as 0, so a move is not found there. */
static inline u32 profile_core(void)
{
    u32 status, core;
    __asm__ volatile("mrs %0, cpsr" : "=r"(status));
    if ((status & 0x1fU) == 0x10U) return 0;
    __asm__ volatile("mrc p15, 0, %0, c0, c0, 5" : "=r"(core));
    return core & 3U;
}
#define PROFILE_START() profile_start_cycles()
#define PROFILE_NOW() profile_now_cycles()
#define PROFILE_CORE() profile_core()
#else
#define PROFILE_START() 0U
#define PROFILE_NOW() 0U
#endif
#endif
#ifndef PROFILE_CORE
#define PROFILE_CORE() 0U
#endif
#ifndef PROFILE_MAP
#define PROFILE_MAP(address) ((volatile u32 *)(address))
#endif
#define PROFILE_DISCARD 0x40000000U

/* The start of a point, or 0 without the profile: the counter value with
   the core in its low 2 bits. */
static __attribute__((unused)) u32 profile_begin(volatile u32 *h)
{
    return h[GRKIT_HOST_PROFILE / 4] ? (PROFILE_START() & ~3U) | PROFILE_CORE() : 0;
}
/* The cycles since `start`, or PROFILE_DISCARD after a move to another core. */
static u32 profile_since(u32 start)
{
    u32 core = PROFILE_CORE(), now = PROFILE_NOW();
    return core == (start & 3U) ? (now & ~3U) - (start & ~3U) : PROFILE_DISCARD;
}
/* Count a call of `cycles` in slot `slot` of the table. Updates from two
   tasks can race: a lost count only lowers the numbers of one period. */
static __attribute__((noinline, unused)) void profile_add(u32 table, u32 slot, u32 cycles)
{
    volatile u32 *s;
    if (cycles >= PROFILE_DISCARD || slot >= GRKIT_PROFILE_SLOT_COUNT) return;
    s = PROFILE_MAP(table + GRKIT_PROFILE_SLOTS + slot * GRKIT_PROFILE_SLOT_SIZE);
    ++s[0];
    s[1] += cycles;
    if (cycles > s[2]) s[2] = cycles;
}
/* End a point that started at `start`. */
static __attribute__((unused)) void profile_end(volatile u32 *h, u32 start, u32 slot)
{
    u32 table = h[GRKIT_HOST_PROFILE / 4];
    if (table) profile_add(table, slot, profile_since(start));
}
/* End a call of module code at `fn` of kind `kind` (GRKIT_PROFILE_PAINT,
   FILTER, or CALL): the slot of the module whose code holds `fn`. */
static __attribute__((noinline, unused)) void profile_end_module(volatile u32 *h, u32 start, u32 fn, u32 kind)
{
    u32 table = h[GRKIT_HOST_PROFILE / 4], cycles, i;
    if (!table) return;
    cycles = profile_since(start);
    for (i = 0; i < GRKIT_PROFILE_MODULES; ++i) {
        volatile u32 *m = PROFILE_MAP(table + i * GRKIT_PROFILE_MODULE_SIZE);
        if (fn >= m[0] && fn < m[1]) {
            profile_add(table, GRKIT_PROFILE_MODULE_BASE + i * GRKIT_PROFILE_KINDS + kind, cycles);
            return;
        }
    }
}
#endif

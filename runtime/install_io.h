#ifndef GRKIT_INSTALL_IO_H
#define GRKIT_INSTALL_IO_H
/* A part of the installer: the file and memory helpers of the other parts.
   installer.c includes it once, after syscall6 (the Linux system call) and
   the mapping accessors at, word, and store. A host test includes it after
   its own versions of them. */
static u32 get32(const u8 *p) { return p[0] | p[1] << 8 | p[2] << 16 | (u32)p[3] << 24; }
static void put32(u8 *p, u32 v) { for (u32 i=0; i<4; ++i) p[i] = v >> (i*8); }
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
/* Whether the file `path` exists. */
static int present(const char *path)
{
    long fd = syscall6(5, (u32)path, 0x8800, 0, 0, 0, 0);
    if (fd < 0) return 0;
    syscall6(6, fd, 0, 0, 0, 0, 0);
    return 1;
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
#endif

#ifndef GRKIT_DIRECTORY_H
#define GRKIT_DIRECTORY_H
/* Directory listing for the installer. The includer defines u8 and u32.

   The camera's exFAT driver returns an empty name for a name longer than 12
   characters, such as the "._" file that macOS writes next to a copied file.
   The Linux of GR IV rejects that entry with an error, and each read at the
   same offset fails again. The driver has already moved past the entry, so a
   read at the next offset continues after it. The older Linux of GR III passes
   the empty name, which is not an effect name. FAT cards always return 8.3
   names.

   The camera also keeps one listing position for all of Linux. After a listing
   that stopped early, the next listing of any directory first returns entries
   of the old directory. A listing that reaches its end clears this state. */

/* The most failed reads that one listing skips. */
#define DIRECTORY_SKIPS 256U
/* A linux_dirent64 record: inode (8 bytes), offset (8), record size (2),
   type (1), then the NUL-terminated name. */
#define DIRECTORY_NAME 19U
#define DIRECTORY_LINK 10U

struct directory_calls {
    /* getdents64: a byte count, 0 at the end, or a negative error. */
    long (*read)(long fd, u8 *buffer, u32 size);
    /* lseek: the new offset or a negative error. whence 0 sets the offset, and
       whence 1 adds to the current offset. */
    long (*seek)(long fd, long offset, u32 whence);
};

/* Call `visit` for each entry of the open directory `fd`, except symbolic
   links. The listing runs two times: the first pass ends any listing that an
   earlier reader left open, and only the second pass calls `visit`.
   Return 0, 30 for a read error that a skip does not clear, or 31 for a bad
   record. */
static int directory_list(long fd, u8 *buffer, u32 size, const struct directory_calls *calls,
                          void (*visit)(const char *name, u32 length, void *context),
                          void *context)
{
    u32 skips=0;
    for(u32 pass=0;pass<2;++pass) {
        if(calls->seek(fd,0,0)<0) return 30;
        for(;;) {
            long got=calls->read(fd,buffer,size);
            if(!got) break;
            if(got<0) {
                long position=calls->seek(fd,0,1);
                if(position<0 || ++skips>DIRECTORY_SKIPS || calls->seek(fd,position+1,0)<0)
                    return 30;
                continue;
            }
            for(u32 offset=0;offset<(u32)got;) {
                const u8 *entry=buffer+offset;
                if((u32)got-offset<DIRECTORY_NAME+1) return 31;
                u32 record=(u32)entry[16]|entry[17]<<8;
                if(record<DIRECTORY_NAME+1 || record>(u32)got-offset) return 31;
                u32 n=0;
                while(n<record-DIRECTORY_NAME && entry[DIRECTORY_NAME+n]) ++n;
                if(n==record-DIRECTORY_NAME) return 31;
                if(pass && entry[18]!=DIRECTORY_LINK)
                    visit((const char *)entry+DIRECTORY_NAME,n,context);
                offset+=record;
            }
        }
    }
    return 0;
}
#endif

#include <libultra/ultratypes.h>
#include <libultra/os_message.h>

// No Controller Pak is attached. Guest-owned OSPfs handles remain opaque to these stubs.
typedef struct OSPfs OSPfs;
typedef struct OSPfsState OSPfsState;

#define PFS_ERR_NOPACK 2

s32 osPfsInitPak(OSMesgQueue *queue, OSPfs *pfs, s32 channel) {
    (void) queue;
    (void) pfs;
    (void) channel;
    return PFS_ERR_NOPACK;
}
s32 osPfsAllocateFile(OSPfs *pfs, u16 companyCode, u32 gameCode, u8 *gameName, u8 *extName, s32 fileSize,
                      s32 *fileNo) {
    (void) pfs;
    (void) companyCode;
    (void) gameCode;
    (void) gameName;
    (void) extName;
    (void) fileSize;
    (void) fileNo;
    return PFS_ERR_NOPACK;
}
s32 osPfsDeleteFile(OSPfs *pfs, u16 companyCode, u32 gameCode, u8 *gameName, u8 *extName) {
    (void) pfs;
    (void) companyCode;
    (void) gameCode;
    (void) gameName;
    (void) extName;
    return PFS_ERR_NOPACK;
}
s32 osPfsFileState(OSPfs *pfs, s32 fileNo, OSPfsState *state) {
    (void) pfs;
    (void) fileNo;
    (void) state;
    return PFS_ERR_NOPACK;
}
s32 osPfsFindFile(OSPfs *pfs, u16 companyCode, u32 gameCode, u8 *gameName, u8 *extName, s32 *fileNo) {
    (void) pfs;
    (void) companyCode;
    (void) gameCode;
    (void) gameName;
    (void) extName;
    (void) fileNo;
    return PFS_ERR_NOPACK;
}
s32 osPfsFreeBlocks(OSPfs *pfs, s32 *leftoverBytes) {
    (void) pfs;
    (void) leftoverBytes;
    return PFS_ERR_NOPACK;
}
s32 osPfsReadWriteFile(OSPfs *pfs, s32 fileNo, u8 flag, s32 offset, s32 size, u8 *data) {
    (void) pfs;
    (void) fileNo;
    (void) flag;
    (void) offset;
    (void) size;
    (void) data;
    return PFS_ERR_NOPACK;
}

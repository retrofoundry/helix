#include <reality64/internal.h>

// MARK: - Methods from libultra (include/PR/os_eeprom.h). The retQueue arg is unused:
// the Rust backend is synchronous, so no completion message is posted here.

s32 osEepromProbe(OSMesgQueue *mq) {
    (void) mq;
    return R64EepromProbe();
}

s32 osEepromLongRead(OSMesgQueue *mq, u8 address, u8 *buffer, int nbytes) {
    (void) mq;
    return R64EepromRead(address, buffer, nbytes);
}

s32 osEepromLongWrite(OSMesgQueue *mq, u8 address, u8 *buffer, int nbytes) {
    (void) mq;
    return R64EepromWrite(address, buffer, nbytes);
}

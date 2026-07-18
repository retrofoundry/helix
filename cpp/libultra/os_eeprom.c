#include <helix/internal.h>

// MARK: - Methods from libultra (include/PR/os_eeprom.h). The retQueue arg is unused:
// the Rust backend is synchronous, so no completion message is posted here.

s32 osEepromProbe(OSMesgQueue *mq) {
    (void) mq;
    return HLXEepromProbe();
}

s32 osEepromLongRead(OSMesgQueue *mq, u8 address, u8 *buffer, int nbytes) {
    (void) mq;
    return HLXEepromRead(address, buffer, nbytes);
}

s32 osEepromLongWrite(OSMesgQueue *mq, u8 address, u8 *buffer, int nbytes) {
    (void) mq;
    return HLXEepromWrite(address, buffer, nbytes);
}

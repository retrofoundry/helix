#include <libultra/os_cont.h>
#include <helix/internal.h>

// Different name than the reference held in helix.c
// This is because on Windows the compiler will complain about
// a redefinition of the variable.
void *_ref_gamepad_manager;

// Method called by helix.c to setup os_cont with the gamepad manager
void _osContInternalSetup(void* gamepad_manager) {
    _ref_gamepad_manager = gamepad_manager;
}

// MARK: - Methods from libultra

s32 osContInit(OSMesgQueue *mq, u8 *controller_bits, OSContStatus *status) {
    return GamepadManagerInit(_ref_gamepad_manager, controller_bits);
}

s32 osContStartReadData(OSMesgQueue *mesg) {
    GamepadManagerProcessEvents(_ref_gamepad_manager);
    return 0;
}

void osContGetReadData(OSContPad *pad) {
    return GamepadManagerGetReadData(_ref_gamepad_manager, pad);
}

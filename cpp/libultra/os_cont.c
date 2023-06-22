#include <libultra/os_cont.h>
#include <helix/internal.h>

void *_gamepad_manager;

void _osContInternalSetup(void* gamepad_manager) {
    _gamepad_manager = GamepadManagerCreate();
}

s32 osContInit(OSMesgQueue *mq, u8 *controller_bits, OSContStatus *status) {
    return GamepadManagerInit(_gamepad_manager, controller_bits);
}

s32 osContStartReadData(OSMesgQueue *mesg) {
    GamepadManagerProcessEvents(_gamepad_manager);
    return 0;
}

void osContGetReadData(OSContPad *pad) {
    return GamepadManagerGetReadData(_gamepad_manager, pad);
}

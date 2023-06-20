#include <libultra/os_cont.h>
#include <helix/internal.h>

void *_gamepad_manager;

s32 osContInit(OSMesgQueue *mq, u8 *controller_bits, OSContStatus *status) {
    if (_gamepad_manager == NULL) {
        _gamepad_manager = GamepadManagerCreate();
    }

    return GamepadManagerInit(_gamepad_manager, controller_bits);
}

s32 osContStartReadData(OSMesgQueue *mesg) {
    GamepadManagerProcessEvents(_gamepad_manager);
    return 0;
}

void osContGetReadData(OSContPad *pad) {
    return GamepadManagerGetReadData(_gamepad_manager, pad);
}

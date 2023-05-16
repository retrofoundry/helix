#include <libultra/os_cont.h>
#include <helix/gamepad.h>

void *_controller_manager;

s32 osContInit(OSMesgQueue *mq, u8 *controller_bits, OSContStatus *status) {
    if (_controller_manager == NULL) {
        _controller_manager = ControllerManagerCreate();
    }

    return ControllerManagerInit(_controller_manager, controller_bits);
}

s32 osContStartReadData(OSMesgQueue *mesg) {
    return 0;
}

void osContGetReadData(OSContPad *pad) {
    return ControllerGetReadData(_controller_manager, pad);
}
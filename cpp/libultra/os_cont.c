#include <libultra/os_cont.h>
#include <reality64/internal.h>

// Different name than the reference held in reality64.c
// This is because on Windows the compiler will complain about
// a redefinition of the variable.
void *_ref_gamepad_manager;

// Method called by reality64.c to setup os_cont with the gamepad manager
void _osContInternalSetup(void* gamepad_manager) {
    _ref_gamepad_manager = gamepad_manager;
}

// MARK: - Methods from libultra

// Runtime-active (R64RuntimeActive()) vs not — see reality64/src/gamepad/snapshot.rs.
// Runtime: this is thread5, which must NEVER touch the `!Send` GamepadManager. Input is pumped
//   and published as a plain snapshot on the winit main thread; thread5 reads the snapshot.
// Otherwise: keep the direct GamepadManager path.

s32 osContInit(OSMesgQueue *mq, u8 *controller_bits, OSContStatus *status) {
    if (R64RuntimeActive()) {
        return R64ControllerInit(controller_bits);
    }
    return GamepadManagerInit(_ref_gamepad_manager, controller_bits);
}

s32 osContStartReadData(OSMesgQueue *mesg) {
    if (R64RuntimeActive()) {
        // Runtime: the main thread already pumps input, so DROP GamepadManagerProcessEvents here.
        // Just unblock thread5's osRecvMesg(&gSIEventMesgQueue) so read_controller_inputs proceeds.
        R64EventPost(OS_EVENT_SI);
    } else {
        // Not runtime-active: sample directly on the (single) calling thread; no SI post.
        GamepadManagerProcessEvents(_ref_gamepad_manager);
    }
    return 0;
}

void osContGetReadData(OSContPad *pad) {
    if (R64RuntimeActive()) {
        R64ControllerRead(pad);
        return;
    }
    GamepadManagerGetReadData(_ref_gamepad_manager, pad);
}

// The guest waits for SI completion after starting a controller query.
s32 osContStartQuery(OSMesgQueue *mq) {
    (void) mq;
    if (R64RuntimeActive()) {
        R64EventPost(OS_EVENT_SI);
    }
    return 0;
}

void osContGetQuery(OSContStatus *data) {
    for (s32 i = 0; i < MAXCONTROLLERS; i++) {
        if (i == 0) {
            data[i].type = CONT_ABSOLUTE | CONT_JOYPORT;
            data[i].status = 0;
            data[i].err_no = 0;
        } else {
            data[i].type = 0;
            data[i].status = 0;
            data[i].err_no = CONT_NO_RESPONSE_ERROR;
        }
    }
}

s32 osContSetCh(u8 ch) {
    (void) ch; // reality64 surfaces a fixed single controller channel
    return 0;
}

// No motor device: report failure so the guest disables rumble.
struct OSPfs;
s32 osMotorInit(OSMesgQueue *ctrlrqueue, struct OSPfs *pfs, s32 channel) {
    (void) ctrlrqueue;
    (void) pfs;
    (void) channel;
    return -1;
}

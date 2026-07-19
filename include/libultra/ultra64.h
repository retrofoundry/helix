#ifndef HELIX_LIBULTRA_ULTRA64_H
#define HELIX_LIBULTRA_ULTRA64_H

// helix's canonical libultra aggregate — the guest-agnostic ABI the C shims compile against.
// Included at the helix-unique path <libultra/ultra64.h> so it never collides with a decomp's
// own <ultra64.h> (SM64/OoT keep their own; the shims use only helix's).

#include <libultra/ultratypes.h>
#include <libultra/os_thread.h>
#include <libultra/os_message.h>
#include <libultra/os_pi.h>
#include <libultra/os_cont.h>
#include <libultra/os_sp.h>
#include <libultra/os_time.h>
#include <libultra/os_misc.h>
#include <libultra/os_vi.h>
#include <libultra/rcp.h>
#include <libultra/abi.h>

#ifndef UNUSED
#define UNUSED __attribute__((unused))
#endif

#endif

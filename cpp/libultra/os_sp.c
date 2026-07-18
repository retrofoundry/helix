#include <libultra/os_sp.h>
#include <helix/internal.h>

// MARK: - Methods from libultra (RCP task engine → ultra/rcp.rs)

void osSpTaskLoad(OSTask *task) {
    // No-op: the runtime accepts the task at StartGo (Load/Yield are no-ops, per design).
    (void) task;
}

void osSpTaskStartGo(OSTask *task) {
    HLXSpTaskStartGo(task);
}

void osSpTaskYield(void) {
    // No-op: one DL is consumed to completion; nothing to preempt.
}

OSYieldResult osSpTaskYielded(OSTask *task) {
    return HLXSpTaskYielded(task);
}

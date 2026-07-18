//! ultra/rcp.rs — RCP task engine (the M_GFXTASK graphics path).
//!
//! Routes an OSTask by `OSTask.t.type` (M_GFXTASK=1, M_AUDTASK=2; include/PR/sptask.h).
//! M_GFXTASK: submit the DL `data_ptr` to the dedicated render thread over an mpsc channel,
//! block the calling guest thread HOLDING the run token until the render thread has consumed
//! the DL (fast3d begin_frame + process_dl), then post OS_EVENT_SP then OS_EVENT_DP in
//! osSendMesg tail order (never jam). M_AUDTASK is handled separately, below.

use std::os::raw::c_void;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Mutex, OnceLock};

use crate::ultra::event::{self, OS_EVENT_DP, OS_EVENT_SP};

#[cfg(not(test))]
extern "C" {
    fn hlx_acmd_process(list: *const c_void, num_cmds: i32);
}

// `cargo test` builds the Rust lib without the C audio interpreter (cpp/audio/acmd.c is compiled by the
// consumer's CMake), so stand in for its symbol — otherwise the MSVC linker rejects the undefined
// external, and no test exercises the interpreter anyway.
#[cfg(test)]
unsafe fn hlx_acmd_process(_list: *const c_void, _num_cmds: i32) {}

/// OSTask type constants (include/PR/sptask.h). This is the SINGLE canonical definition of
/// M_GFXTASK/M_AUDTASK (and `OSTaskT` below) in `rcp.rs`; the M_AUDTASK path reuses these in
/// place — it MUST NOT redefine them (a second definition in this module is an E0428 duplicate).
const M_GFXTASK: u32 = 1;
const M_AUDTASK: u32 = 2;

/// Host-ABI mirror of OSTask_t (include/PR/sptask.h). `repr(C)` reproduces exactly what the
/// host C compiler lays out for the game's OSTask, so `.type_` (off 0) and `.data_ptr` (off 88)
/// read the same bytes the game wrote. Fields other than those two are unread padding here.
#[allow(dead_code)]
#[repr(C)]
struct OSTaskT {
    type_: u32,
    flags: u32,
    ucode_boot: *mut u64,
    ucode_boot_size: u32,
    ucode: *mut u64,
    ucode_size: u32,
    ucode_data: *mut u64,
    ucode_data_size: u32,
    dram_stack: *mut u64,
    dram_stack_size: u32,
    output_buff: *mut u64,
    output_buff_size: *mut u64,
    data_ptr: *mut u64,
    data_size: u32,
    yield_data_ptr: *mut u64,
    yield_data_size: u32,
}

/// Read (type, data_ptr) from a native OSTask pointer (the game passes `&task.t`).
///
/// # Safety
/// `task` must point at a live OSTask whose layout matches `OSTaskT`.
unsafe fn read_task(task: *const c_void) -> (u32, usize) {
    let t = &*(task as *const OSTaskT);
    (t.type_, t.data_ptr as usize)
}

/// The two completion events an M_GFXTASK posts, in osSendMesg tail order: SP drains before
/// DP so the game's handle_sp_complete runs before handle_dp_complete (else handle_dp_complete
/// nulls sCurrentDisplaySPTask under handle_sp_complete). NEVER osJamMesg.
const fn gfx_completion_events() -> [i32; 2] {
    [OS_EVENT_SP, OS_EVENT_DP]
}

/// Post SP then DP to the game's gIntrMesgQueue (via the event table). Called by the guest
/// thread AFTER the render thread consumed the DL — still on the guest thread, token held.
fn post_gfx_completion() {
    for ev in gfx_completion_events() {
        event::post(ev);
    }
}

/// A message to the render thread: a display list to consume, a surface resize, or shutdown. One
/// enum so a single `recv` wakes on any of them — shutdown/resize don't depend on DL traffic.
pub(crate) enum RenderMsg {
    Gfx { data_ptr: usize, done: Sender<()> },
    Resize { width: u32, height: u32 },
    Shutdown,
}

struct GfxChannel {
    tx: Sender<RenderMsg>,
    rx: Mutex<Option<Receiver<RenderMsg>>>,
}

fn gfx() -> &'static GfxChannel {
    static GFX: OnceLock<GfxChannel> = OnceLock::new();
    GFX.get_or_init(|| {
        let (tx, rx) = channel::<RenderMsg>();
        GfxChannel {
            tx,
            rx: Mutex::new(Some(rx)),
        }
    })
}

/// The render thread takes sole ownership of the receiver at spawn. If it later dies, the receiver
/// drops and every `tx.send` returns `Err` — a fast, controlled failure instead of an infinite block.
pub(crate) fn take_render_receiver() -> Receiver<RenderMsg> {
    gfx()
        .rx
        .lock()
        .unwrap()
        .take()
        .expect("render receiver already taken")
}

/// Submit a DL and block (run token held) until the consumer consumes it. `true` on consume, `false`
/// if the render thread is gone — the caller then skips completion so a dead consumer never yields a
/// false SP→DP.
pub(crate) fn submit_and_wait(data_ptr: usize) -> bool {
    let (done_tx, done_rx) = channel::<()>();
    if gfx()
        .tx
        .send(RenderMsg::Gfx {
            data_ptr,
            done: done_tx,
        })
        .is_err()
    {
        return false;
    }
    done_rx.recv().is_ok()
}

/// Main-thread control: resize / shutdown. Best-effort — a gone render thread is fine to ignore.
pub(crate) fn send_render_control(msg: RenderMsg) {
    let _ = gfx().tx.send(msg);
}

/// osSpTaskStartGo: route the RCP task by type. M_GFXTASK → block on the render-thread consume,
/// then post SP+DP. M_AUDTASK is the in-place Acmd interpreter. Load/Yield no-ops.
#[no_mangle]
pub extern "C" fn HLXSpTaskStartGo(task: *mut c_void) {
    if task.is_null() {
        return;
    }
    let (ty, data_ptr) = unsafe { read_task(task) };
    match ty {
        M_GFXTASK => {
            if submit_and_wait(data_ptr) {
                post_gfx_completion();
            }
        }
        M_AUDTASK => {
            // In-place under the held run token: decode the Acmd list and mix.
            // A_SAVEBUFF commands write PCM straight into the game's native buffers
            // (pointers embedded in the list). data_size is in bytes; sizeof(Acmd)=16.
            let t = unsafe { &*(task as *const OSTaskT) };
            let num_cmds = (t.data_size / 16) as i32;
            unsafe { hlx_acmd_process(t.data_ptr as *const c_void, num_cmds) };
            event::post(OS_EVENT_SP);
        }
        _ => {}
    }
}

/// osSpTaskYielded → 0: the cooperative one-DL-per-frame consume is never actually interrupted.
#[no_mangle]
pub extern "C" fn HLXSpTaskYielded(_task: *mut c_void) -> i32 {
    0
}

/// Direct entry for tests/tools. `_out` is unused (A_SAVEBUFF targets are embedded
/// in the list as native pointers); `nsamples` is interpreted as the Acmd count.
#[no_mangle]
pub extern "C" fn HLXAudioProcessCommandList(alist: *mut c_void, _out: *mut i16, nsamples: i32) {
    unsafe { hlx_acmd_process(alist, nsamples) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gfx_channel_round_trip_then_gone_returns_false() {
        let rx = take_render_receiver();
        let (tx_ready, rx_ready) = channel::<()>();
        let consumer = std::thread::spawn(move || {
            match rx.recv().expect("no message") {
                RenderMsg::Gfx { data_ptr, done } => {
                    assert_eq!(data_ptr, 0xDEAD_BEEF, "data_ptr must round-trip intact");
                    done.send(()).expect("guest gone");
                }
                _ => panic!("expected Gfx"),
            }
            tx_ready.send(()).unwrap();
        });
        assert!(submit_and_wait(0xDEAD_BEEF), "consumed → true");
        rx_ready.recv().unwrap();
        consumer.join().unwrap();
        assert!(
            !submit_and_wait(0x1234),
            "receiver dropped → render gone → false, no hang"
        );
    }

    #[test]
    fn gfx_posts_sp_before_dp() {
        let seq = gfx_completion_events();
        let sp = seq.iter().position(|&e| e == OS_EVENT_SP).unwrap();
        let dp = seq.iter().position(|&e| e == OS_EVENT_DP).unwrap();
        assert!(
            sp < dp,
            "SP posts before DP (osSendMesg tail order, never jam)"
        );
    }

    #[test]
    fn ostask_layout_matches_sptask_h() {
        assert_eq!(std::mem::offset_of!(OSTaskT, type_), 0);
        assert_eq!(std::mem::offset_of!(OSTaskT, data_ptr), 88);
    }
}

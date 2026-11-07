//! RCP task engine: routes each OSTask through its guest-registered microcode handler.
//! Graphics tasks hold the guest run token until the render thread consumes the display list,
//! then post SP followed by DP. Audio and njpgdsp handlers run in place and post SP.

use std::os::raw::c_void;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Mutex, OnceLock};

use crate::ultra::event::{self, OS_EVENT_DP, OS_EVENT_SP};
use crate::ultra::rsp::{self, Handler};

use super::acmd::native::NativeContext;

const M_GFXTASK: u32 = 1;

/// Host ABI of OSTask_t in include/libultra/os_sp.h.
#[derive(Clone, Copy, Default)]
#[repr(C)]
pub(crate) struct OSTaskT {
    pub type_: u32,
    pub flags: u32,
    pub ucode_boot: *mut u64,
    pub ucode_boot_size: u32,
    pub ucode: *mut u64,
    pub ucode_size: u32,
    pub ucode_data: *mut u64,
    pub ucode_data_size: u32,
    pub dram_stack: *mut u64,
    pub dram_stack_size: u32,
    pub output_buff: *mut u64,
    pub output_buff_size: *mut u64,
    pub data_ptr: *mut u64,
    pub data_size: u32,
    pub yield_data_ptr: *mut u64,
    pub yield_data_size: u32,
}

///
/// # Safety
/// `task` must point at a live OSTask whose layout matches `OSTaskT`.
unsafe fn read_task(task: *const c_void) -> (u32, usize, usize) {
    let t = &*(task as *const OSTaskT);
    (t.type_, t.data_ptr as usize, t.ucode as usize)
}

fn skip_completion_events(task_type: u32) -> &'static [i32] {
    if task_type == M_GFXTASK {
        &[OS_EVENT_SP, OS_EVENT_DP]
    } else {
        &[OS_EVENT_SP]
    }
}

/// SP must drain before DP: the guest's DP handler may clear the current SP task.
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

/// A message to the render thread: graphics, scanout, resize, or shutdown. One
/// enum so a single `recv` wakes on any of them — shutdown/resize don't depend on DL traffic.
pub(crate) enum RenderMsg {
    Gfx {
        data_ptr: usize,
        microcode: ultragfx::Microcode,
        done: Sender<()>,
    },
    Scanout {
        address: u64,
    },
    Resize {
        width: u32,
        height: u32,
    },
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
pub(crate) fn submit_and_wait(data_ptr: usize, microcode: ultragfx::Microcode) -> bool {
    let (done_tx, done_rx) = channel::<()>();
    if gfx()
        .tx
        .send(RenderMsg::Gfx {
            data_ptr,
            microcode,
            done: done_tx,
        })
        .is_err()
    {
        return false;
    }
    done_rx.recv().is_ok()
}

/// A dropped render receiver is harmless during shutdown.
pub(crate) fn send_render_control(msg: RenderMsg) {
    let _ = gfx().tx.send(msg);
}

/// osSpTaskStartGo: run the registered microcode handler, then post completion.
#[no_mangle]
pub extern "C" fn R64SpTaskStartGo(task: *mut c_void) {
    if task.is_null() {
        return;
    }
    let (ty, data_ptr, ucode_ptr) = unsafe { read_task(task) };
    let handler = rsp::lookup(ucode_ptr).unwrap_or_else(|| {
        log::error!("unregistered RSP task: type {ty}, ucode {ucode_ptr:#x}");
        std::process::abort();
    });
    match handler {
        Handler::Gfx(microcode) => {
            if submit_and_wait(data_ptr, microcode) {
                post_gfx_completion();
            }
        }
        Handler::AudioAcmd => {
            static AUDIO: OnceLock<Mutex<NativeContext>> = OnceLock::new();
            let mut context = AUDIO
                .get_or_init(|| Mutex::new(NativeContext::default()))
                .lock()
                .unwrap();
            if let Err(error) = unsafe { context.task(task) } {
                log::error!("classic audio task failed: {error}");
                std::process::abort();
            }
            event::post(OS_EVENT_SP);
        }
        Handler::JpegNjpgdsp => {
            unsafe { super::njpgdsp::process(data_ptr as *const c_void) };
            event::post(OS_EVENT_SP);
        }
        Handler::Skip => {
            for &ev in skip_completion_events(ty) {
                event::post(ev);
            }
        }
    }
}

/// osSpTaskYielded → 0: the cooperative one-DL-per-frame consume is never actually interrupted.
#[no_mangle]
pub extern "C" fn R64SpTaskYielded(_task: *mut c_void) -> i32 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_task_preserves_data_and_ucode_host_pointers() {
        let task = OSTaskT {
            type_: M_GFXTASK,
            data_ptr: 0x1_dead_beef as *mut u64,
            ucode: 0x2_1234_5678 as *mut u64,
            ..unsafe { std::mem::zeroed() }
        };
        let fields = unsafe { read_task(&task as *const OSTaskT as *const c_void) };
        assert_eq!(fields, (M_GFXTASK, 0x1_dead_beef, 0x2_1234_5678));
    }

    #[test]
    fn gfx_channel_round_trip_then_gone_returns_false() {
        let rx = take_render_receiver();
        let (tx_ready, rx_ready) = channel::<()>();
        let consumer = std::thread::spawn(move || {
            match rx.recv().expect("no message") {
                RenderMsg::Gfx {
                    data_ptr,
                    microcode,
                    done,
                } => {
                    assert_eq!(data_ptr, 0xDEAD_BEEF, "data_ptr must round-trip intact");
                    assert_eq!(microcode, ultragfx::Microcode::S2dex2);
                    done.send(()).expect("guest gone");
                }
                _ => panic!("expected Gfx"),
            }
            tx_ready.send(()).unwrap();
        });
        let binding = rsp::R64RspBinding {
            ucode: 0x1_1234_5678 as *const c_void,
            handler: 4,
        };
        assert_eq!(rsp::R64RspRegister(&binding, 1), 0);
        let mut task = OSTaskT {
            type_: 99,
            data_ptr: 0xDEAD_BEEF as *mut u64,
            ucode: 0x9234_5678 as *mut u64,
            ..unsafe { std::mem::zeroed() }
        };
        R64SpTaskStartGo(&mut task as *mut OSTaskT as *mut c_void);
        rx_ready.recv().unwrap();
        consumer.join().unwrap();
        assert!(
            !submit_and_wait(0x1234, ultragfx::Microcode::F3d),
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
    fn skip_completion_follows_task_type() {
        assert_eq!(
            skip_completion_events(M_GFXTASK),
            &[OS_EVENT_SP, OS_EVENT_DP]
        );
        for task_type in [2, 4, 99] {
            assert_eq!(skip_completion_events(task_type), &[OS_EVENT_SP]);
        }
    }

    #[test]
    fn ostask_layout_matches_sptask_h() {
        assert_eq!(std::mem::size_of::<OSTaskT>(), 0x78);
        assert_eq!(std::mem::offset_of!(OSTaskT, ucode_data), 0x28);
        assert_eq!(std::mem::offset_of!(OSTaskT, ucode_data_size), 0x30);
        assert_eq!(std::mem::offset_of!(OSTaskT, data_size), 0x60);
        assert_eq!(std::mem::offset_of!(OSTaskT, type_), 0);
        assert_eq!(std::mem::offset_of!(OSTaskT, ucode), 24);
        assert_eq!(std::mem::offset_of!(OSTaskT, data_ptr), 88);
    }
}

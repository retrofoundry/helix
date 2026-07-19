//! Synchronous PI (parallel interface) DMA: `HLXPiStartDma` performs a bounded memmove
//! and immediately posts the OSIoMesg completion to the caller's retQueue. No worker
//! thread: sm64's DMA drain loops (heap.c:1160 audio DMA poll, load.c:114) rely on the
//! completion being synchronous with the call, not delivered asynchronously later.

use std::os::raw::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};

use crate::ultra::mesg;

const OS_READ: i32 = 0; // device -> RDRAM (include/PR/os_pi.h)
const OS_WRITE: i32 = 1; // RDRAM -> device

#[no_mangle]
pub extern "C" fn HLXPiStartDma(
    mb: *mut c_void,
    dir: i32,
    dev_addr: usize,
    v_addr: *mut c_void,
    nbytes: usize,
    mq: *mut c_void,
) -> i32 {
    // Panic-safe boundary: any panic — including a poisoned scheduler/queue lock reached through the
    // completion wake inside post_with — becomes -1 instead of unwinding across `extern "C"` (UB).
    catch_unwind(AssertUnwindSafe(|| {
        // Validate the C pointers and the copy range before any deref (ptr::copy requires
        // size <= isize::MAX and a non-wrapping range).
        if mb.is_null() || mq.is_null() || nbytes > isize::MAX as usize {
            return -1;
        }
        // Only device<->RDRAM: an unknown direction must not silently reverse the copy.
        if dir != OS_READ && dir != OS_WRITE {
            return -1;
        }
        if nbytes > 0
            && (v_addr.is_null()
                || dev_addr == 0
                || dev_addr.checked_add(nbytes).is_none()
                || (v_addr as usize).checked_add(nbytes).is_none())
        {
            return -1;
        }
        let dev = dev_addr as *mut u8;
        let v = v_addr as *mut u8;
        // Fill-then-post is one atomic queue-lock section: the memmove runs ONLY with a slot
        // reserved, and the completion post that follows cannot fail. So there is no
        // copy-without-completion, and a full/missing/poisoned retQueue leaves no side effect.
        let rc = mesg::post_with(mq as usize, mb as usize, || {
            if nbytes > 0 {
                // memmove (copy), not memcpy: tolerate a src/dst overlap rather than risk UB.
                unsafe {
                    if dir == OS_READ {
                        std::ptr::copy(dev as *const u8, v, nbytes);
                    } else {
                        std::ptr::copy(v as *const u8, dev, nbytes);
                    }
                }
            }
        });
        if rc != 0 {
            return -1; // no copy happened
        }
        0
    }))
    .unwrap_or(-1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ultra::mesg;
    use crate::ultra::mesg::{HLXMesgQueueCreate, HLXMesgRecv};
    use std::ffi::c_void;
    use std::ptr;

    // Leak unique queue/mesg storage so parallel tests can't alias each other's registrations via
    // reused stack addresses.
    fn fresh_queue(cap: usize) -> *mut c_void {
        let backing = Box::leak(Box::new([0u8; 64])).as_mut_ptr() as *mut c_void;
        let slots: &'static mut [*mut c_void] =
            Box::leak(vec![ptr::null_mut::<c_void>(); cap].into_boxed_slice());
        HLXMesgQueueCreate(backing, slots.as_mut_ptr(), cap as i32);
        backing
    }
    fn fresh_mb() -> *mut c_void {
        Box::leak(Box::new([0u8; 16])).as_mut_ptr() as *mut c_void
    }

    #[test]
    fn read_copies_and_posts_completion() {
        let src = [1u8, 2, 3, 4];
        let mut dst = [0u8; 4];
        let mq = fresh_queue(1);
        let mb = fresh_mb();
        let r = HLXPiStartDma(mb, 0, src.as_ptr() as usize, dst.as_mut_ptr() as *mut c_void, 4, mq);
        assert_eq!(r, 0);
        assert_eq!(dst, src);
        let (ret, m) = mesg::recv(mq as usize, 0);
        assert_eq!(ret, 0);
        assert_eq!(m, mb as usize); // completion carries the OSIoMesg pointer
    }

    #[test]
    fn dma_copies_exactly_nbytes() {
        let mq = fresh_queue(4);
        let src: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
        let mut dst: [u8; 8] = [0xFF; 8];
        let mb = fresh_mb();
        let rc = HLXPiStartDma(mb, 0, src.as_ptr() as usize, dst.as_mut_ptr() as *mut c_void, 4, mq);
        assert_eq!(rc, 0);
        assert_eq!(&dst[..4], &src[..4]);
        assert_eq!(&dst[4..], &[0xFF; 4], "copied exactly nbytes");
        let mut out: *mut c_void = ptr::null_mut();
        assert_eq!(HLXMesgRecv(mq, &mut out, 0), 0);
        assert_eq!(out, mb);
    }

    #[test]
    fn full_retqueue_errors_without_side_effect() {
        // cap-1, pre-filled: HLXPiStartDma reports -1 AND does not copy (checked before the copy).
        let mq = fresh_queue(1);
        assert_eq!(mesg::send(mq as usize, 0x1, 0), 0); // fill the slot
        let src = [1u8, 2, 3, 4];
        let mut dst = [9u8; 4]; // sentinel
        let r = HLXPiStartDma(fresh_mb(), 0, src.as_ptr() as usize, dst.as_mut_ptr() as *mut c_void, 4, mq);
        assert_eq!(r, -1);
        assert_eq!(dst, [9u8; 4], "no copy on a full retQueue (no side effect)");
    }

    #[test]
    fn missing_retqueue_errors_no_panic() {
        let fake = Box::leak(Box::new([0u8; 64])).as_mut_ptr() as *mut c_void; // never created
        let src = [1u8; 4];
        let mut dst = [0u8; 4];
        assert_eq!(
            HLXPiStartDma(fresh_mb(), 0, src.as_ptr() as usize, dst.as_mut_ptr() as *mut c_void, 4, fake),
            -1
        );
    }

    #[test]
    fn null_and_invalid_pointers_error() {
        let mq = fresh_queue(1);
        let mb = fresh_mb();
        let dummy = 1usize as *mut c_void; // non-null; guards return before any deref
        assert_eq!(HLXPiStartDma(ptr::null_mut(), 0, 1, dummy, 4, mq), -1, "null mb");
        assert_eq!(HLXPiStartDma(mb, 0, 1, dummy, 4, ptr::null_mut()), -1, "null mq");
        assert_eq!(HLXPiStartDma(mb, 0, 1, ptr::null_mut(), 4, mq), -1, "null v_addr, nbytes>0");
        let mut d = [0u8; 4];
        assert_eq!(HLXPiStartDma(mb, 0, 0, d.as_mut_ptr() as *mut c_void, 4, mq), -1, "zero dev_addr, nbytes>0");
    }

    #[test]
    fn oversize_and_overflow_error() {
        let mq = fresh_queue(1);
        let mb = fresh_mb();
        let mut d = [0u8; 4];
        let dp = d.as_mut_ptr() as *mut c_void;
        assert_eq!(HLXPiStartDma(mb, 0, 1, dp, isize::MAX as usize + 1, mq), -1, "nbytes > isize::MAX");
        assert_eq!(HLXPiStartDma(mb, 0, usize::MAX, dp, 4, mq), -1, "dev_addr + nbytes wraps");
    }

    #[test]
    fn overlapping_copy_uses_memmove() {
        // dst overlaps src (shift by 1): memmove yields the shifted result; memcpy would corrupt.
        let mq = fresh_queue(1);
        let mut buf = [1u8, 2, 3, 4, 0];
        let base = buf.as_mut_ptr();
        let src = base as usize; // reads [1,2,3,4]
        let dst = unsafe { base.add(1) } as *mut c_void; // writes buf[1..5], overlaps src
        let r = HLXPiStartDma(fresh_mb(), 0, src, dst, 4, mq);
        assert_eq!(r, 0);
        assert_eq!(buf, [1u8, 1, 2, 3, 4], "memmove shifted correctly");
    }

    #[test]
    fn zero_size_is_noop_but_completes() {
        // nbytes == 0: no copy (null dev/dst tolerated), completion still posted.
        let mq = fresh_queue(1);
        let mb = fresh_mb();
        assert_eq!(HLXPiStartDma(mb, 0, 0, ptr::null_mut(), 0, mq), 0);
        let (ret, m) = mesg::recv(mq as usize, 0);
        assert_eq!(ret, 0);
        assert_eq!(m, mb as usize);
    }

    #[test]
    fn write_direction_copies_ram_to_device() {
        // OS_WRITE (dir 1): RDRAM -> device.
        let mut dev = [0u8; 4];
        let src = [9u8, 8, 7, 6];
        let mq = fresh_queue(1);
        let mb = fresh_mb();
        let r = HLXPiStartDma(mb, 1, dev.as_mut_ptr() as usize, src.as_ptr() as *mut c_void, 4, mq);
        assert_eq!(r, 0);
        assert_eq!(dev, src, "write copied RDRAM into the device buffer");
        assert_eq!(mesg::recv(mq as usize, 0), (0, mb as usize));
    }

    #[test]
    fn invalid_direction_rejected_no_side_effect() {
        // Any direction other than OS_READ/OS_WRITE must be rejected, not silently reverse the copy.
        let src = [1u8, 2, 3, 4];
        let mut dst = [0u8; 4];
        let mq = fresh_queue(1);
        assert_eq!(
            HLXPiStartDma(fresh_mb(), 2, src.as_ptr() as usize, dst.as_mut_ptr() as *mut c_void, 4, mq),
            -1
        );
        assert_eq!(dst, [0u8; 4], "no copy on an invalid direction");
        assert_eq!(mesg::recv(mq as usize, 0).0, -1, "no completion on an invalid direction");
    }
}

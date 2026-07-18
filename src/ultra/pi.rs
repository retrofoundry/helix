//! Synchronous PI (parallel interface) DMA: `HLXPiStartDma` performs a bounded memcpy
//! and immediately posts the OSIoMesg completion to the caller's retQueue. No worker
//! thread: sm64's DMA drain loops (heap.c:1160 audio DMA poll, load.c:114) rely on the
//! completion being synchronous with the call, not delivered asynchronously later.

use std::os::raw::c_void;

use crate::ultra::mesg;

const OS_READ: i32 = 0; // device -> RDRAM (include/PR/os_pi.h)

#[no_mangle]
pub extern "C" fn HLXPiStartDma(
    mb: *mut c_void,
    dir: i32,
    dev_addr: usize,
    v_addr: *mut c_void,
    nbytes: usize,
    mq: *mut c_void,
) -> i32 {
    let dev = dev_addr as *mut u8;
    let v = v_addr as *mut u8;
    unsafe {
        if dir == OS_READ {
            std::ptr::copy_nonoverlapping(dev as *const u8, v, nbytes);
        } else {
            std::ptr::copy_nonoverlapping(v as *const u8, dev, nbytes);
        }
    }
    // Immediate completion: post the OSIoMesg to its retQueue (tail, NOBLOCK).
    mesg::send(mq as usize, mb as usize, 0);
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ultra::mesg;
    use crate::ultra::mesg::{HLXMesgQueueCreate, HLXMesgRecv};
    use std::ffi::c_void;
    use std::ptr;

    #[test]
    fn read_copies_and_posts_completion() {
        let src = [1u8, 2, 3, 4];
        let mut dst = [0u8; 4];

        let mut backing = [0u8; 64];
        let mq = backing.as_mut_ptr() as *mut std::os::raw::c_void;
        let mut msgbuf: [*mut std::os::raw::c_void; 1] = [ptr::null_mut()];
        mesg::HLXMesgQueueCreate(mq, msgbuf.as_mut_ptr(), 1);

        let mut mb = [0u8; 16];
        let mbp = mb.as_mut_ptr() as *mut std::os::raw::c_void;

        let r = HLXPiStartDma(
            mbp,
            0, // OS_READ
            src.as_ptr() as usize,
            dst.as_mut_ptr() as *mut std::os::raw::c_void,
            4,
            mq,
        );
        assert_eq!(r, 0);
        assert_eq!(dst, src);

        let (ret, m) = mesg::recv(mq as usize, 0);
        assert_eq!(ret, 0);
        assert_eq!(m, mbp as usize); // completion carries the OSIoMesg pointer
    }

    #[test]
    #[allow(unused_unsafe)]
    fn dma_copies_exactly_nbytes_and_posts_completion() {
        let mq = Box::into_raw(Box::new(0u64)) as *mut c_void;
        let mut qbuf = [std::ptr::null_mut::<c_void>(); 4];
        unsafe { HLXMesgQueueCreate(mq, qbuf.as_mut_ptr(), 4) };

        let src: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
        let mut dst: [u8; 8] = [0xFF; 8];
        let mb = Box::into_raw(Box::new(0u64)) as *mut c_void;

        let rc = unsafe {
            HLXPiStartDma(
                mb,
                /*OS_READ*/ 0,
                src.as_ptr() as usize,
                dst.as_mut_ptr() as *mut c_void,
                4,
                mq,
            )
        };
        assert_eq!(rc, 0);
        assert_eq!(&dst[..4], &src[..4]);
        assert_eq!(&dst[4..], &[0xFF; 4]);

        let mut out: *mut c_void = std::ptr::null_mut();
        assert_eq!(unsafe { HLXMesgRecv(mq, &mut out, 0) }, 0);
        assert_eq!(out, mb);
    }
}

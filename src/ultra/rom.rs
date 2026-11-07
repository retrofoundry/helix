//! Cart payloads contain host-endian guest bytes. DMA and PIO copy them without swapping.

use std::os::raw::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Mutex, OnceLock};

use crate::ultra::mesg;

// R64Status — mirror of the C `int32_t` constants (include/reality64/internal.h).
pub const R64_OK: i32 = 0;
pub const R64_ERR_NO_ROM: i32 = -1;
pub const R64_ERR_RANGE: i32 = -2;
pub const R64_ERR_UNSUPPORTED_DEVICE: i32 = -3;
pub const R64_ERR_BAD_ARG: i32 = -4;
pub const R64_ERR_QUEUE: i32 = -5;
pub const R64_ERR_BAD_IMAGE: i32 = -6;
pub const R64_ERR_INTERNAL: i32 = -7;

// R64DevKind values in include/reality64/internal.h.
pub const R64_DEV_CART: i32 = 0;
pub const R64_DEV_DRIVE: i32 = 1;
pub const R64_DEV_SRAM: i32 = 2;
pub const R64_DEV_DEBUG: i32 = 3;

const OS_READ: i32 = 0;
const ROM_BASE: u32 = 0x1000_0000;
const PHYS_MASK: u32 = 0x1FFF_FFFF;

// Container format v1: a 48-byte fixed little-endian header, then the VROM payload.
const MAGIC: [u8; 8] = *b"R64NROM\0";
const HEADER_LEN: usize = 48;
const FORMAT_VERSION: u32 = 1;
const MAX_PAYLOAD_BYTES: usize = 64 * 1024 * 1024; // cart images are <= 64 MiB
const MAX_CONTAINER_BYTES: usize = HEADER_LEN + MAX_PAYLOAD_BYTES;
// ABI id 0 denotes an unstamped native image.
const EXPECTED_ABI_ID: u32 = 0;
const EXPECTED_PTR_WIDTH: u8 = std::mem::size_of::<usize>() as u8;

#[derive(Debug)]
pub struct Rom {
    payload: Vec<u8>,
}

impl Rom {
    /// Validates the container header and copies its declared host-endian payload.
    pub fn from_container(bytes: &[u8]) -> Result<Rom, i32> {
        if bytes.len() < HEADER_LEN || bytes.len() > MAX_CONTAINER_BYTES {
            return Err(R64_ERR_BAD_IMAGE);
        }
        if bytes[0..8] != MAGIC {
            return Err(R64_ERR_BAD_IMAGE);
        }
        let u32at = |o: usize| u32::from_le_bytes(bytes[o..o + 4].try_into().unwrap());
        let u64at = |o: usize| u64::from_le_bytes(bytes[o..o + 8].try_into().unwrap());
        let format_version = u32at(8);
        let header_len = u32at(12);
        let payload_offset = u64at(16);
        let vrom_len = u64at(24);
        let guest_abi_id = u32at(32);
        let endianness = bytes[36];
        let pointer_width = bytes[37];
        let flags = u16::from_le_bytes(bytes[38..40].try_into().unwrap());
        // Digest verification is unsupported; reject images that claim a digest.
        let digest_zero = bytes[40..48] == [0u8; 8];

        if format_version != FORMAT_VERSION
            || header_len as usize != HEADER_LEN
            || endianness != 0 // 0 = host little-endian
            || flags != 0
            || pointer_width != EXPECTED_PTR_WIDTH
            || guest_abi_id != EXPECTED_ABI_ID
            || !digest_zero
        {
            return Err(R64_ERR_BAD_IMAGE);
        }

        let po: usize = payload_offset.try_into().map_err(|_| R64_ERR_BAD_IMAGE)?;
        let vl: usize = vrom_len.try_into().map_err(|_| R64_ERR_BAD_IMAGE)?;
        if po < HEADER_LEN || vl > MAX_PAYLOAD_BYTES {
            return Err(R64_ERR_BAD_IMAGE);
        }
        let end = po.checked_add(vl).ok_or(R64_ERR_BAD_IMAGE)?;
        if end > bytes.len() {
            return Err(R64_ERR_BAD_IMAGE);
        }
        Ok(Rom {
            payload: bytes[po..end].to_vec(),
        })
    }

    fn slice(&self, vrom: u32, size: usize) -> Result<&[u8], i32> {
        let start = vrom as usize;
        let end = start.checked_add(size).ok_or(R64_ERR_RANGE)?;
        self.payload.get(start..end).ok_or(R64_ERR_RANGE)
    }
}

fn rom_slot() -> &'static Mutex<Option<Rom>> {
    static SLOT: OnceLock<Mutex<Option<Rom>>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(None))
}

#[cfg(test)]
fn rom_slot_reset() {
    if let Ok(mut g) = rom_slot().lock() {
        *g = None;
    }
}

fn cart_vrom(base: u32, dev_addr: u32) -> Result<u32, i32> {
    let phys = (base | dev_addr) & PHYS_MASK;
    phys.checked_sub(ROM_BASE).ok_or(R64_ERR_RANGE)
}

fn cart_dma(
    rom: &Rom,
    mb: usize,
    base: u32,
    dev_addr: u32,
    dram: *mut u8,
    size: usize,
    ret_queue: usize,
) -> i32 {
    let vrom = match cart_vrom(base, dev_addr) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let src = match rom.slice(vrom, size) {
        Ok(s) => s.as_ptr(),
        Err(e) => return e,
    };
    if size > 0 && dram.is_null() {
        return R64_ERR_BAD_ARG;
    }
    match mesg::post_with(ret_queue, mb, || {
        if size > 0 {
            unsafe { std::ptr::copy(src, dram, size) };
        }
    }) {
        -2 => R64_ERR_QUEUE,
        _ => R64_OK,
    }
}

fn cart_read_io(rom: &Rom, base: u32, dev_addr: u32) -> Result<u32, i32> {
    let vrom = cart_vrom(base, dev_addr)?;
    let b: [u8; 4] = rom.slice(vrom, 4)?.try_into().unwrap();
    Ok(u32::from_ne_bytes(b))
}

/// Cart DMA: unsupported devices post no completion; panics return R64_ERR_INTERNAL.
#[no_mangle]
pub extern "C" fn R64EPiDma(
    mb: *mut c_void,
    kind: i32,
    base: u32,
    dev_addr: u32,
    dram: *mut c_void,
    size: usize,
    dir: i32,
    ret_queue: *mut c_void,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        if kind != R64_DEV_CART {
            return R64_ERR_UNSUPPORTED_DEVICE;
        }
        if mb.is_null() || ret_queue.is_null() {
            return R64_ERR_BAD_ARG;
        }
        if dir != OS_READ {
            return R64_ERR_UNSUPPORTED_DEVICE; // cart is read-only
        }
        if size > isize::MAX as usize {
            return R64_ERR_RANGE;
        }
        let slot = match rom_slot().lock() {
            Ok(g) => g,
            Err(_) => return R64_ERR_INTERNAL,
        };
        match slot.as_ref() {
            Some(rom) => cart_dma(
                rom,
                mb as usize,
                base,
                dev_addr,
                dram as *mut u8,
                size,
                ret_queue as usize,
            ),
            None => R64_ERR_NO_ROM,
        }
    }))
    .unwrap_or(R64_ERR_INTERNAL)
}

/// `osEPiReadIo` cart path. `*out` is zeroed before any early return. Cart only. Panic-safe.
#[no_mangle]
pub extern "C" fn R64EPiReadIo(kind: i32, base: u32, dev_addr: u32, out: *mut u32) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        if out.is_null() {
            return R64_ERR_BAD_ARG;
        }
        unsafe { *out = 0 };
        if kind != R64_DEV_CART {
            return R64_ERR_UNSUPPORTED_DEVICE;
        }
        let slot = match rom_slot().lock() {
            Ok(g) => g,
            Err(_) => return R64_ERR_INTERNAL,
        };
        let rom = match slot.as_ref() {
            Some(r) => r,
            None => return R64_ERR_NO_ROM,
        };
        match cart_read_io(rom, base, dev_addr) {
            Ok(v) => {
                unsafe { *out = v };
                R64_OK
            }
            Err(e) => e,
        }
    }))
    .unwrap_or(R64_ERR_INTERNAL)
}

/// Install a validated native cart image once, before any guest DMA. Copies the payload (caller
/// keeps the buffer). Duplicate load is an error. Panic-safe.
#[no_mangle]
pub extern "C" fn R64RomLoad(ptr: *const u8, len: usize) -> i32 {
    catch_unwind(AssertUnwindSafe(|| {
        if ptr.is_null() || len == 0 || len > MAX_CONTAINER_BYTES {
            return R64_ERR_BAD_ARG;
        }
        let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
        let rom = match Rom::from_container(bytes) {
            Ok(r) => r,
            Err(e) => return e,
        };
        let mut slot = match rom_slot().lock() {
            Ok(g) => g,
            Err(_) => return R64_ERR_INTERNAL,
        };
        if slot.is_some() {
            return R64_ERR_BAD_ARG;
        }
        *slot = Some(rom);
        R64_OK
    }))
    .unwrap_or(R64_ERR_INTERNAL)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ultra::mesg;
    use std::ptr;

    fn container(payload: &[u8]) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(b"R64NROM\0");
        v.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
        v.extend_from_slice(&(HEADER_LEN as u32).to_le_bytes());
        v.extend_from_slice(&(HEADER_LEN as u64).to_le_bytes()); // payload_offset
        v.extend_from_slice(&(payload.len() as u64).to_le_bytes()); // vrom_len
        v.extend_from_slice(&0u32.to_le_bytes()); // guest_abi_id
        v.push(0); // endianness = host LE
        v.push(EXPECTED_PTR_WIDTH); // pointer_width
        v.extend_from_slice(&0u16.to_le_bytes()); // flags
        v.extend_from_slice(&0u64.to_le_bytes()); // reserved digest
        assert_eq!(v.len(), HEADER_LEN);
        v.extend_from_slice(payload);
        v
    }

    fn rom(payload: &[u8]) -> Rom {
        Rom::from_container(&container(payload)).unwrap()
    }

    fn fresh_queue(cap: usize) -> usize {
        let backing = Box::leak(Box::new([0u8; 64])).as_mut_ptr() as *mut std::os::raw::c_void;
        let slots: &'static mut [*mut std::os::raw::c_void] =
            Box::leak(vec![ptr::null_mut::<std::os::raw::c_void>(); cap].into_boxed_slice());
        mesg::R64MesgQueueCreate(backing, slots.as_mut_ptr(), cap as i32);
        backing as usize
    }

    fn fresh_mb() -> usize {
        Box::into_raw(Box::new(0u64)) as usize
    }

    // Prevent parallel tests from resetting the ROM while another test uses it.
    static GLOBAL_SLOT_TEST: Mutex<()> = Mutex::new(());

    #[test]
    fn container_accepts_valid_and_rejects_bad() {
        assert!(Rom::from_container(&container(&[1, 2, 3, 4])).is_ok());
        assert_eq!(
            Rom::from_container(&[0x80, 0x37, 0x12, 0x40, 0, 0, 0, 0, 0, 0]).unwrap_err(),
            R64_ERR_BAD_IMAGE
        );
        // bad version
        let mut c = container(&[1, 2, 3, 4]);
        c[8] = 2;
        assert_eq!(Rom::from_container(&c).unwrap_err(), R64_ERR_BAD_IMAGE);
        // payload_offset + vrom_len past end
        let mut c = container(&[1, 2, 3, 4]);
        c[24] = 0xFF; // vrom_len huge
        assert_eq!(Rom::from_container(&c).unwrap_err(), R64_ERR_BAD_IMAGE);
        // nonzero flags
        let mut c = container(&[1, 2, 3, 4]);
        c[38] = 1;
        assert_eq!(Rom::from_container(&c).unwrap_err(), R64_ERR_BAD_IMAGE);
    }

    #[test]
    fn cart_dma_reads_offset_and_completes() {
        let r = rom(&[0, 0, 0, 0, 0xAA, 0xBB, 0xCC, 0xDD]);
        let q = fresh_queue(1);
        let mb = 0x1234usize;
        let mut dst = [0u8; 4];
        let rc = cart_dma(&r, mb, 0xB000_0000, 0x1000_0004, dst.as_mut_ptr(), 4, q);
        assert_eq!(rc, R64_OK);
        assert_eq!(dst, [0xAA, 0xBB, 0xCC, 0xDD]);
        let (ret, m) = mesg::recv(q, 0);
        assert_eq!(ret, 0);
        assert_eq!(m, mb);
    }

    #[test]
    fn cart_dma_bounds_reject_without_side_effect() {
        let r = rom(&[1, 2, 3, 4]);
        let q = fresh_queue(1);
        let mut dst = [9u8; 8];
        assert_eq!(
            cart_dma(&r, 1, 0xB000_0000, 0x1000_0000, dst.as_mut_ptr(), 8, q),
            R64_ERR_RANGE
        );
        assert_eq!(dst, [9u8; 8], "no copy on out-of-range");
        assert_eq!(
            cart_dma(&r, 1, 0, 0x0000_0000, dst.as_mut_ptr(), 4, q),
            R64_ERR_RANGE
        );
    }

    #[test]
    fn cart_dma_full_queue_still_copies() {
        let r = rom(&[0xAA, 0xBB, 0xCC, 0xDD]);
        let q = fresh_queue(1);
        assert_eq!(mesg::send(q, 0x1, 0), 0);
        let mut dst = [0u8; 4];
        assert_eq!(
            cart_dma(&r, 1, 0xB000_0000, 0x1000_0000, dst.as_mut_ptr(), 4, q),
            R64_OK
        );
        assert_eq!(dst, [0xAA, 0xBB, 0xCC, 0xDD]);
    }

    #[test]
    fn read_io_returns_bytes_verbatim() {
        let r = rom(&[0x11, 0x22, b'E', 0x44, 0x78, 0x56, 0x34, 0x12]);
        let w0 = cart_read_io(&r, 0xB000_0000, 0x1000_0000).unwrap();
        assert_eq!(w0.to_ne_bytes(), [0x11, 0x22, b'E', 0x44]);
        let w1 = cart_read_io(&r, 0xB000_0000, 0x1000_0004).unwrap();
        assert_eq!(w1.to_ne_bytes(), [0x78, 0x56, 0x34, 0x12]);
        assert_eq!(
            cart_read_io(&r, 0xB000_0000, 0x1000_0008).unwrap_err(),
            R64_ERR_RANGE
        );
    }

    #[test]
    fn epi_non_cart_kind_is_unsupported_and_zeroes_out() {
        let mut out: u32 = 0xDEAD_BEEF;
        assert_eq!(
            R64EPiReadIo(R64_DEV_DRIVE, 0xB000_0000, 0x1000_0000, &mut out),
            R64_ERR_UNSUPPORTED_DEVICE
        );
        assert_eq!(out, 0);
        let q = fresh_queue(1);
        assert_eq!(
            R64EPiDma(
                1 as *mut _,
                R64_DEV_DRIVE,
                0xB000_0000,
                0x1000_0000,
                ptr::null_mut(),
                0,
                0,
                q as *mut _
            ),
            R64_ERR_UNSUPPORTED_DEVICE
        );
        assert_eq!(
            mesg::recv(q, 0).0,
            -1,
            "unsupported device posts no completion"
        );
    }

    #[test]
    fn epi_readio_null_out_is_bad_arg() {
        assert_eq!(
            R64EPiReadIo(R64_DEV_CART, 0xB000_0000, 0x1000_0000, ptr::null_mut()),
            R64_ERR_BAD_ARG
        );
    }

    #[test]
    fn container_rejects_bad_fields() {
        let mut c = container(&[1, 2, 3, 4]);
        c[0] ^= 0xFF;
        assert_eq!(
            Rom::from_container(&c).unwrap_err(),
            R64_ERR_BAD_IMAGE,
            "bad magic (full header)"
        );
        let mut c = container(&[1, 2, 3, 4]);
        c[32] = 1;
        assert_eq!(
            Rom::from_container(&c).unwrap_err(),
            R64_ERR_BAD_IMAGE,
            "wrong ABI id"
        );
        let mut c = container(&[1, 2, 3, 4]);
        c[36] = 1;
        assert_eq!(
            Rom::from_container(&c).unwrap_err(),
            R64_ERR_BAD_IMAGE,
            "non-LE"
        );
        let mut c = container(&[1, 2, 3, 4]);
        c[37] = EXPECTED_PTR_WIDTH ^ 0x04;
        assert_eq!(
            Rom::from_container(&c).unwrap_err(),
            R64_ERR_BAD_IMAGE,
            "wrong ptr width"
        );
        let mut c = container(&[1, 2, 3, 4]);
        c[40] = 1;
        assert_eq!(
            Rom::from_container(&c).unwrap_err(),
            R64_ERR_BAD_IMAGE,
            "nonzero digest"
        );
        let mut c = container(&[1, 2, 3, 4]);
        c[16] = 8; // po = 8 < HEADER_LEN
        assert_eq!(
            Rom::from_container(&c).unwrap_err(),
            R64_ERR_BAD_IMAGE,
            "payload_offset < header"
        );
    }

    #[test]
    fn cart_dma_zero_len_still_validates_address() {
        let r = rom(&[1, 2, 3, 4]);
        let q = fresh_queue(1);
        assert_eq!(
            cart_dma(&r, 1, 0, 0x0000_0000, ptr::null_mut(), 0, q),
            R64_ERR_RANGE
        );
        assert_eq!(
            cart_dma(&r, 1, 0xB000_0000, 0x1000_0005, ptr::null_mut(), 0, q),
            R64_ERR_RANGE
        );
        assert_eq!(
            mesg::recv(q, 0).0,
            -1,
            "a rejected zero-length DMA posts no completion"
        );
        assert_eq!(
            cart_dma(&r, 0x55, 0xB000_0000, 0x1000_0000, ptr::null_mut(), 0, q),
            R64_OK
        );
        assert_eq!(mesg::recv(q, 0), (0, 0x55));
    }

    #[test]
    fn ffi_global_slot_roundtrip() {
        let _guard = GLOBAL_SLOT_TEST.lock().unwrap();
        rom_slot_reset();
        let mut out: u32 = 0xDEAD_BEEF;
        assert_eq!(
            R64EPiReadIo(R64_DEV_CART, 0xB000_0000, 0x1000_0000, &mut out),
            R64_ERR_NO_ROM
        );
        assert_eq!(out, 0);

        let c = container(&[0, 0, 0, 0, 0xAA, 0xBB, 0xCC, 0xDD]);
        assert_eq!(R64RomLoad(c.as_ptr(), c.len()), R64_OK);
        assert_eq!(
            R64RomLoad(c.as_ptr(), c.len()),
            R64_ERR_BAD_ARG,
            "duplicate load is rejected"
        );

        assert_eq!(
            R64EPiReadIo(R64_DEV_CART, 0xB000_0000, 0x1000_0004, &mut out),
            R64_OK
        );
        assert_eq!(out.to_ne_bytes(), [0xAA, 0xBB, 0xCC, 0xDD]);

        let q = fresh_queue(1);
        let mb = fresh_mb();
        let mut dst = [0u8; 4];
        assert_eq!(
            R64EPiDma(
                mb as *mut c_void,
                R64_DEV_CART,
                0xB000_0000,
                0x1000_0004,
                dst.as_mut_ptr() as *mut c_void,
                4,
                OS_READ,
                q as *mut c_void,
            ),
            R64_OK
        );
        assert_eq!(dst, [0xAA, 0xBB, 0xCC, 0xDD]);
        assert_eq!(mesg::recv(q, 0), (0, mb));

        assert_eq!(
            R64EPiDma(
                mb as *mut c_void,
                R64_DEV_CART,
                0xB000_0000,
                0x1000_0004,
                dst.as_mut_ptr() as *mut c_void,
                4,
                OS_READ + 1,
                q as *mut c_void,
            ),
            R64_ERR_UNSUPPORTED_DEVICE
        );
        assert_eq!(
            mesg::recv(q, 0).0,
            -1,
            "unsupported write posts no completion"
        );
        rom_slot_reset();
    }
}

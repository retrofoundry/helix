use std::ops::Range;
use std::sync::Mutex;

static RANGES: Mutex<Vec<Range<u64>>> = Mutex::new(Vec::new());

pub(crate) fn ranges() -> Vec<Range<u64>> {
    RANGES.lock().unwrap().clone()
}

/// Registers live guest memory for checked display-list reads and write-back until process exit.
#[no_mangle]
pub extern "C" fn R64RegisterGuestMemory(start: *mut libc::c_void, size: usize) {
    register(&mut RANGES.lock().unwrap(), start as u64, size as u64);
}

fn register(ranges: &mut Vec<Range<u64>>, start: u64, size: u64) {
    let Some(end) = start.checked_add(size).filter(|_| start != 0 && size != 0) else {
        log::error!("invalid guest memory range: {start:#x} + {size:#x}");
        return;
    };
    if ranges.is_empty() {
        ranges.extend(image_ranges());
    }
    ranges.push(start..end);
}

#[cfg(target_os = "macos")]
fn image_ranges() -> Vec<Range<u64>> {
    extern "C" {
        fn _dyld_get_image_header(index: u32) -> *const u8;
        fn _dyld_get_image_vmaddr_slide(index: u32) -> isize;
    }
    const LC_SEGMENT_64: u32 = 0x19;
    const VM_PROT_READ: u32 = 1;
    let mut ranges = Vec::new();
    // mach_header_64 is 32 bytes with ncmds at 16; segment_command_64 has vmaddr at 24, vmsize at 32
    // and initprot at 60.
    unsafe {
        let read_u32 = |p: *const u8| p.cast::<u32>().read_unaligned();
        let read_u64 = |p: *const u8| p.cast::<u64>().read_unaligned();
        let header = _dyld_get_image_header(0);
        let slide = _dyld_get_image_vmaddr_slide(0) as u64;
        let mut command = header.add(32);
        for _ in 0..read_u32(header.add(16)) {
            let vmsize = read_u64(command.add(32));
            if read_u32(command) == LC_SEGMENT_64
                && read_u32(command.add(60)) & VM_PROT_READ != 0
                && vmsize != 0
            {
                let start = read_u64(command.add(24)).wrapping_add(slide);
                ranges.push(start..start + vmsize);
            }
            command = command.add(read_u32(command.add(4)) as usize);
        }
    }
    ranges
}

#[cfg(target_os = "linux")]
fn image_ranges() -> Vec<Range<u64>> {
    unsafe extern "C" fn collect(
        info: *mut libc::dl_phdr_info,
        _: usize,
        data: *mut libc::c_void,
    ) -> libc::c_int {
        unsafe {
            let info = &*info;
            if !info.dlpi_name.is_null() && *info.dlpi_name != 0 {
                return 0;
            }
            let ranges = &mut *data.cast::<Vec<Range<u64>>>();
            for header in std::slice::from_raw_parts(info.dlpi_phdr, info.dlpi_phnum as usize) {
                if header.p_type == libc::PT_LOAD
                    && header.p_flags & libc::PF_R != 0
                    && header.p_memsz != 0
                {
                    let start = info.dlpi_addr as u64 + header.p_vaddr as u64;
                    ranges.push(start..start + header.p_memsz as u64);
                }
            }
            1
        }
    }
    let mut ranges = Vec::new();
    unsafe {
        libc::dl_iterate_phdr(Some(collect), (&mut ranges as *mut Vec<Range<u64>>).cast());
    }
    ranges
}

#[cfg(target_os = "windows")]
fn image_ranges() -> Vec<Range<u64>> {
    extern "C" {
        static __ImageBase: u8;
    }
    unsafe {
        let base = std::ptr::addr_of!(__ImageBase);
        let nt_offset = base.add(0x3c).cast::<u32>().read_unaligned() as usize;
        // SizeOfImage has the same offset in PE32 and PE32+ optional headers.
        let size = base.add(nt_offset + 24 + 56).cast::<u32>().read_unaligned();
        let start = base as u64;
        let range = start..start + u64::from(size);
        vec![range]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registration_enables_checks_and_includes_main_image() {
        static GUEST_STATIC: u8 = 17;
        let mut ranges = Vec::new();
        let heap = [0u8; 64];
        let start = heap.as_ptr() as u64;
        register(&mut ranges, start, heap.len() as u64);
        assert!(ranges.contains(&(start..start + heap.len() as u64)));
        assert!(ranges
            .iter()
            .any(|range| range.contains(&(&GUEST_STATIC as *const u8 as u64))));
        let count = ranges.len();
        register(&mut ranges, start, 32);
        assert_eq!(ranges.len(), count + 1);
    }
}

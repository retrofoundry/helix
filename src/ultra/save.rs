//! ultra/save.rs — EEPROM (4K, 512-byte) file backend at a platform path.
//! Replaces the deleted src/pc/ultra_reimplementation.c fopen("sm64_save_file.bin") path.

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

/// EEPROM_MAXBLOCKS(64) * EEPROM_BLOCK_SIZE(8): the 4K EEPROM sm64-US uses (include/PR/os_eeprom.h).
const EEPROM_STORE_SIZE: usize = 512;
/// osEepromLong{Read,Write} `address` is a block index; each block is 8 bytes.
const EEPROM_BLOCK_SIZE: usize = 8;

struct Eeprom {
    store: [u8; EEPROM_STORE_SIZE],
    path: PathBuf,
}

/// `$HELIX_SAVE_PATH` wins (tests/CI); else a per-OS data dir; else the temp dir.
fn save_path() -> PathBuf {
    if let Ok(p) = std::env::var("HELIX_SAVE_PATH") {
        return PathBuf::from(p);
    }
    let dir = if cfg!(target_os = "macos") {
        std::env::var("HOME")
            .ok()
            .map(|h| PathBuf::from(h).join("Library/Application Support"))
    } else if cfg!(target_os = "windows") {
        std::env::var("APPDATA").ok().map(PathBuf::from)
    } else {
        std::env::var("XDG_DATA_HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var("HOME")
                    .ok()
                    .map(|h| PathBuf::from(h).join(".local/share"))
            })
    };
    dir.unwrap_or_else(std::env::temp_dir)
        .join("helix")
        .join("sm64_save_file.bin")
}

impl Eeprom {
    fn load() -> Self {
        let path = save_path();
        let mut store = [0u8; EEPROM_STORE_SIZE];
        if let Ok(bytes) = std::fs::read(&path) {
            let n = bytes.len().min(EEPROM_STORE_SIZE);
            store[..n].copy_from_slice(&bytes[..n]);
        }
        Eeprom { store, path }
    }

    /// Atomic write-through: temp file + fsync + rename, so a crash never leaves a torn save.
    fn persist(&self) -> std::io::Result<()> {
        use std::io::Write;
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = self.path.with_extension("tmp");
        {
            let mut f = std::fs::File::create(&tmp)?;
            f.write_all(&self.store)?;
            f.sync_all()?;
        }
        std::fs::rename(&tmp, &self.path)
    }
}

static EEPROM: OnceLock<Mutex<Eeprom>> = OnceLock::new();
fn eeprom() -> &'static Mutex<Eeprom> {
    EEPROM.get_or_init(|| Mutex::new(Eeprom::load()))
}

/// Flush the in-memory store to disk. No-op if the EEPROM backend was never lazily
/// initialized (no prior HLXEeprom{Probe,Read,Write} call touched it).
/// Called by ultra/mod.rs::teardown on graceful shutdown.
pub fn flush() {
    if let Some(m) = EEPROM.get() {
        if let Err(e) = m.lock().unwrap().persist() {
            log::error!("eeprom flush failed: {e}");
        }
    }
}

// MARK: - C API (FFI — called by helix/cpp/libultra/os_eeprom.c)

#[no_mangle]
pub extern "C" fn HLXEepromProbe() -> i32 {
    let _ = eeprom(); // force lazy load so a subsequent flush() has state
    1 // 4K EEPROM always present in this backend
}

#[no_mangle]
pub unsafe extern "C" fn HLXEepromRead(addr: u8, buf: *mut u8, n: i32) -> i32 {
    if buf.is_null() || n < 0 {
        return -1;
    }
    let n = n as usize;
    let off = addr as usize * EEPROM_BLOCK_SIZE;
    if off + n > EEPROM_STORE_SIZE {
        return -1;
    }
    let ee = eeprom().lock().unwrap();
    std::ptr::copy_nonoverlapping(ee.store.as_ptr().add(off), buf, n);
    0
}

#[no_mangle]
pub unsafe extern "C" fn HLXEepromWrite(addr: u8, buf: *const u8, n: i32) -> i32 {
    if buf.is_null() || n < 0 {
        return -1;
    }
    let n = n as usize;
    let off = addr as usize * EEPROM_BLOCK_SIZE;
    if off + n > EEPROM_STORE_SIZE {
        return -1;
    }
    let mut ee = eeprom().lock().unwrap();
    std::ptr::copy_nonoverlapping(buf, ee.store.as_mut_ptr().add(off), n);
    match ee.persist() {
        Ok(()) => 0,
        Err(e) => {
            log::error!("eeprom write persist failed: {e}");
            -1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eeprom_roundtrips_persists_and_bounds_check() {
        let tmp = std::env::temp_dir().join(format!("helix_eeprom_{}.bin", std::process::id()));
        let _ = std::fs::remove_file(&tmp);
        std::env::set_var("HELIX_SAVE_PATH", &tmp);

        assert_eq!(HLXEepromProbe(), 1);

        let src = [0xAB_u8; 32];
        let mut dst = [0u8; 32];
        unsafe {
            assert_eq!(HLXEepromWrite(2, src.as_ptr(), 32), 0); // block 2 -> byte offset 16
            assert_eq!(HLXEepromRead(2, dst.as_mut_ptr(), 32), 0);
        }
        assert_eq!(dst, src);

        // Write-through hit disk: 512-byte file, payload at offset 2*8 = 16.
        let disk = std::fs::read(&tmp).unwrap();
        assert_eq!(disk.len(), EEPROM_STORE_SIZE);
        assert_eq!(&disk[16..48], &src[..]);

        // Bounds: last block (63*8=504, +8=512) is valid; block 64 overruns.
        let mut b = [0u8; 8];
        unsafe {
            assert_eq!(HLXEepromRead(63, b.as_mut_ptr(), 8), 0);
            assert_eq!(HLXEepromRead(64, b.as_mut_ptr(), 8), -1);
            assert_eq!(HLXEepromWrite(64, src.as_ptr(), 8), -1);
            assert_eq!(HLXEepromRead(0, std::ptr::null_mut(), 8), -1);
            assert_eq!(HLXEepromWrite(0, src.as_ptr(), -1), -1);
        }

        let _ = std::fs::remove_file(&tmp);
    }
}

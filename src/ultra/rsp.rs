use std::os::raw::c_void;
use std::sync::OnceLock;

#[repr(C)]
pub struct HLXRspBinding {
    pub ucode: *const c_void,
    pub handler: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Handler {
    Gfx(fast3d::Microcode),
    AudioAcmd,
    JpegNjpgdsp,
    Skip,
}

struct Registry {
    bindings: Vec<(usize, Handler)>,
    graphics_bindings: Vec<(u64, fast3d::Microcode)>,
}

impl Registry {
    fn lookup(&self, ucode: usize) -> Option<Handler> {
        self.bindings.iter().find_map(|&(address, handler)| {
            (ucode == address || ucode == address.wrapping_sub(0x8000_0000)).then_some(handler)
        })
    }
}

static REGISTRY: OnceLock<Registry> = OnceLock::new();

pub(crate) fn lookup(ucode: usize) -> Option<Handler> {
    REGISTRY.get()?.lookup(ucode)
}

pub(crate) fn graphics_bindings() -> &'static [(u64, fast3d::Microcode)] {
    REGISTRY
        .get()
        .map_or(&[], |registry| &registry.graphics_bindings)
}

fn register(registry: &OnceLock<Registry>, bindings: &[HLXRspBinding]) -> Result<(), ()> {
    if registry.get().is_some() {
        return Err(());
    }
    let mut validated = Registry {
        bindings: Vec::new(),
        graphics_bindings: Vec::new(),
    };
    for binding in bindings {
        let address = binding.ucode as usize;
        let physical = address.wrapping_sub(0x8000_0000);
        if address == 0
            || validated.lookup(address).is_some()
            || validated.lookup(physical).is_some()
        {
            return Err(());
        }
        let handler = match binding.handler {
            1 => Handler::Gfx(fast3d::Microcode::F3d),
            2 => Handler::Gfx(fast3d::Microcode::F3dex2),
            3 => Handler::Gfx(fast3d::Microcode::F3dex2NoN),
            4 => Handler::Gfx(fast3d::Microcode::S2dex2),
            5 => Handler::AudioAcmd,
            6 => Handler::JpegNjpgdsp,
            7 => Handler::Skip,
            _ => return Err(()),
        };
        validated.bindings.push((address, handler));
        if let Handler::Gfx(microcode) = handler {
            validated
                .graphics_bindings
                .extend([(address as u64, microcode), (physical as u64, microcode)]);
        }
    }
    registry.set(validated).map_err(|_| ())
}

#[no_mangle]
pub extern "C" fn HLXRspRegister(bindings: *const HLXRspBinding, count: usize) -> i32 {
    let bindings = if count == 0 {
        &[]
    } else if bindings.is_null() {
        return -1;
    } else {
        unsafe { std::slice::from_raw_parts(bindings, count) }
    };
    match register(&REGISTRY, bindings) {
        Ok(()) => 0,
        Err(()) => {
            log::error!(
                "HLXRspRegister rejected {count} bindings: null or duplicate ucode, unknown handler, \
                 or a second call"
            );
            -1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn binding(ucode: usize, handler: u32) -> HLXRspBinding {
        HLXRspBinding {
            ucode: ucode as *const c_void,
            handler,
        }
    }

    #[test]
    fn rejects_invalid_registrations_without_storing_any_bindings() {
        let registry = OnceLock::new();
        for invalid in [
            vec![binding(0, 1)],
            vec![binding(0x1_1234_5678, 0)],
            vec![binding(0x1_1234_5678, 8)],
            vec![binding(0x1_1234_5678, 1), binding(0x1_1234_5678, 2)],
            vec![binding(0x1_1234_5678, 1), binding(0x9234_5678, 2)],
            vec![binding(0x9234_5678, 2), binding(0x1_1234_5678, 1)],
            vec![binding(0x1_1234_5678, 1), binding(0, 7)],
        ] {
            assert_eq!(register(&registry, &invalid), Err(()));
            assert!(registry.get().is_none());
        }
        assert_eq!(register(&registry, &[binding(0x1_1234_5678, 1)]), Ok(()));
    }

    #[test]
    fn second_registration_preserves_the_first() {
        let registry = OnceLock::new();
        assert_eq!(register(&registry, &[binding(0x1_1234_5678, 5)]), Ok(()));
        assert_eq!(register(&registry, &[binding(0x2_1234_5678, 7)]), Err(()));
        let registry = registry.get().unwrap();
        assert_eq!(registry.lookup(0x1_1234_5678), Some(Handler::AudioAcmd));
        assert_eq!(registry.lookup(0x2_1234_5678), None);
    }

    #[test]
    fn empty_registration_is_still_one_shot() {
        let registry = OnceLock::new();
        assert_eq!(register(&registry, &[]), Ok(()));
        assert_eq!(registry.get().unwrap().lookup(0), None);
        assert_eq!(register(&registry, &[]), Err(()));
    }

    #[test]
    fn resolves_all_handlers_in_both_address_forms_and_derives_graphics_bindings() {
        let registry = OnceLock::new();
        let handlers = [
            Handler::Gfx(fast3d::Microcode::F3d),
            Handler::Gfx(fast3d::Microcode::F3dex2),
            Handler::Gfx(fast3d::Microcode::F3dex2NoN),
            Handler::Gfx(fast3d::Microcode::S2dex2),
            Handler::AudioAcmd,
            Handler::JpegNjpgdsp,
            Handler::Skip,
        ];
        let bindings: Vec<_> = (1..=7)
            .map(|id| binding(0x1000 + id as usize * 8, id))
            .collect();
        assert_eq!(register(&registry, &bindings), Ok(()));
        let registry = registry.get().unwrap();
        let mut expected_graphics = Vec::new();
        for (binding, handler) in bindings.iter().zip(handlers) {
            let address = binding.ucode as usize;
            for token in [address, address.wrapping_sub(0x8000_0000)] {
                assert_eq!(registry.lookup(token), Some(handler));
                if let Handler::Gfx(microcode) = handler {
                    expected_graphics.push((token as u64, microcode));
                }
            }
        }
        assert_eq!(registry.lookup(0xdead_beef), None);
        assert_eq!(registry.graphics_bindings, expected_graphics);
    }
}

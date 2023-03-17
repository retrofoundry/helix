use super::types::{ControllerBits, OSContPad, OSContStatus};
use super::{backends::giapi::GIApi, device::ControllerDevice};
use std::mem::size_of;
use std::os::raw::c_void;
#[cfg(feature = "cpp")]
use std::ptr;

static mut MAX_CONTROLLERS: usize = 4;

pub struct ControllerHub {
    devices: Vec<Box<dyn ControllerDevice>>,
    bits: Option<ControllerBits>,
}

impl ControllerHub {
    pub fn new() -> ControllerHub {
        ControllerHub {
            devices: Vec::new(),
            bits: None,
        }
    }

    pub fn init(&mut self, controller_bits: ControllerBits) {
        self.scan();
        self.bits = Some(controller_bits);
    }

    pub fn register(&mut self, device: Box<dyn ControllerDevice>) {
        self.devices.push(device);
    }

    pub fn scan(&mut self) {
        self.devices.clear();

        // Register gilRs devices
        for device in GIApi::new().scan() {
            self.register(device);
        }

        // TODO: Register more devices
    }

    pub fn write(&mut self, mut data: Vec<&mut OSContPad>) {
        for device in self.devices.iter_mut() {
            for pad in data.iter_mut() {
                device.write(pad);
            }
        }
    }
}

// MARK: - C API

#[cfg(feature = "cpp")]
#[no_mangle]
pub extern "C" fn HLXCreateControllerHub() -> Box<ControllerHub> {
    let hub = ControllerHub::new();
    Box::new(hub)
}

#[cfg(feature = "cpp")]
#[no_mangle]
extern "C" fn HLXControllerInit(hub: Option<&mut ControllerHub>, bits: *mut u8) -> i32 {
    // convert bits to a Box
    hub.unwrap().init(Box::new(0));
    0
}

#[cfg(feature = "cpp")]
#[no_mangle]
extern "C" fn osContStartReadData(_mq: *mut c_void) -> i32 {
    0
}

#[cfg(feature = "cpp")]
#[no_mangle]
unsafe extern "C" fn osContGetReadData(pad: *mut OSContPad) {
    ptr::write_bytes(pad, 0, size_of::<OSContPad>() * MAX_CONTROLLERS);
    let _data = Vec::<OSContPad>::from_raw_parts(pad, size_of::<OSContPad>(), MAX_CONTROLLERS);
    // controller_hub!().write(Box::new(data));
    // ptr::copy_nonoverlapping(data.as_mut_ptr(), pad, data.len());
}

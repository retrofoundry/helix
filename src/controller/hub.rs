#[cfg(feature = "cpp")]
use std::ptr;
use std::mem::size_of;
use std::os::raw::c_void;
use super::types::{OSContPad, OSContStatus, ControllerBits};
use super::{device::ControllerDevice, backends::giapi::GIApi};

static mut MAX_CONTROLLERS: usize = 4;

pub struct ControllerHub {
    devices: Vec<Box<dyn ControllerDevice>>,
    bits: Option<ControllerBits>
}

impl ControllerHub {
    pub fn new() -> ControllerHub {
        ControllerHub {
            devices: Vec::new(),
            bits: None
        }
    }

    pub fn init(&mut self, controllerBits: ControllerBits) {
        self.scan();
        self.bits = Some(controllerBits);
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

    pub fn write(&mut self, data: Box<Vec::<OSContPad>>) {
        for device in self.devices.iter_mut() {
            for (i, pad) in data.iter().enumerate() {
                let mut item = pad;
                device.write(&mut item);
            }
        }
    }
}

// MARK: - C API

#[cfg(feature = "cpp")]
#[no_mangle]
extern "C" fn osContInit(mq: *mut c_void, controllerBits: *mut u8, status: *mut OSContStatus) -> i32 {
    unsafe { *controllerBits = 0; }
    // controller_hub!().init(Box::new(unsafe { *controllerBits }));
    return 0;
}

#[cfg(feature = "cpp")]
#[no_mangle]
extern "C" fn osContStartReadData(mq: *mut c_void) -> i32 {
    return 0;
}

#[cfg(feature = "cpp")]
#[no_mangle]
unsafe extern "C" fn osContGetReadData(pad: *mut OSContPad) {
    ptr::write_bytes(pad, 0, size_of::<OSContPad>() * MAX_CONTROLLERS);
    let data = Vec::<OSContPad>::from_raw_parts(pad, size_of::<OSContPad>(), MAX_CONTROLLERS);
    // controller_hub!().write(Box::new(data));
    // ptr::copy_nonoverlapping(data.as_mut_ptr(), pad, data.len());
}
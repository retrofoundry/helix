use super::providers::gilrs::GirlsControllerProvider;
use super::types::{ControllerBits, OSContPad};
use crate::gamepad::providers::{Controller, ControllerProvider, ControllerService};
use std::mem::size_of;
use std::os::raw::c_void;
use std::ptr;
use std::ptr::null_mut;

pub struct ControllerManager {
    controllers: Vec<Controller>,
    controller_bits: ControllerBits,
    providers: Vec<Box<dyn ControllerProvider>>,
}

impl ControllerManager {
    pub fn new() -> Self {
        Self {
            controllers: Vec::new(),
            controller_bits: null_mut(),
            providers: vec![Box::new(GirlsControllerProvider::new())],
        }
    }

    pub fn init(&mut self, controller_bits: ControllerBits) {
        unsafe {
            *controller_bits = 0;
        }

        self.scan_for_controllers();

        unsafe {
            *controller_bits = 1;
        }
        self.controller_bits = controller_bits;
    }

    fn scan_for_controllers(&mut self) {
        self.controllers.clear();

        for provider in self.providers.iter() {
            for device in provider.scan() {
                self.controllers.push(device);
            }
        }

        // TODO: Register a keyboard device?
    }

    pub fn write(&mut self, pad: *mut OSContPad) {
        // TODO: Handle current slot (*)

        for controller in &self.controllers {
            match controller.service {
                ControllerService::GilRs(_) => {
                    for provider in &self.providers {
                        provider.read(controller, pad);
                    }
                }
                ControllerService::Keyboard() => {
                    todo!("Implement keyboard gamepad");
                }
            }
        }
    }
}

// MARK: - C API

#[no_mangle]
pub extern "C" fn ControllerManagerCreate() -> Box<ControllerManager> {
    let hub = ControllerManager::new();
    Box::new(hub)
}

#[no_mangle]
extern "C" fn ControllerManagerInit(
    manager: Option<&mut ControllerManager>,
    controller_bits: ControllerBits,
) -> i32 {
    let manager = manager.unwrap();
    manager.init(controller_bits);

    0
}

#[no_mangle]
extern "C" fn ControllerGetReadData(manager: Option<&mut ControllerManager>, pad: *mut OSContPad) {
    unsafe {
        ptr::write_bytes(pad, 0, size_of::<OSContPad>());
    }

    let manager = manager.unwrap();
    manager.write(pad);
}

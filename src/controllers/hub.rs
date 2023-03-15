use super::{backends::gidevice::GIController, device::ControllerDevice};

pub struct ControllerHub {
    devices: Vec<Box<dyn ControllerDevice>>,
}

impl ControllerHub {
    pub fn new() -> ControllerHub {
        ControllerHub {
            devices: Vec::new(),
        }
    }

    pub fn init(&mut self) {
        self.devices.push(Box::new(GIController {
            id: 0,
            api: Cell::new(Gilrs::new().unwrap()),
        }));
    }

    pub fn register(&mut self, device: Box<dyn ControllerDevice>) {
        self.devices.push(device);
    }

    pub fn scan(&mut self) {
        self.devices.clear();

    }
}
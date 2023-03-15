use super::device::ControllerDevice;

pub struct ControllerHub {
    giapi: super::backends::giapi::GIApi,
    devices: Vec<Box<dyn ControllerDevice>>,
}

impl ControllerHub {
    pub fn new() -> ControllerHub {
        ControllerHub {
            giapi: super::backends::giapi::GIApi::new(),
            devices: Vec::new(),
        }
    }

    pub fn init(&mut self) {
        self.devices = self.giapi.scan();
    }

    pub fn register(&mut self, device: Box<dyn ControllerDevice>) {
        self.devices.push(device);
    }

    pub fn scan(&mut self) {
        self.devices.clear();

        // go through each backend and register the given devices
    }
}

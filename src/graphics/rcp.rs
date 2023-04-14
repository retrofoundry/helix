use super::{
    gbi::{defines::Gfx, GBIResult, GBI},
    rdp::RDP,
    rsp::RSP, gfx_device::{GfxDevice, C_GfxDevice, self},
};

pub struct RCP {
    gbi: GBI,
    pub rdp: RDP,
    pub rsp: RSP,
    pub gfx_device: Option<GfxDevice>,
}

impl RCP {
    pub fn new() -> Self {
        let mut gbi = GBI::new();
        gbi.setup();

        RCP {
            gbi,
            rdp: RDP::new(),
            rsp: RSP::new(),
            gfx_device: None,
        }
    }

    pub fn bridge(gfx_device: GfxDevice) -> Self {
        let mut gbi = GBI::new();
        gbi.setup();

        RCP {
            gbi,
            rdp: RDP::new(),
            rsp: RSP::new(),
            gfx_device: Some(gfx_device),
        }
    }

    pub fn reset(&mut self) {
        self.rdp.reset();
        self.rsp.reset();
    }

    /// This funtion is called to process a work buffer.
    /// It takes in a pointer to the start of the work buffer and will
    /// process until it hits a `G_ENDDL` inidicating the end.
    pub fn run(&mut self, commands: usize) {
        self.reset();

        // self.rdp.setup_draw();
        self.run_dl(commands);
        // self.rdp.flush();
    }

    fn run_dl(&mut self, commands: usize) {
        let mut commands = commands as *const Gfx;

        loop {
            let w0 = unsafe { (*commands).words.w0 };
            let w1 = unsafe { (*commands).words.w1 };
            let gfx_device = self.gfx_device.as_ref().unwrap();

            match self
                .gbi
                .handle_command(&mut self.rdp, &mut self.rsp, gfx_device, w0, w1)
            {
                GBIResult::Recurse(new_commands) => {
                    self.run_dl(new_commands);
                }
                GBIResult::SetAddress(new_address) => {
                    commands = new_address as *const Gfx;
                    commands = unsafe { commands.sub(1) };
                }
                GBIResult::Return => {
                    return;
                }
                GBIResult::Unknown(opcode) => {
                    println!("Unknown GBI command: {:#x}", opcode);
                }
                _ => {}
            }

            commands = unsafe { commands.add(1) };
        }
    }
}

// MARK: C Bridge

#[no_mangle]
pub extern "C" fn RCPReset(rcp: Option<&mut RCP>) {
    let rcp = rcp.unwrap();
    rcp.reset();
}

#[no_mangle]
pub extern "C" fn RCPCreate(gfx_device: *mut C_GfxDevice) -> Box<RCP> {
    let gfx_device = GfxDevice::new(gfx_device);
    let rcp = RCP::bridge(gfx_device);
    Box::new(rcp)
}

#[no_mangle]
pub extern "C" fn RCPGetGfxDevice(rcp: Option<&mut RCP>) -> *mut C_GfxDevice {
    let rcp = rcp.unwrap();
    rcp.gfx_device.as_mut().unwrap().storage
}
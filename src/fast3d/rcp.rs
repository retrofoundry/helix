use log::trace;

use super::{
    gbi::{defines::Gfx, GBIResult, GBI},
    graphics::GraphicsIntermediateDevice,
    rdp::RDP,
    rsp::RSP,
};

pub struct RCP {
    gbi: GBI,
    pub rdp: RDP,
    pub rsp: RSP,
}

impl Default for RCP {
    fn default() -> Self {
        Self::new()
    }
}

impl RCP {
    pub fn new() -> Self {
        let mut gbi = GBI::default();
        gbi.setup();

        RCP {
            gbi,
            rdp: RDP::default(),
            rsp: RSP::default(),
        }
    }

    pub fn reset(&mut self) {
        self.rdp.reset();
        self.rsp.reset();
    }

    /// This funtion is called to process a work buffer.
    /// It takes in a pointer to the start of the work buffer and will
    /// process until it hits a `G_ENDDL` inidicating the end.
    pub fn run(&mut self, gfx_device: &mut GraphicsIntermediateDevice, commands: usize) {
        self.reset();

        self.run_dl(gfx_device, commands);
        self.rdp.flush(gfx_device);
    }

    fn run_dl(&mut self, gfx_device: &mut GraphicsIntermediateDevice, commands: usize) {
        let mut command = commands as *mut Gfx;

        loop {
            match self
                .gbi
                .handle_command(&mut self.rdp, &mut self.rsp, gfx_device, &mut command)
            {
                GBIResult::Recurse(new_command) => self.run_dl(gfx_device, new_command),
                GBIResult::Unknown(opcode) => {
                    trace!("Unknown GBI command: {:#x}", opcode)
                }
                GBIResult::Return => return,
                GBIResult::Continue => {}
            }

            unsafe { command = command.add(1) };
        }
    }
}

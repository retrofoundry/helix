use log::trace;

use super::{
    gbi::{defines::Gfx, GBIResult, GBI},
    graphics::GraphicsContext,
    rdp::RDP,
    rsp::RSP,
};

pub struct RCP {
    gbi: GBI,
    pub rdp: RDP,
    pub rsp: RSP,
}

impl RCP {
    pub fn new() -> Self {
        let mut gbi = GBI::new();
        gbi.setup();

        RCP {
            gbi,
            rdp: RDP::new(),
            rsp: RSP::new(),
        }
    }

    pub fn reset(&mut self) {
        self.rdp.reset();
        self.rsp.reset();
    }

    /// This funtion is called to process a work buffer.
    /// It takes in a pointer to the start of the work buffer and will
    /// process until it hits a `G_ENDDL` inidicating the end.
    pub fn run(&mut self, gfx_context: &GraphicsContext, commands: usize) {
        self.reset();

        // self.rdp.setup_draw();
        self.run_dl(gfx_context, commands);
        // self.rdp.flush();
    }

    fn run_dl(&mut self, gfx_context: &GraphicsContext, commands: usize) {
        let mut commands = commands as *const Gfx;

        loop {
            let w0 = unsafe { (*commands).words.w0 };
            let w1 = unsafe { (*commands).words.w1 };

            match self
                .gbi
                .handle_command(&mut self.rdp, &mut self.rsp, gfx_context, w0, w1)
            {
                GBIResult::Recurse(new_commands) => {
                    self.run_dl(gfx_context, new_commands);
                }
                GBIResult::SetAddress(new_address) => {
                    commands = new_address as *const Gfx;
                    commands = unsafe { commands.sub(1) };
                }
                GBIResult::Return => {
                    return;
                }
                GBIResult::Unknown(opcode) => {
                    trace!("Unknown GBI command: {:#x}", opcode);
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
pub extern "C" fn RCPCreate() -> Box<RCP> {
    let rcp = RCP::new();
    Box::new(rcp)
}

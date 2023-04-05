use super::{
    gbi::{defines::Gfx, GBIResult, GBI},
    rdp::RDP,
    rsp::RSP,
};

pub struct RCP {
    gbi: GBI,
    rdp: RDP,
    rsp: RSP,
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

            match self
                .gbi
                .handle_command(&mut self.rdp, &mut self.rsp, w0, w1)
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

use super::{
    gbi::{defines::Gfx, GBIResult, GBI},
    rdp::RDP,
    rsp::RSP,
};

pub struct RCP {
    rdp: RDP,
    rsp: RSP,
    gbi: GBI,
}

impl RCP {
    pub fn new() -> Self {
        RCP {
            rdp: RDP::new(),
            rsp: RSP::new(),
            gbi: GBI::new(),
        }
    }

    pub fn initialize(&mut self) {
        self.gbi.setup();
    }

    pub fn reset(&mut self) {
        self.rdp.reset();
        self.rsp.reset();
    }

    /// This funtion is called to process a work buffer.
    /// It takes in a pointer to the start of the work buffer and will
    /// process until it hits a `G_ENDDL` inidicating the end.
    pub fn process_displaylist(&mut self, mut command: *const Gfx) {
        loop {
            match self
                .gbi
                .handle_command(&mut self.rdp, &mut self.rsp, command)
            {
                GBIResult::Recurse(new_command) => {
                    self.process_displaylist(new_command);
                }
                GBIResult::Increase => {
                    command = unsafe { command.add(1) };
                }
                GBIResult::Decrease => {
                    command = unsafe { command.sub(1) };
                }
                GBIResult::SetAddressWithDecrease(new_address) => {
                    let cmd = new_address as *const Gfx;
                    command = unsafe { cmd.sub(1) };
                }
                GBIResult::Return => {
                    return;
                }
                _ => {}
            }

            command = unsafe { command.add(1) };
        }
    }
}

use imgui_glow_renderer::glow;
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
    pub fn run(
        &mut self,
        gl_context: &glow::Context,
        gfx_context: &mut GraphicsContext,
        commands: usize,
    ) {
        self.reset();

        self.run_dl(gl_context, gfx_context, commands);
        self.rdp.flush(gl_context, gfx_context);
    }

    fn run_dl(
        &mut self,
        gl_context: &glow::Context,
        gfx_context: &mut GraphicsContext,
        commands: usize,
    ) {
        let mut command = commands as *mut Gfx;

        loop {
            match self.gbi.handle_command(
                &mut self.rdp,
                &mut self.rsp,
                gl_context,
                gfx_context,
                &mut command,
            ) {
                GBIResult::Recurse(new_command) => {
                    self.run_dl(gl_context, gfx_context, new_command)
                }
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

#[no_mangle]
pub extern "C" fn RCPRunDL(
    rcp: Option<&mut RCP>,
    gfx_context: Option<&mut GraphicsContext>,
    commands: usize,
) {
    let rcp = rcp.unwrap();
    let gfx_context = gfx_context.unwrap();

    unsafe {
        let gl_context = glow::Context::from_loader_function(|_| std::ptr::null());
        rcp.run_dl(&gl_context, gfx_context, commands);
    }
}

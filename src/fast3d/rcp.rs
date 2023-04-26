use log::trace;

use super::{
    gbi::{defines::Gfx, GBIResult, GBI},
    graphics::GraphicsContext,
    rdp::RDP,
    rsp::RSP,
    utils::{color_combiner::CombineParams, texture::TextureState},
};

pub struct DrawCall {
    pub geometry_mode: u32,
    pub texture_state: TextureState,
    pub other_mode_l: u32,
    pub other_mode_h: u32,
    pub combine: CombineParams,

    pub env_color: [u8; 4],
    pub fog_color: [u8; 4],
    pub prim_color: [u8; 4],
    pub fill_color: [u8; 4],

    pub texture_indices: Vec<usize>,

    pub first_index: usize,
    pub num_indices: usize,
}

impl DrawCall {
    pub fn new() -> Self {
        DrawCall {
            geometry_mode: 0,
            texture_state: TextureState::EMPTY,
            other_mode_l: 0,
            other_mode_h: 0,
            combine: CombineParams::ZERO,
            env_color: [0; 4],
            fog_color: [0; 4],
            prim_color: [0; 4],
            fill_color: [0; 4],
            texture_indices: Vec::new(),
            first_index: 0,
            num_indices: 0,
        }
    }
}

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
    pub fn run(&mut self, gfx_context: &GraphicsContext, commands: usize, commands_size: usize) {
        self.reset();

        // self.rdp.setup_draw();
        self.run_dl(gfx_context, commands, commands_size);
        // self.rdp.flush();
    }

    // commands_size is in bytes
    fn run_dl(&mut self, gfx_context: &GraphicsContext, commands: usize, commands_size: usize) {
        let mut command = commands as *mut Gfx;
        let commands_end = commands + commands_size;

        assert!(commands < commands_end);

        loop {
            match self
                .gbi
                .handle_command(&mut self.rdp, &mut self.rsp, gfx_context, command)
            {
                GBIResult::Recurse(new_command) => {
                    let new_cmd_size = commands_end - new_command;
                    self.run_dl(gfx_context, new_command, new_cmd_size)
                }
                GBIResult::Unknown(opcode) => {
                    trace!("Unknown GBI command: {:#x}", opcode);
                }
                GBIResult::Return => return,
                GBIResult::Continue => {}
            }

            command = unsafe { command.add(1) };
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

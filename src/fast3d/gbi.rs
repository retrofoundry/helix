use self::defines::{Gfx, G_RDPFULLSYNC, G_RDPLOADSYNC, G_RDPPIPESYNC, G_RDPTILESYNC};

use super::{graphics::GraphicsIntermediateDevice, rdp::RDP, rsp::RSP};
use std::collections::HashMap;

pub mod defines;
mod f3d;
mod f3dex2;
mod f3dex2e;
mod f3dzex2;
pub mod utils;

pub enum GBIResult {
    Return,
    Recurse(usize),
    Unknown(usize),
    Continue,
}

pub type GBICommand = fn(
    dp: &mut RDP,
    rsp: &mut RSP,
    gfx_device: &mut GraphicsIntermediateDevice,
    command: &mut *mut Gfx,
) -> GBIResult;

trait GBIDefinition {
    fn setup(gbi: &mut GBI);
}

pub struct GBI {
    pub gbi_opcode_table: HashMap<usize, GBICommand>,
}

impl Default for GBI {
    fn default() -> Self {
        Self::new()
    }
}

impl GBI {
    pub fn new() -> Self {
        Self {
            gbi_opcode_table: HashMap::new(),
        }
    }

    pub fn setup(&mut self) {
        self.register(G_RDPLOADSYNC as usize, |_, _, _, _| GBIResult::Continue);
        self.register(G_RDPPIPESYNC as usize, |_, _, _, _| GBIResult::Continue);
        self.register(G_RDPTILESYNC as usize, |_, _, _, _| GBIResult::Continue);
        self.register(G_RDPFULLSYNC as usize, |_, _, _, _| GBIResult::Continue);

        if cfg!(feature = "f3dzex2") {
            f3dzex2::F3DZEX2::setup(self);
        } else if cfg!(feature = "f3dex2e") {
            f3dex2e::F3DEX2E::setup(self);
        } else if cfg!(feature = "f3dex2") {
            f3dex2::F3DEX2::setup(self);
        }
    }

    pub fn register(&mut self, opcode: usize, cmd: GBICommand) {
        self.gbi_opcode_table.insert(opcode, cmd);
    }

    pub fn handle_command(
        &self,
        rdp: &mut RDP,
        rsp: &mut RSP,
        gfx_device: &mut GraphicsIntermediateDevice,
        command: &mut *mut Gfx,
    ) -> GBIResult {
        let w0 = unsafe { (*(*command)).words.w0 };

        let opcode = w0 >> 24;
        let cmd = self.gbi_opcode_table.get(&opcode);

        match cmd {
            Some(cmd) => cmd(rdp, rsp, gfx_device, command),
            None => GBIResult::Unknown(opcode),
        }
    }
}

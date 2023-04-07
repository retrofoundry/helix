#[repr(C)]
#[derive(Clone, Copy)]
pub struct GWords {
    pub w0: libc::uintptr_t,
    pub w1: libc::uintptr_t,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union Gfx {
    pub words: GWords,
    pub force_structure_alignment: libc::c_longlong,
}

#[cfg(feature = "f3dex2")]
#[derive(PartialEq, Eq)]
pub enum G_MTX {
    NOPUSH_MUL_MODELVIEW = 0x00,
    PUSH = 0x01,
    // MUL = 0x00,
    LOAD = 0x02,
    // MODELVIEW = 0x00,
    PROJECTION = 0x04,
}

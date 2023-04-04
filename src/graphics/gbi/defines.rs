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

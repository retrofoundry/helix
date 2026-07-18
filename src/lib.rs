// The `#[no_mangle] extern "C"` entry points take raw pointers the C caller owns; the C ABI is the
// unsafe boundary, so `not_unsafe_ptr_arg_deref` (which asks for a Rust `unsafe` marker) is noise here.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use env_logger::Builder;
pub mod audio;
pub mod gamepad;
pub mod gui;
#[cfg(feature = "network")]
pub mod network;
pub mod render;
#[cfg(feature = "speech")]
pub mod speech;
pub mod ultra;

pub use arie;

pub fn init() {
    let mut builder = Builder::from_default_env();

    #[cfg(debug_assertions)]
    builder.filter_level(log::LevelFilter::Warn);
    #[cfg(not(debug_assertions))]
    builder.filter_level(log::LevelFilter::Info);

    builder.init();
}

// MARK: - C API

#[no_mangle]
pub extern "C" fn HelixInit() {
    init();
}

#[no_mangle]
pub extern "C" fn SpeechFeatureEnabled() -> bool {
    #[cfg(feature = "speech")]
    return true;
    #[cfg(not(feature = "speech"))]
    return false;
}

#[no_mangle]
pub extern "C" fn NetworkFeatureEnabled() -> bool {
    #[cfg(feature = "network")]
    return true;
    #[cfg(not(feature = "network"))]
    return false;
}

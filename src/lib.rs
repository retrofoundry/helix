pub use arie;
use env_logger::Builder;
mod extensions;
pub mod fast3d;
pub mod gamepad;
pub mod gui;
#[cfg(feature = "network")]
pub mod network;
#[cfg(feature = "speech")]
pub mod speech;

// Check for invalid feature combinations
#[cfg(all(feature = "opengl", feature = "wgpu"))]
compile_error!("Cannot enable both OpenGL and WGPU rendering");

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

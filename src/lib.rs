pub use arie;
use env_logger::Builder;
mod extensions;
mod fast3d;
pub mod gui;
#[cfg(feature = "network")]
pub mod network;
#[cfg(feature = "speech")]
pub mod speech;

pub fn init() {
    env_logger::init();
}

// MARK: - C API

#[no_mangle]
pub extern "C" fn HelixInit() {
    let mut builder = Builder::from_default_env();

    #[cfg(debug_assertions)]
    builder.filter_level(log::LevelFilter::Trace);
    #[cfg(not(debug_assertions))]
    builder.filter_level(log::LevelFilter::Info);

    builder.init();
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

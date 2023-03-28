pub mod gui;
#[cfg(feature = "network")]
pub mod network;
#[cfg(feature = "speech")]
pub mod speech;
pub use arie;

// MARK: - C API

#[cfg(feature = "cpp")]
#[no_mangle]
pub extern "C" fn SpeechFeatureEnabled() -> bool {
    #[cfg(feature = "speech")]
    return true;
    #[cfg(not(feature = "speech"))]
    return false;
}

#[cfg(feature = "cpp")]
#[no_mangle]
pub extern "C" fn NetworkFeatureEnabled() -> bool {
    #[cfg(feature = "network")]
    return true;
    #[cfg(not(feature = "network"))]
    return false;
}

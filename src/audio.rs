use crate::HELIX;

pub struct AudioPlayer {
    config: Option<Config>,
}

pub struct Config {}

impl AudioPlayer {
    pub fn new() -> Self {
        AudioPlayer {
            config: Option::None,
        }
    }

    pub fn init(&mut self) -> bool {
        false
    }

    pub fn buffered(&self) -> i32 {
        0
    }

    pub fn desired_buffer(&self) -> i32 {
        2480
    }

    pub fn play_buffer(&self, buf: &[u8]) {}
}

// MARK: - C API

#[no_mangle]
pub extern "C" fn HLXAudioPlayerInit() -> bool {
    return HELIX.lock().unwrap().audio_player.init();
}

#[no_mangle]
pub extern "C" fn HLXAudioPlayerGetBuffered() -> i32 {
    return HELIX.lock().unwrap().audio_player.buffered();
}

#[no_mangle]
pub extern "C" fn HLXAudioPlayerGetDesiredBuffered() -> i32 {
    return HELIX.lock().unwrap().audio_player.desired_buffer();
}

#[no_mangle]
pub extern "C" fn HLXAudioPlayerPlayBuffer(buf: *const u8, len: usize) {
    let buf = unsafe { std::slice::from_raw_parts(buf, len) };
    HELIX.lock().unwrap().audio_player.play_buffer(buf);
}

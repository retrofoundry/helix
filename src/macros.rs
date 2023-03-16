#[cfg(feature = "cpp")]
#[macro_export]
macro_rules! audio {
    () => {
        $crate::AUDIO_PLAYER.lock().unwrap()
    };
}

#[cfg(feature = "cpp")]
#[macro_export]
macro_rules! speech {
    () => {
        $crate::SPEECH_SYNTHESIZER.lock().unwrap()
    };
}

#[cfg(feature = "cpp")]
#[macro_export]
macro_rules! tcp_stream {
    () => {
        $crate::TCP_STREAM.lock().unwrap()
    };
}
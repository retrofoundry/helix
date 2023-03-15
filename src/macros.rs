#[macro_export]
macro_rules! audio {
    () => {
        $crate::AUDIO_PLAYER.lock().unwrap()
    };
}

#[macro_export]
macro_rules! speech {
    () => {
        $crate::SPEECH_SYNTHESIZER.lock().unwrap()
    };
}

#[macro_export]
macro_rules! tcp_stream {
    () => {
        $crate::TCP_STREAM.lock().unwrap()
    };
}

use crate::HELIX;
use byteorder::{ReadBytesExt, LittleEndian};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::{Producer, RingBuffer};

const SAMPLES_HIGH: i32 = 752;

pub struct AudioPlayer {
    backend: Option<Backend>,
}

pub struct Backend {
    buffer_producer: Producer<f32>,
    output_stream: cpal::Stream,
}

unsafe impl Send for AudioPlayer {}

impl AudioPlayer {
    pub fn new() -> Self {
        AudioPlayer {
            backend: Option::None,
        }
    }

    pub fn init(&mut self, sample_rate: u32, channels: u16) -> bool {
        let host = cpal::default_host();
        let output_device = host
            .default_output_device()
            .expect("failed to get default output audio device");
        let config = cpal::StreamConfig {
            channels,
            sample_rate: cpal::SampleRate(sample_rate),
            buffer_size: cpal::BufferSize::Default,
        };

        // Limiting the number of samples in the buffer is better to minimize
        // audio delay in playback, this is because game speed
        // does not 100% match audio playing speed (44100Hz).
        // The buffer holds only audio for 1/4 second, which is good enough for delays,
        // It can be reduced more, but it might cause noise(?) for slower machines
        // or if any CPU intensive process started while the emulator is running
        let buffer = RingBuffer::new(sample_rate as usize / 2);

        let (buffer_producer, mut buffer_consumer) = buffer.split();

        let output_data_fn = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            for sample in data {
                *sample = buffer_consumer.pop().unwrap_or(0.);
            }
        };

        let output_stream = output_device
            .build_output_stream(&config, output_data_fn, Self::err_fn, None)
            .expect("failed to build an output audio stream");
        
        
        self.backend = Some(Backend {
            buffer_producer,
            output_stream
        });

        true
    }

    pub fn deinit(&mut self) {
        if let Some(config) = self.backend.as_mut() {
            config.output_stream.pause().unwrap();

            self.backend = None;
        }
    }

    pub fn buffered(&self) -> i32 {
        if let Some(config) = self.backend.as_ref() {
            return config.buffer_producer.len() as i32 / 4;
        }

        0
    }

    pub fn desired_buffer(&self) -> i32 {
        SAMPLES_HIGH * 4
    }

    pub fn play_buffer(&mut self, buf: &[u8]) {
        if let Some(config) = self.backend.as_mut() {
            let mut cursor = std::io::Cursor::new(buf);
            
            // transform the buffer into a vector of f32 samples
            // buffer data is of 2 channels, 16 bit samples
            let mut samples = Vec::with_capacity(buf.len() / 2);
            while cursor.position() < buf.len() as u64 {
                let sample = cursor.read_i16::<LittleEndian>().unwrap() as f32 / 32768.0;
                samples.push(sample);
            }

            // TODO: write directly to the ring buffer
            config.buffer_producer.push_slice(&samples);
        }
    }

    fn err_fn(err: cpal::StreamError) {
        eprintln!("an error occurred on audio stream: {}", err);
    }
}

// MARK: - C API

#[no_mangle]
pub extern "C" fn HLXAudioPlayerInit(sample_rate: u32, channels: u16) -> bool {
    return HELIX.lock().unwrap().audio_player.init(sample_rate, channels);
}

#[no_mangle]
pub extern "C" fn HLXAudioPlayerDeinit() {
    HELIX.lock().unwrap().audio_player.deinit();
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

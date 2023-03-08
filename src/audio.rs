use crate::HELIX;
use cpal::traits::{DeviceTrait, HostTrait};
use ringbuf::{Producer, RingBuffer};

const SAMPLE_RATE: u32 = 44100;
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

    pub fn init(&mut self) -> bool {
        let host = cpal::default_host();
        let output_device = host
            .default_output_device()
            .expect("failed to get default output audio device");

        let config = cpal::StreamConfig {
            channels: 2,
            sample_rate: cpal::SampleRate(SAMPLE_RATE),
            buffer_size: cpal::BufferSize::Default,
        };

        // set the max length to the desired buffered level, plus
        // 3x the high sample rate, which is what the n64 audio engine
        // can output at one time, x2 to avoid overflow in case of the
        // n64 audio engine running faster than audio engine, all multiplied
        // by 4 because each sample is 4 bytes
        let max_buffer_length = (self.desired_buffer() + 3 * SAMPLES_HIGH * 2) * 4;
        let buffer = RingBuffer::new(max_buffer_length as usize);
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

    pub fn buffered(&self) -> i32 {
        if let Some(config) = self.backend.as_ref() {
            return config.buffer_producer.len() as i32 / 4;
        }

        0
    }

    pub fn desired_buffer(&self) -> i32 {
        2480
    }

    pub fn play_buffer(&mut self, buf: &[u8]) {
        if let Some(config) = self.backend.as_mut() {
            let mut samples = Vec::with_capacity(buf.len() / 2);
            for i in (0..buf.len()).step_by(2) {
                let sample = i16::from_le_bytes([buf[i], buf[i + 1]]);
                samples.push(sample as f32 / 32768.0);
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

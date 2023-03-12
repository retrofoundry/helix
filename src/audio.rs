use crate::helix;
use byteorder::{LittleEndian, ReadBytesExt};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::{HeapProducer, HeapRb};
use rubato::{FftFixedInOut, Resampler};

const SAMPLES_HIGH: i32 = 752;

pub struct AudioPlayer {
    backend: Option<Backend>,
}

pub struct Backend {
    buffer_producer: HeapProducer<f32>,
    resampler: Option<FftFixedInOut<f32>>,
    resample_buffer: Vec<f32>,
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
            .expect("[Audio] Failed to get default output audio device");

        let sample_rate = cpal::SampleRate(sample_rate);

        let conf = output_device
            .supported_output_configs()
            .unwrap()
            .collect::<Vec<_>>();

        let mut found_conf = false;

        for c in &conf {
            // must have 2 channels and f32 format
            // (almost all? devices will have at least one configuration with these)
            if c.channels() == 2
                && c.sample_format() == cpal::SampleFormat::F32
                && c.min_sample_rate() <= sample_rate
                && c.max_sample_rate() >= sample_rate
            {
                found_conf = true;
                break;
            }
        }

        let (output_sample_rate, resampler) = if found_conf {
            (sample_rate, None)
        } else {
            let def_conf = output_device.default_output_config().unwrap();

            if def_conf.channels() != 2 || def_conf.sample_format() != cpal::SampleFormat::F32 {
                eprintln!("[Audio] No supported configuration found for audio device, please open an issue in github `dcvz/helix`\n\
                      list of supported configurations: {:#?}", conf);
                return false;
            }

            (
                def_conf.sample_rate(),
                Some(
                    FftFixedInOut::<f32>::new(
                        sample_rate.0 as usize,
                        def_conf.sample_rate().0 as usize,
                        sample_rate.0 as usize / 60,
                        2,
                    )
                    .unwrap(),
                ),
            )
        };

        let config = cpal::StreamConfig {
            channels,
            sample_rate: output_sample_rate,
            buffer_size: cpal::BufferSize::Default,
        };

        // Limiting the number of samples in the buffer is better to minimize
        // audio delay in playback, this is because game speed
        // does not 100% match audio playing speed (44100Hz).
        // The buffer holds only audio for 1/4 second, which is good enough for delays,
        // It can be reduced more, but it might cause noise(?) for slower machines
        // or if any CPU intensive process started while the emulator is running
        let buffer = HeapRb::new(output_sample_rate.0 as usize / 2);
        let (buffer_producer, mut buffer_consumer) = buffer.split();

        let output_data_fn = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            for sample in data {
                *sample = buffer_consumer.pop().unwrap_or(0.);
            }
        };

        let output_stream = output_device
            .build_output_stream(&config, output_data_fn, Self::err_fn, None)
            .expect("[Audio] Failed to build an output audio stream");

        self.backend = Some(Backend {
            buffer_producer,
            output_stream,
            resampler,
            resample_buffer: Vec::new(),
        });

        true
    }

    pub fn deinit(&mut self) {
        if let Some(backend) = self.backend.as_mut() {
            backend.output_stream.pause().unwrap();
            self.backend = None;
        }
    }

    pub fn buffered(&self) -> i32 {
        if let Some(backend) = self.backend.as_ref() {
            return backend.buffer_producer.len() as i32 / 4;
        }

        0
    }

    pub fn desired_buffer(&self) -> i32 {
        SAMPLES_HIGH * 4
    }

    pub fn play_buffer(&mut self, buf: &[u8]) {
        if let Some(backend) = self.backend.as_mut() {
            // helper method to split channels into separate vectors
            fn read_frames(inbuffer: &Vec<f32>, n_frames: usize, channels: usize) -> Vec<Vec<f32>> {
                let mut wfs = Vec::with_capacity(channels);
                for _chan in 0..channels {
                    wfs.push(Vec::with_capacity(n_frames));
                }
                let mut value: f32;
                let mut inbuffer_iter = inbuffer.iter();
                for _ in 0..n_frames {
                    for wf in wfs.iter_mut().take(channels) {
                        value = *inbuffer_iter.next().unwrap();
                        wf.push(value);
                    }
                }
                wfs
            }

            /// Helper to merge channels into a single vector
            fn write_frames(waves: Vec<Vec<f32>>, outbuffer: &mut Vec<f32>, channels: usize) {
                let nbr = waves[0].len();
                for frame in 0..nbr {
                    for chan in 0..channels {
                        let value = waves[chan][frame];
                        outbuffer.push(value);
                    }
                }
            }

            // transform the buffer into a vector of f32 samples
            // buffer data is of 2 channels, 16 bit samples
            let mut cursor = std::io::Cursor::new(buf);
            let mut samples = Vec::with_capacity(buf.len() / 2);
            while let Ok(sample) = cursor.read_i16::<LittleEndian>() {
                samples.push(sample as f32 / 32768.0);
            }

            if let Some(resampler) = &mut backend.resampler {
                backend.resample_buffer.extend_from_slice(&samples);

                loop {
                    let frames = resampler.input_frames_next();
                    if backend.resample_buffer.len() < frames * 2 {
                        return;
                    }

                    // only read the needed frames
                    let input = read_frames(&mut backend.resample_buffer, frames, 2);
                    let output = resampler.process(&input, None).unwrap();

                    let mut resampled = Vec::with_capacity(output[0].len() * 2);
                    write_frames(output, &mut resampled, 2);

                    backend.buffer_producer.push_slice(&resampled);
                    backend.resample_buffer = backend.resample_buffer.split_off(frames * 2);
                }
            } else {
                backend.buffer_producer.push_slice(&samples);
            }
        }
    }

    fn err_fn(err: cpal::StreamError) {
        eprintln!("[Audio] an error occurred on audio stream: {}", err);
    }
}

// MARK: - C API

#[no_mangle]
pub extern "C" fn HLXAudioPlayerInit(sample_rate: u32, channels: u16) -> bool {
    return helix!().audio_player.init(sample_rate, channels);
}

#[no_mangle]
pub extern "C" fn HLXAudioPlayerDeinit() {
    helix!().audio_player.deinit();
}

#[no_mangle]
pub extern "C" fn HLXAudioPlayerGetBuffered() -> i32 {
    return helix!().audio_player.buffered();
}

#[no_mangle]
pub extern "C" fn HLXAudioPlayerGetDesiredBuffered() -> i32 {
    return helix!().audio_player.desired_buffer();
}

#[no_mangle]
pub extern "C" fn HLXAudioPlayerPlayBuffer(buf: *const u8, len: usize) {
    let buf = unsafe { std::slice::from_raw_parts(buf, len) };
    helix!().audio_player.play_buffer(buf);
}

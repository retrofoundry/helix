mod writer;

use crate::HELIX;
use crate::audio::writer::Writer;
use cpal::{
    SizedSample, FromSample,
    traits::{DeviceTrait, HostTrait, StreamTrait}
};

pub struct AudioPlayer {
    config: Option<Config>,
}

pub struct Config {
    device: cpal::Device,
    output_format: cpal::SampleFormat,
    config: cpal::SupportedStreamConfig,
}

impl AudioPlayer {
    pub fn new() -> Self {
        AudioPlayer {
            config: Option::None,
        }
    }

    pub fn init(&mut self) -> bool {
        let host = cpal::default_host();

        let device = host
            .default_output_device()
            .expect("failed to find a default output device");

        let config = device
            .default_output_config()
            .expect("failed to get default output config");

        self.config = Some(Config {
            device,
            output_format: config.sample_format(),
            config,
        });

        true
    }

    pub fn buffered(&self) -> i32 {
        0
    }

    pub fn desired_buffer(&self) -> i32 {
        2480
    }

    pub fn play_buffer(&self, buf: &[u8]) {
        if let Some(config) = &self.config {
            // take ownership of the buffer
            let buffer = buf.to_vec();

            match config.output_format {
                cpal::SampleFormat::F32 => run::<f32>(buffer, &config.device, &config.config),
                cpal::SampleFormat::I16 => run::<i16>(buffer, &config.device, &config.config),
                cpal::SampleFormat::U16 => run::<u16>(buffer, &config.device, &config.config),
                sample_format => panic!("Unsupported sample format '{sample_format}'"),
            }
        }
    }
}

fn run<T>(buffer: Vec<u8>, device: &cpal::Device, config: &cpal::SupportedStreamConfig)
where
    T: SizedSample + FromSample<f32>
{
    let config = cpal::StreamConfig {
        channels: config.channels(),
        sample_rate: config.sample_rate(),
        buffer_size: cpal::BufferSize::Default,
    };

    let mut writer = Writer {
        buffer,
        device_sample_rate: config.sample_rate.0,
        device_channels: config.channels as usize,
    };

    let stream = device.build_output_stream(
        &config,
        move |data: &mut [T], _| {
            if let Err(e) = writer.write_to(data) {
                println!("failed to write data: {}", e);
            }
        },
        move |err| {
            println!("audio output error: {}", err);
        },
        None,
    ).expect("failed to build output stream");

    stream.play().expect("failed to play stream");
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

use cpal::{SizedSample, FromSample};

pub struct Writer {
    // Buffer of input data
    pub buffer: Vec<u8>,
    // Sample rate expected to be written to the device.
    pub device_sample_rate: u32,
    // Number of channels in the device.
    pub device_channels: usize,
}

impl Writer {
    pub fn write_to<T>(&mut self, out: &mut [T]) -> anyhow::Result<()>
    where
        T: SizedSample + FromSample<f32>
    {
        Ok(())
    }
}
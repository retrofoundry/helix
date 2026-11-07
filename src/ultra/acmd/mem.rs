use super::dmem::{Dmem, HalfwordOrder};
use super::SEGMENT_TABLE;

#[derive(Debug, PartialEq, Eq)]
pub struct Dma {
    pub length: usize,
    pub rows: usize,
    pub skip: usize,
}

impl Dma {
    pub fn decode(value: u32) -> Self {
        Self {
            length: ((value as usize & 4095) | 7) + 1,
            rows: ((value as usize >> 12) & 255) + 1,
            skip: (value as usize >> 20) & 4095,
        }
    }

    pub fn transfer<O: HalfwordOrder>(
        &self,
        dmem: &mut Dmem<O>,
        ram: &mut [u8],
        address: u32,
        dram: u32,
        write: bool,
    ) -> Result<(), String> {
        // RSP DMA truncates both starting addresses to multiples of eight.
        let mut address = address & 0xff8;
        let mut dram = (dram & 0xfffff8) as usize;
        let end = dram + self.rows * self.length + (self.rows - 1) * self.skip;
        if end > ram.len() {
            return Err(format!("audio DMA exceeds RDRAM: {dram:#x}..{end:#x}"));
        }
        for _ in 0..self.rows {
            for i in 0..self.length {
                let offset = address + i as u32;
                if write {
                    ram[dram + i] = dmem.raw(offset);
                } else {
                    dmem.set_raw(offset, ram[dram + i]);
                }
            }
            address = (address + self.length as u32) & 4095;
            dram += self.length + self.skip;
        }
        Ok(())
    }
}

pub(super) trait Memory {
    fn resolve<O: HalfwordOrder>(&mut self, dmem: &Dmem<O>, word: u64) -> Result<u32, String>;
    fn dma<O: HalfwordOrder>(
        &mut self,
        dmem: &mut Dmem<O>,
        address: u32,
        dram: u32,
        value: u32,
        write: bool,
    ) -> Result<(), String>;
}

impl Memory for [u8] {
    fn resolve<O: HalfwordOrder>(&mut self, dmem: &Dmem<O>, word: u64) -> Result<u32, String> {
        let word = word as u32;
        Ok((word & 0xffffff).wrapping_add(dmem.word(SEGMENT_TABLE + (word >> 24) * 4)))
    }

    fn dma<O: HalfwordOrder>(
        &mut self,
        dmem: &mut Dmem<O>,
        address: u32,
        dram: u32,
        value: u32,
        write: bool,
    ) -> Result<(), String> {
        Dma::decode(value).transfer(dmem, self, address, dram, write)
    }
}

//! Classic US aspMain, with caller-owned state and console or native memory.

mod classic;
mod dmem;
mod mem;
pub mod native;
mod vector;

use dmem::{Be, Dmem, HalfwordOrder};
use mem::Memory;

const SEGMENT_TABLE: u32 = 0x320;
const AUDIO_STRUCT: u32 = 0x360;
const NEXT_TASK_ENTRY: u32 = 0x380;
const ADPCM_TABLE: u32 = 0x4c0;
const DMEM_BASE: u32 = 0x5c0;
const TMP_DATA: u32 = 0xf90;
const TASK_DESCRIPTOR: u32 = 0xfc0;
const AUDIO_IN_BUF: u32 = 0x0;
const AUDIO_OUT_BUF: u32 = 0x2;
const AUDIO_COUNT: u32 = 0x4;
const AUDIO_VOL_LEFT: u32 = 0x6;
const AUDIO_VOL_RIGHT: u32 = 0x8;
const AUDIO_AUX_BUF0: u32 = 0xa;
const AUDIO_AUX_BUF1: u32 = 0xc;
const AUDIO_AUX_BUF2: u32 = 0xe;
const AUDIO_LOOP_VALUE: u32 = 0x10;
const AUDIO_TARGET_LEFT: u32 = 0x10;
const AUDIO_TARGET_RIGHT: u32 = 0x16;
const AUDIO_DRY_GAIN: u32 = 0x1c;
const AUDIO_WET_GAIN: u32 = 0x1e;

const TASK_DATA_SIZE: usize = 27;
const TASK_DATA: usize = 28;
const NEXT_COMMAND: usize = 29;
const CHUNK_REMAINING: usize = 30;

pub struct Context<O: HalfwordOrder = Be> {
    pub dmem: Dmem<O>,
    pub vectors: [[u16; 8]; 32],
    pub vco: u16,
    pub vcc: u16,
    pub vce: u8,
    pub scalar: [u32; 32],
}

impl<O: HalfwordOrder> Default for Context<O> {
    fn default() -> Self {
        Self {
            dmem: Dmem::default(),
            vectors: [[0; 8]; 32],
            vco: 0,
            vcc: 0,
            vce: 0,
            scalar: [0; 32],
        }
    }
}

impl Context<Be> {
    #[allow(dead_code, reason = "Used by the offline console oracle bridge.")]
    pub fn console_task(&mut self, ram: &mut [u8], task: &[u32; 16]) -> Result<(), String> {
        for (i, &word) in task.iter().enumerate() {
            self.dmem.set_word(TASK_DESCRIPTOR + i as u32 * 4, word);
        }
        self.dma(ram, 0, task[6], task[7].wrapping_sub(1), false)?;
        self.scalar[TASK_DATA] = self.dmem.word(TASK_DESCRIPTOR + 0x30);
        self.scalar[TASK_DATA_SIZE] = self.dmem.word(TASK_DESCRIPTOR + 0x34);
        self.stage(ram)?;
        self.dmem.set_word(SEGMENT_TABLE, 0);
        loop {
            let address = self.scalar[NEXT_COMMAND];
            let w0 = self.dmem.word(address);
            let w1 = self.dmem.word(address + 4);
            self.scalar[TASK_DATA] = self.scalar[TASK_DATA].wrapping_add(8);
            self.scalar[TASK_DATA_SIZE] = self.scalar[TASK_DATA_SIZE].wrapping_sub(8);
            self.scalar[NEXT_COMMAND] += 8;
            self.scalar[CHUNK_REMAINING] = self.scalar[CHUNK_REMAINING].wrapping_sub(8);
            self.command(ram, w0, u64::from(w1))?;
            if self.scalar[CHUNK_REMAINING] as i32 > 0 {
                continue;
            }
            if self.scalar[TASK_DATA_SIZE] as i32 <= 0 {
                break;
            }
            self.stage(ram)?;
        }
        Ok(())
    }

    fn stage(&mut self, ram: &mut [u8]) -> Result<(), String> {
        let count = self.scalar[TASK_DATA_SIZE].min(ADPCM_TABLE - NEXT_TASK_ENTRY);
        self.dma(
            ram,
            NEXT_TASK_ENTRY,
            self.scalar[TASK_DATA],
            count.wrapping_sub(1),
            false,
        )?;
        self.scalar[CHUNK_REMAINING] = count;
        self.scalar[NEXT_COMMAND] = NEXT_TASK_ENTRY;
        Ok(())
    }
}

impl<O: HalfwordOrder> Context<O> {
    fn dma(
        &mut self,
        ram: &mut (impl Memory + ?Sized),
        address: u32,
        dram: u32,
        value: u32,
        write: bool,
    ) -> Result<(), String> {
        ram.dma(&mut self.dmem, address, dram, value, write)
    }

    fn buffer(&self, offset: u32) -> u32 {
        u32::from(self.dmem.h16(AUDIO_STRUCT + offset))
    }

    fn signed_buffer(&self, offset: u32) -> u32 {
        self.dmem.h16(AUDIO_STRUCT + offset) as i16 as u32
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod oracle_tests;

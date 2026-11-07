use super::mem::Memory;
use super::vector::{mix, sat, signed};
use super::{Context, HalfwordOrder};
use super::{
    ADPCM_TABLE, AUDIO_AUX_BUF0, AUDIO_AUX_BUF1, AUDIO_AUX_BUF2, AUDIO_COUNT, AUDIO_DRY_GAIN,
    AUDIO_IN_BUF, AUDIO_LOOP_VALUE, AUDIO_OUT_BUF, AUDIO_STRUCT, AUDIO_TARGET_LEFT,
    AUDIO_TARGET_RIGHT, AUDIO_VOL_LEFT, AUDIO_VOL_RIGHT, AUDIO_WET_GAIN, DMEM_BASE, SEGMENT_TABLE,
    TMP_DATA,
};

mod adpcm;
mod envelope;
mod resample;

const SPNOOP: u32 = 0x00;
const ADPCM: u32 = 0x01;
const CLEARBUFF: u32 = 0x02;
const ENVMIXER: u32 = 0x03;
const LOADBUFF: u32 = 0x04;
const RESAMPLE: u32 = 0x05;
const SAVEBUFF: u32 = 0x06;
const SEGMENT: u32 = 0x07;
const SETBUFF: u32 = 0x08;
const SETVOL: u32 = 0x09;
const DMEMMOVE: u32 = 0x0a;
const LOADADPCM: u32 = 0x0b;
const MIXER: u32 = 0x0c;
const INTERLEAVE: u32 = 0x0d;
const POLEF: u32 = 0x0e;
const SETLOOP: u32 = 0x0f;

impl<O: HalfwordOrder> Context<O> {
    pub(super) fn command(
        &mut self,
        ram: &mut (impl Memory + ?Sized),
        w0: u32,
        word: u64,
    ) -> Result<(), String> {
        let w1 = word as u32;
        let flags = (w0 >> 16) as u8;
        let count = w1 & 65535;
        // aspMain's dispatch mask ignores the opcode's high bit.
        let opcode = (w0 >> 24) & 127;
        match opcode {
            SPNOOP => {}
            ADPCM => self.adpcm(ram, flags, word)?,
            CLEARBUFF => {
                if count != 0 {
                    self.vectors[1] = [0; 8];
                    let start = DMEM_BASE + (w0 & 65535);
                    for offset in (0..count).step_by(16) {
                        self.store_raw(1, 0, start + offset, 8);
                        self.store_raw(1, 0, start + offset + 8, 8);
                    }
                }
            }
            ENVMIXER => self.envelope(ram, flags, word)?,
            LOADBUFF | SAVEBUFF => {
                let count = self.buffer(AUDIO_COUNT);
                if count != 0 {
                    let write = opcode == SAVEBUFF;
                    let address = ram.resolve(&self.dmem, word)?;
                    self.dma(
                        ram,
                        self.buffer(if write { AUDIO_OUT_BUF } else { AUDIO_IN_BUF }),
                        address,
                        count - 1,
                        write,
                    )?;
                }
            }
            RESAMPLE => self.resample(ram, flags, w0 as u16, word)?,
            SEGMENT => self
                .dmem
                .set_word(SEGMENT_TABLE + 4 * (w1 >> 24), w1 & 0xffffff),
            SETBUFF => {
                let input = w0.wrapping_add(DMEM_BASE) as u16;
                let output = (w1 >> 16).wrapping_add(DMEM_BASE) as u16;
                if flags & 8 != 0 {
                    self.dmem.set_h16(AUDIO_STRUCT + AUDIO_AUX_BUF0, input);
                    self.dmem.set_h16(AUDIO_STRUCT + AUDIO_AUX_BUF1, output);
                    self.dmem.set_h16(
                        AUDIO_STRUCT + AUDIO_AUX_BUF2,
                        w1.wrapping_add(DMEM_BASE) as u16,
                    );
                } else {
                    self.dmem.set_h16(AUDIO_STRUCT + AUDIO_IN_BUF, input);
                    self.dmem.set_h16(AUDIO_STRUCT + AUDIO_OUT_BUF, output);
                    self.dmem.set_h16(AUDIO_STRUCT + AUDIO_COUNT, w1 as u16);
                }
            }
            SETVOL => {
                if flags & 8 != 0 {
                    self.dmem.set_h16(AUDIO_STRUCT + AUDIO_DRY_GAIN, w0 as u16);
                    self.dmem.set_h16(AUDIO_STRUCT + AUDIO_WET_GAIN, w1 as u16);
                } else if flags & 4 != 0 {
                    self.dmem.set_h16(
                        if flags & 2 != 0 {
                            AUDIO_STRUCT + AUDIO_VOL_LEFT
                        } else {
                            AUDIO_STRUCT + AUDIO_VOL_RIGHT
                        },
                        w0 as u16,
                    );
                } else {
                    let address = if flags & 2 != 0 {
                        AUDIO_STRUCT + AUDIO_TARGET_LEFT
                    } else {
                        AUDIO_STRUCT + AUDIO_TARGET_RIGHT
                    };
                    self.dmem.set_h16(address, w0 as u16);
                    self.dmem.set_word(address + 2, w1);
                }
            }
            DMEMMOVE => {
                let source = DMEM_BASE + (w0 & 65535);
                let dest = DMEM_BASE + (w1 >> 16);
                for offset in (0..count).step_by(16) {
                    self.load(1, 0, source + offset, 8);
                    self.load(2, 0, source + offset + 8, 8);
                    self.store(1, 0, dest + offset, 8);
                    self.store(2, 0, dest + offset + 8, 8);
                }
            }
            LOADADPCM => {
                let address = ram.resolve(&self.dmem, word)?;
                self.dma(
                    ram,
                    ADPCM_TABLE,
                    address,
                    (w0 & 65535).wrapping_sub(1),
                    false,
                )?;
            }
            MIXER => self.mixer(w0 as u16, w1),
            INTERLEAVE => self.interleave(w1),
            POLEF => self.pole(ram, flags, w0 as u16, word)?,
            SETLOOP => {
                let address = ram.resolve(&self.dmem, word)?;
                self.dmem.set_word(AUDIO_STRUCT + AUDIO_LOOP_VALUE, address);
            }
            op => return Err(format!("unsupported classic audio command {op:#x}")),
        }
        Ok(())
    }

    fn interleave(&mut self, word: u32) {
        let count = self.buffer(AUDIO_COUNT);
        let mut left = DMEM_BASE + (word >> 16);
        let mut right = DMEM_BASE + (word & 65535);
        let mut output = self.buffer(AUDIO_OUT_BUF);
        for _ in (0..count).step_by(16) {
            self.load_quad(1, left);
            self.load_quad(2, right);
            for i in 0..8 {
                self.dmem
                    .set_h16(output + i * 4, self.vectors[1][i as usize]);
                self.dmem
                    .set_h16(output + i * 4 + 2, self.vectors[2][i as usize]);
            }
            left += 16;
            right += 16;
            output += 32;
        }
    }

    fn mixer(&mut self, gain: u16, word: u32) {
        self.load_quad(31, 0);
        let mut count = self.buffer(AUDIO_COUNT) as i32;
        if count == 0 {
            return;
        }
        let mut source = DMEM_BASE + (word >> 16);
        let mut dest = DMEM_BASE + (word & 65535);
        self.vectors[30][0] = gain;
        self.load_quad(27, dest);
        self.load_quad(29, source);
        self.load_quad(26, dest + 16);
        self.load_quad(28, source + 16);
        loop {
            for i in 0..8 {
                self.vectors[27][i] = mix(
                    self.vectors[27][i],
                    self.vectors[29][i],
                    gain,
                    self.vectors[31][6],
                );
                self.vectors[26][i] = mix(
                    self.vectors[26][i],
                    self.vectors[28][i],
                    gain,
                    self.vectors[31][6],
                );
            }
            source += 32;
            self.store_quad(27, dest);
            self.load_quad(29, source);
            self.load_quad(28, source + 16);
            self.store_quad(26, dest + 16);
            dest += 32;
            self.load_quad(27, dest);
            self.load_quad(26, dest + 16);
            count -= 32;
            if count <= 0 {
                break;
            }
        }
    }

    fn pole(
        &mut self,
        ram: &mut (impl Memory + ?Sized),
        flags: u8,
        gain: u16,
        word: u64,
    ) -> Result<(), String> {
        self.load_quad(31, 0);
        for reg in [28, 17, 18, 19] {
            self.vectors[reg] = [0; 8];
        }
        let mut count = self.buffer(AUDIO_COUNT) as i32;
        let mut source = self.buffer(AUDIO_IN_BUF);
        let mut dest = self.buffer(AUDIO_OUT_BUF);
        if count == 0 {
            return Ok(());
        }
        self.vectors[31][5] = gain;
        let scale = gain.wrapping_mul(4);
        let state = ram.resolve(&self.dmem, word)?;
        for reg in 20..=23 {
            self.vectors[reg] = [0; 8];
        }
        // INIT clears only the first word; the feedback samples at +4 and +6 survive.
        self.store(28, 0, TMP_DATA, 4);
        if flags & 1 == 0 {
            self.dma(ram, TMP_DATA, state, 7, false)?;
        }
        self.vectors[14][0] = 4;
        self.load_quad(24, ADPCM_TABLE + 16);
        self.vectors[16] = self.vectors[24].map(|v| ((signed(v) * i64::from(scale)) >> 16) as u16);
        self.load(28, 8, TMP_DATA, 8);
        self.store_quad(16, ADPCM_TABLE + 16);
        self.load_quad(25, ADPCM_TABLE);
        for i in 0..7 {
            self.load_right(23 - i, ADPCM_TABLE + 30 - i as u32 * 2);
        }
        self.load(30, 0, source, 16);
        loop {
            let input = self.vectors[30];
            let history = self.vectors[28];
            let totals: [i32; 8] = std::array::from_fn(|i| {
                let mut sum = signed(gain) * signed(input[i])
                    + signed(self.vectors[25][i]) * signed(history[6])
                    + signed(self.vectors[24][i]) * signed(history[7]);
                for (j, &sample) in input[..i].iter().enumerate() {
                    sum += signed(self.vectors[23 - j][i]) * signed(sample);
                }
                sum as i32
            });
            self.vectors[26] = totals.map(|v| v as u16);
            self.vectors[16] = totals.map(|v| (v as u16).wrapping_mul(4));
            self.vectors[28] = totals.map(|v| sat(i64::from(v) >> 14));
            source += 16;
            self.load(30, 0, source, 16);
            self.store(28, 0, dest, 16);
            dest += 16;
            count -= 16;
            if count <= 0 {
                break;
            }
        }
        self.dma(ram, dest - 8, state, 7, true)
    }
}

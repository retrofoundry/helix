use super::super::mem::Memory;
use super::super::vector::{fractional, low, sat, signed};
use super::super::{AUDIO_COUNT, AUDIO_IN_BUF, AUDIO_OUT_BUF, TMP_DATA};
use super::{Context, HalfwordOrder};

const RESAMPLE_INPUT_REG: usize = 17;
const RESAMPLE_COEFFICIENT_REG: usize = 9;

impl<O: HalfwordOrder> Context<O> {
    fn resample_addresses(&mut self) {
        let phase = i64::from(self.vectors[23][7]);
        let integral = signed(self.vectors[22][7]) << 16;
        for i in 0..8 {
            let one = self.vectors[31][i];
            let accumulator = signed(one) * (phase + integral)
                + signed(self.vectors[25][i]) * i64::from(self.vectors[18][4]);
            self.vectors[22][i] = sat(accumulator >> 16);
            self.vectors[23][i] = low(accumulator + i64::from(one) * signed(self.vectors[30][0]));
            self.vectors[21][i] = low(i64::from(one) * signed(self.vectors[18][2])
                + i64::from(self.vectors[22][i]) * signed(self.vectors[30][2]));
            let phase_offset =
                (u32::from(self.vectors[23][i]) * u32::from(self.vectors[18][5])) >> 16;
            self.vectors[17][i] = low(i64::from(phase_offset) * signed(self.vectors[30][4])
                + i64::from(one) * signed(self.vectors[18][3]));
        }
    }

    fn resample_scratch(&mut self) {
        self.store_quad(21, TMP_DATA + 0x20);
        self.store_quad(17, TMP_DATA + 0x30);
        for i in 0..8 {
            self.scalar[RESAMPLE_INPUT_REG - i] =
                self.dmem.h16(TMP_DATA + 0x20 + i as u32 * 2) as i16 as u32;
            self.scalar[RESAMPLE_COEFFICIENT_REG - i] =
                self.dmem.h16(TMP_DATA + 0x30 + i as u32 * 2) as i16 as u32;
        }
    }

    pub(super) fn resample(
        &mut self,
        ram: &mut (impl Memory + ?Sized),
        flags: u8,
        pitch: u16,
        word: u64,
    ) -> Result<(), String> {
        let mut source = self.signed_buffer(AUDIO_IN_BUF);
        let mut dest = self.signed_buffer(AUDIO_OUT_BUF);
        let mut count = self.buffer(AUDIO_COUNT) as i16 as i32;
        let state = ram.resolve(&self.dmem, word)?;
        self.dmem.set_word(TMP_DATA + 0x40, state);
        if flags & 1 != 0 {
            self.dmem.set_h16(TMP_DATA + 8, 0);
            self.vectors[16] = [0; 8];
            self.store(16, 0, TMP_DATA, 8);
        } else {
            self.dma(ram, TMP_DATA, state, 31, false)?;
        }
        if flags & 2 != 0 {
            let correction = self.dmem.h16(TMP_DATA + 10) as i16 as u32;
            self.load_quad(3, TMP_DATA + 0x10);
            self.store(3, 0, source.wrapping_sub(16), 16);
            source = source.wrapping_sub(correction);
        }
        source = source.wrapping_sub(8);
        self.load(23, 14, TMP_DATA + 8, 2);
        self.load(16, 0, TMP_DATA, 8);
        self.store(16, 0, source, 8);
        self.vectors[18][2] = source as u16;
        self.vectors[18][3] = 0xc0;
        self.vectors[18][4] = pitch;
        self.vectors[18][5] = 64;
        self.load_quad(31, 0x50);
        self.load_quad(25, 0x40);
        for i in 0..8 {
            let value = sat(signed(self.vectors[25][i])
                - signed(self.vectors[31][i])
                - i64::from((self.vco >> i) & 1));
            self.vectors[25][i] = sat(signed(value) - signed(self.vectors[31][i]));
        }
        self.vco = 0;
        for (i, reg) in [30, 29, 28, 27, 26, 24].into_iter().enumerate() {
            self.load_quad(reg, 0x60 + i as u32 * 16);
        }
        self.vectors[22] = [0; 8];
        self.resample_addresses();
        self.load_quad(25, 0x40);
        self.resample_scratch();
        self.store(23, 7, TMP_DATA + 8, 2);
        loop {
            for i in 0..4 {
                let samples = 16 - 2 * i;
                let coefficients = samples - 1;
                self.load(samples, 0, self.scalar[RESAMPLE_INPUT_REG - i], 8);
                self.load(samples, 8, self.scalar[RESAMPLE_INPUT_REG - 4 - i], 8);
                self.load(
                    coefficients,
                    0,
                    self.scalar[RESAMPLE_COEFFICIENT_REG - i],
                    8,
                );
                self.load(
                    coefficients,
                    8,
                    self.scalar[RESAMPLE_COEFFICIENT_REG - 4 - i],
                    8,
                );
            }
            self.resample_addresses();
            for i in 0..4 {
                let samples = self.vectors[16 - 2 * i];
                let coefficients = self.vectors[15 - 2 * i];
                let products: [u16; 8] =
                    std::array::from_fn(|j| fractional(samples[j], coefficients[j]));
                let pairs: [u16; 8] =
                    std::array::from_fn(|j| sat(signed(products[j]) + signed(products[j | 1])));
                self.vectors[8 - i] =
                    std::array::from_fn(|j| sat(signed(pairs[j]) + signed(pairs[(j & 4) + 2])));
            }
            self.resample_scratch();
            self.vectors[4] = std::array::from_fn(|i| {
                let group = i & 4;
                low((0..4)
                    .map(|j| {
                        i64::from(self.vectors[29 - j][i]) * signed(self.vectors[8 - j][group])
                    })
                    .sum())
            });
            self.store_quad(4, dest);
            count -= 16;
            if count <= 0 {
                break;
            }
            dest = dest.wrapping_add(16);
        }
        self.store(23, 0, TMP_DATA + 8, 2);
        let next = self.scalar[RESAMPLE_INPUT_REG];
        self.load(16, 0, next, 8);
        self.store(16, 0, TMP_DATA, 8);
        let end = next.wrapping_add(8);
        let input = self.signed_buffer(AUDIO_IN_BUF);
        let residue = end.wrapping_sub(input) & 15;
        let aligned = end.wrapping_sub(residue);
        let correction = if residue == 0 { 0 } else { 16 - residue };
        self.dmem.set_h16(TMP_DATA + 10, correction as u16);
        self.load(3, 0, aligned, 16);
        self.store_quad(3, TMP_DATA + 0x10);
        self.dma(ram, TMP_DATA, self.dmem.word(TMP_DATA + 0x40), 31, true)
    }
}

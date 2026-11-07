use super::super::mem::Memory;
use super::super::vector::{low, sat, signed};
use super::super::{
    ADPCM_TABLE, AUDIO_COUNT, AUDIO_IN_BUF, AUDIO_LOOP_VALUE, AUDIO_OUT_BUF, AUDIO_STRUCT,
};
use super::{Context, HalfwordOrder};

impl<O: HalfwordOrder> Context<O> {
    fn adpcm_header(&mut self, source: u32) -> (u32, i32) {
        let header = self.dmem.raw(source);
        let book = ADPCM_TABLE + u32::from(header & 15) * 32;
        let shift = 12 - i32::from(header >> 4);
        self.vectors[22][0] = (32768u32 >> ((shift - 1) as u32 & 31)) as u16;
        for i in 0..8 {
            self.vectors[3][i] = self.vectors[25][i] & self.vectors[1][0];
            self.vectors[4][i] = self.vectors[24][i] & self.vectors[1][1];
            self.vectors[5][i] = self.vectors[25][i] & self.vectors[1][2];
            self.vectors[6][i] = self.vectors[24][i] & self.vectors[1][3];
        }
        (book, shift)
    }

    fn adpcm_book(&mut self, address: u32) {
        self.load_quad(21, address);
        self.load_quad(20, address + 16);
        for i in 0..7 {
            self.load_right(19 - i, address + 30 - i as u32 * 2);
        }
    }

    fn adpcm_group(&self, residual: [u16; 8], history: [u16; 8]) -> [i32; 8] {
        std::array::from_fn(|i| {
            let mut total = signed(self.vectors[21][i]) * signed(history[6])
                + signed(self.vectors[20][i]) * signed(history[7])
                + signed(residual[i]) * signed(self.vectors[31][5]);
            for (j, &sample) in residual[..i].iter().enumerate() {
                total += signed(self.vectors[19 - j][i]) * signed(sample);
            }
            total as i32
        })
    }

    pub(super) fn adpcm(
        &mut self,
        ram: &mut (impl Memory + ?Sized),
        flags: u8,
        word: u64,
    ) -> Result<(), String> {
        self.load_quad(31, 0);
        for reg in [27, 25, 24, 13, 14, 15, 16, 17, 18, 19] {
            self.vectors[reg] = [0; 8];
        }
        let mut source = self.buffer(AUDIO_IN_BUF);
        let mut dest = self.buffer(AUDIO_OUT_BUF);
        let mut count = self.buffer(AUDIO_COUNT) as i32;
        let state = ram.resolve(&self.dmem, word)?;
        self.store_quad(27, dest);
        self.store_quad(27, dest + 16);
        if flags & 1 == 0 {
            let history = if flags & 2 != 0 {
                self.dmem.word(AUDIO_STRUCT + AUDIO_LOOP_VALUE)
            } else {
                state
            };
            self.dma(ram, dest, history, 31, false)?;
        }
        self.load(25, 0, 0x30, 8);
        self.load(24, 8, 0x30, 8);
        self.load(23, 0, 0x38, 8);
        self.load(23, 8, 0x38, 8);
        self.load_quad(27, dest + 16);
        dest += 32;
        self.load_raw(1, 0, source + 1, 8);
        if count != 0 {
            let (book, mut shift) = self.adpcm_header(source);
            self.adpcm_book(book);
            loop {
                for (dest_reg, a, b) in [(30, 3, 4), (29, 5, 6)] {
                    for i in 0..8 {
                        let total = i64::from(self.vectors[a][i]) * signed(self.vectors[23][i])
                            + i64::from(self.vectors[b][i]) * signed(self.vectors[23][i]);
                        let residual = low(total);
                        self.vectors[dest_reg][i] = if shift > 0 {
                            ((signed(residual) * i64::from(self.vectors[22][0])) >> 16) as u16
                        } else {
                            residual
                        };
                    }
                }
                source += 9;
                self.load_raw(1, 0, source + 1, 8);
                let (next_book, next_shift) = self.adpcm_header(source);
                let scale = signed(self.vectors[31][4]);
                let first = self.adpcm_group(self.vectors[30], self.vectors[27]);
                self.vectors[28] = first.map(|v| sat((i64::from(v) * scale) >> 16));
                let second = self.adpcm_group(self.vectors[29], self.vectors[28]);
                self.vectors[26] = second.map(|v| v as u16);
                self.vectors[2] = second.map(|v| (i64::from(v as u16) * scale) as u16);
                self.vectors[27] = second.map(|v| sat((i64::from(v) * scale) >> 16));
                self.adpcm_book(next_book);
                self.store(28, 0, dest, 16);
                self.store(27, 0, dest + 16, 16);
                dest += 32;
                count -= 32;
                shift = next_shift;
                if count <= 0 {
                    break;
                }
            }
        }
        self.dma(ram, dest - 32, state, 31, true)
    }
}

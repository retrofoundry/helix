use super::super::mem::Memory;
use super::super::vector::{fractional, low, mix, sat, signed, wrap48};
use super::super::{
    AUDIO_AUX_BUF0, AUDIO_AUX_BUF1, AUDIO_AUX_BUF2, AUDIO_COUNT, AUDIO_IN_BUF, AUDIO_OUT_BUF,
    AUDIO_STRUCT, AUDIO_TARGET_LEFT, AUDIO_VOL_LEFT, AUDIO_VOL_RIGHT, TMP_DATA,
};
use super::{Context, HalfwordOrder};

impl<O: HalfwordOrder> Context<O> {
    fn envelope_initial(&mut self, integral: usize, fraction: usize, volume: u16, rate: usize) {
        let high = signed(self.vectors[24][rate]);
        let low_rate = i64::from(self.vectors[24][rate + 1]);
        let product = wrap48(signed(volume) * ((high << 16) + low_rate));
        let delta = sat(signed(sat(product >> 16)) - signed(volume));
        let fractional = low(product);
        for i in 0..8 {
            let ramp = i64::from(self.vectors[30][i]);
            let acc = wrap48(
                ((ramp * i64::from(fractional)) >> 16)
                    + ramp * signed(delta)
                    + ((signed(volume) * signed(self.vectors[31][i])) << 16),
            );
            self.vectors[integral][i] = sat(acc >> 16);
            self.vectors[fraction][i] = low(acc);
        }
        self.vco = 0;
    }

    fn envelope_advance(&mut self, integral: usize, fraction: usize, rate: usize) {
        let high = signed(self.vectors[24][rate]);
        let lo = i64::from(self.vectors[24][rate + 1]);
        for i in 0..8 {
            let v = signed(self.vectors[integral][i]);
            let f = i64::from(self.vectors[fraction][i]);
            let partial = wrap48(((f * lo) >> 16) + v * lo + f * high);
            let acc = wrap48(partial + ((v * high) << 16));
            self.vectors[23][i] = low(partial);
            self.vectors[integral][i] = sat(acc >> 16);
            self.vectors[fraction][i] = low(acc);
        }
    }

    fn envelope_mix(&mut self, volume: usize, main: usize, wet: usize) {
        for i in 0..8 {
            self.vectors[16][i] = fractional(self.vectors[volume][i], self.vectors[24][6]);
            self.vectors[15][i] = fractional(self.vectors[volume][i], self.vectors[24][7]);
            self.vectors[main][i] = mix(
                self.vectors[main][i],
                self.vectors[17][i],
                self.vectors[16][i],
                self.vectors[10][6],
            );
            self.vectors[wet][i] = mix(
                self.vectors[wet][i],
                self.vectors[17][i],
                self.vectors[15][i],
                self.vectors[10][6],
            );
        }
    }

    pub(super) fn envelope(
        &mut self,
        ram: &mut (impl Memory + ?Sized),
        flags: u8,
        word: u64,
    ) -> Result<(), String> {
        let state = ram.resolve(&self.dmem, word)?;
        self.vectors[0] = [0; 8];
        self.load_quad(31, 0x50);
        self.load_quad(10, 0);
        self.load_quad(24, AUDIO_STRUCT + AUDIO_TARGET_LEFT);
        if flags & 1 == 0 {
            self.dma(ram, TMP_DATA, state, 79, false)?;
            for (i, reg) in [20, 21, 18, 19, 24].into_iter().enumerate() {
                self.load_quad(reg, TMP_DATA + i as u32 * 16);
            }
        }
        let mut input = self.signed_buffer(AUDIO_IN_BUF);
        let mut left = self.signed_buffer(AUDIO_OUT_BUF);
        let mut right = self.signed_buffer(AUDIO_AUX_BUF0);
        let mut wet_left = if flags & 8 != 0 {
            self.signed_buffer(AUDIO_AUX_BUF1)
        } else {
            TMP_DATA + 0x50
        };
        let mut wet_right = if flags & 8 != 0 {
            self.signed_buffer(AUDIO_AUX_BUF2)
        } else {
            TMP_DATA + 0x50
        };
        let wet_step = if flags & 8 != 0 { 16 } else { 0 };
        let mut count = self.buffer(AUDIO_COUNT) as i16 as i32;
        let left_positive = signed(self.vectors[24][1]) > 0;
        let right_positive = signed(self.vectors[24][4]) > 0;
        self.load_quad(30, 0xb0);
        if flags & 1 != 0 {
            self.load_quad(17, input);
            self.load_quad(29, left);
            self.load_quad(27, wet_left);
            self.envelope_initial(20, 21, self.dmem.h16(AUDIO_STRUCT + AUDIO_VOL_LEFT), 1);
            self.clip(20, self.vectors[24][0], left_positive);
            self.envelope_mix(20, 29, 27);
            self.store_quad(29, left);
            self.store_quad(27, wet_left);
            self.load_quad(28, right);
            self.load_quad(26, wet_right);
            self.envelope_initial(18, 19, self.dmem.h16(AUDIO_STRUCT + AUDIO_VOL_RIGHT), 4);
            self.clip(18, self.vectors[24][3], right_positive);
            self.envelope_mix(18, 28, 26);
            self.store_quad(28, right);
            self.store_quad(26, wet_right);
            count -= 16;
            input = input.wrapping_add(16);
            left = left.wrapping_add(16);
            right = right.wrapping_add(16);
            wet_left = wet_left.wrapping_add(wet_step);
            wet_right = wet_right.wrapping_add(wet_step);
        }
        self.envelope_advance(20, 21, 1);
        loop {
            self.load_quad(17, input);
            self.clip(20, self.vectors[24][0], left_positive);
            self.envelope_advance(18, 19, 4);
            self.load_quad(29, left);
            self.load_quad(27, wet_left);
            self.store_quad(20, TMP_DATA);
            self.store_quad(21, TMP_DATA + 0x10);
            self.envelope_mix(20, 29, 27);
            self.load_quad(28, right);
            self.load_quad(26, wet_right);
            self.store_quad(29, left);
            self.clip(18, self.vectors[24][3], right_positive);
            self.store_quad(27, wet_left);
            self.envelope_advance(20, 21, 1);
            self.envelope_mix(18, 28, 26);
            count -= 16;
            left = left.wrapping_add(16);
            wet_left = wet_left.wrapping_add(wet_step);
            input = input.wrapping_add(16);
            self.store_quad(28, right);
            right = right.wrapping_add(16);
            self.store_quad(26, wet_right);
            if count <= 0 {
                break;
            }
            wet_right = wet_right.wrapping_add(wet_step);
        }
        self.store_quad(18, TMP_DATA + 0x20);
        self.store_quad(19, TMP_DATA + 0x30);
        self.store_quad(24, TMP_DATA + 0x40);
        self.dma(ram, TMP_DATA, state, 79, true)
    }
}

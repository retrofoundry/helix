use super::{Context, HalfwordOrder};

pub(super) fn signed(value: u16) -> i64 {
    i64::from(value as i16)
}

pub(super) fn sat(value: i64) -> u16 {
    value.clamp(-32768, 32767) as u16
}

pub(super) fn wrap48(value: i64) -> i64 {
    (value << 16) >> 16
}

pub(super) fn low(value: i64) -> u16 {
    if value < i64::from(i32::MIN) {
        0
    } else if value > i64::from(i32::MAX) {
        65535
    } else {
        value as u16
    }
}

pub(super) fn fractional(a: u16, b: u16) -> u16 {
    sat((signed(a) * signed(b) + 16384) >> 15)
}

pub(super) fn mix(old: u16, input: u16, gain: u16, unity: u16) -> u16 {
    sat((signed(old) * signed(unity) + signed(input) * signed(gain) + 16384) >> 15)
}

impl<O: HalfwordOrder> Context<O> {
    pub(super) fn vbyte(&self, reg: usize, byte: usize) -> u8 {
        (self.vectors[reg][(byte & 15) / 2] >> (8 * (1 - (byte & 1)))) as u8
    }

    pub(super) fn set_vbyte(&mut self, reg: usize, byte: usize, value: u8) {
        let shift = 8 * (1 - (byte & 1));
        let lane = &mut self.vectors[reg][(byte & 15) / 2];
        *lane = (*lane & !(255 << shift)) | (u16::from(value) << shift);
    }

    pub(super) fn load(&mut self, reg: usize, element: usize, address: u32, size: usize) {
        for i in 0..size.min(16 - element) {
            let half = self.dmem.h16(address.wrapping_add((i & !1) as u32));
            self.set_vbyte(reg, element + i, (half >> (8 * (1 - (i & 1)))) as u8);
        }
    }

    pub(super) fn store(&mut self, reg: usize, element: usize, address: u32, size: usize) {
        for i in (0..size).step_by(2) {
            let value = u16::from(self.vbyte(reg, element + i)) << 8
                | u16::from(self.vbyte(reg, element + i + 1));
            if i + 1 < size {
                self.dmem.set_h16(address.wrapping_add(i as u32), value);
            } else {
                self.dmem
                    .set_raw(address.wrapping_add(i as u32), (value >> 8) as u8);
            }
        }
    }

    pub(super) fn load_raw(&mut self, reg: usize, element: usize, address: u32, size: usize) {
        for i in 0..size.min(16 - element) {
            self.set_vbyte(
                reg,
                element + i,
                self.dmem.raw(address.wrapping_add(i as u32)),
            );
        }
    }

    pub(super) fn store_raw(&mut self, reg: usize, element: usize, address: u32, size: usize) {
        for i in 0..size {
            self.dmem
                .set_raw(address.wrapping_add(i as u32), self.vbyte(reg, element + i));
        }
    }

    pub(super) fn load_quad(&mut self, reg: usize, address: u32) {
        self.load(reg, 0, address, 16 - (address as usize & 15));
    }

    pub(super) fn store_quad(&mut self, reg: usize, address: u32) {
        self.store(reg, 0, address, 16 - (address as usize & 15));
    }

    pub(super) fn load_right(&mut self, reg: usize, address: u32) {
        let size = (address & 15) as usize;
        self.load(reg, 16 - size, address & !15, size);
    }

    pub(super) fn clip(&mut self, reg: usize, target: u16, positive: bool) {
        let mut flags = if positive { self.vcc } else { 0 };
        for i in 0..8 {
            let a = self.vectors[reg][i];
            let bit = 1 << i;
            let high = bit << 8;
            self.vectors[reg][i] = if positive {
                if self.vco & bit != 0 {
                    if self.vco & high == 0 {
                        let total = u32::from(a) + u32::from(target);
                        let zero = total as u16 == 0;
                        let overflow = total > 65535;
                        let choose = if self.vce & (1 << i) != 0 {
                            zero || !overflow
                        } else {
                            zero && !overflow
                        };
                        flags = (flags & !bit) | if choose { bit } else { 0 };
                    }
                    if flags & bit != 0 {
                        target.wrapping_neg()
                    } else {
                        a
                    }
                } else {
                    if self.vco & high == 0 {
                        flags = (flags & !high) | if a >= target { high } else { 0 };
                    }
                    if flags & high != 0 {
                        target
                    } else {
                        a
                    }
                }
            } else {
                let equal = a == target && self.vco & bit != 0 && self.vco & high != 0;
                if signed(a) > signed(target) || (a == target && !equal) {
                    flags |= bit;
                    a
                } else {
                    target
                }
            };
        }
        self.vcc = flags;
        self.vco = 0;
        if positive {
            self.vce = 0;
        }
    }
}

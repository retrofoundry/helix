use std::marker::PhantomData;

pub trait HalfwordOrder {
    fn read(bytes: [u8; 2]) -> u16;
    fn write(value: u16) -> [u8; 2];
}

pub enum Be {}
pub enum Ne {}

impl HalfwordOrder for Be {
    fn read(bytes: [u8; 2]) -> u16 {
        u16::from_be_bytes(bytes)
    }
    fn write(value: u16) -> [u8; 2] {
        value.to_be_bytes()
    }
}

impl HalfwordOrder for Ne {
    fn read(bytes: [u8; 2]) -> u16 {
        u16::from_ne_bytes(bytes)
    }
    fn write(value: u16) -> [u8; 2] {
        value.to_ne_bytes()
    }
}

pub struct Dmem<O> {
    pub bytes: [u8; 4096],
    order: PhantomData<O>,
}

impl<O: HalfwordOrder> Default for Dmem<O> {
    fn default() -> Self {
        Self {
            bytes: [0; 4096],
            order: PhantomData,
        }
    }
}

impl<O: HalfwordOrder> Dmem<O> {
    pub fn raw(&self, address: u32) -> u8 {
        self.bytes[address as usize & 4095]
    }

    pub fn set_raw(&mut self, address: u32, value: u8) {
        self.bytes[address as usize & 4095] = value;
    }

    pub fn h16(&self, address: u32) -> u16 {
        O::read([self.raw(address), self.raw(address.wrapping_add(1))])
    }

    pub fn set_h16(&mut self, address: u32, value: u16) {
        let bytes = O::write(value);
        self.set_raw(address, bytes[0]);
        self.set_raw(address.wrapping_add(1), bytes[1]);
    }

    pub fn word(&self, address: u32) -> u32 {
        (u32::from(self.h16(address)) << 16) | u32::from(self.h16(address.wrapping_add(2)))
    }

    pub fn set_word(&mut self, address: u32, value: u32) {
        self.set_h16(address, (value >> 16) as u16);
        self.set_h16(address.wrapping_add(2), value as u16);
    }

    #[cfg(test)]
    pub fn table_byte(&self, address: u32) -> u8 {
        (self.h16(address & !1) >> (8 * (1 - (address & 1)))) as u8
    }
}

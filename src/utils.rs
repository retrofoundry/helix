trait I8MathExt {
    fn ushr(self, n: u32) -> u8;
}

trait I16MathExt {
    fn ushr(self, n: u32) -> u16;
}

pub trait U16MathExt {
    fn shr(self, n: u32) -> u16;
    fn ushr(self, n: u32) -> u16;
}

pub trait I32MathExt {
    fn ushr(self, n: u32) -> u32;
}

impl I8MathExt for i8 {
    fn ushr(self, n: u32) -> u8 {
        ((self >> n) & ((1 << (8 - n)) - 1)) as u8
    }
}

impl I16MathExt for i16 {
    fn ushr(self, n: u32) -> u16 {
        ((self >> n) & ((1 << (16 - n)) - 1)) as u16
    }
}

impl U16MathExt for u16 {
    fn shr(self, n: u32) -> u16 {
        (self >> n) as u16
    }

    fn ushr(self, n: u32) -> u16 {
        (self >> n) as u16
    }
}

impl I32MathExt for i32 {
    fn ushr(self, n: u32) -> u32 {
        ((self >> n) & ((1 << (32 - n)) - 1)) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ursi32() {
        assert_eq!(5i32.ushr(2), 1);
        assert_eq!((-1 * 5i32).ushr(2), 1073741822);
        assert_eq!(9i32.ushr(2), 2);
        assert_eq!((-1 * 9i32).ushr(2), 1073741821);
    }

    #[test]
    fn test_ursi16() {
        assert_eq!(5i16.ushr(2), 1);
        assert_eq!((-1 * 5i16).ushr(2), 16382);
        assert_eq!(9i16.ushr(2), 2);
        assert_eq!((-1 * 9i16).ushr(2), 16381);
    }

    #[test]
    fn test_ursi8() {
        assert_eq!(5i8.ushr(2), 1);
        assert_eq!((-1 * 5i8).ushr(2), 62);
        assert_eq!(9i8.ushr(2), 2);
        assert_eq!((-1 * 9i8).ushr(2), 61);
    }
}

use crate::utils::I32MathExt;

pub fn get_c0(word: usize, pos: u32, width: u32) -> usize {
    (word >> pos) & ((1 << width) - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_c0() {
        let word: usize = 84939284;
        let a = get_c0(word, 16, 8) / 2;
        let b = get_c0(word, 8, 8) / 2;
        let c = get_c0(word, 0, 8) / 2;

        assert_eq!(a, 8);
        assert_eq!(b, 9);
        assert_eq!(c, 10);

        assert_eq!(a, ((((word as i32).ushr(16)) & 0xFF) / 2) as usize);
    }
}

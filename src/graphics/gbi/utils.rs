use crate::utils::I32MathExt;

pub fn get_cmd(word: usize, pos: u32, width: u32) -> usize {
    (word >> pos) & ((1 << width) - 1)
}

pub fn get_segmented_address(w1: usize) -> usize {
    return w1;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_cmd() {
        let word: usize = 84939284;
        let a = get_cmd(word, 16, 8) / 2;
        let b = get_cmd(word, 8, 8) / 2;
        let c = get_cmd(word, 0, 8) / 2;

        assert_eq!(a, 8);
        assert_eq!(b, 9);
        assert_eq!(c, 10);

        assert_eq!(a, ((((word as i32).ushr(16)) & 0xFF) / 2) as usize);
    }
}

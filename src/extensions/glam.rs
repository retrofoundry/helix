use glam::Mat4;

pub trait FromFixedPoint {
    fn from_fixed_point(mtx: &[i32]) -> Self;
}

impl FromFixedPoint for Mat4 {
    fn from_fixed_point(addr: &[i32]) -> Mat4 {
        assert!(addr.len() == 16);

        let mut f_mtx = [[0.0; 4]; 4];
        for i in 0..4 {
            for j in (0..4).step_by(2) {
                let int_part = addr[i * 2 + j / 2];
                let frac_part = addr[8 + i * 2 + j / 2] as u32;

                let a = (int_part as u32 & 0xFFFF0000) as i32;
                let b = (frac_part >> 16) as i32;
                let c = (int_part << 16) as i32;
                let d = frac_part as i32 & 0xFFFF;

                f_mtx[i][j] = (a | b) as f32 / 65536.0;
                f_mtx[i][j + 1] = (c | d) as f32 / 65536.0;
            }
        }

        Mat4::from_cols_array(&[
            f_mtx[0][0],
            f_mtx[1][0],
            f_mtx[2][0],
            f_mtx[3][0],
            f_mtx[0][1],
            f_mtx[1][1],
            f_mtx[2][1],
            f_mtx[3][1],
            f_mtx[0][2],
            f_mtx[1][2],
            f_mtx[2][2],
            f_mtx[3][2],
            f_mtx[0][3],
            f_mtx[1][3],
            f_mtx[2][3],
            f_mtx[3][3],
        ])
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use glam::Mat4;

    use crate::extensions::glam::FromFixedPoint;

    #[test]
    fn test_from_fixed_point() {
        let input = [
            0,
            0,
            -65536,
            -65536,
            -65536,
            0,
            -32833310,
            -10485759,
            963418071,
            -973996032,
            -1651933,
            1681260544,
            109454045,
            765460480,
            -1201364593,
            -1451294720,
        ];
        let expected = Mat4::from_cols_array(&[
            0.224304,
            -0.000396729,
            -0.974518,
            -500.28,
            0.593124,
            0.793503,
            0.136185,
            226.631,
            0.773224,
            -0.608551,
            0.178223,
            -159.338,
            0.0,
            0.0,
            0.0,
            1.0,
        ]);
        let result = Mat4::from_fixed_point(&input);
        assert_relative_eq!(result, expected, epsilon = 1e-3);
    }
}

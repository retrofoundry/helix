use crate::fast3d::gbi::defines::Light;
use glam::{Mat4, Vec3A, Vec4Swizzles};

pub fn matrix_from_floats(addr: &[f32]) -> [[f32; 4]; 4] {
    addr
        .chunks(4)
        .map(|row| [row[0], row[1], row[2], row[3]])
        .collect::<Vec<[f32; 4]>>()
        .try_into()
        .unwrap()
}

pub fn matrix_from_fixed_point(addr: &[i32]) -> [[f32; 4]; 4] {
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

    f_mtx
}

pub trait MatrixFrom {
    fn from_fixed_point(addr: &[i32]) -> Self;
    fn from_floats(addr: &[f32]) -> Self;
}

impl MatrixFrom for Mat4 {
    fn from_fixed_point(addr: &[i32]) -> Self {
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
            f_mtx[0][0], f_mtx[1][0], f_mtx[2][0], f_mtx[3][0],
            f_mtx[0][1], f_mtx[1][1], f_mtx[2][1], f_mtx[3][1],
            f_mtx[0][2], f_mtx[1][2], f_mtx[2][2], f_mtx[3][2],
            f_mtx[0][3], f_mtx[1][3], f_mtx[2][3], f_mtx[3][3],
        ])
    }

    fn from_floats(addr: &[f32]) -> Self {
        Mat4::from_cols_array(&[
            addr[0], addr[4], addr[8], addr[12],
            addr[1], addr[5], addr[9], addr[13],
            addr[2], addr[6], addr[10], addr[14],
            addr[3], addr[7], addr[11], addr[15],
        ])
    }
}

pub fn matrix_multiply(a: &[[f32; 4]; 4], b: &[[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut result = [[0.0; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            result[i][j] =
                a[i][0] * b[0][j] + a[i][1] * b[1][j] + a[i][2] * b[2][j] + a[i][3] * b[3][j];
        }
    }
    result
}

pub trait Vec3AMul {
    fn mul_mat4(&self, matrix: &Mat4) -> Self;
}

impl Vec3AMul for Vec3A {
    #[inline]
    fn mul_mat4(&self, matrix: &Mat4) -> Self {
        Self {
            x: self.dot(matrix.row(0).xyz().into()),
            y: self.dot(matrix.row(1).xyz().into()),
            z: self.dot(matrix.row(2).xyz().into()),
        }
    }
}

pub fn transposed_matrix_multiply(light_dir: &[f32; 3], matrix: &[[f32; 4]; 4]) -> [f32; 3] {
    let mut result = [0.0; 3];
    for i in 0..3 {
        result[i] =
            light_dir[0] * matrix[i][0] + light_dir[1] * matrix[i][1] + light_dir[2] * matrix[i][2];
    }
    result
}

pub fn calculate_norm_dir(light: &Light, matrix: &[[f32; 4]; 4], coeffs: &mut [f32; 3]) {
    let light_dir = [
        light.dir[0] as f32 / 127.0,
        light.dir[1] as f32 / 127.0,
        light.dir[2] as f32 / 127.0,
    ];

    coeffs.copy_from_slice(&transposed_matrix_multiply(&light_dir, matrix));
    normalize_vector(coeffs);
}

pub fn calculate_normal_dir(light: &Light, matrix: &Mat4, output: &mut Vec3A) {
    let light_dir = Vec3A::new(
        light.dir[0] as f32 / 127.0,
        light.dir[1] as f32 / 127.0,
        light.dir[2] as f32 / 127.0,
    );

    *output = light_dir.mul_mat4(matrix).normalize_or_zero();
} 

pub fn normalize_vector(v: &mut [f32; 3]) {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len != 0.0 {
        v[0] /= len;
        v[1] /= len;
        v[2] /= len;
    }
}

#[cfg(test)]
mod tests {
    use glam::{Mat4, Vec3A};
    use crate::{extensions::matrix::Vec3AMul, fast3d::gbi::defines::Light};

    use super::{matrix_from_fixed_point, transposed_matrix_multiply, MatrixFrom, calculate_normal_dir, calculate_norm_dir, matrix_from_floats};

    #[test]
    fn test_matrix_from_fixed_point() {
        let addr = [
            1065353216, 0, 0, 0, 0, 1065353216, 0, 0, 0, 0, 1065353216, 0, 0, 0, 0, 1065353216,
            1073741824, 1073741824, 1073741824, 1073741824, 1073741824, 1073741824, 1073741824,
            1073741824, 1073741824, 1073741824, 1073741824, 1073741824, 1073741824, 1073741824,
            1073741824, 1073741824,
        ];

        let expected = matrix_from_fixed_point(&addr);
        let mut expected_transposed = [[0.0; 4]; 4];
        for i in 0..4 {
            for j in 0..4 {
                expected_transposed[i][j] = expected[j][i];
            }
        }

        let actual = Mat4::from_fixed_point(&addr);
        assert_eq!(expected_transposed, actual.to_cols_array_2d());
    }

    #[test]
    fn test_matrix_from_floats() {
        let float_data = [
            1.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0,
        ];

        let expected = matrix_from_floats(&float_data);
        let mut expected_transposed = [[0.0; 4]; 4];
        for i in 0..4 {
            for j in 0..4 {
                expected_transposed[i][j] = expected[j][i];
            }
        }

        let actual = Mat4::from_floats(&float_data);
        assert_eq!(expected_transposed, actual.to_cols_array_2d());
    }

    #[test]
    fn test_matrix_multiply() {
        let addr = [
            1065353216, 0, 0, 0, 0, 1065353216, 0, 0, 0, 0, 1065353216, 0, 0, 0, 0, 1065353216,
            1073741824, 1073741824, 1073741824, 1073741824, 1073741824, 1073741824, 1073741824,
            1073741824, 1073741824, 1073741824, 1073741824, 1073741824, 1073741824, 1073741824,
            1073741824, 1073741824,
        ];

        let addr2 = [
            1065353216, 0, 0, 0, 0, 1065353216, 0, 0, 0, 0, 1065353216, 0, 0, 0, 0, 1065353216,
            1073741824, 1073741824, 1073741824, 1073741824, 1073741824, 1073741824, 1073741824,
            1073741824, 1073741824, 1073741824, 1073741824, 1073741824, 1073741824, 1073741824,
            1073741824, 1073741824,
        ];

        let mo1 = matrix_from_fixed_point(&addr);
        let mo2 = matrix_from_fixed_point(&addr2);
        let expected = super::matrix_multiply(&mo1, &mo2);

        let ma1 = Mat4::from_fixed_point(&addr);
        let ma2 = Mat4::from_fixed_point(&addr2);
        let actual = ma1 * ma2;

        let mut expected_transposed = [[0.0; 4]; 4];
        for i in 0..4 {
            for j in 0..4 {
                expected_transposed[i][j] = expected[j][i];
            }
        }

        assert_eq!(expected_transposed, actual.to_cols_array_2d());
    }

    #[test]
    fn test_transposed_matrix_multiply() {
        let light_dir: [f32; 3] = [1.0, 2.0, 3.0];

        let addr = [
            1065353216, 0, 0, 0, 0, 1065353216, 0, 0, 0, 0, 1065353216, 0, 0, 0, 0, 1065353216,
            1073741824, 1073741824, 1073741824, 1073741824, 1073741824, 1073741824, 1073741824,
            1073741824, 1073741824, 1073741824, 1073741824, 1073741824, 1073741824, 1073741824,
            1073741824, 1073741824,
        ];

        // Test the old implementation
        let mo1 = matrix_from_fixed_point(&addr);
        let expected = transposed_matrix_multiply(&light_dir, &mo1);

        // Test the new implementation
        let new_light_dir = Vec3A::new(light_dir[0], light_dir[1], light_dir[2]);
        let new_matrix = Mat4::from_fixed_point(&addr);
        let actual = new_light_dir.mul_mat4(&new_matrix);

        // Check that the results are equal
        assert_eq!(expected, [actual.x, actual.y, actual.z]);
    }

    #[test]
    fn test_calculate_norm_dir() {
        let addr = [
            1065353216, 0, 0, 0, 0, 1065353216, 0, 0, 0, 0, 1065353216, 0, 0, 0, 0, 1065353216,
            1073741824, 1073741824, 1073741824, 1073741824, 1073741824, 1073741824, 1073741824,
            1073741824, 1073741824, 1073741824, 1073741824, 1073741824, 1073741824, 1073741824,
            1073741824, 1073741824,
        ];

        let light = Light::new([0, 0, 0], [0, 0, 0], [0, 127, 0]);

        let mo1 = matrix_from_fixed_point(&addr);
        let mut expected: [f32; 3] = [0.0, 0.0, 0.0];
        calculate_norm_dir(&light, &mo1, &mut expected);

        let ma1 = Mat4::from_fixed_point(&addr);
        let mut actual: Vec3A = Vec3A::ZERO;
        calculate_normal_dir(&light, &ma1, &mut actual);

        assert_eq!(expected, [actual.x, actual.y, actual.z]);
    }
}

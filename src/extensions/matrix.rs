use crate::graphics::gbi::defines::Light;

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

pub fn transposed_matrix_multiply(light_dir: &[f32; 3], matrix: &[[f32; 4]; 4]) -> [f32; 3] {
    let mut result = [0.0; 3];
    for i in 0..3 {
        result[i] =
            light_dir[0] * matrix[i][0] + light_dir[1] * matrix[i][1] + light_dir[2] * matrix[i][2];
    }
    result
}

pub fn calculate_normal_dir(light: &Light, matrix: &[[f32; 4]; 4], coeffs: &mut [f32; 3]) {
    let light_dir = [
        light.dir[0] as f32 / 127.0,
        light.dir[1] as f32 / 127.0,
        light.dir[2] as f32 / 127.0,
    ];

    coeffs.copy_from_slice(&transposed_matrix_multiply(&light_dir, matrix));
    normalize_vector(coeffs);
}

pub fn normalize_vector(v: &mut [f32; 3]) {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len != 0.0 {
        v[0] /= len;
        v[1] /= len;
        v[2] /= len;
    }
}

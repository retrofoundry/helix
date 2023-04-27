use crate::fast3d::gbi::defines::Light;
use glam::{Mat4, Vec3A, Vec4Swizzles};

pub trait MatrixFrom {
    fn from_fixed_point(addr: &[i32]) -> Self;
    fn from_floats(addr: &[f32]) -> Self;
}

impl MatrixFrom for Mat4 {
    fn from_fixed_point(addr: &[i32]) -> Self {
        let mut f_mtx = Mat4::ZERO;
        for i in 0..4 {
            for j in (0..4).step_by(2) {
                let int_part = addr[i * 2 + j / 2];
                let frac_part = addr[8 + i * 2 + j / 2] as u32;

                let a = (int_part as u32 & 0xFFFF0000) as i32;
                let b = (frac_part >> 16) as i32;
                let c = (int_part << 16) as i32;
                let d = frac_part as i32 & 0xFFFF;

                f_mtx.col_mut(j)[i] = (a | b) as f32 / 65536.0;
                f_mtx.col_mut(j + 1)[i] = (c | d) as f32 / 65536.0;
            }
        }

        f_mtx
    }

    fn from_floats(addr: &[f32]) -> Self {
        Mat4::from_cols_array(&[
            addr[0], addr[4], addr[8], addr[12], addr[1], addr[5], addr[9], addr[13], addr[2],
            addr[6], addr[10], addr[14], addr[3], addr[7], addr[11], addr[15],
        ])
    }
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

pub fn calculate_normal_dir(light: &Light, matrix: &Mat4, output: &mut Vec3A) {
    let light_dir = Vec3A::new(
        light.dir[0] as f32 / 127.0,
        light.dir[1] as f32 / 127.0,
        light.dir[2] as f32 / 127.0,
    );

    *output = light_dir.mul_mat4(matrix).normalize_or_zero();
}

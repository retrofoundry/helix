use std::f64::consts::{FRAC_1_SQRT_2, PI};
use std::os::raw::c_void;
use std::sync::{Once, OnceLock};

const ZIGZAG: [usize; 64] = [
    0, 1, 8, 16, 9, 2, 3, 10, 17, 24, 32, 25, 18, 11, 4, 5, 12, 19, 26, 33, 40, 48, 41, 34, 27, 20,
    13, 6, 7, 14, 21, 28, 35, 42, 49, 56, 57, 50, 43, 36, 29, 22, 15, 23, 30, 37, 44, 51, 58, 59,
    52, 45, 38, 31, 39, 46, 53, 60, 61, 54, 47, 55, 62, 63,
];

#[repr(C, align(8))]
struct JpegTaskData {
    address: usize,
    mb_count: u32,
    mode: u32,
    q_table_y: usize,
    q_table_u: usize,
    q_table_v: usize,
    _mb_size: u32,
}

fn from_k0_physical(address: usize) -> usize {
    // Native OS_K0_TO_PHYSICAL subtracts KSEG0 without narrowing the pointer.
    address.wrapping_add(0x8000_0000)
}

fn inverse_dct(coefficients: &[u16], table: &[u16; 64]) -> [f64; 64] {
    static BASIS: OnceLock<[[f64; 8]; 8]> = OnceLock::new();
    let basis = BASIS.get_or_init(|| {
        std::array::from_fn(|x| {
            std::array::from_fn(|u| {
                let scale = if u == 0 { FRAC_1_SQRT_2 } else { 1.0 };
                0.5 * scale * ((2 * x + 1) as f64 * u as f64 * PI / 16.0).cos()
            })
        })
    });
    let mut natural = [0.0; 64];
    for (i, &position) in ZIGZAG.iter().enumerate() {
        natural[position] = f64::from(coefficients[i] as i16) * f64::from(table[i]);
    }
    let mut rows = [0.0; 64];
    for (v, row) in natural.chunks_exact(8).enumerate() {
        for (x, weights) in basis.iter().enumerate() {
            rows[v * 8 + x] = row.iter().zip(weights).map(|(c, w)| c * w).sum();
        }
    }
    std::array::from_fn(|i| {
        let value: f64 = (0..8).map(|v| rows[v * 8 + i % 8] * basis[i / 8][v]).sum();
        (value + 128.0).round().clamp(0.0, 255.0)
    })
}

fn rgba5551(y: f64, cb: f64, cr: f64) -> u16 {
    let cb = cb - 128.0;
    let cr = cr - 128.0;
    let channel = |value: f64| (value.round().clamp(0.0, 255.0) as u16) >> 3;
    let r = channel(y + 1.402 * cr);
    let g = channel(y - 0.34414 * cb - 0.71414 * cr);
    let b = channel(y + 1.772 * cb);
    (r << 11) | (g << 6) | (b << 1) | 1
}

/// # Safety
/// `data` must be null or a native JpegTaskData with live tables and exclusive macroblock buffers.
pub(super) unsafe fn process(data: *const c_void) {
    if data.is_null() {
        return;
    }
    let task = &*data.cast::<JpegTaskData>();
    let y_blocks = match task.mode {
        0 => 2,
        2 => 4,
        mode => {
            static UNKNOWN_MODE: Once = Once::new();
            UNKNOWN_MODE.call_once(|| log::warn!("NJPEG: unsupported mode {mode}"));
            return;
        }
    };
    if task.mb_count == 0 {
        return;
    }
    let tables = [task.q_table_y, task.q_table_u, task.q_table_v]
        .map(|address| *(from_k0_physical(address) as *const [u16; 64]));
    let address = from_k0_physical(task.address) as *mut u16;
    let stride = (y_blocks + 2) * 64;
    for mb in 0..task.mb_count as usize {
        let buffer = std::slice::from_raw_parts_mut(address.add(mb * stride), stride);
        let mut blocks = [[0.0; 64]; 6];
        for (block, output) in blocks[..y_blocks + 2].iter_mut().enumerate() {
            let component = if block < y_blocks {
                0
            } else {
                block - y_blocks + 1
            };
            *output = inverse_dct(&buffer[block * 64..][..64], &tables[component]);
        }
        for y in 0..y_blocks * 4 {
            for x in 0..16 {
                let luma = blocks[(y / 8) * 2 + x / 8][(y % 8) * 8 + x % 8];
                let chroma = (y / (y_blocks / 2)) * 8 + x / 2;
                // The RDP samples these pixels, so they go out in N64 (big-endian) byte order.
                buffer[y * 16 + x] =
                    rgba5551(luma, blocks[y_blocks][chroma], blocks[y_blocks + 1][chroma]).to_be();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn physical<T>(ptr: *const T) -> usize {
        (ptr as usize).wrapping_sub(0x8000_0000)
    }

    fn task(data: &mut [u16], tables: &[[u16; 64]; 3], mode: u32, count: u32) -> JpegTaskData {
        JpegTaskData {
            address: physical(data.as_ptr()),
            mb_count: count,
            mode,
            q_table_y: physical(tables[0].as_ptr()),
            q_table_u: physical(tables[1].as_ptr()),
            q_table_v: physical(tables[2].as_ptr()),
            _mb_size: 0xdead_beef,
        }
    }

    fn encode(samples: &[f64; 64], table: &[u16; 64]) -> [u16; 64] {
        let mut output = [0; 64];
        let mut index = 0;
        for diagonal in 0usize..15 {
            let start = diagonal.saturating_sub(7);
            let end = diagonal.min(7);
            for step in start..=end {
                let v = if diagonal % 2 == 0 {
                    end + start - step
                } else {
                    step
                };
                let u = diagonal - v;
                let mut sum = 0.0;
                for y in 0..8 {
                    for x in 0..8 {
                        sum += (samples[y * 8 + x] - 128.0)
                            * ((2 * x + 1) as f64 * u as f64 * PI / 16.0).cos()
                            * ((2 * y + 1) as f64 * v as f64 * PI / 16.0).cos();
                    }
                }
                let cu = if u == 0 { FRAC_1_SQRT_2 } else { 1.0 };
                let cv = if v == 0 { FRAC_1_SQRT_2 } else { 1.0 };
                output[index] =
                    (sum * cu * cv / (4.0 * f64::from(table[index]))).round() as i16 as u16;
                index += 1;
            }
        }
        output
    }

    fn image_round_trip(mode: u32, height: usize) {
        let tables =
            std::array::from_fn(|c| std::array::from_fn(|i| (1 + (i * 3 + c * 2) % 5) as u16));
        let stride = (height / 4 + 2) * 64;
        let mut data = vec![0xa55a; 4 * stride + 16];
        let mut images = Vec::new();
        for mb in 0..4 {
            let image: Vec<[f64; 3]> = (0..height * 16)
                .map(|i| {
                    let x = (i % 16 / 2) as f64;
                    let y = (i / 16 / (height / 8)) as f64;
                    let base = 45.0 + mb as f64 * 22.0;
                    [
                        base + 6.0 * x + 2.0 * y,
                        base + 2.0 * x + 7.0 * y,
                        base + 4.0 * x + 3.0 * y + 12.0,
                    ]
                })
                .collect();
            let components: Vec<[f64; 3]> = image
                .iter()
                .map(|&[r, g, b]| {
                    [
                        0.299 * r + 0.587 * g + 0.114 * b,
                        -0.1687 * r - 0.3313 * g + 0.5 * b + 128.0,
                        0.5 * r - 0.4187 * g - 0.0813 * b + 128.0,
                    ]
                })
                .collect();
            for block in 0..height / 4 + 2 {
                let component = if block < height / 4 {
                    0
                } else {
                    block - height / 4 + 1
                };
                let samples = std::array::from_fn(|i| {
                    let x = if component == 0 {
                        (block % 2) * 8 + i % 8
                    } else {
                        (i % 8) * 2
                    };
                    let y = if component == 0 {
                        (block / 2) * 8 + i / 8
                    } else {
                        (i / 8) * (height / 8)
                    };
                    components[y * 16 + x][component]
                });
                let offset = mb * stride + block * 64;
                data[offset..offset + 64].copy_from_slice(&encode(&samples, &tables[component]));
            }
            images.push(image);
        }
        let original = data.clone();
        let task = task(&mut data, &tables, mode, 4);
        unsafe { process((&task as *const JpegTaskData).cast()) };
        for (mb, image) in images.iter().enumerate() {
            for (i, expected) in image.iter().enumerate() {
                let pixel = u16::from_be(data[mb * stride + i]);
                assert_eq!(pixel & 1, 1, "alpha at macroblock {mb}, pixel {i}");
                for (c, shift) in [11, 6, 1].into_iter().enumerate() {
                    let actual = f64::from((pixel >> shift) & 31) * 255.0 / 31.0;
                    assert!(
                        (actual - expected[c]).abs() <= 9.0,
                        "mode {mode}, macroblock {mb}, pixel {i}, channel {c}: {actual} != {}",
                        expected[c]
                    );
                }
            }
            let tail = mb * stride + height * 16..(mb + 1) * stride;
            assert_eq!(data[tail.clone()], original[tail]);
        }
        assert_eq!(data[4 * stride..], original[4 * stride..]);
    }

    #[test]
    fn mode_0_round_trips_422_image() {
        image_round_trip(0, 8);
    }

    #[test]
    fn mode_2_round_trips_420_image() {
        image_round_trip(2, 16);
    }

    #[test]
    fn k0_physical_round_trips_native_pointer_and_wraps() {
        let data = [17u16; 64];
        assert_eq!(
            from_k0_physical(physical(data.as_ptr())),
            data.as_ptr() as usize
        );
        for address in [0usize, 1, 0x8000_0000, usize::MAX] {
            assert_eq!(from_k0_physical(address.wrapping_sub(0x8000_0000)), address);
        }
    }

    #[test]
    fn unknown_mode_leaves_buffer_untouched() {
        let tables = [[1; 64]; 3];
        let mut data = [0x1234; 384];
        let mut task = task(&mut data, &tables, 1, 1);
        task.q_table_y = physical::<u16>(std::ptr::null());
        unsafe { process((&task as *const JpegTaskData).cast()) };
        assert_eq!(data, [0x1234; 384]);
    }

    #[test]
    fn zero_macroblocks_do_not_access_buffers() {
        let task = JpegTaskData {
            address: physical::<u16>(std::ptr::null()),
            mb_count: 0,
            mode: 2,
            q_table_y: physical::<u16>(std::ptr::null()),
            q_table_u: physical::<u16>(std::ptr::null()),
            q_table_v: physical::<u16>(std::ptr::null()),
            _mb_size: 0,
        };
        unsafe { process((&task as *const JpegTaskData).cast()) };
    }

    #[test]
    fn dc_blocks_level_shift_and_clamp_to_opaque_rgb() {
        for (dc, expected) in [
            ([-2048, 0, 0], 0x0001),
            ([2048, 0, 0], 0xffff),
            ([0, 0, 0], 0x8421),
            ([0, -2048, 2048], 0xfa81),
        ] {
            let tables = [[1; 64]; 3];
            let mut data = [0; 384];
            for block in 0..4 {
                data[block * 64] = dc[0] as i16 as u16;
            }
            data[256] = dc[1] as i16 as u16;
            data[320] = dc[2] as i16 as u16;
            let task = task(&mut data, &tables, 2, 1);
            unsafe { process((&task as *const JpegTaskData).cast()) };
            assert!(
                data[..256]
                    .iter()
                    .all(|&pixel| u16::from_be(pixel) == expected),
                "DC {dc:?}: {:#06x}",
                data[0]
            );
        }
    }
}

use super::dmem::Ne;
use super::mem::Dma;
use super::*;

fn accessors<O: HalfwordOrder>() {
    let mut d = Dmem::<O>::default();
    d.set_h16(4095, 0xa361);
    assert_eq!(d.h16(4095), 0xa361);
    assert_eq!([d.raw(4095), d.raw(0)], O::write(0xa361));
    d.set_word(4094, 0x1234abcd);
    assert_eq!(d.word(4094), 0x1234abcd);
    assert_eq!(d.h16(4094), 0x1234);
    assert_eq!(d.h16(0), 0xabcd);
    assert_eq!(
        (4094..4098).map(|a| d.table_byte(a)).collect::<Vec<_>>(),
        [0x12, 0x34, 0xab, 0xcd]
    );
    d.set_raw(23, 0x87);
    assert_eq!(d.raw(4119), 0x87);
}

#[test]
fn memory_orders_preserve_raw_streams_and_halfword_words() {
    accessors::<Be>();
    accessors::<Ne>();
}

#[test]
fn dma_underflow_decodes_without_transferring() {
    assert_eq!(
        Dma::decode(0u32.wrapping_sub(1)),
        Dma {
            length: 4096,
            rows: 256,
            skip: 4095
        }
    );
}

#[test]
fn dma_rounding_rows_skip_alignment_wrap_and_bounds() {
    for (count, length, rows) in [
        (1u32, 8, 1),
        (8, 8, 1),
        (9, 16, 1),
        (4096, 4096, 1),
        (4097, 8, 2),
        (65535, 4096, 16),
    ] {
        assert_eq!(
            Dma::decode(count - 1),
            Dma {
                length,
                rows,
                skip: 0
            }
        );
    }
    let mut context = Context::<Ne>::default();
    let mut ram: Vec<u8> = (0..256).map(|i| i as u8).collect();
    let before = ram.clone();
    let dma = Dma::decode((5 << 20) | (2 << 12) | 9);
    dma.transfer(&mut context.dmem, &mut ram, 0xffd, 11, false)
        .unwrap();
    for row in 0..3 {
        for i in 0..16 {
            assert_eq!(
                context.dmem.raw((0xff8 + row * 16 + i) as u32),
                before[8 + row * 21 + i]
            );
        }
    }
    dma.transfer(&mut context.dmem, &mut ram, 0xffb, 132, true)
        .unwrap();
    for row in 0..3 {
        assert_eq!(
            &ram[128 + row * 21..144 + row * 21],
            &before[8 + row * 21..24 + row * 21]
        );
    }
    let dmem = context.dmem.bytes;
    assert!(Dma::decode(u32::MAX)
        .transfer(&mut context.dmem, &mut ram, 0, 0, false)
        .is_err());
    assert_eq!(context.dmem.bytes, dmem);
}

fn synthetic<O: HalfwordOrder>() -> Context<O> {
    let mut c = Context::default();
    for i in 0..8 {
        c.dmem.set_h16(0x40 + i * 2, (2 * (i + 1)) as u16);
        c.dmem.set_h16(0x50 + i * 2, 1);
        c.dmem.set_h16(0xb0 + i * 2, (i * 4000) as u16);
    }
    c.dmem.set_h16(8, 1 << 5);
    c.dmem.set_h16(10, 1 << 11);
    c.dmem.set_h16(12, i16::MAX as u16);
    c.dmem.set_h16(0x64, 2);
    c.dmem.set_h16(0x68, 8);
    for i in 0..4 {
        c.dmem.set_h16(0x30 + 2 * i, 15 << (12 - 4 * i));
        c.dmem.set_h16(0x38 + 2 * i, 1 << (4 * i));
        c.dmem.set_h16(0x70 + 16 * i + 2 * i, 1);
        c.dmem.set_h16(0x78 + 16 * i + 2 * i, 1);
    }
    c
}

fn command<O: HalfwordOrder>(c: &mut Context<O>, ram: &mut [u8], w0: u32, w1: u32) {
    c.command(ram, w0, u64::from(w1)).unwrap();
}

fn buffers<O: HalfwordOrder>(c: &mut Context<O>, input: u16, output: u16, count: u16) {
    c.dmem.set_h16(0x360, input);
    c.dmem.set_h16(0x362, output);
    c.dmem.set_h16(0x364, count);
}

#[test]
fn task_startup_and_forty_command_refills() {
    for count in [1usize, 39, 40, 41, 80, 81, 170] {
        let mut c = Context::default();
        c.dmem.bytes.fill(0xa5);
        c.vectors[5] = [123; 8];
        c.vco = 0x579b;
        let mut ram = vec![0u8; 0x2000];
        ram[..24].fill(0x6b);
        for i in 0..count {
            ram[0x100 + i * 8..0x104 + i * 8]
                .copy_from_slice(&(0x09040000 | i as u32).to_be_bytes());
        }
        let task = [
            2,
            0,
            0,
            0,
            0,
            0,
            0,
            19,
            0,
            0,
            0,
            0,
            0x100,
            count as u32 * 8,
            0,
            0,
        ];
        c.console_task(&mut ram, &task).unwrap();
        assert_eq!(&c.dmem.bytes[..24], &[0x6b; 24]);
        assert_eq!(c.dmem.word(0x320), 0);
        assert_eq!(c.dmem.word(0x324), 0xa5a5a5a5);
        assert_eq!(c.dmem.h16(0x368), (count - 1) as u16);
        assert_eq!(c.vectors[5], [123; 8]);
        assert_eq!(c.vco, 0x579b);
        let last = (count - 1) / 40 * 40;
        assert_eq!(
            &c.dmem.bytes[0x380..0x380 + (count - last) * 8],
            &ram[0x100 + last * 8..0x100 + count * 8]
        );
        for (i, &word) in task.iter().enumerate() {
            assert_eq!(c.dmem.word(0xfc0 + i as u32 * 4), word);
        }
        assert_eq!(c.scalar[TASK_DATA], 0x100 + count as u32 * 8);
        assert_eq!(c.scalar[TASK_DATA_SIZE], 0);
    }
}

#[test]
fn parameter_commands_alias_the_loop_word_and_preserve_main_buffers() {
    let mut c = Context::<Be>::default();
    let mut ram = [];
    command(&mut c, &mut ram, 0x0800fffd, 0xfffe0021);
    assert_eq!((c.buffer(0), c.buffer(2), c.buffer(4)), (0x5bd, 0x5be, 33));
    command(&mut c, &mut ram, 0x08080040, 0x008000c0);
    assert_eq!(
        (c.buffer(10), c.buffer(12), c.buffer(14)),
        (0x600, 0x640, 0x680)
    );
    assert_eq!(c.buffer(4), 33);
    command(&mut c, &mut ram, 0x07000000, 0x03000123);
    command(&mut c, &mut ram, 0x0f000000, 0x03000005);
    assert_eq!(c.dmem.word(0x370), 0x128);
    command(&mut c, &mut ram, 0x09021234, 0xabcd5678);
    assert_eq!(c.dmem.word(0x370), 0x1234abcd);
    assert_eq!(c.dmem.h16(0x374), 0x5678);
    command(&mut c, &mut ram, 0x0900789a, 0x98765432);
    command(&mut c, &mut ram, 0x09060077, 0);
    command(&mut c, &mut ram, 0x09040088, 0);
    command(&mut c, &mut ram, 0x090f1122, 0x3344);
    assert_eq!(c.dmem.h16(0x366), 0x77);
    assert_eq!(c.dmem.h16(0x368), 0x88);
    assert_eq!(c.dmem.h16(0x376), 0x789a);
    assert_eq!(c.dmem.word(0x378), 0x98765432);
    assert_eq!(c.dmem.word(0x37c), 0x11223344);
    let before = c.dmem.bytes;
    command(&mut c, &mut ram, 0x00ffffff, u32::MAX);
    assert_eq!(c.dmem.bytes, before);
}

#[test]
fn transfers_clear_and_move_keep_rounding_overlap_and_vector_tails() {
    let mut c = Context::<Be>::default();
    let mut ram: Vec<u8> = (0..256).map(|i| i as u8).collect();
    buffers(&mut c, 0x803, 0x80f, 9);
    command(&mut c, &mut ram, 0x04000000, 11);
    assert_eq!(&c.dmem.bytes[0x800..0x810], &ram[8..24]);
    command(&mut c, &mut ram, 0x06000000, 67);
    assert_eq!(&ram[64..80], &c.dmem.bytes[0x808..0x818]);
    command(&mut c, &mut ram, 0x0b000009, 19);
    assert_eq!(&c.dmem.bytes[0x4c0..0x4d0], &ram[16..32]);
    for i in 0..96 {
        c.dmem.set_raw(0x800 + i, i as u8);
    }
    c.vectors[1] = [0x9182; 8];
    c.vectors[2] = [0x7364; 8];
    command(&mut c, &mut ram, 0x0a000241, 0x02490011);
    let mut expected = (0..96).map(|v| v as u8).collect::<Vec<_>>();
    for offset in [0, 16] {
        let block = expected[1 + offset..17 + offset].to_vec();
        expected[9 + offset..25 + offset].copy_from_slice(&block);
    }
    assert_eq!(&c.dmem.bytes[0x800..0x860], expected);
    assert_eq!(&c.vectors[1][4..], &[0x9182; 4]);
    assert_eq!(&c.vectors[2][4..], &[0x7364; 4]);
    command(&mut c, &mut ram, 0x02000243, 17);
    assert!(c.dmem.bytes[0x803..0x823].iter().all(|&v| v == 0));
    assert_eq!(c.dmem.raw(0x823), expected[35]);
    let before = c.dmem.bytes;
    buffers(&mut c, 0x803, 0x80f, 0);
    for (w0, w1) in [
        (0x04000000, u32::MAX),
        (0x06000000, u32::MAX),
        (0x02000240, 0),
        (0x0a000240, 0x02480000),
    ] {
        command(&mut c, &mut ram, w0, w1);
    }
    assert_eq!(&c.dmem.bytes[..0x360], &before[..0x360]);
    assert_eq!(&c.dmem.bytes[0x366..], &before[0x366..]);
    assert!(c.command(ram.as_mut_slice(), 0x0b000000, 0).is_err());
}

fn mix_and_interleave<O: HalfwordOrder>() {
    let mut ram = [];
    for gain in [0, 1, 16384, 32767, 32768, 65535] {
        let mut c = synthetic::<O>();
        let input = [-32768i16, 32767, 1, -1, 20000, -20000, 16383, -16383];
        let old = [32767i16, -32768, 1, 1, 25000, -25000, 0, 0];
        for i in 0..32 {
            c.dmem.set_h16(0x800 + i * 2, input[i as usize % 8] as u16);
            c.dmem.set_h16(0x900 + i * 2, old[i as usize % 8] as u16);
        }
        buffers(&mut c, 0, 0xa00, 33);
        command(&mut c, &mut ram, 0x0cfe0000 | gain, 0x02400340);
        for i in 0..32 {
            let lane = i as usize % 8;
            let expected = ((i64::from(old[lane]) * 32767
                + i64::from(input[lane]) * i64::from(gain as i16)
                + 16384)
                >> 15)
                .clamp(-32768, 32767);
            assert_eq!(c.dmem.h16(0x900 + i * 2) as i16, expected as i16);
        }
        command(&mut c, &mut ram, 0x0dffffff, 0x02400340);
        for i in 0..24 {
            assert_eq!(c.dmem.h16(0xa00 + i * 4), input[i as usize % 8] as u16);
            assert_eq!(c.dmem.h16(0xa02 + i * 4), c.dmem.h16(0x900 + i * 2));
        }
        assert_eq!(c.dmem.h16(0xa60), 0);
        buffers(&mut c, 0, 0xa00, 0);
        let before = c.dmem.bytes;
        command(&mut c, &mut ram, 0x0c008000, 0x02400340);
        command(&mut c, &mut ram, 0x0d000000, 0x02400340);
        assert_eq!(c.dmem.bytes, before);
    }
}

#[test]
fn mixer_rounding_saturation_counts_and_stereo_in_both_orders() {
    mix_and_interleave::<Be>();
    mix_and_interleave::<Ne>();
}

#[test]
fn unaligned_interleave_consumes_retained_vector_bytes() {
    let mut c = synthetic::<Be>();
    let mut ram = [];
    for i in 0..96 {
        c.dmem.set_raw(0x800 + i, (i * 37 + 13) as u8);
    }
    buffers(&mut c, 0, 0x900, 16);
    command(&mut c, &mut ram, 0x0d000000, 0x02400280);
    let mut left: Vec<_> = (0..16).map(|i| c.vbyte(1, i)).collect();
    let mut right: Vec<_> = (0..16).map(|i| c.vbyte(2, i)).collect();
    for i in 0..13 {
        left[i] = c.dmem.raw(0x803 + i as u32);
        right[i] = c.dmem.raw(0x843 + i as u32);
    }
    buffers(&mut c, 0, 0x943, 1);
    command(&mut c, &mut ram, 0x0d000000, 0x02430283);
    let expected: Vec<_> = (0..8)
        .flat_map(|i| [left[2 * i], left[2 * i + 1], right[2 * i], right[2 * i + 1]])
        .collect();
    assert_eq!(&c.dmem.bytes[0x943..0x963], expected);
    c.vectors[1][7] ^= 0x1234;
    command(&mut c, &mut ram, 0x0d000000, 0x02430283);
    assert_ne!(&c.dmem.bytes[0x943..0x963], expected);
}

fn decode<O: HalfwordOrder>(flags: u8, count: u16, large: bool) {
    let mut c = synthetic::<O>();
    let mut ram = vec![0; 512];
    let a: [i16; 8] = std::array::from_fn(|i| if large { 32767 } else { i as i16 * 87 - 300 });
    let b: [i16; 8] = std::array::from_fn(|i| if large { 32767 } else { 900 - i as i16 * 73 });
    for i in 0..8 {
        c.dmem.set_h16(0x4c0 + i as u32 * 2, a[i] as u16);
        c.dmem.set_h16(0x4d0 + i as u32 * 2, b[i] as u16);
    }
    for i in 0..16 {
        ram[128 + i * 2..130 + i * 2].copy_from_slice(&O::write((i as i16 * 197 - 900) as u16));
        ram[256 + i * 2..258 + i * 2].copy_from_slice(&O::write((1600 - i as i16 * 173) as u16));
    }
    let source_history = if flags & 2 != 0 { 256 } else { 128 };
    let history: Vec<u16> = (0..16)
        .map(|i| {
            if flags & 1 != 0 {
                0
            } else {
                O::read([ram[source_history + i * 2], ram[source_history + i * 2 + 1]])
            }
        })
        .collect();
    let mut expected = history.clone();
    for frame in 0..2 {
        c.dmem.set_raw(
            0x800 + frame * 9,
            if large {
                0xc0
            } else {
                ((frame * 15) as u8) << 4
            },
        );
        for i in 0..8 {
            c.dmem.set_raw(
                0x801 + frame * 9 + i,
                if large { 0x77 } else { (i * 29 + 37) as u8 },
            );
        }
    }
    for frame in 0..u32::from(count).div_ceil(32) {
        let scale = (c.dmem.raw(0x800 + frame * 9) >> 4).min(12);
        for half in 0..2 {
            let residual: [i64; 8] = std::array::from_fn(|i| {
                let byte = c.dmem.raw(0x801 + frame * 9 + half * 4 + i as u32 / 2);
                let nibble = if i % 2 == 0 { byte >> 4 } else { byte & 15 };
                i64::from((nibble << 4) as i8 >> 4) << scale
            });
            let h0 = i64::from(expected[expected.len() - 2] as i16);
            let h1 = i64::from(expected[expected.len() - 1] as i16);
            for i in 0..8 {
                let sum = (residual[i] << 11)
                    + i64::from(a[i]) * h0
                    + i64::from(b[i]) * h1
                    + (0..i)
                        .map(|j| i64::from(b[i - j - 1]) * residual[j])
                        .sum::<i64>();
                expected.push(((sum as i32) >> 11).clamp(-32768, 32767) as u16);
            }
        }
    }
    buffers(&mut c, 0x800, 0x900, count);
    command(&mut c, &mut ram, 0x0f000000, 256);
    command(&mut c, &mut ram, 0x01000000 | (u32::from(flags) << 16), 128);
    for (i, sample) in expected.iter().enumerate() {
        assert_eq!(c.dmem.h16(0x900 + i as u32 * 2), *sample);
    }
    for i in 0..16 {
        assert_eq!(
            O::read([ram[128 + i * 2], ram[129 + i * 2]]),
            expected[expected.len() - 16 + i]
        );
    }
    assert_eq!(c.dmem.h16(0x900 + expected.len() as u32 * 2), 0);
}

#[test]
fn adpcm_init_continue_loop_scale_clipping_and_wrapped_accumulation() {
    for flags in [0, 1, 2, 3] {
        for count in [0, 1, 33] {
            decode::<Be>(flags, count, false);
            decode::<Ne>(flags, count, true);
        }
    }
}

fn pole<O: HalfwordOrder>(init: bool) {
    let mut c = synthetic::<O>();
    let mut ram = vec![0; 256];
    let gain = 23001u16;
    let a: [i16; 8] = std::array::from_fn(|i| 9000 - i as i16 * 1500);
    let b: [i16; 8] = std::array::from_fn(|i| i as i16 * 2700 - 12000);
    let scaled = b.map(|v| ((i64::from(v) * i64::from(gain.wrapping_mul(4))) >> 16) as i16);
    for i in 0..8 {
        c.dmem.set_h16(0x4c0 + i as u32 * 2, a[i] as u16);
        c.dmem.set_h16(0x4d0 + i as u32 * 2, b[i] as u16);
    }
    let input: [i16; 16] = std::array::from_fn(|i| (i as i32 * 3701 - 30000) as i16);
    for (i, &sample) in input.iter().enumerate() {
        c.dmem.set_h16(0x800 + i as u32 * 2, sample as u16);
    }
    c.dmem.set_word(0xf90, 0xabcdef12);
    c.dmem.set_h16(0xf94, (-7000i16) as u16);
    c.dmem.set_h16(0xf96, 8000);
    ram[132..134].copy_from_slice(&O::write(13000));
    ram[134..136].copy_from_slice(&O::write((-15000i16) as u16));
    let (mut h0, mut h1) = if init {
        (-7000i64, 8000i64)
    } else {
        (13000, -15000)
    };
    let mut expected = Vec::new();
    for block in input.as_chunks::<8>().0 {
        for i in 0..8 {
            let total = i64::from(gain) * i64::from(block[i])
                + i64::from(a[i]) * h0
                + i64::from(b[i]) * h1
                + (0..i)
                    .map(|j| i64::from(scaled[i - j - 1]) * i64::from(block[j]))
                    .sum::<i64>();
            expected.push(((total as i32) >> 14).clamp(-32768, 32767) as i16);
        }
        h0 = i64::from(expected[expected.len() - 2]);
        h1 = i64::from(expected[expected.len() - 1]);
    }
    buffers(&mut c, 0x800, 0x900, 17);
    command(
        &mut c,
        &mut ram,
        0x0e000000 | (u32::from(init) << 16) | u32::from(gain),
        128,
    );
    assert_eq!(c.dmem.word(0xf90), 0);
    for (i, &sample) in expected.iter().enumerate() {
        assert_eq!(c.dmem.h16(0x900 + i as u32 * 2) as i16, sample);
    }
    for (i, &value) in scaled.iter().enumerate() {
        assert_eq!(c.dmem.h16(0x4d0 + i as u32 * 2) as i16, value);
    }
    for i in 0..4 {
        assert_eq!(
            O::read([ram[128 + 2 * i], ram[129 + 2 * i]]) as i16,
            expected[12 + i]
        );
    }
    buffers(&mut c, 0x800, 0x900, 0);
    let before = c.dmem.bytes;
    let state = ram.clone();
    command(&mut c, &mut ram, 0x0e010000, 128);
    assert_eq!(c.dmem.bytes, before);
    assert_eq!(ram, state);
}

#[test]
fn pole_partial_init_continuation_coefficient_mutation_and_saturation() {
    pole::<Be>(true);
    pole::<Ne>(false);
}

fn resample<O: HalfwordOrder>(flags: u8, count: u16, pitch: u16, carry: u16) {
    let mut c = synthetic::<O>();
    let mut ram = vec![0x5a; 256];
    for phase in 0..64 {
        let coefficients = [
            30000 - phase * 3,
            -18000 + phase * 5,
            25000 - phase * 7,
            22000 + phase * 11,
        ];
        for (i, coefficient) in coefficients.into_iter().enumerate() {
            c.dmem
                .set_h16(0xc0 + phase as u32 * 8 + i as u32 * 2, coefficient as u16);
        }
    }
    for i in 0..128 {
        c.dmem
            .set_h16(0x7e0 + i * 2, (i as i32 * 7919 - 16000) as u16);
    }
    for i in 0..16 {
        let value = match i {
            4 => 0xb111,
            5 => 6,
            _ => (i as i32 * 1289 - 27000) as u16,
        };
        ram[128 + 2 * i..130 + 2 * i].copy_from_slice(&O::write(value));
    }
    c.dmem.set_word(0xf9c, 0x7381abcd);
    let residue = if flags & 1 != 0 {
        0x7381abcd
    } else {
        (u32::from(O::read([ram[140], ram[141]])) << 16) | u32::from(O::read([ram[142], ram[143]]))
    };
    let phase = if flags & 1 != 0 { 0i64 } else { 0xb111 };
    let correction = if flags & 2 != 0 { 6 } else { 0 };
    let start = 0x7f8 - correction;
    buffers(&mut c, 0x800, 0xa00, count);
    c.vco = carry;
    c.vcc = 0x4b72;
    c.vce = 0xa7;
    command(
        &mut c,
        &mut ram,
        0x05000000 | (u32::from(flags) << 16) | u32::from(pitch),
        128,
    );
    let samples = (i32::from(count as i16).max(1) as u32).div_ceil(16) * 8;
    let frac = |x: i64, y: i64| ((x * y + 16384) >> 15).clamp(-32768, 32767);
    for i in 0..samples {
        let borrow = if i < 8 {
            (carry >> i) & 1
        } else {
            (carry >> 7) & 1
        };
        let acc = phase + (2 * i64::from(i) - i64::from(borrow)) * i64::from(pitch);
        let source = (i64::from(start) + 2 * (acc >> 16)) as u32;
        let table = 0xc0 + (((acc as u32 & 65535) >> 10) * 8);
        let products: [i64; 4] = std::array::from_fn(|j| {
            frac(
                i64::from(c.dmem.h16(source + j as u32 * 2) as i16),
                i64::from(c.dmem.h16(table + j as u32 * 2) as i16),
            )
        });
        let expected = ((products[0] + products[1]).clamp(-32768, 32767)
            + (products[2] + products[3]).clamp(-32768, 32767))
        .clamp(-32768, 32767);
        assert_eq!(
            c.dmem.h16(0xa00 + i * 2) as i16,
            expected as i16,
            "sample {i}"
        );
    }
    let next = phase + (i64::from(samples) * 2 - i64::from((carry >> 7) & 1)) * i64::from(pitch);
    let source = (i64::from(start) + 2 * (next >> 16)) as u32;
    assert_eq!(O::read([ram[136], ram[137]]), next as u16);
    for i in 0..4 {
        assert_eq!(
            O::read([ram[128 + 2 * i], ram[129 + 2 * i]]),
            c.dmem.h16(source + i as u32 * 2)
        );
    }
    assert_eq!(c.dmem.word(0xf9c), residue);
    let remainder = (source + 8).wrapping_sub(0x800) & 15;
    assert_eq!(
        O::read([ram[138], ram[139]]),
        if remainder == 0 {
            0
        } else {
            (16 - remainder) as u16
        }
    );
    assert_eq!((c.vco, c.vcc, c.vce), (0, 0x4b72, 0xa7));
}

#[test]
fn resample_rounds_products_before_adds_and_preserves_state_residue() {
    for flags in [0, 1, 2] {
        for count in [0, 17, 65535] {
            for pitch in [0, 0x6001, 0x8000, 0xffff] {
                resample::<Be>(flags, count, pitch, 0);
                resample::<Ne>(flags, count, pitch, 0x8585);
            }
        }
    }
}

fn envelope_setup<O: HalfwordOrder>() -> Context<O> {
    let mut c = synthetic::<O>();
    buffers(&mut c, 0x800, 0x900, 0);
    c.dmem.set_h16(0x36a, 0xa00);
    c.dmem.set_h16(0x36c, 0xb00);
    c.dmem.set_h16(0x36e, 0xc00);
    for i in 0..32 {
        c.dmem.set_h16(0x800 + 2 * i, 16000);
        c.dmem.set_h16(0x900 + 2 * i, 1000);
        c.dmem.set_h16(0xa00 + 2 * i, 2000);
        c.dmem.set_h16(0xb00 + 2 * i, 3000);
        c.dmem.set_h16(0xc00 + 2 * i, 4000);
    }
    let mut ram = [];
    for (w0, w1) in [
        (0x09064000, 0),
        (0x09042000, 0),
        (0x09024000, 0x10000),
        (0x09002000, 0x10000),
        (0x09084000, 0x2000),
    ] {
        command(&mut c, &mut ram, w0, w1);
    }
    c
}

fn envelope_constant<O: HalfwordOrder>() {
    let mut c = envelope_setup::<O>();
    let mut ram = vec![0xa5; 256];
    command(&mut c, &mut ram, 0x03090000, 128);
    for i in 0..16 {
        assert_eq!(c.dmem.h16(0x900 + i * 2), 5000);
        assert_eq!(c.dmem.h16(0xa00 + i * 2), 4000);
        assert_eq!(c.dmem.h16(0xb00 + i * 2), 5000);
        assert_eq!(c.dmem.h16(0xc00 + i * 2), 5000);
    }
    assert_eq!(c.dmem.h16(0x920), 1000);
    for i in 0..8 {
        assert_eq!(O::read([ram[128 + i * 2], ram[129 + i * 2]]), 16384);
        assert_eq!(O::read([ram[144 + i * 2], ram[145 + i * 2]]), 0);
        assert_eq!(O::read([ram[160 + i * 2], ram[161 + i * 2]]), 8192);
    }
    command(&mut c, &mut ram, 0x03080000, 128);
    for i in 0..8 {
        assert_eq!(c.dmem.h16(0x900 + i * 2), 9000);
        assert_eq!(c.dmem.h16(0xa00 + i * 2), 6000);
    }
    assert_eq!(c.dmem.h16(0x910), 5000);
    c.dmem.bytes[0xfe0..0xff0].fill(0);
    command(&mut c, &mut ram, 0x03000000, 128);
    for i in 0..8 {
        assert_eq!(c.dmem.h16(0xfe0 + i * 2), 1000);
    }
    c.dmem.bytes[0xfe0..0xff0].fill(0);
    command(&mut c, &mut ram, 0x03010000, 128);
    for i in 0..8 {
        assert_eq!(c.dmem.h16(0xfe0 + i * 2), 4000);
    }
}

#[test]
fn envelope_minimum_counts_continuation_and_aux_scratch_order() {
    envelope_constant::<Be>();
    envelope_constant::<Ne>();
}

#[test]
fn envelope_ramps_fractional_state_and_clips_targets() {
    let mut c = envelope_setup::<Be>();
    let mut ram = vec![0; 256];
    c.dmem.set_h16(0x366, 10000);
    c.dmem.set_h16(0x368, 20000);
    c.dmem.set_h16(0x370, 13000);
    c.dmem.set_word(0x372, 0x18000);
    c.dmem.set_h16(0x376, 10000);
    c.dmem.set_word(0x378, 0x8000);
    command(&mut c, &mut ram, 0x03090000, 128);
    for i in 0..8 {
        assert_eq!(c.dmem.h16(0xf90 + i * 2), 13000);
        assert_eq!(c.dmem.h16(0xfb0 + i * 2), 10000);
    }
    assert_eq!(c.dmem.h16(0xfa0), 0);
    assert_eq!(c.dmem.h16(0xfa2), 50048);
    assert_eq!(c.dmem.h16(0xfc2), 54016);
    assert_eq!(c.vectors[20][0], 19500);
    command(&mut c, &mut ram, 0x03080000, 128);
    assert_eq!(c.dmem.h16(0xf90), 13000);
    assert_eq!(c.dmem.h16(0xfb0), 10000);
}

#[test]
fn clip_consumes_vco_vcc_and_vce() {
    let mut c = Context::<Be>::default();
    c.vectors[20] = [100, 65526, 65526, 100, 100, 100, 100, 100];
    c.vco = 0b00001111 | (0b00001000 << 8);
    c.vce = 0b00000001;
    c.vcc = 0b00001000;
    c.clip(20, 10, true);
    assert_eq!(
        c.vectors[20],
        [
            65526,
            100u16.wrapping_sub(110),
            65526,
            65526,
            10,
            10,
            10,
            10
        ]
    );
    assert_eq!((c.vco, c.vce), (0, 0));
    c.vectors[20] = [10; 8];
    c.vco = 0x0101;
    c.clip(20, 10, false);
    assert_eq!(c.vcc, 0xfe);
}

#[test]
fn dispatcher_ignores_the_opcode_high_bit_for_save() {
    let mut c = Context::<Be>::default();
    let mut ram = vec![0xa5; 128];
    c.dmem.bytes[0x800..0x810].fill(0x37);
    buffers(&mut c, 0x900, 0x800, 16);
    command(&mut c, &mut ram, 0x86000000, 32);
    assert_eq!(&ram[32..48], &[0x37; 16]);
    assert_eq!(&c.dmem.bytes[0x900..0x910], &[0; 16]);
}

fn move_then_partial_interleave<O: HalfwordOrder>() -> Vec<u16> {
    let mut c = synthetic::<O>();
    let mut ram = [];
    for i in 0..16 {
        c.dmem.set_h16(0x800 + 2 * i, (i * 279 + 351) as u16);
        c.dmem.set_h16(0x840 + 2 * i, (i * 97 + 813) as u16);
    }
    command(&mut c, &mut ram, 0x0a000240, 0x03800010);
    buffers(&mut c, 0, 0xa00, 16);
    command(&mut c, &mut ram, 0x0d000000, 0x024a028a);
    (0..16).map(|i| c.dmem.h16(0xa00 + i * 2)).collect()
}

#[test]
fn move_retains_halfword_lanes_for_later_partial_interleave() {
    let big = move_then_partial_interleave::<Be>();
    let native = move_then_partial_interleave::<Ne>();
    assert_eq!(big, native);
    assert_eq!(big[6], 3 * 279 + 351);
    assert_eq!(big[7], 7 * 279 + 351);
}

#[test]
fn envelope_continue_consumes_inherited_clip_flags() {
    for (vco, vcc, vce, expected) in [
        (0, 0, 0, 305),
        (1, 0, 0, 305),
        (1, 0, 1, -610),
        (0x101, 0, 0, 305),
        (0x101, 1, 0, -610),
    ] {
        let mut c = synthetic::<Be>();
        let mut ram = vec![0; 256];
        buffers(&mut c, 0x800, 0x900, 0);
        for (offset, address) in [(10, 0xa00), (12, 0xb00), (14, 0xc00)] {
            c.dmem.set_h16(0x360 + offset, address);
        }
        for i in 0..8 {
            c.dmem.set_h16(0x800 + i * 2, 10000);
            ram[128 + i as usize * 2..130 + i as usize * 2].copy_from_slice(&1000u16.to_be_bytes());
        }
        for (i, value) in [2000u16, 1, 0, 0, 0, 0, 32767, 0].into_iter().enumerate() {
            ram[192 + i * 2..194 + i * 2].copy_from_slice(&value.to_be_bytes());
        }
        c.vco = vco;
        c.vcc = vcc;
        c.vce = vce;
        command(&mut c, &mut ram, 0x03080000, 128);
        assert_eq!(c.dmem.h16(0x900) as i16, expected);
        assert_eq!(c.dmem.h16(0x902), 305);
        assert_eq!((c.vco, c.vce), (0, 0));
    }
}

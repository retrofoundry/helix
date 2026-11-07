use super::*;

#[test]
fn tokens_preserve_full_addresses_residues_and_reject_modified_words() {
    let mut tokens = Tokens::default();
    for residue in 0..8 {
        let address = 0x1_1234_5600 + residue;
        let token = tokens.insert(address).unwrap();
        assert_eq!(token & 7, residue as u32);
        assert_eq!(tokens.insert(address).unwrap(), token);
        assert_eq!(tokens.address(token).unwrap(), address);
        assert!(tokens.address(token ^ 0x10000).is_err());
        assert!(tokens.address(token ^ 1).is_err());
    }
    assert!(tokens.address(0).is_err());
}

#[test]
fn data_load_converts_halfwords_without_clearing_carried_state() {
    let mut context = NativeContext::default();
    context.state.dmem.bytes.fill(0xa5);
    context.state.vco = 0x1234;
    context.load_data(&[0x12, 0x34, 0xab, 0xcd]);
    assert_eq!(context.state.dmem.h16(0), 0x1234);
    assert_eq!(context.state.dmem.word(0), 0x1234abcd);
    assert_eq!(context.state.dmem.raw(4), 0xa5);
    assert_eq!(context.state.vco, 0x1234);
}

unsafe fn run(context: &mut NativeContext, packets: &[[usize; 2]]) -> Result<(), String> {
    unsafe { context.commands(packets.as_ptr(), (packets.len() * 16) as u32) }
}

#[test]
fn packets_use_low_immediates_full_pointers_and_ignore_segments() {
    let mut context = NativeContext::default();
    let mut source = [0x1122334455667788u64; 8];
    let mut dest = [0u64; 8];
    let input = source.as_mut_ptr() as usize;
    let output = dest.as_mut_ptr() as usize;
    if cfg!(target_pointer_width = "64") {
        assert!(input > u32::MAX as usize);
        assert!(output > u32::MAX as usize);
    }
    let packets = [
        [0x12345678_07000000, 0xdeadbeef_00200000],
        [0x12345678_08000243, 0xdeadbeef_03430009],
        [0x12345678_04000000, input + 3],
        [0x12345678_08000343, 0xdeadbeef_02430009],
        [0x12345678_86000000, output + 5],
    ];
    unsafe { run(&mut context, &packets) }.unwrap();
    assert_eq!(&dest[..2], &source[..2]);
    assert_eq!(dest[2], 0);
    assert_eq!(context.state.dmem.word(0x320), 0x200000);
    assert_eq!(context.state.dmem.h16(0x360), 0x903);
    assert_eq!(context.state.dmem.h16(0x362), 0x803);
    assert_eq!(context.state.dmem.h16(0x364), 9);
    assert_eq!(&context.state.dmem.bytes[0x380..0x4c0], &[0; 320]);
}

#[test]
fn host_dma_rounds_wraps_and_keeps_row_skips_unaligned() {
    let mut context = NativeContext::default();
    let mut source = [0u64; 32];
    let mut dest = [0xa5a5a5a5a5a5a5a5u64; 32];
    let input = source.as_mut_ptr().cast::<u8>();
    for i in 0..256 {
        unsafe { input.add(i).write(i as u8) };
    }
    let from = context.memory.tokens.insert(input as usize + 3).unwrap();
    let to = context
        .memory
        .tokens
        .insert(dest.as_mut_ptr() as usize + 7)
        .unwrap();
    let value = (5 << 20) | (2 << 12) | 9;
    context
        .memory
        .dma(&mut context.state.dmem, 0xffd, from, value, false)
        .unwrap();
    context
        .memory
        .dma(&mut context.state.dmem, 0xffb, to, value, true)
        .unwrap();
    let bytes = unsafe { std::slice::from_raw_parts(dest.as_ptr().cast::<u8>(), 256) };
    for row in 0..3 {
        for i in 0..16 {
            assert_eq!(bytes[row * 21 + i], (row * 21 + i) as u8);
            assert_eq!(
                context.state.dmem.raw((0xff8 + row * 16 + i) as u32),
                (row * 21 + i) as u8
            );
        }
        assert!(bytes[row * 21 + 16..row * 21 + 21]
            .iter()
            .all(|&v| v == 0xa5));
    }
}

#[test]
fn loop_token_survives_task_reload_and_setvol_can_alias_or_invalidate_it() {
    let mut context = NativeContext::default();
    let mut history = [0x1234123412341234u64; 6];
    let mut output = [0u64; 4];
    let pointer = history.as_mut_ptr() as usize + 3;
    unsafe { run(&mut context, &[[0x0f000000, pointer]]) }.unwrap();
    let token = context.state.dmem.word(0x370);
    assert_eq!(token & 7, 3);
    context.load_data(&[0; DATA_SIZE]);
    let decode = [
        [0x08000240, 0x03400000],
        [0x01020000, output.as_mut_ptr() as usize],
    ];
    unsafe { run(&mut context, &decode) }.unwrap();
    assert_eq!(context.state.dmem.h16(0x900), 0x1234);
    assert!(output.iter().all(|&v| v == 0x1234123412341234));
    unsafe {
        run(
            &mut context,
            &[[0x09020000 | (token >> 16) as usize, (token << 16) as usize]],
        )
    }
    .unwrap();
    assert_eq!(context.state.dmem.word(0x370), token);
    unsafe { run(&mut context, &decode) }.unwrap();
    unsafe { run(&mut context, &[[0x0902ffff, 0xffffffff]]) }.unwrap();
    assert_eq!(context.state.dmem.word(0x370), u32::MAX);
    assert!(unsafe { run(&mut context, &decode) }
        .unwrap_err()
        .contains("unmapped"));
}

#[test]
fn resample_scratch_retains_a_token_and_packet_count_uses_sixteen_bytes() {
    let mut context = NativeContext::default();
    let mut state = [0u64; 4];
    let packets = [
        [0x08000240, 0x03400000],
        [0x05010000, state.as_mut_ptr() as usize],
    ];
    unsafe { context.commands(packets.as_ptr(), 47) }.unwrap();
    let token = context.state.dmem.word(0xfd0);
    assert_eq!(
        context.memory.tokens.address(token).unwrap(),
        state.as_ptr() as usize
    );
    unsafe { context.commands(std::ptr::null(), 15) }.unwrap();
}

#[test]
fn task_rejects_wrong_extent_or_hash_before_executing_commands() {
    let mut context = NativeContext::default();
    let mut data = [0u64; DATA_SIZE / 8];
    let mut task = OSTaskT {
        ucode_data: data.as_mut_ptr(),
        ucode_data_size: DATA_SIZE as u32,
        data_ptr: std::ptr::dangling_mut(),
        data_size: 16,
        ..OSTaskT::default()
    };
    let error = unsafe { context.task((&task as *const OSTaskT).cast()) }.unwrap_err();
    assert!(error.contains(REVISION) && error.contains("SHA-256"));
    task.ucode_data_size -= 1;
    task.ucode_data = std::ptr::dangling_mut();
    let error = unsafe { context.task((&task as *const OSTaskT).cast()) }.unwrap_err();
    assert!(error.contains(REVISION) && error.contains("704"));
}

#[test]
fn packets_are_read_in_place_after_a_dma_overwrites_the_next_command() {
    let mut context = NativeContext::default();
    let mut packets = [
        [0x08000000usize, 0x02400010],
        [0x06000000, 0],
        [0x7f000000, 0],
    ];
    packets[1][1] = packets.as_mut_ptr() as usize + 32;
    for (i, value) in [0x09061234usize, 0].into_iter().enumerate() {
        context.state.dmem.bytes[0x800 + i * 8..0x808 + i * 8]
            .copy_from_slice(&value.to_ne_bytes());
    }
    unsafe { context.commands(packets.as_ptr(), 48) }.unwrap();
    assert_eq!(context.state.dmem.h16(0x366), 0x1234);
}

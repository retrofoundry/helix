use super::native::NativeContext;
use super::Context;
use crate::ultra::rcp::OSTaskT;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

fn read_json(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

fn number(value: &Value) -> usize {
    value.as_u64().unwrap() as usize
}

fn hex(value: &Value) -> Vec<u8> {
    let text = value.as_str().unwrap();
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[test]
#[ignore]
fn console_oracle() {
    let Some(suite) = std::env::var_os("REALITY64_AUDIO_ORACLE") else {
        eprintln!("skipping console oracle: REALITY64_AUDIO_ORACLE is unset");
        return;
    };
    let suite = Path::new(&suite);
    let Some(output) = std::env::var_os("REALITY64_AUDIO_ORACLE_OUT") else {
        eprintln!("skipping console oracle: REALITY64_AUDIO_ORACLE_OUT is unset");
        return;
    };
    let output = Path::new(&output);
    assert!(
        !output.exists(),
        "choose a fresh REALITY64_AUDIO_ORACLE_OUT directory"
    );
    let build = std::env::var_os("REALITY64_AUDIO_ORACLE_BUILD")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| suite.parent().unwrap().to_owned());
    let runs = read_json(&suite.join("runs.json"));
    let mut total = 0;
    for run in runs.as_array().unwrap() {
        if number(&run["games"]["sm64"]) == 0 {
            continue;
        }
        let name = run["run"].as_str().unwrap();
        assert_eq!(Path::new(name).components().count(), 1);
        let manifest_path = suite.join(name).join("cases.json");
        let manifest = read_json(&manifest_path);
        let dest = output.join(name);
        fs::create_dir_all(&dest).unwrap();
        fs::copy(&manifest_path, dest.join("cases.json")).unwrap();
        let mut ram = vec![0; 0x800000];
        let mut context = Context::default();
        let mut records = Vec::new();
        for case in manifest["cases"].as_array().unwrap() {
            if case["game"] != "sm64" {
                continue;
            }
            if case["reset"].as_bool().unwrap() {
                ram[0x200000..0x210000].fill(0);
                context = Context::default();
            }
            for input in case["inputs"].as_array().unwrap() {
                let bytes = hex(&input["hex"]);
                let start = number(&input["address"]);
                ram[start..start + bytes.len()].copy_from_slice(&bytes);
            }
            let commands = hex(&case["commands"]);
            ram[0x203000..0x203000 + commands.len()].copy_from_slice(&commands);
            for (kind, address) in [("text", 0x200000), ("data", 0x201000), ("boot", 0x202000)] {
                let blob_name = format!("sm64-{kind}");
                let spec = &manifest["blobs"][&blob_name];
                let mut blob = fs::read(build.join(format!("{blob_name}.bin"))).unwrap();
                assert!(
                    hash(&blob) == spec["sha256"].as_str().unwrap(),
                    "oracle blob identity mismatch"
                );
                assert_eq!(blob.len(), number(&spec["size"]));
                blob.resize(number(&spec["embedded_size"]), 0);
                ram[address..address + blob.len()].copy_from_slice(&blob);
            }
            let task = [
                2,
                0,
                0x202000,
                number(&manifest["blobs"]["sm64-boot"]["size"]) as u32,
                0x200000,
                0x800,
                0x201000,
                number(&manifest["blobs"]["sm64-data"]["size"]) as u32,
                0,
                0,
                0,
                0,
                0x203000,
                commands.len() as u32,
                0,
                0,
            ];
            for (i, word) in task.iter().enumerate() {
                ram[0x20f000 + 4 * i..0x20f004 + 4 * i].copy_from_slice(&word.to_be_bytes());
            }
            context
                .console_task(&mut ram, &task)
                .unwrap_or_else(|error| panic!("{}: {error}", case["id"]));
            let mut bytes = Vec::new();
            for region in case["regions"].as_array().unwrap() {
                let start = number(&region["address"]);
                bytes.extend_from_slice(&ram[start..start + number(&region["size"])]);
            }
            bytes.extend_from_slice(&context.dmem.bytes);
            let index = number(&case["index"]);
            fs::write(dest.join(format!("case-{index:03}.bin")), &bytes).unwrap();
            fs::write(
                dest.join(format!("case-{index:03}-dmem.bin")),
                context.dmem.bytes,
            )
            .unwrap();
            records.push(json!({"index": index, "id": case["id"], "sha256": hash(&bytes)}));
            total += 1;
        }
        fs::write(
            dest.join("results.json"),
            serde_json::to_vec_pretty(&json!({
                "build_id": manifest["build_id"], "results": records
            }))
            .unwrap(),
        )
        .unwrap();
    }
    assert!(total > 0);
    eprintln!("wrote {total} classic oracle results");
}

fn swap(bytes: &mut [u8]) {
    if cfg!(target_endian = "little") {
        for pair in bytes.as_chunks_mut::<2>().0 {
            pair.swap(0, 1);
        }
    }
}

// Recipe tags: 1 is a host-order halfword byte; 2..=5 are the four bytes of an address.
fn normalize(bytes: &mut [u8], types: &[Value], context: &NativeContext, base: usize) {
    let original = bytes.to_vec();
    let mut i = 0;
    while i < bytes.len() {
        match number(&types[i]) {
            1 if i + 1 < bytes.len() && number(&types[i + 1]) == 1 => {
                swap(&mut bytes[i..i + 2]);
                i += 2;
            }
            2 if i + 3 < bytes.len() && (0..4).all(|j| number(&types[i + j]) == j + 2) => {
                let hi = u16::from_ne_bytes([original[i], original[i + 1]]);
                let lo = u16::from_ne_bytes([original[i + 2], original[i + 3]]);
                let address = context
                    .retained_address((u32::from(hi) << 16) | u32::from(lo))
                    .unwrap();
                let console = u32::try_from(address - base + 0x200000).unwrap();
                bytes[i..i + 4].copy_from_slice(&console.to_be_bytes());
                i += 4;
            }
            _ => i += 1,
        }
    }
}

#[test]
#[ignore]
fn native_oracle() {
    let Some(recipe) = std::env::var_os("REALITY64_AUDIO_NATIVE") else {
        eprintln!("skipping native oracle: REALITY64_AUDIO_NATIVE is unset");
        return;
    };
    let recipe = Path::new(&recipe);
    let manifest = read_json(&recipe.join("recipe.json"));
    let build = Path::new(manifest["build"].as_str().unwrap());
    let data = fs::read(build.join("sm64-data.bin")).unwrap();
    let mut storage = vec![0; 0x10000 + 7];
    let alignment = storage.as_ptr().align_offset(8);
    let arena = &mut storage[alignment..alignment + 0x10000];
    let base = arena.as_mut_ptr() as usize;
    assert_eq!(base & 7, 0);
    let mut context = NativeContext::default();
    let mut residues = [false; 8];
    for case in manifest["cases"].as_array().unwrap() {
        if case["reset"].as_bool().unwrap() {
            context = NativeContext::default();
            arena.fill(0);
        }
        for input in case["inputs"].as_array().unwrap() {
            let mut bytes = hex(&input["hex"]);
            if input["halfwords"].as_bool().unwrap() {
                swap(&mut bytes);
            }
            let start = number(&input["address"]) - 0x200000;
            arena[start..start + bytes.len()].copy_from_slice(&bytes);
        }
        for (kind, offset) in [("text", 0), ("data", 0x1000), ("boot", 0x2000)] {
            let mut bytes = fs::read(build.join(format!("sm64-{kind}.bin"))).unwrap();
            if kind == "data" {
                swap(&mut bytes);
            }
            arena[offset..offset + bytes.len()].copy_from_slice(&bytes);
        }
        let packets: Vec<[usize; 2]> = case["packets"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| {
                let word = number(&p[1]);
                let operand = if p[2].as_bool().unwrap() {
                    assert!((0x200000..0x210000).contains(&word));
                    residues[word & 7] = true;
                    base + word - 0x200000
                } else {
                    word | 0xdeadbeef00000000usize
                };
                [number(&p[0]) | 0x1234567800000000usize, operand]
            })
            .collect();
        let task = OSTaskT {
            type_: 2,
            ucode_data: data.as_ptr() as *mut u64,
            ucode_data_size: data.len() as u32,
            data_ptr: packets.as_ptr() as *mut u64,
            data_size: (packets.len() * 16) as u32,
            ..OSTaskT::default()
        };
        unsafe { context.task((&task as *const OSTaskT).cast()) }
            .unwrap_or_else(|error| panic!("{}: {error}", case["id"]));
        let mut bytes = Vec::new();
        for region in case["regions"].as_array().unwrap() {
            let start = number(&region["address"]) - 0x200000;
            bytes.extend_from_slice(&arena[start..start + number(&region["size"])]);
        }
        bytes.extend_from_slice(&context.state.dmem.bytes);
        normalize(
            &mut bytes,
            case["types"].as_array().unwrap(),
            &context,
            base,
        );
        fs::write(
            recipe.join(format!("{}.bin", case["id"].as_str().unwrap())),
            bytes,
        )
        .unwrap();
    }
    fs::write(
        recipe.join("host.json"),
        serde_json::to_vec_pretty(&json!({
            "pointers_above_4g": base > u32::MAX as usize,
            "residues": (0..8).filter(|&i| residues[i]).collect::<Vec<_>>()
        }))
        .unwrap(),
    )
    .unwrap();
    eprintln!(
        "wrote {} native classic results",
        manifest["cases"].as_array().unwrap().len()
    );
}

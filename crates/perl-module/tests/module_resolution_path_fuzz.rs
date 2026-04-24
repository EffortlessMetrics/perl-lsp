use perl_module::resolution::path::resolve_module_path;
use std::path::PathBuf;

fn next_u64(state: &mut u64) -> u64 {
    *state ^= *state >> 12;
    *state ^= *state << 25;
    *state ^= *state >> 27;
    state.wrapping_mul(0x2545F4914F6CDD1D)
}

fn fuzz_segment(state: &mut u64, max_len: usize) -> String {
    let len = (next_u64(state) as usize % max_len).saturating_add(1);
    let mut out = String::with_capacity(len);
    for _ in 0..len {
        let byte = (next_u64(state) & 0x7F) as u8;
        let ch = match byte {
            0..=31 => b'a' + (byte % 26) as u8,
            b':' | b'/' | b'\\' => b'x',
            _ => byte,
        } as char;
        out.push(ch);
    }
    out
}

fn fuzz_module_segment(state: &mut u64, max_len: usize) -> String {
    let len = (next_u64(state) as usize % max_len).saturating_add(1);
    let mut out = String::with_capacity(len);
    for _ in 0..len {
        let idx = (next_u64(state) % 63) as u8;
        let ch = match idx {
            0..=25 => (b'A' + idx) as char,
            26..=51 => (b'a' + (idx - 26)) as char,
            52..=61 => (b'0' + (idx - 52)) as char,
            _ => '_',
        };
        out.push(ch);
    }
    out
}

fn fuzz_include_path(state: &mut u64) -> String {
    let mode = next_u64(state) % 8;
    match mode {
        0 => ".".to_string(),
        1 => "..".to_string(),
        2 => format!("../{}", fuzz_segment(state, 10)),
        3 => format!("./{}", fuzz_segment(state, 10)),
        4 => format!("{}/../{}", fuzz_segment(state, 8), fuzz_segment(state, 8)),
        5 => format!("{}//{}", fuzz_segment(state, 8), fuzz_segment(state, 8)),
        6 => format!("\\{}\\{}", fuzz_segment(state, 8), fuzz_segment(state, 8)),
        _ => fuzz_segment(state, 12),
    }
}

#[test]
fn fuzz_inputs_never_panic_and_return_root_prefixed_path() {
    let mut seed = 0xC0FFEE_u64;
    let root = PathBuf::from("/workspace");

    for _ in 0..2000 {
        let module = format!(
            "{}::{}",
            fuzz_module_segment(&mut seed, 12),
            fuzz_module_segment(&mut seed, 12),
        );

        // Include path generation intentionally covers traversal-like and odd separator inputs
        // to exercise workspace path-security validation in resolution.
        let include_paths: Vec<String> = (0..4).map(|_| fuzz_include_path(&mut seed)).collect();

        let resolved = resolve_module_path(&root, &module, &include_paths);

        assert!(resolved.is_some(), "resolver unexpectedly returned None");

        if let Some(resolved) = resolved {
            assert!(resolved.to_string_lossy().ends_with(".pm"));
            assert!(
                resolved.starts_with(&root),
                "resolved path escaped workspace root: {}",
                resolved.display()
            );
        }
    }
}

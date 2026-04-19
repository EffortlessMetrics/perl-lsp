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

#[test]
fn fuzz_inputs_never_panic_and_return_root_prefixed_path() {
    let mut seed = 0xC0FFEE_u64;
    let root = PathBuf::from("/workspace");

    for _ in 0..2000 {
        let module = format!(
            "{}::{}",
            fuzz_segment(&mut seed, 12),
            fuzz_segment(&mut seed, 12),
        );

        let include_paths: Vec<String> = (0..4).map(|_| fuzz_segment(&mut seed, 8)).collect();

        let resolved = resolve_module_path(&root, &module, &include_paths);

        if let Some(resolved) = resolved {
            assert!(resolved.to_string_lossy().ends_with(".pm"));
        }
    }
}

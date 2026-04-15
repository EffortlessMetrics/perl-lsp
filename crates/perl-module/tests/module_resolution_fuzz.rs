use perl_module::resolution::{ModuleUriResolution, resolve_module_uri};
use std::path::PathBuf;
use std::time::Duration;

fn next_u64(state: &mut u64) -> u64 {
    // xorshift64*
    *state ^= *state >> 12;
    *state ^= *state << 25;
    *state ^= *state >> 27;
    state.wrapping_mul(0x2545F4914F6CDD1D)
}

fn fuzz_string(state: &mut u64, max_len: usize) -> String {
    let len = (next_u64(state) as usize % max_len).saturating_add(1);
    let mut out = String::with_capacity(len);
    for _ in 0..len {
        let byte = (next_u64(state) & 0x7F) as u8;
        let ch = match byte {
            0..=31 => '_',
            b':' | b'/' | b'\\' => byte as char,
            _ => byte as char,
        };
        out.push(ch);
    }
    out
}

#[test]
fn fuzz_resolution_inputs_never_panic_and_emit_valid_uri_shape() {
    let mut seed = 0xC0FFEE_u64;

    for _ in 0..2000 {
        let module_name = fuzz_string(&mut seed, 40);

        let open_docs: Vec<String> =
            (0..3).map(|_| format!("file:///{}", fuzz_string(&mut seed, 24))).collect();
        let workspace_folders: Vec<String> = (0..3)
            .map(|_| {
                if (next_u64(&mut seed) & 1) == 0 {
                    format!("file:///{}", fuzz_string(&mut seed, 24))
                } else {
                    fuzz_string(&mut seed, 24)
                }
            })
            .collect();
        let include_paths: Vec<String> = (0..4).map(|_| fuzz_string(&mut seed, 12)).collect();
        let system_inc: Vec<PathBuf> =
            (0..2).map(|_| PathBuf::from(fuzz_string(&mut seed, 20))).collect();

        let result = resolve_module_uri(
            &module_name,
            &open_docs,
            &workspace_folders,
            &include_paths,
            (next_u64(&mut seed) & 1) == 0,
            &system_inc,
            Duration::from_millis(5),
        );

        if let ModuleUriResolution::Resolved(uri) = result {
            assert!(uri.starts_with("file://"));
        }
    }
}

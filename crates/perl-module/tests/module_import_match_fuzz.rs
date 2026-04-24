use perl_module::import::parse_module_import_head;
use perl_module::import_match::line_references_module_import;

fn next_u64(state: &mut u64) -> u64 {
    *state ^= *state >> 12;
    *state ^= *state << 25;
    *state ^= *state >> 27;
    state.wrapping_mul(0x2545F4914F6CDD1D)
}

fn fuzz_string(state: &mut u64, max_len: usize) -> String {
    const ALPHABET: &[u8] =
        b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_:'\";()[]{} \t";

    let len = (next_u64(state) as usize % max_len).saturating_add(1);
    let mut out = String::with_capacity(len);

    for _ in 0..len {
        let idx = (next_u64(state) as usize) % ALPHABET.len();
        out.push(ALPHABET[idx] as char);
    }

    out
}

#[test]
fn fuzz_import_match_predicate_never_panics_and_only_matches_import_lines() {
    let mut seed = 0xF00DBABE_u64;

    for _ in 0..5000 {
        let line = fuzz_string(&mut seed, 128);
        let module = fuzz_string(&mut seed, 48).replace(' ', "::");

        let matched = line_references_module_import(&line, &module);

        if matched {
            assert!(parse_module_import_head(&line).is_some());
            assert!(!module.is_empty());
        }
    }
}

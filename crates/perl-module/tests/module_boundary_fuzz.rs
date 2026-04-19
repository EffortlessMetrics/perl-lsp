use perl_module::boundary::{
    contains_standalone_module_token, find_standalone_module_token_ranges,
};

fn next_u64(state: &mut u64) -> u64 {
    *state ^= *state >> 12;
    *state ^= *state << 25;
    *state ^= *state >> 27;
    state.wrapping_mul(0x2545F4914F6CDD1D)
}

fn fuzz_string(state: &mut u64, max_len: usize) -> String {
    let len = (next_u64(state) as usize % max_len).saturating_add(1);
    let mut out = String::with_capacity(len);

    const ALPHABET: &[u8] =
        b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_:'; ()[]{}";

    for _ in 0..len {
        let idx = (next_u64(state) as usize) % ALPHABET.len();
        out.push(ALPHABET[idx] as char);
    }

    out
}

#[test]
fn fuzz_boundary_scanner_never_panics_and_emits_valid_ranges() {
    let mut seed = 0xB0A1_DA7A_u64;

    for _ in 0..5000 {
        let line = fuzz_string(&mut seed, 256);
        let module_name = fuzz_string(&mut seed, 24);

        let ranges = find_standalone_module_token_ranges(&line, &module_name).collect::<Vec<_>>();
        assert_eq!(
            contains_standalone_module_token(&line, &module_name),
            !ranges.is_empty()
        );

        let mut prev_end = 0usize;
        for range in ranges {
            assert!(range.start <= range.end);
            assert!(range.end <= line.len());
            assert!(line.is_char_boundary(range.start));
            assert!(line.is_char_boundary(range.end));
            assert!(range.start >= prev_end);
            assert_eq!(&line[range.start..range.end], module_name);
            prev_end = range.end;
        }
    }
}

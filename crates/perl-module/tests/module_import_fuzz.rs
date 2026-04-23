use perl_module::import::parse_module_import_head;

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
fn fuzz_import_head_parser_preserves_range_and_token_invariants() {
    let mut seed = 0xD00D_FEED_u64;

    for _ in 0..5000 {
        let line = fuzz_string(&mut seed, 128);
        let parsed = parse_module_import_head(&line);

        if let Some(head) = parsed {
            assert!(head.token_start < head.token_end);
            assert!(head.token_end <= line.len());
            assert_eq!(&line[head.token_start..head.token_end], head.token);
            assert!(!head.token.is_empty());
        }
    }
}

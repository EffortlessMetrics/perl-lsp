use perl_module::token::{contains_module_token, replace_module_token};

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
        b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_:';,()[]{} -/\t";

    for _ in 0..len {
        let idx = (next_u64(state) as usize) % ALPHABET.len();
        out.push(ALPHABET[idx] as char);
    }

    out
}

#[test]
fn fuzz_module_token_replacement_preserves_core_invariants() {
    let mut seed = 0xBADC0FFEE_u64;

    for _ in 0..5000 {
        let line = fuzz_string(&mut seed, 128);
        let from = fuzz_string(&mut seed, 32).replace(' ', "::");
        let to = fuzz_string(&mut seed, 32).replace(' ', "::");

        let (rewritten, changed) = replace_module_token(&line, &from, &to);

        assert!(rewritten.is_char_boundary(rewritten.len()));

        if !changed {
            assert_eq!(rewritten, line);
        }

        if changed && from != to {
            assert_ne!(rewritten, line);
        }

        let (_, self_changed) = replace_module_token(&line, &from, &from);
        assert_eq!(contains_module_token(&line, &from), self_changed);
    }
}

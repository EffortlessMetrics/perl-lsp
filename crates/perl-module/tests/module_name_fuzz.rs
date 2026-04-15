use perl_module::name::{
    legacy_package_separator, module_variant_pairs, normalize_package_separator,
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

    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_:'";

    for _ in 0..len {
        let idx = (next_u64(state) as usize) % ALPHABET.len();
        out.push(ALPHABET[idx] as char);
    }

    out
}

#[test]
fn fuzz_module_name_helpers_preserve_core_invariants() {
    let mut seed = 0xFACE_B00C_u64;

    for _ in 0..5000 {
        let old_module = fuzz_string(&mut seed, 64);
        let new_module = fuzz_string(&mut seed, 64);

        let normalized_old = normalize_package_separator(&old_module).into_owned();
        let normalized_new = normalize_package_separator(&new_module).into_owned();
        let legacy_old = legacy_package_separator(&normalized_old).into_owned();
        let legacy_new = legacy_package_separator(&normalized_new).into_owned();

        assert!(normalized_old.is_char_boundary(normalized_old.len()));
        assert!(normalized_new.is_char_boundary(normalized_new.len()));
        assert!(!normalized_old.contains('\''));
        assert!(!normalized_new.contains('\''));

        assert_eq!(normalize_package_separator(&legacy_old), normalized_old);
        assert_eq!(normalize_package_separator(&legacy_new), normalized_new);

        let pairs = module_variant_pairs(&old_module, &new_module);
        assert!(!pairs.is_empty());
        assert!(pairs.len() <= 2);
        assert_eq!(pairs[0].0, normalized_old);
        assert_eq!(pairs[0].1, normalized_new);

        if pairs.len() == 2 {
            assert_ne!(pairs[0], pairs[1]);
        }
    }
}

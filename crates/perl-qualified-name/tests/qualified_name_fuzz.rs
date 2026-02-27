use perl_qualified_name::{split_qualified_name, validate_perl_qualified_name};

fn next_u64(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    state.wrapping_mul(0x2545F4914F6CDD1D)
}

fn fuzz_name(state: &mut u64, max_len: usize) -> String {
    let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_'::";
    let len = 1 + (next_u64(state) as usize % max_len);
    let mut out = String::with_capacity(len);

    for _ in 0..len {
        let idx = (next_u64(state) as usize) % alphabet.len();
        out.push(alphabet[idx] as char);
    }

    out
}

#[test]
fn fuzz_qualified_name_helpers_preserve_split_and_validation_invariants() {
    let mut state = 0xFACE_B00C_DEAD_F00D_u64;

    for _ in 0..5000 {
        let input = fuzz_name(&mut state, 48);
        let (package, bare) = split_qualified_name(&input);
        let reconstructed = match package {
            Some(pkg) => format!("{pkg}::{bare}"),
            None => bare.to_string(),
        };
        assert_eq!(reconstructed, input);

        if validate_perl_qualified_name(&input).is_ok() {
            if let Some(idx) = input.rfind("::") {
                assert_eq!(package, Some(&input[..idx]));
                assert_eq!(bare, &input[idx + 2..]);
            } else {
                assert_eq!(package, None);
                assert_eq!(bare, input.as_str());
            }
        }
    }
}

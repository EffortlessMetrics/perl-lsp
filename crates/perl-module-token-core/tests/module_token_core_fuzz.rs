use perl_module_token_core::{has_standalone_module_token_boundaries, parse_module_token};

fn next_u64(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

fn gen_ascii(seed: &mut u64) -> char {
    const ALPHABET: &[u8] =
        b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_:/\\' ;()[]{}";
    ALPHABET[(next_u64(seed) as usize) % ALPHABET.len()] as char
}

fn gen_string(seed: &mut u64, max_len: usize) -> String {
    let len = (next_u64(seed) % (max_len as u64 + 1)) as usize;
    let mut out = String::with_capacity(len);
    for _ in 0..len {
        out.push(gen_ascii(seed));
    }
    out
}

#[test]
fn fuzz_token_core_does_not_panic_on_random_inputs() {
    let mut state = 0xF00D_BAAD_BEEF_CAFE_u64;

    for _ in 0..5000 {
        let line = gen_string(&mut state, 128);
        let start = (next_u64(&mut state) as usize) % (line.len() + 1);

        let span = parse_module_token(&line, start);
        if let Some(span) = span {
            assert!(span.start <= span.end);
            assert!(span.end <= line.len());
            let token = &line[span.start..span.end];
            assert!(!token.is_empty());
            let _ = has_standalone_module_token_boundaries(&line, span.start, span.end);
        }

        let end = (next_u64(&mut state) as usize) % (line.len() + 1);
        let (start, end) = if start <= end { (start, end) } else { (end, start) };
        let _ = has_standalone_module_token_boundaries(&line, start, end);
    }
}

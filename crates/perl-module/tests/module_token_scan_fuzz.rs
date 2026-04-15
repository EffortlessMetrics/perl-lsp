use perl_module::token_parser::parse_module_token;

fn next_u64(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

fn gen_ascii(seed: &mut u64) -> char {
    let candidates = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_:/\\' ';";
    candidates[(next_u64(seed) as usize) % candidates.len()] as char
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
fn fuzz_token_parser_does_not_panic_on_random_inputs() {
    let mut state = 0xC0FFEE_u64;

    for _ in 0..5000 {
        let line = gen_string(&mut state, 64);
        let start = (next_u64(&mut state) as usize) % (line.len().max(1));
        let span = parse_module_token(&line, start);

        if let Some(span) = span {
            assert!(span.start <= span.end);
            assert!(span.end <= line.len());
        }
    }
}

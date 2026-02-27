use perl_text_line::line_bounds_at;

fn next_u64(state: &mut u64) -> u64 {
    *state ^= *state >> 12;
    *state ^= *state << 25;
    *state ^= *state >> 27;
    state.wrapping_mul(0x2545_F4_91_4F_6C_DD_1D)
}

fn fuzz_text(state: &mut u64, max_len: usize) -> String {
    const ALPHABET: &[u8] =
        b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_':;\n \t()/";
    let len = (next_u64(state) as usize % max_len).saturating_add(1);
    let mut out = String::with_capacity(len);

    for _ in 0..len {
        let idx = (next_u64(state) as usize) % ALPHABET.len();
        out.push(ALPHABET[idx] as char);
    }

    out
}

#[test]
fn fuzz_line_bounds_do_not_panic_or_return_invalid_ranges() {
    let mut seed = 0xCAFEBABEu64;

    for _ in 0..5000 {
        let text = fuzz_text(&mut seed, 512);
        let mut cursor_state = seed ^ 0xC0DE_C0DE_u64;
        let cursor = (next_u64(&mut cursor_state) as usize) % (text.len().saturating_add(1));

        let (start, end) = line_bounds_at(&text, cursor);
        assert!(start <= cursor);
        assert!(cursor <= end);
        assert!(end <= text.len());
        assert!(!text[start..end].contains('\n'));
    }
}

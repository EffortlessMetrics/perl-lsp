use perl_module::reference::{extract_module_reference, find_module_reference};

fn next_u64(state: &mut u64) -> u64 {
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
            0..=31 => ' ',
            _ => byte as char,
        };
        out.push(ch);
    }

    out
}

#[test]
fn fuzz_reference_extraction_never_panics_and_emits_consistent_ranges() {
    let mut seed = 0xBAD5EED_u64;

    for _ in 0..3000 {
        let text = fuzz_string(&mut seed, 256);
        let cursor = (next_u64(&mut seed) as usize) % (text.len().saturating_add(1));

        let found = find_module_reference(&text, cursor);
        if let Some(reference) = found {
            assert!(reference.module_start <= reference.module_end);
            assert!(reference.module_end <= text.len());
            assert!(!reference.module_name.is_empty());
            assert_eq!(reference.module_name, &text[reference.module_start..reference.module_end]);

            let extracted = extract_module_reference(&text, cursor);
            assert!(extracted.is_some());
        } else {
            let extracted = extract_module_reference(&text, cursor);
            assert!(extracted.is_none());
        }
    }
}

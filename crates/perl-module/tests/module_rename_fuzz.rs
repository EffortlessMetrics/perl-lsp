use perl_module::rename::{apply_module_rename_edits, plan_module_rename_edits};

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
fn fuzz_module_rename_never_panics_and_emits_valid_line_edits() {
    let mut seed = 0xA11CE_u64;

    for _ in 0..2500 {
        let old_module = fuzz_string(&mut seed, 32).replace(' ', "::");
        let new_module = fuzz_string(&mut seed, 32).replace(' ', "::");

        let mut source = String::new();
        let lines = (next_u64(&mut seed) as usize % 8).saturating_add(1);
        for _ in 0..lines {
            source.push_str(&fuzz_string(&mut seed, 96));
            source.push('\n');
        }

        let edits = plan_module_rename_edits(&source, &old_module, &new_module);

        let source_lines: Vec<&str> = source.lines().collect();
        for edit in &edits {
            assert!(edit.line < source_lines.len());
            assert_eq!(edit.start_character, 0);
            assert!(edit.end_character <= source_lines[edit.line].len());
        }

        let rewritten = apply_module_rename_edits(&source, &edits);
        assert!(rewritten.is_char_boundary(rewritten.len()));
    }
}

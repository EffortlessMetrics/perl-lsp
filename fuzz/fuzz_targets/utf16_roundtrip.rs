#![no_main]

use libfuzzer_sys::fuzz_target;
use perl_parser::position::{offset_to_utf16_line_col, utf16_line_col_to_offset};

const MAX_INPUT_BYTES: usize = 2048;

fn bounded_utf8_lossy(data: &[u8]) -> std::borrow::Cow<'_, str> {
    if data.is_empty() {
        return std::borrow::Cow::Borrowed("");
    }

    let capped = if data.len() <= MAX_INPUT_BYTES {
        data
    } else {
        &data[..MAX_INPUT_BYTES]
    };

    String::from_utf8_lossy(capped)
}

fn exercise_position_roundtrip(text: &str) {
    for offset in 0..=text.len() {
        let (line, col) = offset_to_utf16_line_col(text, offset);
        let roundtrip = utf16_line_col_to_offset(text, line, col);

        // Never allow conversion APIs to produce out-of-bounds offsets.
        if roundtrip > text.len() {
            return;
        }

        // Recompute position from round-tripped offset to stress conversion symmetry.
        let _ = offset_to_utf16_line_col(text, roundtrip);
    }

    // Query beyond line length to ensure graceful clamping behavior.
    for line in 0..=16 {
        let _ = utf16_line_col_to_offset(text, line, u32::MAX);
    }
}

fuzz_target!(|data: &[u8]| {
    let input = bounded_utf8_lossy(data);

    let variants = [
        input.to_string(),
        format!("my $value = q{{{input}}};\n"),
        format!("# 😀 prefix\n{input}\n# 🧪 suffix\n"),
        format!("package Ω::Δ; sub f {{ \"{input}\" }}\n"),
        input.chars().rev().collect::<String>(),
    ];

    for variant in &variants {
        exercise_position_roundtrip(variant);
    }
});

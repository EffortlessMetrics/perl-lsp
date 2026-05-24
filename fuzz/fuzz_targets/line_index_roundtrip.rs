#![no_main]

use libfuzzer_sys::fuzz_target;
use perl_line_index::LineIndex;

const MAX_INPUT_BYTES: usize = 4096;
const MAX_PROBE_POSITIONS: usize = 64;

fn bounded_utf8_lossy(data: &[u8]) -> std::borrow::Cow<'_, str> {
    let capped = if data.len() <= MAX_INPUT_BYTES { data } else { &data[..MAX_INPUT_BYTES] };
    String::from_utf8_lossy(capped)
}

fuzz_target!(|data: &[u8]| {
    let input = bounded_utf8_lossy(data);
    let idx = LineIndex::new(&input);

    // Roundtrip invariant: for every char boundary, mapping byte -> (line, col)
    // -> byte must return the original byte. This is the central correctness
    // property of LineIndex and the source of past UTF-8 boundary bugs.
    for (byte, _) in input.char_indices().take(MAX_PROBE_POSITIONS) {
        let (line, col) = idx.byte_to_position(byte);
        assert_eq!(
            idx.position_to_byte(line, col),
            Some(byte),
            "roundtrip failed at byte {byte} in input of len {}",
            input.len()
        );
        // Checked variant must agree with the unchecked one whenever the
        // unchecked one returns Some - they only differ for inputs past the
        // line boundary.
        assert_eq!(
            idx.position_to_byte_checked(line, col),
            Some(byte),
            "checked variant disagreed at byte {byte}"
        );
    }

    // The end of the input is also addressable.
    let end = input.len();
    let (line_end, col_end) = idx.byte_to_position(end);
    assert_eq!(idx.position_to_byte(line_end, col_end), Some(end));

    // Probing out-of-range coordinates must return None, never panic.
    let _ = idx.position_to_byte(usize::MAX, 0);
    let _ = idx.position_to_byte(0, usize::MAX);
    let _ = idx.position_to_byte_checked(usize::MAX, 0);
    let _ = idx.position_to_byte_checked(0, usize::MAX);

    // Probing a byte offset past text_len must not panic - exercises the
    // binary-search saturation path.
    let _ = idx.byte_to_position(end.saturating_add(1));
    let _ = idx.byte_to_position(usize::MAX / 2);

    // Use any extra fuzzer bytes as a deterministic line/column probe so the
    // fuzzer can drive arbitrary (line, col) lookups against the same input.
    if data.len() >= 4 {
        let line = data[0] as usize;
        let col = data[1] as usize;
        let _ = idx.position_to_byte(line, col);
        let _ = idx.position_to_byte_checked(line, col);
    }
});

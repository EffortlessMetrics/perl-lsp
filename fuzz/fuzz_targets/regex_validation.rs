#![no_main]

use libfuzzer_sys::fuzz_target;
use perl_regex::{RegexAnalyzer, RegexValidator};

const MAX_INPUT_BYTES: usize = 2048;
const MAX_MODIFIER_BYTES: usize = 16;

fn bounded_utf8_lossy(data: &[u8]) -> std::borrow::Cow<'_, str> {
    let capped = if data.len() <= MAX_INPUT_BYTES { data } else { &data[..MAX_INPUT_BYTES] };
    String::from_utf8_lossy(capped)
}

fuzz_target!(|data: &[u8]| {
    let input = bounded_utf8_lossy(data);

    // Split the input into a pattern body and a modifier suffix. This lets the
    // fuzzer explore both pattern grammar and modifier handling together.
    let (pattern_bytes, modifier_bytes) = if data.len() > MAX_MODIFIER_BYTES {
        data.split_at(data.len() - MAX_MODIFIER_BYTES)
    } else {
        (data, &[][..])
    };
    let pattern = bounded_utf8_lossy(pattern_bytes);
    let modifiers = String::from_utf8_lossy(modifier_bytes);

    // Validator must never panic on arbitrary input. All five public probes are
    // exercised against the same pattern to cover both the parsing pass
    // (`validate`) and the heuristic detectors (`detects_*`, `find_*`).
    let validator = RegexValidator::new();
    let _ = validator.validate(&input, 0);
    let _ = validator.detects_code_execution(&input);
    let _ = validator.detect_nested_quantifiers(&input);
    let _ = validator.find_code_execution(&input, 0);
    let _ = validator.find_nested_quantifier(&input, 0);

    // Offset propagation path - start_pos shouldn't change error-handling shape,
    // but it exercises arithmetic on the offset that is reported in diagnostics.
    let _ = validator.validate(&pattern, 17);
    let _ = validator.find_code_execution(&pattern, 17);

    // Analyzer surfaces - both must accept arbitrary patterns/modifiers.
    let captures = RegexAnalyzer::extract_named_captures(&pattern);
    // Invariant: every reported capture group must point inside the input.
    for capture in &captures {
        assert!(
            capture.pattern.len() <= pattern.len(),
            "extracted subpattern longer than input pattern"
        );
    }

    let _ = RegexAnalyzer::hover_text_for_regex(&pattern, &modifiers);
    // Empty modifier list is a separate code path (no "Modifiers" section).
    let _ = RegexAnalyzer::hover_text_for_regex(&pattern, "");
});

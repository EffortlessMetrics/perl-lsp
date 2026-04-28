#![no_main]

use libfuzzer_sys::fuzz_target;
use perl_dap::eval::SafeEvaluator;

const MAX_INPUT_BYTES: usize = 1024;

fuzz_target!(|data: &[u8]| {
    // The SafeEvaluator guards debugger evaluate requests against dangerous
    // Perl operations. It must never panic regardless of input — only return
    // Ok or a typed ValidationError.
    let Ok(input) = std::str::from_utf8(data) else { return };
    let input = if input.len() <= MAX_INPUT_BYTES { input } else { &input[..MAX_INPUT_BYTES] };

    let evaluator = SafeEvaluator::new();

    // validate() is the security boundary — panic here would be a bug.
    let _ = evaluator.validate(input);

    // Exercise multi-line and embedded-newline paths explicitly.
    let with_newline = format!("{input}\n{input}");
    let _ = evaluator.validate(&with_newline);

    // Patterns that trip real detection logic: operator lookalike sigils,
    // backtick variants, compound s/// mutations.
    let stress_forms = [
        format!("${input}"),
        format!("`{input}`"),
        format!("s/{input}/{input}/g"),
        format!("system({input})"),
        format!("eval {{ {input} }}"),
    ];
    for form in &stress_forms {
        let _ = evaluator.validate(form);
    }
});

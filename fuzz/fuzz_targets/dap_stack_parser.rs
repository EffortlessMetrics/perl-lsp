#![no_main]

use libfuzzer_sys::fuzz_target;
use perl_dap::stack::PerlStackParser;

const MAX_INPUT_BYTES: usize = 2048;

fn bounded_utf8_lossy(data: &[u8]) -> std::borrow::Cow<'_, str> {
    let capped = if data.len() <= MAX_INPUT_BYTES { data } else { &data[..MAX_INPUT_BYTES] };
    String::from_utf8_lossy(capped)
}

fuzz_target!(|data: &[u8]| {
    let input = bounded_utf8_lossy(data);

    // parse_frame parses a single line of Perl debugger output through several
    // compiled regexes. It must never panic.
    let mut parser = PerlStackParser::new();
    let _ = parser.parse_frame(&input, 0);

    // parse_stack_trace handles multi-line backtrace output.
    let _ = parser.parse_stack_trace(&input);

    // parse_context handles the '@ = Package::func(file.pl:42):' form.
    let _ = parser.parse_context(&input);

    // looks_like_frame is a static classifier used before full parsing.
    let _ = PerlStackParser::looks_like_frame(&input);

    // Exercise parser configuration variants.
    let mut parser_with_unknown = PerlStackParser::new().with_unknown_frames(true);
    let _ = parser_with_unknown.parse_frame(&input, 0);

    let mut parser_no_auto = PerlStackParser::new().with_auto_ids(false).with_starting_id(100);
    let _ = parser_no_auto.parse_stack_trace(&input);

    // Simulate a realistic multi-line backtrace built from the fuzz input.
    let multiline = format!(
        "  #0  main::foo at {input} line 1\n  #1  main::bar at {input} line 2\n"
    );
    if multiline.len() <= MAX_INPUT_BYTES * 3 {
        let mut p = PerlStackParser::new();
        let _ = p.parse_stack_trace(&multiline);
    }
});

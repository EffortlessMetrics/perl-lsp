#![no_main]

use libfuzzer_sys::fuzz_target;
use perl_parser::{
    position::{offset_to_utf16_line_col, utf16_line_col_to_offset},
    quote_parser::{
        extract_regex_parts, extract_substitution_parts, extract_transliteration_parts,
    },
    Parser,
};

const MAX_INPUT_BYTES: usize = 1000;

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

fn parse_without_panicking(input: &str) {
    let mut parser = Parser::new(input);
    if let Ok(ast) = parser.parse() {
        let _ = ast.count_nodes();
        let _ = ast.to_sexp();
    }

    let _ = parser.errors().len();
}

fn exercise_utf16_roundtrip(input: &str) {
    let mut offsets = vec![0, input.len()];

    for (offset, _) in input.char_indices() {
        offsets.push(offset);
    }

    offsets.sort_unstable();
    offsets.dedup();

    for offset in offsets {
        let (line, col) = offset_to_utf16_line_col(input, offset);
        let roundtrip = utf16_line_col_to_offset(input, line, col);
        let _ = roundtrip <= input.len();
    }
}

fuzz_target!(|data: &[u8]| {
    let input = bounded_utf8_lossy(data);

    parse_without_panicking(&input);

    let parser_wrappers = [
        format!("my $value = {};", input),
        format!("sub fuzzed {{ {} }}", input),
        format!("package Fuzz::Case; {}", input),
        format!("if ({}) {{ {} }}", input, input),
        format!("my $regex = qr/{}/;", input),
        format!("$value =~ s/{0}/{0}/gr;", input),
        format!("$value =~ tr/{0}/{0}/d;", input),
        format!("print <<'EOF';\n{}\nEOF", input),
    ];

    for wrapper in &parser_wrappers {
        if wrapper.len() <= MAX_INPUT_BYTES * 2 {
            parse_without_panicking(wrapper);
            exercise_utf16_roundtrip(wrapper);
        }
    }

    let quote_inputs = [
        format!("m/{}/", input),
        format!("qr{{{}}}ims", input),
        format!("s/{0}/{0}/gr", input),
        format!("s[{0}][{0}]e", input),
        format!("tr/{0}/{0}/cd", input),
        format!("y{{{0}}}{{{0}}}r", input),
    ];

    for quote_input in &quote_inputs {
        let (pattern, body, modifiers) = extract_regex_parts(quote_input);
        let _ = pattern.len() <= quote_input.len();
        let _ = body.len() <= quote_input.len();
        let _ = modifiers.len() <= quote_input.len();

        let (search, replacement, sub_modifiers) = extract_substitution_parts(quote_input);
        let _ = search.len() <= quote_input.len();
        let _ = replacement.len() <= quote_input.len();
        let _ = sub_modifiers.len() <= quote_input.len();

        let (from, to, tr_modifiers) = extract_transliteration_parts(quote_input);
        let _ = from.len() <= quote_input.len();
        let _ = to.len() <= quote_input.len();
        let _ = tr_modifiers.len() <= quote_input.len();
    }

    exercise_utf16_roundtrip(&input);
});

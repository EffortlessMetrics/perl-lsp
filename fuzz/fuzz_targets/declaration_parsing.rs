#![no_main]

use libfuzzer_sys::fuzz_target;
use perl_parser::Parser;

const MAX_INPUT_BYTES: usize = 1500;

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

fn truncate_chars(input: &str, max_chars: usize) -> String {
    input.chars().take(max_chars).collect()
}

fuzz_target!(|data: &[u8]| {
    let input = bounded_utf8_lossy(data);
    let short = truncate_chars(&input, 64);
    let escaped_single = short.replace('\\', "\\\\").replace('\'', "\\'");
    let escaped_double = short.replace('\\', "\\\\").replace('"', "\\\"");

    // Focus on declaration/import forms where Perl allows complex expressions
    // in import lists (refs, blocks, quoted words, and mixed separators).
    let declarations = [
        format!("use {short};"),
        format!("no {short};"),
        format!("require {short};"),
        format!("use {short} qw({short});"),
        format!("no {short} qw({short});"),
        format!("use {short} ({short}, \\\${short}, \%{short});"),
        format!("no {short} ({short}, \\\@{short}, \&{short});"),
        format!("use {short} '{{{short}}}', \"{escaped_double}\", '{escaped_single}';"),
        format!("use if {short}, {short}, qw({short});"),
        format!("use lib '{escaped_single}', \"{escaped_double}\";"),
    ];

    for declaration in &declarations {
        let mut parser = Parser::new(declaration);
        let _ = parser.parse();
    }

    // Stress integration with package/sub declarations and nested blocks.
    let integration = [
        format!("package {short}::Pkg; use {short}; sub run {{ {short}; }}"),
        format!("{{ use {short} qw({short}); no {short}; require {short}; }}"),
        format!("BEGIN {{ use {short} ({short}); }} CHECK {{ no {short}; }}"),
    ];

    for snippet in &integration {
        let mut parser = Parser::new(snippet);
        let _ = parser.parse();
    }
});

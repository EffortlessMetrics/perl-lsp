//! Integration test: `perl-lsp-critic-parser` public API reachable via `perl_lsp_rs_core::critic_parser`.

use perl_lsp_rs_core::critic_parser::*;

#[test]
fn critic_parser_module_exposes_parsed_critic_line_struct() {
    // Verify that ParsedCriticLine is accessible post-absorption
    let _: Option<ParsedCriticLine> = None;
}

#[test]
fn critic_parser_module_exposes_parse_perlcritic_output() {
    // Verify that parse_perlcritic_output is accessible post-absorption
    let output = "lib/Foo.pm:10:5:2:ProhibitComplexMappings:Mapping blocks are not complex enough";
    let lines = parse_perlcritic_output(output);
    assert!(!lines.is_empty(), "parse_perlcritic_output should parse valid lines");
}

#[test]
fn critic_parser_module_exposes_parse_perlcritic_line() {
    // Verify that parse_perlcritic_line is accessible post-absorption
    let line = "lib/Foo.pm:10:5:2:ProhibitComplexMappings:Mapping blocks are not complex enough";
    let parsed = parse_perlcritic_line(line);
    assert!(parsed.is_some(), "parse_perlcritic_line should parse valid lines");
}

#[test]
fn critic_parser_parsed_critic_line_has_expected_fields() {
    // Verify that ParsedCriticLine struct has all expected fields
    let line = "lib/Foo.pm:10:5:2:ProhibitComplexMappings:Test message";
    if let Some(parsed) = parse_perlcritic_line(line) {
        assert!(!parsed.file.is_empty(), "file field should be present");
        assert!(parsed.line > 0, "line field should be > 0");
        assert!(parsed.column > 0, "column field should be > 0");
        assert!(!parsed.policy.is_empty(), "policy field should be present");
        assert!(!parsed.message.is_empty(), "message field should be present");
    }
}

#[test]
fn critic_parser_handles_empty_input() {
    // Verify that parse_perlcritic_output gracefully handles empty input
    let lines = parse_perlcritic_output("");
    assert!(lines.is_empty(), "empty input should produce empty output");
}

#[test]
fn critic_parser_rejects_zero_line_or_column() {
    let line_zero = "lib/Foo.pm:0:5:3:Perl::Critic::Policy::ValuesAndExpressions::ProhibitMagicNumbers:no magic numbers";
    let col_zero = "lib/Foo.pm:4:0:3:Perl::Critic::Policy::ValuesAndExpressions::ProhibitMagicNumbers:no magic numbers";

    assert!(
        parse_perlcritic_line(line_zero).is_none(),
        "line number 0 would underflow when converted to 0-indexed ranges"
    );
    assert!(
        parse_perlcritic_line(col_zero).is_none(),
        "column number 0 is invalid for Perl::Critic diagnostics"
    );
}

#[test]
fn critic_parser_rejects_out_of_range_severity() {
    let too_low = "lib/Foo.pm:4:2:0:Perl::Critic::Policy::ValuesAndExpressions::ProhibitMagicNumbers:no magic numbers";
    let too_high = "lib/Foo.pm:4:2:9:Perl::Critic::Policy::ValuesAndExpressions::ProhibitMagicNumbers:no magic numbers";

    assert!(
        parse_perlcritic_line(too_low).is_none(),
        "severity 0 is outside Perl::Critic's supported 1..=5 range"
    );
    assert!(
        parse_perlcritic_line(too_high).is_none(),
        "severity values greater than 5 should be dropped as malformed"
    );
}

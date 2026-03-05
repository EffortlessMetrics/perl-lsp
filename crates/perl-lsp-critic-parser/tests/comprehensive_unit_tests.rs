use perl_lsp_critic_parser::{parse_perlcritic_line, parse_perlcritic_output};
use perl_tdd_support::must_some;

#[test]
fn parses_policy_with_double_colons() {
    let parsed = must_some(parse_perlcritic_line(
        "test.pl:5:1:3:TestingAndDebugging::RequireUseStrict:Code does not use strict",
    ));

    assert_eq!(parsed.file, "test.pl");
    assert_eq!(parsed.line, 5);
    assert_eq!(parsed.column, 1);
    assert_eq!(parsed.severity, 3);
    assert_eq!(parsed.policy, "TestingAndDebugging::RequireUseStrict");
    assert_eq!(parsed.message, "Code does not use strict");
}

#[test]
fn preserves_colons_inside_message() {
    let parsed = must_some(parse_perlcritic_line(
        "lib/App.pm:14:7:2:ValuesAndExpressions::ProhibitMagicNumbers:Avoid magic number: use constants",
    ));

    assert_eq!(parsed.policy, "ValuesAndExpressions::ProhibitMagicNumbers");
    assert_eq!(parsed.message, "Avoid magic number: use constants");
}

#[test]
fn parses_windows_style_path() {
    let parsed = must_some(parse_perlcritic_line(
        "C:\\repo\\lib\\App.pm:11:2:4:Subroutines::ProhibitBuiltinHomonyms:Avoid shadowing builtin",
    ));

    assert_eq!(parsed.file, "C:\\repo\\lib\\App.pm");
    assert_eq!(parsed.line, 11);
}

#[test]
fn skips_invalid_lines() {
    assert!(parse_perlcritic_line("").is_none());
    assert!(parse_perlcritic_line("not-a-valid-line").is_none());
}

#[test]
fn parses_multiple_lines() {
    let output = "test.pl:1:1:5:CodeLayout::RequireTidyCode:Needs tidying\n\
                  test.pl:2:3:3:TestingAndDebugging::RequireUseWarnings:Add warnings\n\
                  malformed";

    let parsed = parse_perlcritic_output(output);
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[1].policy, "TestingAndDebugging::RequireUseWarnings");
}

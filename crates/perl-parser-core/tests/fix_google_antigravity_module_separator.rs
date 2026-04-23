mod cpan_test_helpers;

use cpan_test_helpers::parse;
use perl_parser_core::NodeKind;

#[test]
fn parse_use_with_legacy_quote_package_separator() {
    let ast = parse("use Google'Antigravity;");
    let NodeKind::Program { statements } = ast.kind else {
        assert!(false, "expected Program node");
        return;
    };
    assert_eq!(statements.len(), 1, "expected a single use statement");
    let NodeKind::Use { module, .. } = &statements[0].kind else {
        assert!(false, "expected Use node");
        return;
    };
    assert_eq!(module, "Google::Antigravity");
}

#[test]
fn parse_no_with_legacy_quote_package_separator() {
    let ast = parse("no Google'Antigravity;");
    let NodeKind::Program { statements } = ast.kind else {
        assert!(false, "expected Program node");
        return;
    };
    assert_eq!(statements.len(), 1, "expected a single no statement");
    let NodeKind::No { module, .. } = &statements[0].kind else {
        assert!(false, "expected No node");
        return;
    };
    assert_eq!(module, "Google::Antigravity");
}

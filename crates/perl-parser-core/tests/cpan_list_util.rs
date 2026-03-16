//! CPAN Pattern Tests: List::Util / List::MoreUtils

mod cpan_test_helpers;
use cpan_test_helpers::*;
use perl_parser_core::NodeKind;

#[test]
fn reduce_sum() {
    let code = "my $sum = reduce { $a + $b } 0, @numbers;";
    assert_clean_parse(code);
}

#[test]
fn first_match() {
    let code = "my $first = first { $_->is_valid } @objects;";
    assert_clean_parse(code);
}

#[test]
fn any_check() {
    let code = "my $found = any { $_ eq 'target' } @items;";
    assert_clean_parse(code);
}

#[test]
fn all_check() {
    let code = "my $ok = all { defined $_ } @values;";
    assert_clean_parse(code);
}

#[test]
fn none_check() {
    let code = "my $clean = none { /error/i } @log_lines;";
    assert_clean_parse(code);
}

#[test]
fn max_by() {
    let code = "my $longest = max_by { length $_ } @strings;";
    assert_clean_parse(code);
}

#[test]
fn uniq_values() {
    let code = "my @unique = uniq @items;";
    assert_clean_parse(code);
}

#[test]
fn zip_lists() {
    let code = "my @pairs = zip @keys, @values;";
    assert_clean_parse(code);
}

#[test]
fn use_list_util_qw() {
    let code = "use List::Util qw(reduce first any all none max min sum);";
    assert_clean_parse(code);
    let ast = parse(code);
    if let NodeKind::Program { statements } = &ast.kind {
        assert_eq!(statements.len(), 1);
        if let NodeKind::Use { module, args, .. } = &statements[0].kind {
            assert_eq!(module, "List::Util");
            // qw() imports are stored as a single string arg
            let qw_str = args.join(" ");
            assert!(
                qw_str.contains("reduce") && qw_str.contains("first"),
                "expected qw args containing reduce and first, got: {:?}",
                args
            );
        }
    }
}

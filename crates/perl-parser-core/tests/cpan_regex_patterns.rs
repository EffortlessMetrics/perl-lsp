//! CPAN Pattern Tests: Regex Patterns

mod cpan_test_helpers;
use cpan_test_helpers::*;

#[test]
fn simple_match() {
    let code = r#"if ($line =~ /^#/) { next }"#;
    assert_clean_parse(code);
}

#[test]
fn match_with_captures() {
    let code = r#"
if ($line =~ /^(\d{4})-(\d{2})-(\d{2})\s+(\d{2}:\d{2}:\d{2})\s+(.*)$/) {
    my ($year, $month, $day, $time, $msg) = ($1, $2, $3, $4, $5);
}
"#;
    assert_clean_parse(code);
}

#[test]
fn named_captures() {
    let code = r#"my @matches = ($text =~ m/(?<name>\w+):\s*(?<value>\d+)/g);"#;
    assert_clean_parse(code);
}

#[test]
fn substitution() {
    let code = r#"
$text =~ s/^\s+//;
$text =~ s/\s+$//;
"#;
    assert_clean_parse(code);
}

#[test]
fn global_substitution_with_eval() {
    let code = r#"$text =~ s/\$(\w+)/$vars{$1}/ge;"#;
    assert_clean_parse(code);
}

#[test]
fn transliteration() {
    let code = "($count = $str) =~ tr/aeiou//;";
    assert_clean_parse(code);
}

#[test]
fn regex_in_grep() {
    let code = "my @matches = grep { /pattern/i } @lines;";
    assert_clean_parse(code);
}

#[test]
fn negative_match() {
    let code = "next unless $line !~ /^\\s*$/;";
    assert_clean_parse(code);
}

#[test]
fn split_with_regex() {
    let code = r#"my @fields = split /\s*,\s*/, $line;"#;
    assert_clean_parse(code);
}

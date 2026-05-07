// Quick integration test
mod cpan_test_helpers;
use cpan_test_helpers::*;

#[test]
fn test_proto_double_dollar() {
    assert_clean_parse(r#"sub foo($$) { return 1; }"#);
}

#[test]
fn test_proto_triple_dollar() {
    assert_clean_parse(r#"sub foo($$$) { return 1; }"#);
}

#[test]
fn test_proto_dollar_semicolon() {
    assert_clean_parse(r#"sub ok ($$;$) { return 1; }"#);
}

#[test]
fn test_proto_single_dollar() {
    assert_clean_parse(r#"sub foo ($) { return 1; }"#);
}

#[test]
fn test_anon_sub_single_dollar_proto() {
    assert_clean_parse(r#"my $x = sub ($) { $_[0] >= 0x20 };"#);
}

#[test]
fn test_anon_sub_double_dollar_proto() {
    assert_clean_parse(r#"my $x = sub ($$) { return 1; };"#);
}

#[test]
fn test_fetch_sub_pattern() {
    assert_clean_parse(
        r#"
if (defined(my $sub = _fetch_sub utf8 => 'is_utf8')) {
    *is_utf8 = $sub;
}
"#,
    );
}

#[test]
fn test_carp_pm_is_utf8_pattern() {
    let src = r#"
BEGIN {
    *is_safe_printable_codepoint =
        1 ?
            sub ($) {
                my $u = shift;
                $u >= 0x20 && $u <= 0x7e;
            }
        :
            sub ($) { $_[0] >= 0x20 && $_[0] <= 0x7e }
        ;
}
"#;
    assert_clean_parse(src);
}

#[test]
fn test_carp_sub_full_pattern() {
    let src = r#"
    eval(q(sub ($) {
        my $u = utf8::native_to_unicode($_[0]);
        $u >= 0x20 && $u <= 0x7e;
    }))
"#;
    assert_clean_parse(src);
}

#[test]
fn test_carp_pm_representative_patterns() {
    let source = r#"
BEGIN {
    *is_safe_printable_codepoint =
        sub ($) { $_[0] >= 0x20 && $_[0] <= 0x7e };
}

if (defined(my $sub = _fetch_sub utf8 => 'is_utf8')) {
    *is_utf8 = $sub;
}

my $gv =
    (_fetch_sub B => 'svref_2object' or return '')
        ->($func)->GV;

$mess .= sprintf ", <%s> %s %d",
    *${+LAST_FH}{NAME},
    ($/ eq "\n" ? "line" : "chunk"), $.;
"#;
    assert_clean_parse(source);
}

#[test]
fn test_method_chain_after_parens_with_or() {
    // (_fetch_sub B => 'svref_2object' or return '') ->($func)->GV
    let src = r#"
my $gv =
    (_fetch_sub B => 'svref_2object' or return '')
        ->($func)->GV;
"#;
    assert_clean_parse(src);
}

#[test]
fn test_glob_deref_with_unary_plus_key() {
    // *${+LAST_FH}{NAME}
    let src = r#"my $x = *${+LAST_FH}{NAME};"#;
    assert_clean_parse(src);
}

#[test]
fn test_braced_numeric_capture_still_parses() {
    assert_clean_parse(r#"my $capture = ${1};"#);
}

#[test]
fn test_braced_punctuation_special_vars_still_parse() {
    assert_clean_parse(r#"my $errno = ${!}; my $last = ${+};"#);
}

#[test]
fn test_sprintf_with_glob_deref() {
    let src = r#"
$mess .= sprintf ", <%s> %s %d",
    *${+LAST_FH}{NAME},
    ($/ eq "\n" ? "line" : "chunk"), $.;
"#;
    assert_clean_parse(src);
}

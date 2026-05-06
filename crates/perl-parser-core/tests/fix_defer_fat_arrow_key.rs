mod cpan_test_helpers;
use cpan_test_helpers::*;

// === Fix: `defer` as a fat-arrow hash key ===
//
// Perl's fat-arrow (`=>`) autoquotes any bareword on its left side, including
// reserved keywords.  `defer` (Perl 5.36+ experimental) was missing from both
// `is_keyword_token` (statement-level autoquote gate) and `parse_primary`
// (expression-level dispatch), causing "expected expression, found 'defer'"
// errors in real-world code like core/feature.pm.
//
// Reference failing file: /usr/share/perl/5.38/feature.pm
//   our %feature = ( ..., defer => 'feature_defer', ... );

// --- Statement-level: defer as the opening key of a bare hash pair ---

#[test]
fn test_defer_fat_arrow_stmt_level() {
    assert_clean_parse("defer => 1;");
}

#[test]
fn test_defer_fat_arrow_in_hash_assign() {
    assert_clean_parse("my %h = (defer => 'feature_defer');");
}

#[test]
fn test_defer_fat_arrow_mixed_pairs() {
    // Mirrors the actual feature.pm pattern: multiple keyword keys in one hash
    assert_clean_parse(
        r#"our %feature = (
            switch  => 'feature_switch',
            say     => 'feature_say',
            defer   => 'feature_defer',
            try     => 'feature_try',
        );"#,
    );
}

// --- Expression-level: defer as a non-first key inside a hash literal ---

#[test]
fn test_defer_fat_arrow_second_pair() {
    // `defer` appears as a non-first key so the expression parser handles it
    assert_clean_parse("my %h = (a => 1, defer => 2);");
}

#[test]
fn test_defer_fat_arrow_in_hash_ref() {
    assert_clean_parse("my $h = { defer => 1, try => 2 };");
}

#[test]
fn test_defer_fat_arrow_in_list_context() {
    assert_clean_parse("my @pairs = (defer => 'x', class => 'y');");
}

#[test]
fn test_defer_fat_arrow_in_function_arg() {
    assert_clean_parse("foo(defer => 1);");
}

// --- Regression guard: `defer { }` block must still parse as a defer block ---

#[test]
fn test_defer_block_still_works() {
    assert_clean_parse("defer { cleanup() };");
}

#[test]
fn test_defer_block_in_sub() {
    assert_clean_parse("sub foo { defer { 1 } }");
}

// --- Regression: defer mixed with other keyword keys in the same hash ---

#[test]
fn test_defer_alongside_try_catch() {
    assert_clean_parse("my %h = (defer => 1, try => 2, catch => 3, finally => 4);");
}

#[test]
fn test_defer_alongside_control_keywords() {
    assert_clean_parse("my %h = (if => 1, for => 2, defer => 3, while => 4);");
}

// --- Real-world pattern from feature.pm ---

#[test]
fn test_feature_pm_hash_snippet() {
    // Directly mirrors the failing region in /usr/share/perl/5.38/feature.pm
    assert_clean_parse(
        r#"our %feature = (
            switch          => 'feature_switch',
            say             => 'feature_say',
            state           => 'feature_state',
            unicode_strings => 'feature_unicode',
            unicode_eval    => 'feature_unicode_eval',
            evalbytes       => 'feature_evalbytes',
            current_sub     => 'feature___SUB__',
            refaliasing     => 'feature_refaliasing',
            postderef_qq    => 'feature_postderef',
            signatures      => 'feature_signatures',
            isa             => 'feature_isa',
            indirect        => 'feature_indirect',
            multidimensional => 'feature_multidimensional',
            bareword_filehandles => 'feature_bareword_filehandles',
            try             => 'feature_try',
            defer           => 'feature_defer',
            extra_paired_delimiters => 'feature_more_delims',
            builtin         => 'feature_builtin',
            class           => 'feature_class',
        );"#,
    );
}

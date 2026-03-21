mod cpan_test_helpers;
use cpan_test_helpers::*;

// Pattern C from issue #2149: Sub with leading :: qualifier
// In Perl, `sub ::PCDATA { }` declares a subroutine in the main package.
// The parser fails because DoubleColon is not handled as the start of a
// qualified subroutine name in parse_subroutine().

/// Extract subroutine name from the first statement of a parsed program.
fn sub_name(source: &str) -> Option<String> {
    let ast = parse(source);
    match &ast.kind {
        perl_parser_core::NodeKind::Program { statements } => {
            for stmt in statements {
                // Unwrap ExpressionStatement if needed
                let node = match &stmt.kind {
                    perl_parser_core::NodeKind::ExpressionStatement { expression } => expression,
                    _ => stmt,
                };
                if let perl_parser_core::NodeKind::Subroutine { name, .. } = &node.kind {
                    return name.clone();
                }
            }
            None
        }
        _ => None,
    }
}

#[test]
fn test_sub_leading_double_colon_simple() {
    // sub ::PCDATA { '#PCDATA' } — from XML::Twig
    assert_clean_parse(r#"sub ::PCDATA { '#PCDATA' }"#);
    assert_eq!(sub_name(r#"sub ::PCDATA { '#PCDATA' }"#), Some("::PCDATA".to_string()));
}

#[test]
fn test_sub_leading_double_colon_cdata() {
    // sub ::CDATA { '#CDATA' } — from XML::Twig
    assert_clean_parse(r#"sub ::CDATA { '#CDATA' }"#);
    assert_eq!(sub_name(r#"sub ::CDATA { '#CDATA' }"#), Some("::CDATA".to_string()));
}

#[test]
fn test_sub_leading_double_colon_qualified() {
    // sub ::DB_File::splice { &SPLICE } — from DB_File
    assert_clean_parse(r#"sub ::DB_File::splice { &SPLICE }"#);
    assert_eq!(
        sub_name(r#"sub ::DB_File::splice { &SPLICE }"#),
        Some("::DB_File::splice".to_string()),
    );
}

#[test]
fn test_sub_leading_double_colon_deeply_qualified() {
    // Deeply qualified name with leading ::
    assert_clean_parse(r#"sub ::Foo::Bar::baz { 1 }"#);
    assert_eq!(sub_name(r#"sub ::Foo::Bar::baz { 1 }"#), Some("::Foo::Bar::baz".to_string()));
}

#[test]
fn test_sub_leading_double_colon_with_body() {
    // Leading :: with a more complex body
    assert_clean_parse(r#"sub ::main_func { my $x = 1; return $x }"#);
    assert_eq!(
        sub_name(r#"sub ::main_func { my $x = 1; return $x }"#),
        Some("::main_func".to_string())
    );
}

// Regression tests: existing patterns must still work

#[test]
fn test_sub_normal_still_works() {
    assert_clean_parse(r#"sub normal_sub { 1 }"#);
}

#[test]
fn test_sub_qualified_still_works() {
    // Package::method style — already works
    assert_clean_parse(r#"sub Foo::bar { 1 }"#);
}

#[test]
fn test_sub_keyword_named_still_works() {
    // Keyword-named subs — the original fix from issue #2149
    assert_clean_parse(r#"sub return { 1 }"#);
    assert_clean_parse(r#"sub try { 1 }"#);
}

#[test]
fn test_sub_anonymous_still_works() {
    // Anonymous sub must still work
    assert_clean_parse(r#"my $f = sub { 1 };"#);
}

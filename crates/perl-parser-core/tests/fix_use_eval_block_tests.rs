mod cpan_test_helpers;
use cpan_test_helpers::*;

#[test]
fn test_use_constant_eval_require() {
    let source = r#"use constant ROLES => !!(eval { require Role::Tiny; 1 })"#;
    assert_clean_parse(source);
}

#[test]
fn test_use_constant_eval_with_env_guard() {
    let source = r#"use constant JSON_XS => $ENV{X} ? 0 : !!eval { require Foo; 1 }"#;
    assert_clean_parse(source);
}

#[test]
fn test_use_constant_eval_with_fallback() {
    let source = r#"use constant HAS_FOO => eval { require Foo::Bar; Foo::Bar->import; 1 } || 0"#;
    assert_clean_parse(source);
}

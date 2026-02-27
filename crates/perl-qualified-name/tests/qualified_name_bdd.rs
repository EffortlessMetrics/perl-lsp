use perl_qualified_name::{split_qualified_name, validate_perl_qualified_name};

#[test]
fn given_unqualified_name_when_split_then_package_is_none() {
    let (package, bare) = split_qualified_name("process");

    assert_eq!(package, None);
    assert_eq!(bare, "process");
}

#[test]
fn given_qualified_name_when_split_then_package_and_bare_are_extracted() {
    let (package, bare) = split_qualified_name("Tools::Process");

    assert_eq!(package, Some("Tools"));
    assert_eq!(bare, "Process");
}

#[test]
fn given_multi_segment_name_when_split_then_last_segment_is_bare_name() {
    let (package, bare) = split_qualified_name("A::B::Process");

    assert_eq!(package, Some("A::B"));
    assert_eq!(bare, "Process");
}

#[test]
fn given_valid_name_when_validated_then_validation_passes() {
    assert!(validate_perl_qualified_name("My::Module").is_ok());
    assert!(validate_perl_qualified_name("Müller::Util").is_ok());
}

#[test]
fn given_name_with_sigil_when_validated_then_validation_fails() {
    assert!(validate_perl_qualified_name("$foo").is_err());
    assert!(validate_perl_qualified_name("&bar").is_err());
}

#[test]
fn given_name_with_trailing_separator_when_validated_then_validation_fails() {
    assert!(validate_perl_qualified_name("Foo::").is_err());
    assert!(validate_perl_qualified_name("Foo::Bar::").is_err());
    assert!(validate_perl_qualified_name("Foo::::Bar").is_err());
}

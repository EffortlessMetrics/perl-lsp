use perl_pragma_catalog::{PRAGMA_MODULES, is_pragma_module};

#[test]
fn detects_known_pragmas() {
    assert!(is_pragma_module("strict"));
    assert!(is_pragma_module("warnings"));
    assert!(is_pragma_module("autodie"));
}

#[test]
fn rejects_non_pragmas() {
    assert!(!is_pragma_module("Foo::Bar"));
    assert!(!is_pragma_module("Data::Dumper"));
}

#[test]
fn exported_pragma_list_is_stable_and_nonempty() {
    assert!(PRAGMA_MODULES.len() >= 30);
    assert_eq!(PRAGMA_MODULES.first(), Some(&"attributes"));
}

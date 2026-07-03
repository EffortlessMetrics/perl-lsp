//! Integration coverage for Test2 import/export awareness exposed by
//! `perl_lsp_rs_core::providers::testing::test2`, verified from the LSP server
//! crate that consumes it (`test2_imports`).

use perl_lsp_rs_core::providers::testing::test2::{
    Test2Facts, is_test2_bundle, is_test2_module, module_default_exports, resolve_import,
};
use perl_tdd_support::must_some;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// The canonical "done enough" Test2 file from the integration spec must not
/// produce false unknown-symbol conditions: every assertion/plan/subtest name
/// resolves as an imported Test2 symbol.
#[test]
fn test2_imports_done_enough_example_symbols_all_resolve() {
    let source = "use Test2::V0;\n\
        subtest 'user lookup' => sub {\n\
            ok(my $user = find_user('a@example.com'), 'found user');\n\
            is($user->{email}, 'a@example.com', 'email matches');\n\
        };\n\
        done_testing;\n";
    let facts = Test2Facts::from_source(source);

    assert!(facts.uses_test2());
    assert!(facts.uses_test2_bundle());
    for sym in ["ok", "is", "subtest", "done_testing"] {
        assert!(facts.is_imported(sym), "Test2 symbol `{sym}` should be imported");
    }
    // strict/warnings are provided by the bundle.
    assert!(facts.strict);
    assert!(facts.warnings);
    // A user's own sub is not a Test2 import.
    assert!(!facts.is_imported("find_user"));
}

#[test]
fn test2_imports_public_api_is_reachable() -> TestResult {
    assert!(is_test2_module("Test2::V0"));
    assert!(is_test2_bundle("Test2::V0"));
    assert!(!is_test2_bundle("Test2::Tools::Basic"));

    let defaults = must_some(module_default_exports("Test2::V0"));
    assert!(defaults.contains(&"ok"));
    assert!(defaults.contains(&"subtest"));

    let resolved = must_some(resolve_import("Test2::V0", "'!ok'"));
    assert!(!resolved.symbols.contains("ok"));
    assert!(resolved.symbols.contains("is"));
    Ok(())
}

#[test]
fn test2_imports_test_more_is_not_test2() {
    // Test::More is a different framework — the Test2 fact table must not claim it.
    assert!(!is_test2_module("Test::More"));
    let facts = Test2Facts::from_source("use Test::More;\nok(1);\ndone_testing;\n");
    assert!(!facts.uses_test2());
    assert!(!facts.strict, "Test::More does not imply strict via the Test2 table");
}

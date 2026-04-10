//! BDD-style workflow coverage for `perl-builtins`.
//!
//! These scenarios focus on user-visible behaviors relied on by parser and LSP
//! layers: builtin recognition, parameter metadata lookups, and signature
//! catalog stability.

use std::error::Error;

use perl_builtins::builtin_signatures::create_builtin_signatures;
use perl_builtins::builtin_signatures_phf::{builtin_count, get_param_names, is_builtin};

struct Scenario {
    name: &'static str,
}

impl Scenario {
    fn new(name: &'static str) -> Self {
        eprintln!("[BDD] Scenario: {name}");
        Self { name }
    }

    fn given(&self, message: &str) {
        eprintln!("[{}] Given {message}", self.name);
    }

    fn when(&self, message: &str) {
        eprintln!("[{}] When {message}", self.name);
    }

    fn then(&self, message: &str) {
        eprintln!("[{}] Then {message}", self.name);
    }
}

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn given_known_builtin_when_querying_phf_then_it_is_recognized_with_parameter_names() -> TestResult
{
    let scenario = Scenario::new("known builtin lookup");

    scenario.given("a common Perl builtin function name");
    let builtin_name = "open";

    scenario.when("the PHF helpers are queried");
    let builtin = is_builtin(builtin_name);
    let params = get_param_names(builtin_name);

    scenario.then("the builtin is recognized and exposes positional parameter names");
    assert!(builtin);
    assert_eq!(params, ["FILEHANDLE", "MODE", "FILENAME"]);
    Ok(())
}

#[test]
fn given_unknown_symbol_when_querying_phf_then_it_is_not_treated_as_builtin() -> TestResult {
    let scenario = Scenario::new("unknown symbol lookup");

    scenario.given("an identifier not present in Perl builtin catalogs");
    let unknown_name = "definitely_not_a_perl_builtin";

    scenario.when("lookup helpers are called");
    let builtin = is_builtin(unknown_name);
    let params = get_param_names(unknown_name);

    scenario.then("recognition fails gracefully without parameter metadata");
    assert!(!builtin);
    assert!(params.is_empty());
    Ok(())
}

#[test]
fn given_file_test_operator_when_querying_then_operator_is_handled_like_builtin() -> TestResult {
    let scenario = Scenario::new("file test operator metadata");

    scenario.given("a Perl file test operator");
    let operator = "-e";

    scenario.when("the PHF builtins catalog is queried");
    let builtin = is_builtin(operator);
    let params = get_param_names(operator);
    let map = create_builtin_signatures();

    scenario
        .then("it is surfaced as a builtin with FILE parameter and documented signature metadata");
    assert!(builtin);
    assert_eq!(params, ["FILE"]);
    let Some(sig) = map.get(operator) else {
        return Err("expected file test operator signature metadata to exist".into());
    };
    assert!(!sig.signatures.is_empty());
    Ok(())
}

#[test]
fn given_hashmap_catalog_when_initialized_multiple_times_then_once_lock_reuses_cache() -> TestResult
{
    let scenario = Scenario::new("hashmap signature cache stability");

    scenario.given("the builtin signature HashMap provider");

    scenario.when("create_builtin_signatures is called repeatedly");
    let first = create_builtin_signatures();
    let second = create_builtin_signatures();

    scenario.then("the same cached allocation is returned and remains non-empty");
    assert!(std::ptr::eq(first, second));
    assert!(!first.is_empty());
    Ok(())
}

#[test]
fn given_signature_catalogs_when_cross_checking_core_entries_then_behavior_is_consistent()
-> TestResult {
    let scenario = Scenario::new("cross-catalog consistency for core entries");

    scenario.given("the PHF and HashMap builtin catalogs");
    let phf_count = builtin_count();
    let map = create_builtin_signatures();
    let core_entries = ["print", "say", "open", "length", "substr", "split", "time"];

    scenario.when("representative builtins are verified in both catalogs");
    for builtin in core_entries {
        assert!(is_builtin(builtin), "{builtin} missing from PHF catalog");
        assert!(map.contains_key(builtin), "{builtin} missing from HashMap catalog");
    }

    scenario.then("both catalogs provide broad stable coverage");
    assert!(phf_count >= core_entries.len());
    assert!(map.len() >= core_entries.len());
    Ok(())
}

//! Unit tests for the Test2 fact table (`test2_imports`).

use super::*;

#[test]
fn test2_imports_recognizes_bundles_and_tools() {
    assert!(is_test2_module("Test2::V0"));
    assert!(is_test2_module("Test2::V1"));
    assert!(is_test2_module("Test2::Bundle::Extended"));
    assert!(is_test2_module("Test2::Tools::Basic"));
    assert!(is_test2_module("Test2::Tools::Compare"));
    assert!(is_test2_module("Test2::Plugin::UTF8"));
    assert!(is_test2_module("Test2::API"));

    assert!(!is_test2_module("Test::More"));
    assert!(!is_test2_module("strict"));
    assert!(!is_test2_module("Moose"));
}

#[test]
fn test2_imports_bundle_classification() {
    assert!(is_test2_bundle("Test2::V0"));
    assert!(is_test2_bundle("Test2::V1"));
    assert!(is_test2_bundle("Test2::Bundle::Extended"));
    assert!(is_test2_bundle("Test2::Suite"));

    // Individual tool modules do NOT turn on strict/warnings.
    assert!(!is_test2_bundle("Test2::Tools::Basic"));
    assert!(!is_test2_bundle("Test2::Plugin::UTF8"));
    assert!(!is_test2_bundle("Test2::API"));
}

#[test]
fn test2_imports_v0_default_exports_cover_common_tools() {
    let defaults = module_default_exports("Test2::V0").expect("V0 has a default set");
    for expected in [
        "ok",
        "pass",
        "fail",
        "diag",
        "note",
        "todo",
        "skip",
        "plan",
        "skip_all",
        "done_testing",
        "bail_out",
        "is",
        "isnt",
        "like",
        "unlike",
        "cmp_ok",
        "subtest",
        "isa_ok",
        "can_ok",
        "DOES_ok",
        "dies",
        "lives",
        "try_ok",
        "warns",
        "warning",
        "warnings",
        "no_warnings",
        "ref_ok",
        "mock",
    ] {
        assert!(defaults.contains(&expected), "V0 default set should export `{expected}`");
    }
    // `subtest_buffered` is renamed to `subtest` in V0 — the raw name is not a
    // V0 default export.
    assert!(!defaults.contains(&"subtest_buffered"));
}

#[test]
fn test2_imports_plain_use_v0_scope_and_pragmas() {
    let facts = Test2Facts::from_source("use Test2::V0;\nok(1);\ndone_testing;\n");
    assert!(facts.uses_test2());
    assert!(facts.uses_test2_bundle());
    assert!(facts.is_imported("ok"));
    assert!(facts.is_imported("done_testing"));
    assert!(facts.is_imported("is"));
    assert!(facts.is_imported("subtest"));
    // Plain `use Test2::V0;` turns strict + warnings on.
    assert!(facts.strict);
    assert!(facts.warnings);
    assert!(!facts.is_imported("find_user"), "unrelated names are not imported");
}

#[test]
fn test2_imports_no_strict_option_disables_strict_only() {
    let facts = Test2Facts::from_source("use Test2::V0 -no_strict => 1;\nok(1);\n");
    assert!(!facts.strict, "-no_strict disables strict");
    assert!(facts.warnings, "-no_strict leaves warnings on");
    assert!(facts.is_imported("ok"));
}

#[test]
fn test2_imports_no_warnings_option_disables_warnings_only() {
    let facts = Test2Facts::from_source("use Test2::V0 -no_warnings;\nok(1);\n");
    assert!(facts.strict);
    assert!(!facts.warnings);
}

#[test]
fn test2_imports_no_pragmas_disables_both() {
    let facts = Test2Facts::from_source("use Test2::V0 -no_pragmas;\nok(1);\n");
    assert!(!facts.strict);
    assert!(!facts.warnings);
}

#[test]
fn test2_imports_exclusion_removes_symbol() {
    let resolved = resolve_import("Test2::V0", "'!ok'").expect("recognized module");
    assert!(!resolved.symbols.contains("ok"), "!ok excludes ok");
    assert!(resolved.symbols.contains("is"), "other defaults remain");
    // Exclusion alone keeps pragmas on.
    assert_eq!(resolved.pragmas, Some(Test2Pragmas { strict: true, warnings: true }));
}

#[test]
fn test2_imports_rename_as_installs_alias_not_original() {
    let resolved =
        resolve_import("Test2::V0", "ok => {-as => 'my_ok'}").expect("recognized module");
    assert!(resolved.symbols.contains("my_ok"), "alias my_ok is imported");
    assert!(!resolved.symbols.contains("ok"), "original ok is not imported after rename");
    // A rename is an explicit selection: only the alias is in scope.
    assert!(!resolved.symbols.contains("is"), "explicit selection replaces the default set");
}

#[test]
fn test2_imports_default_tag_keeps_full_set_with_rename() {
    let resolved =
        resolve_import("Test2::V0", "':DEFAULT', ok => {-as => 'my_ok'}").expect("recognized");
    assert!(resolved.symbols.contains("my_ok"));
    assert!(resolved.symbols.contains("is"), ":DEFAULT restores the default set");
}

#[test]
fn test2_imports_explicit_qw_list_replaces_default() {
    let resolved = resolve_import("Test2::V0", "qw/ok is/").expect("recognized module");
    assert!(resolved.symbols.contains("ok"));
    assert!(resolved.symbols.contains("is"));
    assert!(!resolved.symbols.contains("like"), "explicit list does not pull in the rest");
}

#[test]
fn test2_imports_prefix_rename() {
    let resolved = resolve_import("Test2::V0", "ok => {-prefix => 'my_'}").expect("recognized");
    assert!(resolved.symbols.contains("my_ok"));
    assert!(!resolved.symbols.contains("ok"));
}

#[test]
fn test2_imports_standalone_tool_has_no_pragmas() {
    let facts = Test2Facts::from_source("use Test2::Tools::Basic;\nok(1);\n");
    assert!(facts.is_imported("ok"));
    assert!(facts.is_imported("done_testing"));
    // A bare tool import does not enable strict/warnings.
    assert!(!facts.strict);
    assert!(!facts.warnings);
}

#[test]
fn test2_imports_compare_standalone_default_is_narrow() {
    // Standalone Test2::Tools::Compare only default-exports is/like.
    let defaults = module_default_exports("Test2::Tools::Compare").expect("known tool");
    assert_eq!(defaults, &["is", "like"]);
    // But an explicit import of an EXPORT_OK symbol is trusted verbatim.
    let resolved = resolve_import("Test2::Tools::Compare", "qw/hash array/").expect("recognized");
    assert!(resolved.symbols.contains("hash"));
    assert!(resolved.symbols.contains("array"));
}

#[test]
fn test2_imports_subtest_tool_standalone_names() {
    let defaults = module_default_exports("Test2::Tools::Subtest").expect("known tool");
    assert!(defaults.contains(&"subtest_streamed"));
    assert!(defaults.contains(&"subtest_buffered"));
    // The bundle-level `subtest` alias is not a standalone-tool default name.
    assert!(!defaults.contains(&"subtest"));
}

#[test]
fn test2_imports_ignores_commented_out_use() {
    let facts = Test2Facts::from_source("# use Test2::V0;\nmy $x = 1;\n");
    assert!(!facts.uses_test2(), "commented-out use must not register");
    assert!(!facts.strict);
}

#[test]
fn test2_imports_multiline_import_list() {
    let src = "use Test2::V0 qw(\n    ok\n    is\n);\n";
    let facts = Test2Facts::from_source(src);
    assert!(facts.is_imported("ok"));
    assert!(facts.is_imported("is"));
    assert!(!facts.is_imported("like"));
    assert!(facts.strict, "bundle still enables strict with an explicit list");
}

#[test]
fn test2_imports_non_test2_module_returns_none() {
    assert!(resolve_import("Test::More", "").is_none());
    assert!(resolve_import("strict", "").is_none());
}

#[test]
fn test2_imports_target_option_does_not_drop_exports() {
    let resolved = resolve_import("Test2::V0", "-target => 'Foo::Bar'").expect("recognized");
    // -target is an option, not a symbol selection, so the default set stays.
    assert!(resolved.symbols.contains("ok"));
    assert!(resolved.symbols.contains("is"));
    assert_eq!(resolved.pragmas, Some(Test2Pragmas { strict: true, warnings: true }));
}

#[test]
fn expand_qw_only_fires_on_word_boundary_and_real_delimiter() {
    // Genuine qw list expands to space-separated words.
    assert_eq!(expand_qw("qw(ok is like)"), " ok is like ");
    assert_eq!(expand_qw("qw{ok is}"), " ok is ");
    // `qw` preceded by a word char (`eqw`) is not the operator — must stay intact.
    assert_eq!(expand_qw("eqw(x)"), "eqw(x)");
    // A word char immediately after `qw` is not a delimiter (`qwoo` is a bareword).
    assert_eq!(expand_qw("qwoo"), "qwoo");
    // A symbol name embedding `qw` at a word boundary is left intact.
    assert_eq!(expand_qw("my_qw"), "my_qw");
}

#[test]
fn expand_qw_does_not_panic_on_non_ascii() {
    // `raw[i..]` must never split a multi-byte codepoint (critic-path panic).
    assert_eq!(expand_qw("qw(ok café)"), " ok café ");
    // Non-ASCII outside a qw list is copied through unchanged.
    assert_eq!(expand_qw("-target => 'Café::Módulo'"), "-target => 'Café::Módulo'");
    // A multi-byte char immediately before `qw` still parses safely.
    assert_eq!(expand_qw("café qw(ok)"), "café  ok ");
    // A non-ASCII byte must never be treated as a qw delimiter: doing so would
    // set `close` to a lead byte and slice `raw` mid-codepoint (panic). Here the
    // qw is left unexpanded rather than crashing.
    assert_eq!(expand_qw("qw é a é b"), "qw é a é b");
}

#[test]
fn test2_non_ascii_target_does_not_panic() {
    // Reaches expand_qw via the critic path; must not crash on valid UTF-8.
    let facts = Test2Facts::from_source("use Test2::V0 -target => 'Café::Módulo';\n");
    assert!(facts.is_imported("ok"));
    assert_eq!((facts.strict, facts.warnings), (true, true));
}

#[test]
fn test2_empty_import_list_imports_nothing_and_no_pragmas() {
    // `use Test2::V0 ();` loads the module but does not call import(): no
    // symbols, and no strict/warnings are provided.
    let resolved = resolve_import("Test2::V0", "()").expect("module still recognized");
    assert!(resolved.symbols.is_empty(), "empty () import must import no symbols");
    assert_eq!(resolved.pragmas, None, "empty () import provides no pragmas");

    let facts = Test2Facts::from_source("use Test2::V0 ();\n");
    assert!(facts.modules.iter().any(|m| m == "Test2::V0"), "module is still loaded");
    assert!(!facts.is_imported("ok"), "no default exports for ()");
    assert_eq!((facts.strict, facts.warnings), (false, false), "() provides no pragmas");
}

#[test]
fn use_statements_survive_escaped_quotes() {
    // An escaped quote inside a double-quoted string must not close the string
    // early; otherwise the `;` terminator is missed and the following `use`
    // statement is never seen. The second Test2 module only appears if the
    // first statement terminated at the right `;`.
    let src = "use Test2::V0 -target => \"a\\\"b\";\nuse Test2::Tools::Basic;\n";
    let facts = Test2Facts::from_source(src);
    assert!(facts.modules.iter().any(|m| m == "Test2::V0"), "modules: {:?}", facts.modules);
    assert!(
        facts.modules.iter().any(|m| m == "Test2::Tools::Basic"),
        "second statement lost after escaped quote: {:?}",
        facts.modules
    );
}

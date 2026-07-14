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
fn test2_v1_default_exports_only_t2_and_no_pragmas() {
    // Test2::V1's ONLY default export is the `T2()` handle, and it enables NO
    // pragmas by default — its tools are methods on the handle, not bare subs
    // (oracle: metacpan Test2::V1 — "Only 1 export by default: T2()", "NO
    // PRAGMAS ARE ENABLED BY DEFAULT").
    let defaults = module_default_exports("Test2::V1").expect("V1 has a default set");
    assert_eq!(defaults, &["T2"], "V1 default-exports only the T2 handle");

    let facts = Test2Facts::from_source("use Test2::V1;\n");
    assert!(facts.uses_test2_bundle(), "Test2::V1 is still a bundle module");
    assert_eq!((facts.strict, facts.warnings), (false, false), "V1 enables no pragmas by default");
    assert!(facts.is_imported("T2"), "V1 imports the T2 handle");
    assert!(!facts.is_imported("ok"), "V1 does not export bare ok by default");
    assert!(!facts.is_imported("is"), "V1 does not export bare is by default");
    assert!(!facts.is_imported("subtest"), "V1 does not export bare subtest by default");
}

#[test]
fn test2_v1_import_option_brings_in_the_full_bare_set() {
    // `-import` (and its `-i` shorthand) imports all tools as bare subs, like V0.
    for src in ["use Test2::V1 -import;\n", "use Test2::V1 -i;\n"] {
        let facts = Test2Facts::from_source(src);
        assert!(facts.is_imported("ok"), "{src:?}: -import brings in bare ok");
        assert!(facts.is_imported("is"), "{src:?}: -import brings in bare is");
        assert!(facts.is_imported("subtest"), "{src:?}: -import brings in bare subtest");
    }
}

#[test]
fn test2_v1_pragmas_require_an_explicit_option() {
    // V1 strict/warnings only via -strict/-warnings/-p/-pragmas.
    let pragmas = Test2Facts::from_source("use Test2::V1 -pragmas;\n");
    assert_eq!((pragmas.strict, pragmas.warnings), (true, true), "-pragmas enables both");
    let strict_only = Test2Facts::from_source("use Test2::V1 -strict;\n");
    assert_eq!(
        (strict_only.strict, strict_only.warnings),
        (true, false),
        "-strict enables strict only"
    );
}

#[test]
fn test2_v1_grouped_short_flags() {
    // `use Test2::V1 -ipP;` — grouped short flags: -i (import) + -p (pragmas)
    // + -P (plugins). Oracle: metacpan Test2::V1 SYNOPSIS.
    let facts = Test2Facts::from_source("use Test2::V1 -ipP;\n");
    assert!(facts.is_imported("ok"), "grouped -i imports the bare set");
    assert!(facts.is_imported("is"));
    assert!(facts.is_imported("subtest"));
    assert_eq!((facts.strict, facts.warnings), (true, true), "grouped -p enables pragmas");

    // `-P` alone (plugins) is neither import nor pragmas.
    let plugins_only = Test2Facts::from_source("use Test2::V1 -P;\n");
    assert!(!plugins_only.is_imported("ok"), "-P (plugins) does not import the bare set");
    assert_eq!((plugins_only.strict, plugins_only.warnings), (false, false));
}

#[test]
fn v1_short_flag_predicate_boundary_discriminators() {
    // Direct unit tests for the three char-class terms in v1_short_flag's split
    // predicate (`c.is_ascii_alphanumeric() || c == '_' || c == '-'`), each
    // constructed so flipping that term's truth value would flip the result.

    // `c == '-'`: a leading `-` must be kept (not treated as a delimiter) so the
    // flag token survives intact. Without this term, `-i` would split into an
    // empty token and `i` (no leading `-` to strip), and `i` alone would never
    // match because `strip_prefix('-')` requires the leading dash.
    assert!(v1_short_flag("-i", 'i'), "leading '-' must stay attached to the flag letters");

    // `c == '_'`: an embedded underscore must also be kept, so `-i_p` stays one
    // token ("i_p" after stripping the dash) and fails the all-known-letters
    // check (the '_' is not in {i,p,P,x}). If underscore were instead treated as
    // a delimiter, `-i_p` would split into `-i` and `p`, and `-i` alone would
    // match the import flag — a different (wrong) result.
    assert!(!v1_short_flag("-i_p", 'i'), "embedded '_' must not act as a delimiter");
    assert!(!v1_short_flag("-i_p", 'p'), "embedded '_' must not act as a delimiter");

    // `c.is_ascii_alphanumeric()`: an embedded digit must also be kept (kept
    // together via alphanumeric, not just alphabetic), so `-i2p` stays one token
    // ("i2p") and fails the all-known-letters check (the '2' is not in
    // {i,p,P,x}). If digits were treated as delimiters instead, `-i2p` would
    // split into `-i` and `p`, and `-i` alone would match — a different (wrong)
    // result.
    assert!(!v1_short_flag("-i2p", 'i'), "embedded digit must not act as a delimiter");
    assert!(!v1_short_flag("-i2p", 'p'), "embedded digit must not act as a delimiter");

    // Sanity: the true-match path still works once the token is exactly the
    // known short-flag letters (no interfering '_'/digit).
    assert!(v1_short_flag("-ip", 'p'), "clean grouped short flags still match");
}

#[test]
fn v1_import_all_predicate_exact_module_match_boundary() {
    // `resolve_import`'s `v1_import_all` boundary (test2.rs:324) is
    // `module == "Test2::V1" && (...)`. Hold the raw args fixed at `-import`
    // and flip only the module name: for Test2::V1 this pulls in the full bare
    // tool set (V0_DEFAULT); for any other Test2 module, `-import` is not a
    // recognized option and has no effect on that module's own default set.
    // This proves the predicate needs an *exact* "Test2::V1" match, not "any
    // bundle" or "any Test2 module reachable via is_test2_module".
    let v1 = resolve_import("Test2::V1", "-import").expect("Test2::V1 recognized");
    assert!(v1.symbols.contains("subtest"), "-import brings the full bare set into Test2::V1");
    assert!(v1.symbols.contains("ok"));

    let basic =
        resolve_import("Test2::Tools::Basic", "-import").expect("Test2::Tools::Basic recognized");
    assert!(
        !basic.symbols.contains("subtest"),
        "-import has no special meaning outside Test2::V1; Test2::Tools::Basic keeps its own          fixed default set"
    );
}

#[test]
fn v1_pragma_default_predicate_exact_module_match_boundary() {
    // `resolve_import`'s bundle-pragma boundary (test2.rs:336) is
    // `module == "Test2::V1"`. Hold the raw args fixed (empty import list) and
    // flip only the module name between two bundles: Test2::V1 (no pragmas by
    // default) and Test2::Suite (pragmas on by default, like Test2::V0). This
    // proves the predicate needs an *exact* "Test2::V1" match, not "any
    // bundle".
    let v1 = resolve_import("Test2::V1", "").expect("Test2::V1 recognized");
    assert_eq!(
        v1.pragmas,
        Some(Test2Pragmas { strict: false, warnings: false }),
        "Test2::V1 enables no pragmas by default"
    );

    let suite = resolve_import("Test2::Suite", "").expect("Test2::Suite recognized");
    assert_eq!(
        suite.pragmas,
        Some(Test2Pragmas { strict: true, warnings: true }),
        "Test2::Suite (a bundle that is not Test2::V1) enables both pragmas by default"
    );
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

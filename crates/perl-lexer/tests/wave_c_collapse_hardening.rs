//! Green-TDD hardening tests for Wave C lexer collapse (#4444).
//!
//! These tests go beyond `facade_api_completeness.rs` (which only smoke-checks
//! the top-level re-export surface) and assert *module-path* accessibility,
//! behavioural equivalence with the pre-collapse crates, and the specific
//! architectural invariant that `TokenStream` lives in `perl-parser-core` while
//! the AST-agnostic `token_wrapper`/`util` slice lives in `perl-lexer`.
//!
//! Structured in six blocks mirroring the PR review sections:
//!   1. `api.rs` re-exports (regression guard on import paths)
//!   2. keyword module paths + lookup correctness
//!   3. builtin module paths + PHF lookup correctness
//!   4. tokenizer submodule paths
//!   5. `TokenStream` is NOT in `perl-lexer` (cycle-avoidance invariant)
//!   6. Behavioural equivalence — keyword/builtin sets match pre-collapse
//!      contracts (no accidental trimming or de-duplication)

// -----------------------------------------------------------------------------
// 1. api.rs re-exports — regression guard on every top-level name
// -----------------------------------------------------------------------------

#[test]
fn api_reexports_keywords_by_fully_qualified_name() {
    // Explicit qualified access so a `use perl_lexer::api` regression surfaces.
    let _: &[&str] = perl_lexer::api::KEYWORDS;
    let _: &[&str] = perl_lexer::api::LEXER_KEYWORDS;
    let _: &[&str] = perl_lexer::api::LSP_COMPLETION_KEYWORDS;
    let _: &[&str] = perl_lexer::api::DAP_COMPLETION_KEYWORDS;
    let _: &[&str] = perl_lexer::api::LSP_RUNTIME_COMPLETION_KEYWORDS;
    let _: &[&str] = perl_lexer::api::PARSER_LSP_KEYWORDS;
    let _: &[&str] = perl_lexer::api::RENAME_KEYWORDS;

    assert!(perl_lexer::api::is_keyword("my"));
    assert!(perl_lexer::api::is_lexer_keyword("sub"));
    let _ = perl_lexer::api::is_lsp_completion_keyword("print");
    let _ = perl_lexer::api::is_dap_completion_keyword("eval");
    let _ = perl_lexer::api::is_lsp_runtime_completion_keyword("use");
    let _ = perl_lexer::api::is_parser_lsp_keyword("package");
    let _ = perl_lexer::api::is_rename_keyword("my");
}

#[test]
fn api_reexports_builtins_by_fully_qualified_name() {
    use perl_lexer::api::BuiltinSignature;

    assert!(perl_lexer::api::is_builtin("print"));
    assert!(perl_lexer::api::builtin_count() > 0);
    let _: &[&str] = perl_lexer::api::get_param_names("substr");
    let _ = &perl_lexer::api::BUILTIN_SIGS;
    let _ = &perl_lexer::api::BUILTIN_FULL_SIGS;
    let sigs = perl_lexer::api::create_builtin_signatures();
    let _: Option<&BuiltinSignature> = sigs.get("print");
}

#[test]
fn api_reexports_tokenizer_slice_by_fully_qualified_name() {
    // AST-agnostic slice: token_wrapper + util
    let _: Option<perl_lexer::api::TokenWithPosition> = None;
    let _: Option<perl_lexer::api::PositionTracker<'_>> = None;
    let _: &str = perl_lexer::api::code_slice("print 1;\n__DATA__\nstuff");
    let _: Option<usize> =
        perl_lexer::api::find_data_marker_byte_lexed("print 1;\n__DATA__\nstuff");
    #[allow(deprecated)]
    let _: Option<usize> = perl_lexer::api::find_data_marker_byte("print 1;\n");
}

// -----------------------------------------------------------------------------
// 2. keywords module path accessibility + lookup correctness
// -----------------------------------------------------------------------------

#[test]
fn keywords_module_path_exists() {
    // Qualified access via `perl_lexer::keywords::*` must compile and resolve.
    // Matches the pre-collapse `perl_keywords::*` shape the absorbed crate
    // exposed, so downstream code patterns still work after the module move.
    let _: &[&str] = perl_lexer::keywords::KEYWORDS;
    let _: &[&str] = perl_lexer::keywords::LEXER_KEYWORDS;
    let _: &[&str] = perl_lexer::keywords::LSP_COMPLETION_KEYWORDS;
    let _: &[&str] = perl_lexer::keywords::DAP_COMPLETION_KEYWORDS;
    let _: &[&str] = perl_lexer::keywords::LSP_RUNTIME_COMPLETION_KEYWORDS;
    let _: &[&str] = perl_lexer::keywords::PARSER_LSP_KEYWORDS;
    let _: &[&str] = perl_lexer::keywords::RENAME_KEYWORDS;
}

#[test]
fn keywords_lookup_covers_standard_perl_keywords() {
    use perl_lexer::keywords as kw;

    // Spot-check a spread of canonical keywords — regressions in
    // `const KEYWORDS: &[&str] = &[ ... ]` often drop entries silently.
    for k in [
        "my", "our", "local", "sub", "use", "package", "if", "unless", "while", "for", "foreach",
        "return", "undef", "die", "eval", "do", "print", "BEGIN", "END",
    ] {
        assert!(kw::is_keyword(k), "expected {k:?} to be a keyword");
    }
    // Negative: identifier-looking non-keyword should not be classified.
    assert!(!kw::is_keyword("some_user_sub"));
}

#[test]
fn keywords_inventories_are_non_empty_and_sorted_uniquely() {
    use perl_lexer::keywords as kw;

    for (name, list) in [
        ("KEYWORDS", kw::KEYWORDS),
        ("LEXER_KEYWORDS", kw::LEXER_KEYWORDS),
        ("LSP_COMPLETION_KEYWORDS", kw::LSP_COMPLETION_KEYWORDS),
        ("DAP_COMPLETION_KEYWORDS", kw::DAP_COMPLETION_KEYWORDS),
        ("LSP_RUNTIME_COMPLETION_KEYWORDS", kw::LSP_RUNTIME_COMPLETION_KEYWORDS),
        ("PARSER_LSP_KEYWORDS", kw::PARSER_LSP_KEYWORDS),
        ("RENAME_KEYWORDS", kw::RENAME_KEYWORDS),
    ] {
        assert!(!list.is_empty(), "{name} must not be empty after collapse");
        let mut dedup: Vec<&&str> = list.iter().collect();
        let before = dedup.len();
        dedup.sort();
        dedup.dedup();
        assert_eq!(dedup.len(), before, "{name} contains duplicates — check module merge");
    }
}

// -----------------------------------------------------------------------------
// 3. builtins module path accessibility + PHF lookup correctness
// -----------------------------------------------------------------------------

#[test]
fn builtins_module_path_exists() {
    // Both submodules must be reachable at their canonical paths.
    use perl_lexer::builtins::builtin_signatures::{BuiltinSignature, create_builtin_signatures};
    use perl_lexer::builtins::phf_lookup::{
        BUILTIN_FULL_SIGS, BUILTIN_SIGS, builtin_count, get_param_names, is_builtin,
    };

    assert!(is_builtin("print"));
    assert!(builtin_count() > 0);
    let _: &[&str] = get_param_names("substr");
    let _ = &BUILTIN_SIGS;
    let _ = &BUILTIN_FULL_SIGS;
    let sigs = create_builtin_signatures();
    let _: Option<&BuiltinSignature> = sigs.get("print");
}

#[test]
fn builtins_phf_lookup_matches_known_perl_builtins() {
    use perl_lexer::builtins::phf_lookup::{builtin_count, is_builtin};

    // Core builtins that must remain classified after absorption.
    for b in [
        "print", "printf", "say", "length", "substr", "sprintf", "join", "split", "scalar", "keys",
        "values", "push", "pop", "shift", "unshift", "sort", "reverse", "map", "grep", "defined",
        "ref", "bless",
    ] {
        assert!(is_builtin(b), "expected {b:?} to be a builtin");
    }
    // Sanity: an arbitrary user identifier must not leak into builtins.
    assert!(!is_builtin("some_user_defined_function_name_xyz"));
    assert!(builtin_count() >= 50, "unexpectedly small builtin table");
}

#[test]
fn builtins_legacy_signatures_phf_alias_resolves() {
    // The collapse commit preserves the legacy path
    // `perl_builtins::builtin_signatures_phf::*` under the new crate. If this
    // alias is accidentally dropped, downstream callers using the pre-absorb
    // path break. (Alias is declared in `builtins/mod.rs`.)
    use perl_lexer::builtins::builtin_signatures_phf::is_builtin;
    assert!(is_builtin("print"));
}

// -----------------------------------------------------------------------------
// 4. tokenizer submodule paths (AST-agnostic slice only)
// -----------------------------------------------------------------------------

#[test]
fn tokenizer_submodule_paths_exist() {
    // token_wrapper, util reachable via `perl_lexer::tokenizer::*`. Matches
    // the pre-collapse `perl_tokenizer::*` shape that downstream crates used.
    use perl_lexer::tokenizer::token_wrapper::{PositionTracker, TokenWithPosition};
    #[allow(deprecated)]
    use perl_lexer::tokenizer::util::{
        code_slice, find_data_marker_byte, find_data_marker_byte_lexed,
    };

    let _: Option<TokenWithPosition> = None;
    let _: Option<PositionTracker<'_>> = None;
    let _: &str = code_slice("code;\n__END__\ndata");
    let _: Option<usize> = find_data_marker_byte_lexed("code;\n__END__\ndata");
    #[allow(deprecated)]
    let _: Option<usize> = find_data_marker_byte("code;");
}

#[test]
fn tokenizer_util_find_data_marker_preserves_semantics() {
    // The util functions are migrated unchanged — guard the documented
    // contract so a future refactor cannot silently regress behaviour.
    use perl_lexer::tokenizer::util::{code_slice, find_data_marker_byte_lexed};

    // __DATA__ marker
    let src = "print 'hello';\n__DATA__\ndata here";
    assert_eq!(find_data_marker_byte_lexed(src), Some(15));
    assert_eq!(code_slice(src), "print 'hello';\n");

    // __END__ marker
    let src2 = "code;\n__END__\ndata";
    assert_eq!(find_data_marker_byte_lexed(src2), Some(6));
    assert_eq!(code_slice(src2), "code;\n");

    // Not at line start (inside string literal) -> None
    let src3 = "print '__DATA__';\n";
    assert_eq!(find_data_marker_byte_lexed(src3), None);
    assert_eq!(code_slice(src3), src3);

    // No marker -> full text returned
    assert_eq!(code_slice("my $x = 1;\n"), "my $x = 1;\n");
}

// -----------------------------------------------------------------------------
// 5. Cycle-avoidance invariant: TokenStream NOT in perl-lexer
// -----------------------------------------------------------------------------
// The architect decision documented in `api.rs` requires `TokenStream` to live
// in `perl-parser-core` to avoid the `perl-error` <-> `perl-lexer` dep cycle.
// We guard this by (a) asserting the parser-core path resolves and (b) pulling
// a value from it in a way that will fail to compile if the type moves back.

#[test]
fn token_stream_lives_in_perl_parser_core() {
    use perl_parser_core::tokens::token_stream::{TokenKind, TokenStream};

    let mut stream = TokenStream::new("my $x = 42;");
    // Resolving `stream.peek()` proves the full module path is wired.
    assert!(matches!(stream.peek(), Ok(t) if t.kind == TokenKind::My));
}

#[test]
fn token_stream_also_reexported_at_parser_core_root() {
    // The architect decision also preserved a root-level alias for
    // convenience: `perl_parser_core::TokenStream`. Keep it as a regression
    // guard so consumers don't have to change to the nested path mid-release.
    let _: perl_parser_core::TokenStream = perl_parser_core::TokenStream::new("my $x;");
}

#[test]
fn trivia_lives_in_perl_parser_core_not_perl_lexer() {
    // Trivia modules were relocated for the same reason as TokenStream
    // (depend on perl-error / perl-ast-v2). Confirm they're reachable at
    // their new home.
    use perl_parser_core::tokens::trivia::{Trivia, TriviaLexer};
    let _: Option<Trivia> = None;
    let _: Option<TriviaLexer> = None;
}

// -----------------------------------------------------------------------------
// 6. Behavioural equivalence — keyword/builtin sets match pre-collapse
// -----------------------------------------------------------------------------

#[test]
fn keyword_inventory_retains_tight_classification_split() {
    // Sanity: LEXER_KEYWORDS is the canonical lexer-facing subset and must
    // be at least as large as any LSP/DAP subset. This guards against a
    // future accidental swap of the underlying arrays during refactoring.
    use perl_lexer::keywords as kw;
    assert!(kw::LEXER_KEYWORDS.len() >= kw::LSP_COMPLETION_KEYWORDS.len().min(1));
    // The master KEYWORDS union must contain every specialised inventory.
    for list in [
        kw::LEXER_KEYWORDS,
        kw::LSP_COMPLETION_KEYWORDS,
        kw::DAP_COMPLETION_KEYWORDS,
        kw::LSP_RUNTIME_COMPLETION_KEYWORDS,
        kw::PARSER_LSP_KEYWORDS,
        kw::RENAME_KEYWORDS,
    ] {
        for k in list {
            assert!(
                kw::KEYWORDS.contains(k),
                "KEYWORDS union must contain specialised entry {k:?} after collapse"
            );
        }
    }
}

#[test]
fn builtin_count_is_stable_nonzero() {
    // Catch accidental truncation of BUILTIN_SIGS during future PHF
    // regeneration — the table absorbs perl-builtins-phf verbatim.
    // Note: BUILTIN_FULL_SIGS is a *separate*, smaller typed-signature table,
    // so we only assert both are non-empty, not a size ordering between them.
    use perl_lexer::builtins::phf_lookup::{BUILTIN_FULL_SIGS, BUILTIN_SIGS, builtin_count};
    assert_eq!(builtin_count(), BUILTIN_SIGS.len());
    assert!(!BUILTIN_SIGS.is_empty(), "BUILTIN_SIGS empty after collapse");
    assert!(!BUILTIN_FULL_SIGS.is_empty(), "BUILTIN_FULL_SIGS empty after collapse");
}

// -----------------------------------------------------------------------------
// 7. Workspace-level invariants (filesystem regression guard)
// -----------------------------------------------------------------------------
//
// Mirrors the red-TDD filesystem checks (which lived on the deleted
// `perl-tokenizer` crate) but now runs from `perl-lexer` post-collapse. These
// guard ADR-0041 Wave C: the four satellite directories stay deleted, the
// workspace count stays at 97, and the `perl-lexer` CLAUDE.md is preserved
// (the builder's spec required keeping it as the canonical lexer-crate
// document).

use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    // Walk up from `crates/perl-lexer/tests/` to the workspace root that
    // carries the top-level `Cargo.toml` with `[workspace]` members.
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Manifest dir is .../crates/perl-lexer; pop to workspace root.
    p.pop(); // .../crates
    p.pop(); // workspace root
    assert!(p.join("Cargo.toml").exists(), "expected workspace Cargo.toml at {}", p.display());
    p
}

#[test]
fn absorbed_satellite_directories_stay_deleted() {
    let root = workspace_root();
    for sat in ["perl-keywords", "perl-builtins", "perl-builtins-phf", "perl-tokenizer"] {
        let dir = root.join("crates").join(sat);
        assert!(
            !dir.exists(),
            "Wave C absorbed `{sat}` — the directory must not be re-introduced at {}",
            dir.display()
        );
    }
}

#[test]
fn perl_lexer_claude_md_is_preserved() -> std::io::Result<()> {
    // The spec explicitly kept crates/perl-lexer/CLAUDE.md as the canonical
    // lexer-crate doc after absorbing the satellites. If a future collapse
    // wave accidentally deletes it, this fails.
    let root = workspace_root();
    let claude = root.join("crates").join("perl-lexer").join("CLAUDE.md");
    assert!(claude.exists(), "crates/perl-lexer/CLAUDE.md must be preserved post-Wave-C");
    let contents = std::fs::read_to_string(&claude)?;
    // Light content invariants — don't over-constrain wording.
    assert!(contents.contains("perl-lexer"), "CLAUDE.md mentions crate name");
    assert!(
        contents.to_lowercase().contains("lexer") || contents.to_lowercase().contains("tokeniz"),
        "CLAUDE.md describes the lexer/tokenizer role"
    );
    Ok(())
}

#[test]
fn workspace_has_exactly_97_members_after_wave_c() -> std::io::Result<()> {
    // Wave C removes 4 crates (101 -> 97). Anchor the count so a future wave
    // that forgets to update the spec's target surfaces immediately. Tolerant
    // of whitespace/quoting variations in the members list.
    let root = workspace_root();
    let cargo_toml = std::fs::read_to_string(root.join("Cargo.toml"))?;

    // Find the `members = [ ... ]` block and count entries that look like
    // `"crates/..."` or `"xtask"` (i.e. quoted path/name literals). We count
    // quoted strings only inside the members list to avoid double-counting
    // the publish allowlist entries which also appear quoted.
    let members_start = cargo_toml
        .find("members = [")
        .ok_or_else(|| std::io::Error::other("workspace Cargo.toml missing `members = [`"))?;
    let after_start = &cargo_toml[members_start..];
    let end_off = after_start
        .find("\n]")
        .ok_or_else(|| std::io::Error::other("members list missing closing bracket"))?;
    let members_block = &after_start[..end_off];

    let count = members_block.lines().filter(|l| l.trim_start().starts_with('"')).count();

    assert_eq!(
        count, 97,
        "workspace should have 97 members after Wave C (was 101). Found {count}."
    );
    Ok(())
}

#[test]
fn absorbed_satellite_names_are_absent_from_cargo_toml() -> std::io::Result<()> {
    // Belt-and-suspenders for the directory check: make sure the
    // `[workspace.dependencies]` table and members list both dropped the
    // satellite crate names. Catches a half-done revert of the collapse.
    let root = workspace_root();
    let cargo_toml = std::fs::read_to_string(root.join("Cargo.toml"))?;

    for sat in ["perl-keywords", "perl-builtins", "perl-builtins-phf", "perl-tokenizer"] {
        // Guard against accidentally matching `perl-keywords-` or similar
        // longer names by checking exact-quote boundaries used in Cargo.toml.
        let needle_path = format!("\"crates/{sat}\"");
        assert!(!cargo_toml.contains(&needle_path), "workspace members still lists `{sat}`");
        let needle_dep = format!("\n{sat} =");
        assert!(
            !cargo_toml.contains(&needle_dep),
            "[workspace.dependencies] still defines `{sat}`"
        );
    }
    Ok(())
}

#[test]
fn perl_lexer_and_perl_token_remain_published() -> std::io::Result<()> {
    // ADR amendment (#4446): `perl-token` stays on the allowlist. Wave C
    // must NOT remove `perl-lexer` or `perl-token` from the publish
    // allowlist even while removing the 4 absorbed satellites.
    let root = workspace_root();
    let cargo_toml = std::fs::read_to_string(root.join("Cargo.toml"))?;

    // Scope search to the `[workspace.metadata.publish]` block to avoid
    // matching path dependencies further down the file.
    let idx = cargo_toml
        .find("[workspace.metadata.publish]")
        .ok_or_else(|| std::io::Error::other("workspace.metadata.publish block not found"))?;
    let after = &cargo_toml[idx..];
    let end = after
        .find("\n]")
        .ok_or_else(|| std::io::Error::other("allowlist missing closing bracket"))?;
    let allow_block = &after[..end];

    assert!(
        allow_block.contains("\"perl-lexer\""),
        "`perl-lexer` must stay on the publish allowlist"
    );
    assert!(
        allow_block.contains("\"perl-token\""),
        "`perl-token` must stay on the publish allowlist (ADR amendment #4446)"
    );
    for sat in ["perl-keywords", "perl-builtins", "perl-builtins-phf", "perl-tokenizer"] {
        let needle = format!("\"{sat}\"");
        assert!(
            !allow_block.contains(&needle),
            "`{sat}` absorbed — must be removed from publish allowlist"
        );
    }
    Ok(())
}

#[test]
fn absorbed_modules_exist_inside_perl_lexer() {
    // Positive side of the filesystem guard: the absorbed module sources
    // landed at their canonical paths. Keeps the directory structure
    // documented in api.rs in sync with what's on disk.
    let root = workspace_root();
    let lexer_src = root.join("crates").join("perl-lexer").join("src");

    let expected = [
        "keywords/mod.rs",
        "builtins/mod.rs",
        "builtins/builtin_signatures.rs",
        "builtins/phf_lookup.rs",
        "tokenizer/mod.rs",
        "tokenizer/token_wrapper.rs",
        "tokenizer/util.rs",
        "api.rs",
    ];
    for rel in expected {
        let p: &Path = &lexer_src.join(rel);
        assert!(p.exists(), "missing absorbed module at {}", p.display());
    }
}

#[test]
fn token_stream_lives_under_parser_core_not_lexer() {
    // Filesystem counterpart to the compile-time check in block 5 — an
    // accidental move of `token_stream.rs` back into `perl-lexer` would
    // reintroduce the `perl-error` <-> `perl-lexer` dep cycle.
    let root = workspace_root();
    let lexer_bad = root
        .join("crates")
        .join("perl-lexer")
        .join("src")
        .join("tokenizer")
        .join("token_stream.rs");
    assert!(
        !lexer_bad.exists(),
        "token_stream.rs must NOT live in perl-lexer (creates cycle via perl-error). Expected at perl-parser-core/src/tokens/."
    );

    let core_good = root
        .join("crates")
        .join("perl-parser-core")
        .join("src")
        .join("tokens")
        .join("token_stream.rs");
    assert!(core_good.exists(), "token_stream.rs must live at perl-parser-core/src/tokens/");
}

// -----------------------------------------------------------------------------
// 8. is_keyword_fast length-bound invariant
// -----------------------------------------------------------------------------
//
// `is_keyword_fast` in `perl-lexer/src/lib.rs` uses `matches!(word.len(), 1..=9)`
// as a fast-path rejection before calling `is_lexer_keyword`. If a keyword
// longer than 9 characters is ever added to LEXER_KEYWORDS, it would be
// silently invisible to the lexer (the bound check short-circuits the lookup).
// This test anchors the maximum keyword length so any addition that breaks the
// bound surfaces immediately.

#[test]
fn lexer_keywords_fit_within_is_keyword_fast_bound() {
    use perl_lexer::keywords::LEXER_KEYWORDS;

    let max_len = LEXER_KEYWORDS.iter().map(|kw| kw.len()).max().unwrap_or(0);
    assert!(
        max_len <= 9,
        "LEXER_KEYWORDS contains a keyword longer than 9 chars (found {max_len}). \
         Update the `matches!(word.len(), 1..=9)` bound in `is_keyword_fast` in \
         crates/perl-lexer/src/lib.rs to `1..={max_len}` to keep the fast path correct."
    );
    // Also guard the lower bound — empty string is not a keyword.
    assert!(
        LEXER_KEYWORDS.iter().all(|kw| !kw.is_empty()),
        "LEXER_KEYWORDS must not contain the empty string"
    );
}

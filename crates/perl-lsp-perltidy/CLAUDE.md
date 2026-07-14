# CLAUDE.md (perl-lsp-perltidy)

## Role

Perl source formatting: a native-first Rust formatter, plus an optional
subprocess-backed adapter for projects that need exact `perltidy` output
compatibility.

## Owns

- `native` module -- `NativeFormatter` (the default LSP formatter),
  `PerlFormatter` trait, `FormatConfig`, `FormatDoc`, `FormatResult`,
  `TextEdit`, `FormatDiagnostic`, and layout-option types
  (`BracePlacement`, `ElsePlacement`, `KeywordSpacing`, `TrailingComma`,
  `FinalNewline`).
- `PerlTidyConfig` / `PerlTidyFormatter` -- subprocess-backed adapter that
  shells out to `perltidy` via `perl-subprocess-runtime` for exact
  compatibility when required.
- `BuiltInFormatter` / `FormatSuggestion` -- built-in style presets.

## Does not own

- Full Perl parsing beyond what `perl-parser-core` provides.
- The LSP `textDocument/formatting` request wiring itself -- that lives in
  `perl-lsp-rs-core::providers` (this crate is the formatting engine, not
  the provider).

## Neighbors

- Upstream: `perl-parser-core`, `perl-subprocess-runtime`, `serde`.
- Downstream: `perl-lsp-rs-core` (depends on this directly for its
  tooling/formatting providers), and transitively `perl-lsp-rs`.

## Read first

- `src/lib.rs` -- facade, `PerlTidyConfig`, `PerlTidyFormatter`.
- `src/native.rs` -- the `PerlFormatter` trait and `NativeFormatter`; this
  is where the actual default formatting logic lives (the largest file in
  the crate).

## Focused validation

`cargo test -p perl-lsp-perltidy`. `tests/native_contract_tests.rs` and
`tests/native_formatter_parse_gate_tests.rs` guard the native formatter's
parse-preserving contract (output must still parse as valid Perl).
`tests/subprocess_tests.rs` covers the `perltidy` subprocess path and skips
gracefully when `perltidy` isn't installed on the runner.

## Review hotspots

- `native.rs` -- any change must preserve round-trip parseability; a
  formatter that produces unparseable output is a correctness regression,
  not a style nit.
- `PerlTidyConfig`'s `extra_args` / `timeout_secs` -- the subprocess
  boundary is the security-adjacent surface (arbitrary args passed to an
  external process).

## Claim boundary

Describes formatter architecture and configuration surface as authored.
Does not assert byte-for-byte output parity with any specific installed
`perltidy` version -- that's an external-tool compatibility claim this crate
cannot make on its own.

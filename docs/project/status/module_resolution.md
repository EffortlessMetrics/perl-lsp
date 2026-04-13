# @INC / Module Resolution Conformance

Consumer-consistency matrix — verified end-to-end through all three LSP consumers
(PL701 diagnostic, goto-definition, hover) for each `@INC` resolution mode.

**Test**: `cargo test -p perl-lsp-ux-tests --test ux_scenario_14_inc_conformance -- --nocapture`

## Consumer Consistency Matrix

Each cell indicates whether the consumer agrees on module resolution for the given mode.
A `+` means the consumer produced the expected answer (resolved or not-resolved consistently).

| Resolution Mode | PL701 diagnostic | goto-definition | hover | Notes |
|---|---|---|---|---|
| Workspace `includePaths` | + | + | + | Config-driven: `includePaths: ["lib"]` |
| Absolute `includePaths` | + | + | + | Config-driven: absolute path entry |
| Lexical `use lib` | + | + | + | In-source pragma extraction |
| `no lib` cancellation | + | + | + | Position-aware negative case |
| FindBin-relative | + | + | + | `$FindBin::Bin/lib` pattern |
| System `@INC` (PERL5LIB) | + | + | + | PERL5LIB injection |

**Key**: Consumer cells are `+` (consistent) or `-` (divergent / unimplemented).
Conformance means all consumers agree — not necessarily that every mode resolves.

## Resolution Mode Details

### Workspace `includePaths`

Configured via `workspace/didChangeConfiguration`:

```json
{ "settings": { "perl": { "workspace": { "includePaths": ["lib"] } } } }
```

Module lives at `lib/Module.pm` relative to the workspace root.

### Absolute `includePaths`

Configured via `workspace/didChangeConfiguration` with an absolute folder path:

```json
{ "settings": { "perl": { "workspace": { "includePaths": ["/abs/path/to/lib"] } } } }
```

Module lives at `/abs/path/to/lib/AbsoluteModule.pm`.

### Lexical `use lib`

Source-level pragma: `use lib 'lib';` before `use Module;`. The LSP extracts
`use lib` and `no lib` operations in lexical order via
`resolve_use_lib_paths_from_source()` (`crates/perl-module-resolution/src/use_lib.rs:147`).

### `no lib` Cancellation

Position-aware negative test: `use lib 'lib'; no lib 'lib'; use GoneModule;`.
The module file exists on disk but must NOT resolve because `no lib` cancelled
the earlier `use lib` before the `use GoneModule` line.

### FindBin-Relative

Pattern: `use FindBin; use lib "$FindBin::Bin/lib";`. `$FindBin::Bin` resolves
to the directory containing the script being analyzed. The module must be at
`<script_dir>/lib/Module.pm`.

### System `@INC` (PERL5LIB)

Module lives outside the workspace in a directory injected via the `PERL5LIB`
environment variable. Requires `usePerl5lib: true` in workspace config.

## Implementation Notes

- Position-aware resolution is implemented in `crates/perl-module-resolution/src/use_lib.rs` via `resolve_use_lib_paths_from_source()` (lines 147-173).
- The three LSP consumers all call either `resolve_module_to_path_with_doc()` or `resolve_module_path_with_uri()` from `crates/perl-lsp/src/runtime/lifecycle/module_resolution.rs`.
- Consumer call sites:
  - PL701 diagnostic: `crates/perl-lsp/src/runtime/diagnostics.rs` — calls at lines 110, 280, 521
  - goto-definition: `crates/perl-lsp/src/runtime/language/navigation.rs` — call at line 966
  - hover: `crates/perl-lsp/src/runtime/language/hover.rs` — calls at lines 1100, 1118

## Follow-up Scope (PR 2)

- `inc_perl5lib_env` — PERL5LIB separate from system @INC flag
- `inc_nested_use_lib` — `use lib` inside `BEGIN` block
- `inc_qw_use_lib` — `use lib qw(lib t/lib)` multi-path form
- Cross-scorecard: add `expected.json` diagnostic sidecars to all `inc_*` fixtures

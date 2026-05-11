# perl-lsp

LSP server binary and integration tests.

## Test Threading
ALWAYS use threading constraints:
```bash
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs -- --test-threads=2
```

## After Async Migration
- LspServer no longer needs &mut self — use `let server` not `let mut server`
- Test harness uses interior mutability (Arc<Mutex>)

## Verify
```bash
cargo fmt --all
cargo clippy -p perl-lsp-rs --tests
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs -- --test-threads=2
```

## Include-root semantics: `.` is a wildcard, not a path

When filtering workspace-symbol candidates through `EffectiveIncContext`, treat the
workspace-root `.` entry distinctly from configured-relative or lexical-`use lib`
entries. If `.` is treated like any other include root, almost every workspace file
becomes reachable and the filter cannot reject e.g. `lib/GoneModule.pm` after
`no lib 'lib'`.

Sources of include roots — semantically distinct, do NOT collapse to "path roots":

- `WorkspaceDefaultDot` — `.` from the workspace folder. Acts as a wildcard for
  everything under the workspace; not subject to `no lib` cancellation.
- `WorkspaceConfiguredRelative` — explicit `additionalIncPaths` entries from
  config (e.g. `lib`, `t/lib`).
- `LexicalUseLib` — `use lib '...'` from the source under analysis. Position-
  scoped; subject to `no lib '...'` cancellation downstream of the cancel point.
- `Perl5LibEnv` — `PERL5LIB` from the LSP process / `usePerl5lib` config.
- `InterpreterStartup` — output of the `perl -e 'print @INC'` probe (via the
  subprocess oracle contract — see #8551).

When implementing a new filter or routing decision over include roots, branch on
the kind, not on the path. The wildcard `.` is the most common source of
false-positives in workspace-symbol filtering.

Related: `crates/perl-lsp-rs/src/runtime/lifecycle/inc_context.rs`.

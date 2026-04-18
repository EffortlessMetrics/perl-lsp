# ADR — Formalize print-statement lint enforcement in perl-lsp-launcher

## Status
Accepted

## Context

Issue #3224 reported 210 debug print statements (`println!`/`eprintln!`/`dbg!`) across library code in the perl-lsp Rust workspace. PR #2446 ("feat(logging): add file-based log rotation and migrate eprintln to tracing", merged 2026-03-20) migrated the bulk of these to `tracing`. Investigation in work-1efc01c2 confirms that **only 1 genuine library-code print statement remains** after applying the issue's own exclusion criteria (test code, `#[cfg(debug_assertions)]` blocks, doc comments, CLI binaries).

The remaining statement is `crates/perl-lsp-launcher/src/lib.rs:779` — the startup banner `eprintln!` — which is intentional because it fires before the tracing subscriber is configured.

Critically, **lint enforcement is inconsistent across crates**:

| Crate | `#![deny(clippy::print_stderr, clippy::print_stdout)]` |
|---|---|
| `perl-lsp` | ✅ Present (`lib.rs:312`) |
| `perl-dap` | ✅ Present (`lib.rs:361`) |
| `perl-lsp-transport` | ✅ Present (`lib.rs:46`) |
| `perl-lsp-protocol` | ✅ Present (`lib.rs:12`) |
| `perl-semantic-analyzer` | ✅ Present (`lib.rs:10`) |
| `perl-corpus` | ✅ Present (`lib.rs:219`) |
| `perl-lsp-launcher` | ❌ **Missing** |

`perl-lsp-launcher` is the only library crate that lacks lint enforcement. Without it, future contributions may add print statements that bypass `tracing`, undermining the structured logging goal that PR #2446 established.

## Decision

1. **Add lint enforcement** to `crates/perl-lsp-launcher/src/lib.rs`:
   ```rust
   // Lint enforcement: library code must use tracing, not direct stderr/stdout prints.
   #![deny(clippy::print_stderr, clippy::print_stdout)]
   #![cfg_attr(test, allow(clippy::print_stderr, clippy::print_stdout))]
   ```

2. **Formalize the startup banner exception** with an explicit `#[allow]` annotation on the `startup_banner` function:
   ```rust
   /// Emit the process-start banner to stderr.
   ///
   /// Fires before the LSP handshake begins. Writes directly to stderr, not through
   /// tracing, so it is visible regardless of whether `--log` is active.
   /// Suppressed when `PERL_LSP_QUIET` is set in the environment.
   #[allow(clippy::print_stderr)]
   pub fn startup_banner(version: &str, profile: FeatureProfile, transport: TransportMode) {
       // ...
   }
   ```

3. **Close GitHub issue #3224** as substantially resolved — the mechanical migration work from PR #2446 already addressed 209 of the 210 reported findings.

## Consequences

### Benefits
- **Consistent enforcement**: All library crates in the workspace now enforce the same lint, setting clear expectations for contributors.
- **Formalized exception**: The `startup_banner` function's intentional `eprintln!` is now explicitly exempted rather than relying on undocumented behavior.
- **Regression prevention**: Future contributions that add `println!`/`eprintln!` in library code will fail CI until they either migrate to `tracing` or add their own `#[allow]` annotation with justification.
- **No false positives**: The `#[cfg_attr(test, allow(...))]` attribute ensures test code is not affected.

### Tradeoffs / Risks
- **Minimal code change**: A 2-line addition to `perl-lsp-launcher/src/lib.rs` and a 1-line attribute on `startup_banner`. Low risk.
- **CI noise if contributors bypass it**: Contributors who add print statements must consciously add `#[allow]`, which is the desired behavior.

## Alternatives Considered

### 1. Close #3224 as resolved without lint enforcement (Option A from initial plan)
**Rejected.** This leaves `perl-lsp-launcher` as an outlier. Without enforcement, the exception documented in the `startup_banner` doc comment is not machine-checkable. Future contributors have no signal that their new `eprintln!` is problematic. This creates an inconsistent expectation across the three main LSP crates (`perl-lsp`, `perl-dap`, `perl-lsp-launcher`).

### 2. Remove the startup banner `eprintln!` entirely
**Rejected.** The startup banner must be visible before the tracing subscriber is initialized. Moving it after tracing initialization would suppress output when `--log` is not active, which defeats its purpose as a user-facing diagnostic visible in all execution modes. The comment at line 773 documents this constraint precisely.

### 3. Defer to a future PR
**Rejected.** The migration work has already been done. This is a 2-line fix. Deferring it means the lint enforcement gap persists indefinitely, and any subsequent PR that adds a print statement to `perl-lsp-launcher` would be in scope.

## References
- GitHub issue: #3224 — "quality(prints): 210 debug prints in library code"
- PR #2446 — "feat(logging): add file-based log rotation and migrate eprintln to tracing"
- Work item: work-1efc01c2
- Existing lint enforcement pattern: `crates/perl-lsp/src/lib.rs:312`, `crates/perl-dap/src/lib.rs:361`

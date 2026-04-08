# UX Testing Reference

This document describes the UX regression gate that protects the first-5-minutes
user experience from regressions on every PR.

## Background

User-visible breakages are the most damaging kind. A user who cannot start the
LSP server, open a Perl file, or see a meaningful error message will not use the
tool. The UX regression gate systematically finds, tests, and keeps fixed the
scenarios that matter most in the first few minutes of use.

## How the CI Gate Works

The `ux-regression-gate` GitHub Actions workflow runs on every PR that touches
UX-relevant paths:

- `crates/perl-lsp*/**` — LSP server code
- `crates/perl-dap*/**` — Debug Adapter Protocol code
- `crates/perl-lsp-ux-tests/**` — the UX test harness itself
- `vscode-extension/**` — VS Code extension
- `features.toml` — LSP capability definitions
- `.github/workflows/ux-regression-gate.yml` — the gate itself

PRs that touch only `docs/`, `test_corpus/`, `archive/`, or `xtask/` do NOT
trigger the gate. This keeps CI fast for non-UX changes.

### Trigger conditions

The gate runs on:

1. Every PR push to a UX-relevant file (path filters above)
2. PRs with the `merge-ready` label (matching the ci.yml pattern)

### What the gate does

1. Installs system dependencies: `perl`, `perltidy`, `perlcritic`
2. Detects whether `just ux-tests` is available (graceful degradation if not)
3. Runs `just ux-tests` — the test harness in `crates/perl-lsp-ux-tests/`
4. Parses results and writes a per-scenario pass/fail summary to the job summary
5. Uploads a results artifact on failure
6. Posts a commit status (`ci/ux-regression-gate`) to the PR

### Coverage tracking

At the end of each run, the gate prints:

```
UX Coverage: N scenarios across M categories
  startup: X scenario(s)
  first-open: Y scenario(s)
  ...
```

Watch this metric grow as new scenarios are added.

### UX test categories

| Category | What it covers |
| --- | --- |
| `startup` | LSP server process starts cleanly, no crash on init |
| `first-open` | Opening a `.pl` file produces diagnostics within timeout |
| `missing-dep` | Graceful degradation when `perltidy`/`perlcritic` absent |
| `bad-config` | Clear, actionable error on malformed `.perltidyrc` / config |
| `protocol-handling` | LSP/DAP protocol correctness for basic operations |
| `error-messages` | User-visible error messages are clear and actionable |

## What to Do When the Gate Fails on Your PR

1. **Read the job summary** on the failed PR check. It lists each failed scenario
   and the specific assertion that broke.

2. **Reproduce locally:**
   ```bash
   just ux-tests
   ```
   This runs the same test harness as CI. You need `perl`, `perltidy`, and
   `perlcritic` installed locally.

3. **Find the failing test** in `crates/perl-lsp-ux-tests/`. Each scenario is a
   named test case. The test name will appear in the output.

4. **Fix the regression.** The gate exists to protect users — fix the behavior,
   not the test. If the behavior change was intentional, update the test and
   explain why in the PR description.

5. **Re-run `just ux-tests`** to confirm the fix before pushing.

## Graceful Degradation (Pre-Harness Period)

While `crates/perl-lsp-ux-tests/` is being developed, the gate **does not block**
PRs. Instead it:

- Detects that `just ux-tests` is unavailable
- Prints a loud warning in the job log
- Marks the job as SKIPPED (success) so the commit status is green

Once the harness PR lands and `just ux-tests` is registered in the justfile,
the gate becomes active automatically — no workflow change needed.

## How to Add a New Scenario

When you discover a new UX blocker:

1. **File a GitHub issue** labelled `ux-regression` describing the failure and
   which category it belongs to (see table above).

2. **Write a test** in `crates/perl-lsp-ux-tests/tests/` that reproduces the
   failure. Follow the naming convention: `test_<category>_<description>`.

3. **Make the test pass** by fixing the underlying behavior.

4. **Verify coverage** by running `just ux-tests` and checking that the new
   scenario appears in the output under its category.

5. **Reference the issue** in the test docstring so future readers know why
   the test exists.

### Example test structure

```rust
/// Startup: LSP server should not crash on an empty workspace.
/// Regression for: https://github.com/owner/perl-lsp/issues/XXXX
#[test]
fn test_startup_empty_workspace() -> Result<()> {
    // startup category
    let server = UxTestServer::start()?;
    let init_result = server.initialize_empty_workspace()?;
    assert!(init_result.capabilities.text_document_sync.is_some(),
        "LSP server must advertise textDocumentSync on startup");
    Ok(())
}
```

## Running the Gate Locally

```bash
# Full UX test suite
just ux-tests

# With verbose output
cargo test -p perl-lsp-ux-tests -- --nocapture

# Single scenario
cargo test -p perl-lsp-ux-tests -- test_startup_empty_workspace --exact
```

## Related

- `crates/perl-lsp-ux-tests/` — the test harness crate
- `.github/workflows/ux-regression-gate.yml` — the CI gate workflow
- `features.toml` — LSP capability definitions (source of truth for features)
- [LSP_IMPLEMENTATION_GUIDE.md](LSP_IMPLEMENTATION_GUIDE.md)

# Verification Findings — work-aa31d3df

## Confidence Assessment

**Medium** — The verification was constrained by missing tools (`cargo deny`, `cargo machete`) and incomplete git history for SBOM files. Many claims could not be independently verified, though the confirmed correct findings carry high confidence.

## Confirmed Findings

### Finding 5a — `run_test_sub` lacks `sub_name` identifier validation
**Status: CONFIRMED**

The `run_test_sub` function at `crates/perl-lsp/src/execute_command/provider.rs:236-260` accepts a `sub_name` parameter and passes it directly to a Perl one-liner:

```rust
pub(crate) fn run_test_sub(&self, file_path: &Path, sub_name: &str) -> Result<Value, String> {
    let perl_code = r#"
        my ($file, $sub) = @ARGV;
        do $file;
        if (defined &$sub) {
            no strict 'refs';
            &$sub();
        } else {
            die "Subroutine $sub not found";
        }
    "#;
    let mut perl_cmd = Command::new("perl");
    perl_cmd.arg("-e").arg(perl_code).arg("--").arg(ext_path.as_os_str()).arg(sub_name);
    ...
}
```

The `sub_name` is passed as a raw argument with no validation. The `file_path` IS validated via `resolve_path_from_args`, but `sub_name` is not validated for Perl identifier shape. The use of `no strict 'refs'` and `&$sub()` means any valid Perl symbol table entry can be called after `do $file` loads.

### Finding 5b — `validate_expression` name overstates coverage
**Status: CONFIRMED**

`crates/perl-dap/src/security/mod.rs:73-79`:

```rust
pub fn validate_expression(expression: &str) -> Result<(), SecurityError> {
    if expression.contains('\n') || expression.contains('\r') {
        return Err(SecurityError::InvalidExpression);
    }
    Ok(())
}
```

This only rejects newlines/carriage returns. It does not reject backticks (`` ` ``), `system(...)`, `open(..., "|...")`, `qx{...}`, `eval "..."`, etc. The function name `validate_expression` overstates its actual coverage.

### Windows extended-length path fix (#4089)
**Status: CONFIRMED**

Git history shows commit `ef92dd20` with message:
> "fix(execute-command): strip Windows extended-length prefix before external command (#4085) (#4089)"

The `normalize_path_for_external_command` function at `provider.rs:28-57` is correctly implemented with proper handling for UNC paths on Windows.

### `perl-ci-hygiene` crate
**Status: CONFIRMED**

The crate exists at `crates/perl-ci-hygiene/` in the workspace.

### `deny.toml` structure
**Status: PARTIALLY CONFIRMED**

`deny.toml` at lines 20-29 shows the `[advisories].ignore` section with only `RUSTSEC-2023-0089` (atomic-polyfill) ignored. `RUSTSEC-2026-0097` is NOT present. However, I could not run `cargo deny check advisories` to verify the actual advisory check behavior since `cargo deny` is not installed.

## Corrected Findings

### Finding 8 — SBOMs out of sync (MATERIALLY INCORRECT)

**The research agent's Finding 8 is based on a false premise.**

The research agent stated:
> "Files: `sbom-cyclonedx.json`, `sbom-spdx.json` (last updated 2026-03-29) vs `Cargo.lock` (2026-04-04)"

**Reality: These files do NOT exist in the repository.**

Verification:
```bash
$ git ls-files | grep -i sbom
# (no output - no SBOM files tracked)

$ ls /home/hermes/repos/perl-lsp/sbom-*
# No SBOM files found at repo root
```

The SBOM files are generated **on-demand** during the release process via `justfile`:
```
# justfile lines 249-269
sbom-spdx:
    cargo sbom --output-format spdx_json_2_3 > sbom-spdx.json

sbom-cyclonedx:
    cargo sbom --output-format cyclone_dx_json_1_6 > sbom-cyclonedx.json

sbom: sbom-spdx sbom-cyclonedx
```

The `release-gate` target (line 2236) includes `sbom-verify` which checks that the files exist after generation. There is **no SBOM drift issue** because the files are generated from `Cargo.lock` at release time, not stored and tracked separately.

**This finding should be removed from the backlog.** The SBOMs are already fresh by design — they are generated from the current `Cargo.lock` during the release process.

### Finding 4 — `cargo machete` false positive (UNVERIFIED/INCORRECTLY CHARACTERIZED)

The research agent stated:
> "`perllsp` is a public Cargo facade that re-exports `perl-lsp-rs` from its binary target (`src/main.rs`). `cargo machete` only inspects `src/lib.rs`, so it reports `perl-lsp-rs` as unused."

**This claim is likely incorrect.** Looking at the actual code:

- `crates/perllsp/Cargo.toml` declares `perl-lsp-rs = { workspace = true }`
- `crates/perllsp/src/lib.rs` has `pub use perl_lsp::*;`
- `crates/perl-lsp/Cargo.toml` defines `[lib] name = "perl_lsp"` (the library name differs from the package name `perl-lsp-rs`)

The `perl-lsp-rs` package's library is named `perl_lsp` (defined in `[lib]` section), so `pub use perl_lsp::*` IS a use of the `perl-lsp-rs` dependency. `cargo machete` should understand this because it analyzes the dependency graph, not just textual patterns. However, I cannot verify because `cargo machete` is not installed.

**The proposed fix** (`[package.metadata.cargo-machete] ignored = ["perl-lsp-rs"]`) may not be needed at all, or may be needed for a different reason than stated.

## New Findings

### SBOM "freshness gate" claim is inapplicable

The research agent recommended:
> "Add a CI job (or extend existing release job) that compares timestamps of SBOMs vs `Cargo.lock` and fails if SBOM is older."

Since SBOM files are not stored in the repo and are regenerated from `Cargo.lock` at release time, this "freshness gate" is **not applicable**. The SBOM is always freshly generated from the current `Cargo.lock` during the release. There is nothing to compare timestamps against in the repo.

### `perl.runTestSub` vs `perl.runSubtest` command names

The `execute_command` function at lines 99-106 and 115-122 shows two commands that call `run_test_sub`:
- `"perl.runTestSub"` at line 99
- `"perl.runSubtest"` at line 115

Both route to the same `run_test_sub` function. This duplication may be intentional (aliasing) or an oversight. The research agent noted both commands but did not flag the duplication.

## Scope Assessment

The issue title is:
> "security: 2026-04-11 end-of-session broad sweep"

The scope described in the research analysis matches the issue: eight security checks across banned constructs, unsafe blocks, dependency vulnerabilities, supply-chain health, command-injection/path-traversal surfaces, hardcoded credentials, and the Windows extended-length path fix.

**However, Finding 8 (SBOM drift) was based on non-existent files and should be removed.** This was a material error in the research.

## Verification Methodology

1. **File existence checks**: Used `git ls-files`, `find`, and `ls` to verify existence of SBOM files — found none
2. **Code inspection**: Read `provider.rs`, `security/mod.rs`, `deny.toml`, `perllsp/Cargo.toml`, `perllsp/src/lib.rs` directly to verify claims
3. **Git history**: Checked `git log --all --oneline` for relevant commits (#4089 found)
4. **Tool availability**: Attempted `cargo deny` and `cargo machete` — neither is installed, limiting verification of those claims
5. **Workspace structure**: Inspected `Cargo.toml` for dependency declarations and `justfile` for SBOM generation commands

### Commands run:
```bash
git ls-files | grep -i sbom        # No SBOM files tracked
ls /home/hermes/repos/perl-lsp/sbom-*  # No SBOM files at root
git log --oneline | grep 4089     # Found ef92dd20
grep -n "RUSTSEC-2026-0097" deny.toml  # Not found (not ignored)
cargo deny check advisories        # Tool not installed
cargo machete                     # Tool not installed
```

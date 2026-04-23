# Building Quality Into an LSP From Day One

How the perl-lsp project uses a layered quality system -- spanning static analysis, mutation testing, fuzz testing, corpus validation, supply chain security, and codified technical debt tracking -- to keep a 52,000-line, 121-crate Rust workspace reliable.

---

## The No-Fatal-Constructs Policy

Most Rust projects rely on the language's safety guarantees and call it a day. perl-lsp goes further: **production code contains zero calls to `unwrap()`, `expect()`, `panic!()`, `todo!()`, `unimplemented!()`, or `dbg!()`**. The current ratchet baseline is literally zero for all of these.

This is not a convention enforced by code review alone. Three separate automated gates enforce it on every merge:

1. **Clippy lint gates** (`clippy::unwrap_used`, `clippy::expect_used`) run against all library and binary targets, including tests. The merge gate runs two passes: one for library code (`--workspace --lib --bins`) and a stricter pass across all targets (`--workspace --all-targets`).

2. **A ratchet script** (`ci/check_unwraps_prod.sh`) scans production source files and compares counts against a baseline. If the count increases, the gate fails. This catches patterns that Clippy's lints might miss.

3. **A forbidden-constructs gate** (`scripts/forbid-fatal-constructs.sh`) catches `std::process::abort()`, misplaced `std::process::exit()` calls (only allowed in `bin/` directories and `lifecycle.rs`), and other fatal patterns that fall outside Clippy's scope. This gate is implemented as a dedicated Rust binary (`perl-ci-hygiene`), not a fragile shell script with regex.

A parallel **unsafe syntax ratchet** (`ci/check_unsafe_prod.sh`) enforces zero explicit `unsafe` blocks in production source.

The only exception is a single centralized `#[allow(clippy::expect_used)]` for an `lsp_types::Uri` fallback in `crates/perl-lsp-rs/src/util/uri.rs`. In tests, the project uses `Result<()>` return types and dedicated `perl_tdd_support::must`/`must_some` helpers instead of assertions that could panic.

The philosophy: an LSP server that crashes takes down your editor. Graceful degradation is not optional.

---

## Three-Tier CI: Fast Feedback, Gate, Deep Validation

The CI system is organized into three tiers with distinct purposes, time budgets, and enforcement levels. The design philosophy is **local-first**: developers run `nix develop -c just ci-gate` before pushing, and CI on GitHub is a safety net, not the primary feedback mechanism.

### Tier A: PR-Fast (~1-2 minutes)

Purpose: catch obvious issues before review begins. Runs:
- `cargo fmt --check --all`
- Clippy on core crates only (perl-parser, perl-lexer)
- Unit tests on core crates only (library tests, no integration tests)

This tier is designed so developers can run it dozens of times per day without friction.

### Tier B: Merge Gate (~3-5 minutes)

Purpose: full verification before code lands on master. This tier composes Tier A plus:
- Full-workspace Clippy (including the unwrap/expect enforcement pass)
- All three ratchet checks (unwrap/panic, unsafe, forbidden constructs)
- Full workspace library tests (1,543 tests across 121 crates)
- LSP smoke tests (capabilities, protocol basics)
- LSP behavioral tests (semantic definitions, completion, code actions, security)
- DAP smoke tests (launch, breakpoint, step, evaluate, disconnect)
- Common corpus zero-error gate (pinned Perl modules must parse without errors)
- V2 parser parity check (legacy Pest parser output matches native parser)
- Policy checks (documentation baseline, features.toml invariants, version sync)
- Workflow audit (prevents ungated expensive jobs from sneaking into CI)
- Nested lockfile detection (prevents footgun from running gates in subdirectories)

Every step is individually timed and reported. The gate produces a structured JSON receipt (`target/receipts/receipt.json`) that captures pass/fail status, duration, and artifacts for each step.

### Tier C: Nightly (~15-30 minutes)

Purpose: catch subtle issues and track metrics over time. Non-blocking but tracked:
- Mutation testing (cargo-mutants across the workspace)
- Fuzz testing (5 libFuzzer targets, 60 seconds each in bounded mode, 600 seconds in nightly CI)
- Performance benchmarks (Criterion, with regression detection and PR comments)
- Code coverage (cargo-llvm-cov, uploaded to Codecov)
- Full OS/toolchain matrix (Ubuntu, Windows, macOS x stable, MSRV 1.92, beta)
- Corpus sweep with ratchet baseline
- Determinism checks (run tests 3 times, diff outputs)
- SemVer compatibility checks (cargo-semver-checks against last release tag)
- Tautology detection (catches `assert!(true)` and similar vacuous assertions)

The nightly tier runs on schedule at 3 AM UTC and can be triggered on-demand via labels (`ci:mutation`, `ci:bench`, `ci:coverage`) or workflow dispatch.

### The Escape Hatch System

The gate policy codifies escape hatches with escalating severity:
- **Skip a specific gate**: Add `[skip-gate:GATE_NAME]` to commit message. Requires maintainer approval. Auto-creates tracking issue.
- **Force merge**: Requires two maintainer approvals. Must document justification in PR.
- **Disable all gates**: Nuclear option. Requires repository admin. Auto-reverts after 24 hours. Sends alert.

Some gates can never be skipped: `fmt`, `clippy_core`, `unit_core`, `unit_full`.

---

## Mutation Testing: Are Your Tests Actually Testing?

Having tests pass is necessary but insufficient. Tests that never fail are useless. perl-lsp uses [cargo-mutants](https://mutants.rs/) to verify that its tests actually catch regressions.

Mutation testing works by systematically modifying source code (replacing `+` with `-`, deleting function bodies, changing return values) and verifying that at least one test fails for each mutation. Mutants that survive indicate blind spots in the test suite.

The project achieves an **87% mutation score**, meaning 87% of mutations are caught by existing tests. This is tracked as a metric in `CURRENT_STATUS.md` and verified in the nightly CI tier.

The merge gate runs mutation testing against `perl-parser-core` with a 60-second timeout per mutant and 2 parallel jobs. The nightly tier runs a broader sweep across the workspace. Results are non-blocking but tracked for trend analysis.

From the justfile:

```
cargo mutants --workspace -j 2 --timeout 60
```

---

## Fuzz Testing: Finding What You Didn't Think to Test

The project maintains **7 fuzz targets** powered by libFuzzer via `cargo-fuzz`, each targeting a specific attack surface:

| Target | Purpose |
|--------|---------|
| `builtin_functions` | map/grep/sort with malformed block arguments |
| `heredoc_parsing` | Heredoc delimiters, boundary conditions, unterminated quotes |
| `substitution_parsing` | Regex substitution operator edge cases |
| `lsp_navigation` | LSP go-to-definition with adversarial inputs |
| `unicode_positions` | UTF-16 position mapping with multi-byte characters |
| `lsp_cancellation_registry` | Cancellation token handling under adversarial conditions |
| `fuzz_target_1` | General parser fuzzing |

Each fuzz target is designed to exercise a specific bug class. For example, the heredoc fuzzer specifically targets the off-by-one boundary fix in commit `cd7a2442`, generating patterns like unterminated heredoc quotes (`<<"` without closing), empty delimiters (`<<""`), and single-character edge cases. The unicode positions fuzzer exercises UTF-16 boundary handling with emoji identifiers and multi-byte characters.

In the nightly CI, each target runs for **600 seconds** (10 minutes) with `max_len=1000` to bound input size and `timeout=25` seconds per individual test case. The bounded local mode (`just fuzz-bounded`) runs each for 60 seconds. Crash artifacts are automatically uploaded as CI artifacts for investigation.

The project also supports continuous fuzzing for local development (`just fuzz-continuous`), fuzz corpus coverage analysis (`just fuzz-coverage`), and crash minimization (`just fuzz-minimize`).

---

## Corpus Validation: Real Perl as the Ultimate Test

Synthetic tests verify what you think to test. Real-world code verifies everything else. perl-lsp maintains two corpus collections:

### Tree-sitter Test Corpus (46 directories)

Located in `tree-sitter-perl/test/corpus/`, this corpus contains approximately **611 test sections** covering Perl syntax categories: operators, expressions, heredocs, regex, interpolation, functions, pod, quote-like operators, object-oriented patterns, and more.

### Test Corpus (60+ Perl files)

Located in `test_corpus/`, these are complete Perl source files covering production patterns: class methods, try/catch, signatures, regex substitution, format statements, heredocs, edge cases, error recovery, and more. A subdirectory `edge_cases/` contains 6 additional files targeting specific parser failure modes.

### The Common Corpus Zero-Error Gate

A subset of widely-used Perl modules is **pinned in `.ci/common-corpus-manifest.txt`** and must parse with zero errors on every merge. This currently includes core pragmas (`XSLoader`, `bytes`, `integer`, `utf8`, `subs`) and core modules (`Config`, `File::Spec`, `MIME::Base64`, `I18N::Langinfo`, `PerlIO`, `Encode::Encoding`, `Tie::Scalar`).

This manifest grows as parser coverage improves. Modules listed as targets for future expansion include `Exporter`, `Carp`, `File::Find`, `Getopt::Long`, `Test::More`, `Data::Dumper`, and `ExtUtils::MakeMaker`.

The gate runs via `xtask`:

```
cargo run -p xtask -- parser-corpus-sweep \
    --manifest .ci/common-corpus-manifest.txt --enforce --receipt
```

### The Corpus Sweep Ratchet

Beyond the zero-error pinned modules, a broader sweep runs against a baseline (`--baseline .ci/parser-corpus-baseline.json`). This is a **ratchet**: the number of successfully parsed modules can only increase. If a change causes a previously-passing module to fail, the gate catches the regression.

---

## Supply Chain Security

perl-lsp treats dependencies as an attack surface, not an afterthought.

### cargo-deny (Policy Enforcement)

The `deny.toml` configuration enforces:
- **License allowlist**: Only MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Unicode-3.0, CC0-1.0, and Zlib licenses are permitted.
- **Advisory scanning**: Dependencies are checked against the RustSec advisory database. Known exceptions (like `RUSTSEC-2023-0089` for `atomic-polyfill` in legacy transitive deps) are documented with rationale.
- **Source restrictions**: Only `crates.io` is allowed as a registry source. Unknown git sources generate warnings.
- **Duplicate detection**: Multiple versions of the same crate produce warnings.

### cargo-audit (Vulnerability Scanning)

Runs in the merge gate (Tier B) and daily on schedule. The security CI workflow (`ci-security.yml`) runs cargo-audit with JSON output, uploads results as artifacts, and retains them for 30 days.

### Trivy (Comprehensive Scanning)

A dedicated security workflow runs three Trivy scans:
1. **Repository scan**: Filesystem-mode scanning for vulnerabilities, misconfigurations, and secrets across the entire codebase.
2. **Docker image scan**: Scans the built container image for vulnerabilities.
3. **SARIF integration**: Results are uploaded to GitHub's Security tab for unified vulnerability tracking.

CRITICAL and HIGH findings on PRs fail the workflow. Scheduled daily scans report without blocking.

### SBOM (Software Bill of Materials)

Every release includes SBOMs in two industry-standard formats:
- **SPDX v2.3** (ISO/IEC 5962:2021)
- **CycloneDX v1.6** (OWASP standard)

SBOM generation is a release gate: `just sbom-verify` must pass before release.

### SLSA Provenance (Level 2)

Release artifacts include cryptographic attestations via GitHub Attestations (Sigstore-based). Users can verify any artifact:

```bash
gh attestation verify perl-lsp-v0.10.0-x86_64-unknown-linux-gnu.tar.gz \
    --owner EffortlessMetrics
```

### Dependabot Configuration

Dependency updates are automated via Dependabot across three ecosystems (Cargo, GitHub Actions, npm for the VSCode extension). Dependencies are grouped by domain (serde, tokio, tracing, lsp, testing, tree-sitter, pest) to reduce PR noise. Major version updates for critical dependencies (tree-sitter, lsp-types, tower-lsp, tokio) are excluded from automation and require manual review.

---

## Technical Debt as a First-Class Citizen

Most projects track technical debt informally, if at all. perl-lsp codifies it in a structured YAML ledger (`.ci/debt-ledger.yaml`) with budgets, expiration dates, and CI enforcement.

### The Debt Ledger

Three categories are tracked:
1. **Quarantined tests**: Flaky tests excluded from the merge gate but still executed for visibility. Each entry has an owner, a tracking issue, an expiration date, and documented failure patterns.
2. **Known issues**: Acknowledged problems with status tracking (accepted, deferred, monitoring, wontfix) and target versions.
3. **Technical debt**: Architectural and code quality items with priority (critical/high/medium/low) and categories (architecture, error_handling, testing, performance, documentation, security, dependencies).

### Budget Enforcement

Each category has a hard budget:
- Maximum quarantined tests: 10
- Maximum known issues: 20
- Maximum technical debt items: 30

Warning thresholds fire at 80% of budget; critical at 95%. The merge gate fails if budgets are exceeded or quarantines have expired. This prevents the common pattern where debt accumulates silently until it becomes unmanageable.

### Quarantine Mechanics

When a test is quarantined:
- It still **runs** as part of the gate
- Results are **reported** in receipts
- Failures do not **block** merges
- The quarantine has a **shelf life** (typically 7-14 days)
- When it expires, it must be fixed, renewed with justification, or (rarely) disabled

### Historical Tracking

Resolved items are recorded with resolution details, PR references, and time spent in quarantine. Weekly summaries track trend data: how many items were added vs. resolved, overall debt levels. This creates an institutional memory of quality work.

As of the latest ledger update, the project has **0 quarantined tests**, **0 known issues**, and **4 technical debt items** -- well within all budgets.

---

## The Gate Policy: Codifying Quality Standards

The entire CI system is configured declaratively in `.ci/gate-policy.yaml`, a 350+ line YAML document that serves as the single source of truth for what checks run, when they run, how strictly they are enforced, and what their time budgets are.

Each gate entry specifies:
- **Tier**: When it runs (pr_fast, merge_gate, nightly, release)
- **Command**: The exact command to execute
- **Timeout**: Maximum allowed duration
- **Duration budget**: Expected completion time (alerts if exceeded by 150%)
- **Retry count**: Automatic retries for transient failures
- **Quarantine status**: Whether failures are blocking
- **Tags**: For filtering and reporting

The policy also defines flake management (auto-quarantine after 3 failures in a week), audit trail retention (90 days), success rate alerting (below 95%), and escape hatch monthly limits (maximum 3 per month).

This declarative approach means the quality system is version-controlled, reviewable, and auditable -- the same standards apply to the quality infrastructure as to the product code.

---

## Metrics and Results

| Metric | Value |
|--------|-------|
| Total library tests | 1,543 |
| Crates in workspace | 121 |
| Lines of Rust source | ~52,000 |
| Mutation score | 87% |
| Quarantined tests | 0 |
| Known issues | 0 |
| Technical debt items | 4 |
| LSP feature coverage | 100% (53/53 advertised features) |
| Protocol compliance | 100% (97/97 including plumbing) |
| Fuzz targets | 7 |
| Corpus test sections | ~611 (tree-sitter) + 60+ Perl files |
| Production unwrap/expect count | 0 |
| Production panic/todo/unimplemented count | 0 |
| Explicit unsafe blocks | 0 |
| SLSA level | 2 |
| SBOM formats | SPDX 2.3, CycloneDX 1.6 |
| CI cost per PR | ~$0.05 |

---

## Lessons for Other Rust Projects

**1. Ban fatal constructs early.** It is vastly easier to establish a zero-unwrap baseline from the start than to retrofit it. The cost is learning to write `?` and `ok_or_else()` fluently, which is a skill that improves your error handling design.

**2. Layer your CI into tiers.** A single 30-minute gate means developers stop running it locally. A 2-minute fast tier with a 5-minute merge gate and a 30-minute nightly tier means feedback matches the cost of interruption.

**3. Mutation testing reveals vacuous tests.** An 87% mutation score means 13% of possible regressions would slip through. Without mutation testing, you have no idea what that number is.

**4. Fuzz with purpose.** Each fuzz target should correspond to a specific bug class or attack surface, not just "throw random bytes at the parser." The heredoc fuzzer targets a specific off-by-one fix; the unicode fuzzer targets UTF-16 boundary handling.

**5. Use real-world code as your corpus.** If your parser handles synthetic tests but fails on `Config.pm`, it does not work. Pinning real modules to a zero-error gate creates an honest measure of progress.

**6. Track debt with budgets, not good intentions.** A YAML ledger with expiration dates and CI enforcement prevents the "we'll fix it later" accumulation that eventually makes a codebase unmaintainable.

**7. Codify your quality standards.** A gate policy YAML is reviewable, version-controlled, and unambiguous. "We run clippy" is vague; a gate policy with exact commands, timeouts, and enforcement levels is precise.

**8. Make CI cost-conscious.** perl-lsp tracks CI cost per PR (~$0.05). Expensive jobs are label-gated or nightly-only. Concurrency groups cancel in-flight runs on new pushes. This is not frugality for its own sake -- it is what makes local-first development feasible.

---

*This article describes the quality infrastructure of perl-lsp as of v0.10.0. All metrics are computed, not hand-edited, and can be verified by running `just ci-gate` and `just status-check`.*

# Tooling Profile

Static analysis tooling stack for the perl-lsp workspace. This document covers what tools run when, why, and how to interpret outputs.

> **Purpose**: Define what analysis tools run where, eliminating "why didn't we run X?" debates.
>
> **Contract**: This document is the canonical source for tooling scope. If a tool isn't listed here, it's not in scope.

---

## 1. Philosophy

**Quality comes first; tools are servants.**

Tools in this repository fall into three categories:

1. **Leading analysis** - Tools that actually pay rent: they prevent rot, catch real issues, and maintain codebase integrity. These run on every PR.

2. **Exhibit-grade analysis** - Deeper tools reserved for high-risk changes, release candidates, or forensic investigations. These are expensive but valuable when applied selectively.

3. **Research tier** - Experimental or one-off analysis tools for deep dives, architectural investigations, and specific troubleshooting. NOT gate-blocking, run only when needed.

The tradeoff is always-on vs. exhibit-grade vs. research:

| Always-On | Exhibit-Grade | Research |
|-----------|---------------|----------|
| Fast (<5 min) | Slow (10-60 min) | Varies (5s-10min) |
| Low noise | High signal depth | High noise or experimental |
| Blocks merge | Informs decisions | Exploratory insights |
| Every PR | Selective application | On-demand only |

**Principle**: If a tool has a high false positive rate or excessive runtime, it does not belong in `ci-gate`. Put it in exhibit-grade and run it when it matters. If a tool is experimental or useful only for specific investigations, put it in research tier.

### Lane Selection Matrix

| Change Type | Always-on | Exhibit-grade | Research | Security/Policy |
|-------------|-----------|---------------|----------|-----------------|
| Any PR | Required | Optional | As needed | Optional |
| Parser changes | Required | Recommended | As needed | Optional |
| Dependency updates | Required | Optional | cargo-outdated, cargo-udeps | Required |
| Release candidate | Required | Required | cargo-bloat, typos | Required |
| LSP protocol changes | Required | Recommended | As needed | Optional |
| CI/build changes | Required | Optional | actionlint | Recommended |
| Major refactors | Required | Required | cargo-modules, rust-code-analysis | Optional |
| Macro work | Required | Recommended | cargo-expand | Optional |
| Script changes | Required | Optional | shellcheck | Optional |

---

## 2. Always-On Tools (ci-gate)

These run on every PR via the local gate. **All must pass before merge.**

```bash
nix develop -c just ci-gate  # ~2-5 min
```

| Tool | Command | What it catches | Failure = |
|------|---------|-----------------|-----------|
| **fmt** | `cargo fmt --check --all` | Style drift | Block |
| **clippy** | `cargo clippy --workspace --lib --locked -- -D warnings -A missing_docs` | Lint debt, common bugs | Block |
| **tests** | `cargo test --workspace --lib --locked` | Regressions | Block |
| **audit** | `cargo audit` | Known vulnerabilities | Warn |
| **status-check** | `just status-check` | Doc/metric drift | Block |

### fmt (`cargo fmt --check`)

**What it prevents**: Style drift that makes diffs noisy and reviews harder.

| Attribute | Value |
|-----------|-------|
| **Command** | `cargo fmt --check --all` |
| **Lane** | Always-on |
| **What it measures** | Code formatting consistency |
| **Failure meaning** | Code not formatted per rustfmt rules |
| **Installation** | Bundled with Rust (via `rust-toolchain.toml`) |

**How to interpret failures**: Any failure means unformatted code. Run `cargo fmt --all` to fix. Never push unformatted code; it wastes reviewer attention.

**Common false positives**: None. This is purely mechanical.

**Fix**:
```bash
cargo fmt --all
```

### clippy (`cargo clippy`)

**What it prevents**: Common bugs, performance anti-patterns, deprecated APIs, style issues.

| Attribute | Value |
|-----------|-------|
| **Command** | `cargo clippy --workspace --lib --locked -- -D warnings -A missing_docs` |
| **Lane** | Always-on |
| **What it measures** | Common Rust anti-patterns and potential bugs |
| **Failure meaning** | Code violates clippy lints at warning level |
| **Installation** | Bundled with Rust (via `rust-toolchain.toml`) |
| **Config** | `clippy.toml` |

The `-A missing_docs` allow is temporary during documentation backfill (see 8-week plan in CURRENT_STATUS.md).

**How to interpret failures**:
- Warnings are treated as errors (`-D warnings`)
- Each warning includes a link to the lint documentation
- Fix by following the suggested action or adding an allow attribute with justification

**Common false positives**:
- `clippy::too_many_arguments` - Sometimes necessary for builder patterns
- `clippy::type_complexity` - Sometimes type aliases harm readability

**Fix pattern**:
```rust
#[allow(clippy::too_many_arguments)]  // Builder pattern requires these
fn complex_constructor(...) { }
```

**Thresholds** (from `clippy.toml`):
- `cognitive-complexity-threshold`: 50
- `too-many-arguments-threshold`: 8
- `too-many-lines-threshold`: 500
- `type-complexity-threshold`: 350

### tests (`cargo test`)

**What it prevents**: Regressions, broken functionality.

| Attribute | Value |
|-----------|-------|
| **Command** | `cargo test --workspace --lib --locked` |
| **Lane** | Always-on |
| **What it measures** | Library unit test correctness |
| **Failure meaning** | Test assertion failed or panic occurred |
| **Installation** | Bundled with Rust |

Only library tests run in the gate. Integration tests run in `ci-full`.

**How to interpret failures**:
- Any test failure blocks merge
- Check the test name and failure output for the specific assertion
- `#[ignore]` tests are tracked in `CURRENT_STATUS.md` as test debt

**Threading considerations** (for flaky tests):
```bash
RUST_TEST_THREADS=2 cargo test -p perl-lsp -- --test-threads=2
```

**Test Metrics**:

| Metric | Healthy | Warning | Critical |
|--------|---------|---------|----------|
| **Pass rate** | 100% | < 100% | < 95% |
| **Ignored tests** | 0-5 | 5-15 | > 15 |
| **Test debt** | 0-5 | 5-10 | > 10 |

**Current status**: 337 passed, 1 ignored, 9 tracked test debt (8 bug, 1 manual)

### LSP Semantic Definition Tests

| Attribute | Value |
|-----------|-------|
| **Command** | `RUSTC_WRAPPER="" RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 cargo test -p perl-lsp --test semantic_definition -- --test-threads=1` |
| **Lane** | Always-on |
| **What it measures** | LSP go-to-definition semantic correctness |
| **Failure meaning** | Definition resolution is broken |
| **Installation** | Bundled with Rust |
| **Config** | Thread-constrained for determinism |

### Policy Checks

| Attribute | Value |
|-----------|-------|
| **Command** | `.ci/scripts/check-from-raw.sh`, `just status-check` |
| **Lane** | Always-on |
| **What it measures** | Repository policy compliance (from_raw_parts ban, metric freshness) |
| **Failure meaning** | Policy violation detected |
| **Installation** | Shell scripts in repo |

**What it prevents**: Security-sensitive patterns, specifically `from_raw_parts` calls that could be unsafe; documentation/metric drift where CURRENT_STATUS.md gets out of sync.

**Fix**:
```bash
just status-update  # Regenerate computed metrics
```

---

## 3. Exhibit-Grade Tools

Run selectively on "casebook candidates" and risky refactors. These are too expensive or noisy for every PR.

| Tool | Command | When to run | What it catches |
|------|---------|-------------|-----------------|
| **semver-checks** | `cargo semver-checks check-release` | Library crate changes | API breaks |
| **geiger** | `cargo geiger` | Unsafe-touching PRs | Unsafe growth |
| **llvm-cov** | `cargo llvm-cov` | Coverage-critical changes | Coverage regression |
| **rust-code-analysis** | `rust-code-analysis --metrics` | Complexity-risk PRs | Complexity growth |
| **cargo-mutants** | `cargo mutants` | Parser/validator changes | Hollow tests |

### semver-checks (`cargo semver-checks`)

**When to use**:
- Before publishing to crates.io
- After any change to `pub` items in `perl-parser` or `perl-lexer`
- When the STABILITY.md promises might be affected

| Attribute | Value |
|-----------|-------|
| **Command** | `cargo semver-checks check-release -p perl-parser` |
| **Lane** | Exhibit-grade |
| **Trigger** | Label `ci:semver` or workflow dispatch |
| **What it measures** | SemVer compliance - detects breaking API changes |
| **Failure meaning** | Breaking change detected requiring major version bump |
| **Installation** | `cargo install cargo-semver-checks --locked` |

**How to interpret**:

| Result | Meaning | Action |
|--------|---------|--------|
| **Pass** | No breaking changes | Continue |
| **Breaking (internal)** | Private API changed | Document or revert |
| **Breaking (public)** | Public API changed | Major version bump required |

**Cost/benefit**: ~30s per crate. High value for library crates.

### geiger (`cargo geiger`)

**When to use**:
- PRs touching FFI code
- PRs adding new dependencies
- Periodic security reviews

| Attribute | Value |
|-----------|-------|
| **Command** | `cargo geiger --all-features` |
| **Lane** | Exhibit-grade |
| **What it measures** | Unsafe code usage in dependency tree |
| **Installation** | `cargo install cargo-geiger --locked` |

**How to interpret**:
- Reports `unsafe` block counts by crate
- Compare before/after to detect unsafe growth
- Goal: unsafe count should not increase without explicit justification

**Cost/benefit**: ~2-5 min. Run on unsafe-touching PRs only.

### llvm-cov (`cargo llvm-cov`)

**When to use**:
- Major refactors
- Coverage-critical paths (parser, validators)
- Release candidates

| Attribute | Value |
|-----------|-------|
| **Command** | `cargo llvm-cov -p perl-parser --tests --lcov` |
| **Lane** | Exhibit-grade |
| **Trigger** | Label `ci:coverage` or workflow dispatch |
| **What it measures** | Line and branch coverage |
| **Installation** | `cargo install cargo-llvm-cov --locked` |
| **Config** | Requires `llvm-tools-preview` component |

**How to interpret**:
- Look for uncovered branches in critical paths
- Compare to baseline coverage
- Not all uncovered code is a problem (error paths, debug code)

**Cost/benefit**: ~5-10 min. Run on coverage-sensitive PRs.

### cargo-mutants (Mutation Testing)

**When to use**:
- Parser changes
- Validator changes
- Any PR claiming "hardening" or "security"

| Attribute | Value |
|-----------|-------|
| **Command** | `cargo mutants --package perl-parser --timeout 300` |
| **Lane** | Exhibit-grade |
| **Trigger** | Label `ci:mutation` or `just ci-test-mutation` |
| **What it measures** | Test suite mutation coverage - surviving mutants indicate untested code paths |
| **Installation** | `cargo install cargo-mutants --locked` |

**How to interpret**:

| Score | Meaning | Action |
|-------|---------|--------|
| **> 90%** | Excellent test coverage | Maintain |
| **80-90%** | Good coverage | Review surviving mutants |
| **< 80%** | Coverage gaps | Prioritize test debt |

**Quality gates** (from MUTATION_TESTING_METHODOLOGY.md):
- Critical components (parser core, security): >= 90%
- Important components (LSP providers): >= 85%
- Supporting components (utilities): >= 75%

**Current baseline**: 87% mutation score (target: 87%+)

**Cost/benefit**: ~15-30 min. High value for finding hollow tests.

See [MUTATION_TESTING_METHODOLOGY.md](MUTATION_TESTING_METHODOLOGY.md) for detailed methodology.

### clippy (strict mode)

| Attribute | Value |
|-----------|-------|
| **Command** | `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::{all,pedantic,nursery,cargo}` |
| **Lane** | Exhibit-grade |
| **Trigger** | Label `ci:strict` or workflow dispatch |
| **What it measures** | Extended lint coverage including pedantic rules |
| **Installation** | Bundled with Rust |

### nextest (parallel test runner)

| Attribute | Value |
|-----------|-------|
| **Command** | `cargo nextest run --workspace --lib --locked` |
| **Lane** | Exhibit-grade (Nix default) |
| **What it measures** | Same as cargo test, with parallel execution and better output |
| **Installation** | `cargo install cargo-nextest --locked` or via Nix |

---

## 4. Research Tier (Experimental/On-Demand)

These tools are NOT gate-blocking and do NOT run in CI. They're useful for deep dives, specific investigations, and one-off analyses. Run these when you need insights, not as part of regular development.

**Philosophy**: Research tools are expensive, noisy, or experimental. They provide value for specific questions but would harm developer flow if run routinely.

### Rust Analysis Tools

| Tool | Purpose | When to run | Installation |
|------|---------|-------------|--------------|
| **cargo-modules** | Module graph and cycle detection | Major refactors, architecture reviews | `cargo install cargo-modules` |
| **rust-code-analysis** | Complexity metrics (cyclomatic, cognitive) | Hotspot deltas, refactoring planning | `cargo install rust-code-analysis-cli` |
| **cargo-bloat** | Binary size analysis | Release audits, size regressions | `cargo install cargo-bloat` |
| **cargo-udeps** | Unused dependency detection | Dependency cleanup, minimization | `cargo install cargo-udeps --locked` |
| **cargo-outdated** | Dependency freshness check | Upgrade planning, security reviews | `cargo install cargo-outdated` |
| **cargo-expand** | Macro expansion debugging | Macro troubleshooting, proc-macro work | `cargo install cargo-expand` |
| **cargo-depgraph** | Dependency visualization | Understanding dep tree, cycle detection | `cargo install cargo-depgraph` |

#### cargo-modules

**When to use**:
- Before major module refactorings
- When suspecting circular dependencies
- When planning crate splits or merges

**Command**:
```bash
cargo modules generate graph --package perl-parser | dot -Tpng > module-graph.png
cargo modules structure --package perl-parser
```

**How to interpret**:
- Look for unexpected cycles
- Identify overly coupled modules
- Find orphaned or poorly connected modules

**Cost**: ~10s. Generates graphviz output.

**Warning**: Output can be overwhelming for large crates. Use `--focus-on` to zoom into specific modules.

#### rust-code-analysis

**When to use**:
- Comparing complexity before/after refactor
- Identifying high-complexity hotspots for refactoring
- Establishing complexity baselines for new features

**Command**:
```bash
rust-code-analysis --metrics -p crates/perl-parser/src/
rust-code-analysis --metrics -p crates/perl-parser/src/ -O json > complexity-before.json
```

**How to interpret**:
- **Cyclomatic Complexity**: Measures decision points (if/match/loop). Target: <15 per function.
- **Cognitive Complexity**: Measures mental effort to understand. Target: <20 per function.
- Compare before/after metrics to prove refactoring reduced complexity.

**Cost**: ~30s for perl-parser. High noise if used for absolute scores.

**Warning**: Do NOT use absolute scores as quality gates. Only use for delta analysis (before vs after).

#### cargo-bloat

**When to use**:
- Release audits to understand binary size contributors
- Investigating unexpected binary size growth
- Optimizing for embedded or size-constrained targets

**Command**:
```bash
cargo bloat --release -p perl-lsp --crates
cargo bloat --release -p perl-lsp -n 50  # Top 50 symbols
```

**How to interpret**:
- Shows size contribution by crate and symbol
- Look for unexpectedly large dependencies
- Identify monomorphization bloat from generics

**Cost**: ~1-2 min (requires release build).

#### cargo-udeps

**When to use**:
- Periodic dependency cleanup (quarterly)
- Before releases to minimize attack surface
- After removing features or refactoring

**Command**:
```bash
cargo +nightly udeps --workspace
```

**How to interpret**:
- Lists dependencies that aren't actually used
- May have false positives for target-specific deps
- Verify before removing (check if used in tests, examples, benches)

**Cost**: ~30-60s. Requires nightly toolchain.

**Warning**: Can miss dependencies used only in cfg-gated code. Always test after removing deps.

#### cargo-outdated

**When to use**:
- Planning dependency update sprints
- Security reviews (find dependencies with known CVEs)
- Quarterly maintenance

**Command**:
```bash
cargo outdated --workspace
cargo outdated --workspace --root-deps-only  # Only direct deps
```

**How to interpret**:
- Shows current, latest compatible, and latest versions
- Color-coded by severity (red = major behind)
- Cross-reference with `cargo audit` for security implications

**Cost**: ~10-20s.

#### cargo-expand

**When to use**:
- Debugging procedural macros
- Understanding derive macro expansions
- Troubleshooting macro hygiene issues

**Command**:
```bash
cargo expand --package perl-parser --lib
cargo expand --package perl-parser parser::expression  # Specific module
```

**How to interpret**:
- Shows fully expanded Rust code after macro processing
- Useful for understanding what generated code looks like
- Can be verbose; pipe through `less` or save to file

**Cost**: ~5-10s per module.

#### cargo-depgraph

**When to use**:
- Visualizing dependency relationships
- Finding duplicate dependencies in the tree
- Understanding why a specific crate is included

**Command**:
```bash
cargo depgraph --workspace-only | dot -Tpng > depgraph.png
cargo tree -d  # Text-based duplicate detection
```

**How to interpret**:
- Visual graph of crate dependencies
- Identify opportunities to consolidate versions
- Find unexpected transitive dependencies

**Cost**: ~5s. Requires graphviz.

### Script and Config Linters

| Tool | Purpose | When to run | Installation |
|------|---------|-------------|--------------|
| **shellcheck** | Shell script linting | Before committing scripts/*, forensics audits | `apt install shellcheck` or `brew install shellcheck` |
| **actionlint** | GitHub Actions workflow linting | Before committing .github/workflows/* changes | Download from [actionlint releases](https://github.com/rhysd/actionlint/releases) |
| **typos** | Typo detection in docs and code | Before releases, documentation updates | `cargo install typos-cli` |

#### shellcheck

**When to use**:
- Before committing any script in `scripts/`
- Especially critical for `scripts/forensics/*` (they're self-referential)
- When debugging shell script issues

**Command**:
```bash
shellcheck scripts/*.sh
shellcheck scripts/forensics/*.sh
find scripts -name "*.sh" -exec shellcheck {} +
```

**How to interpret**:
- Reports common shell pitfalls (unquoted variables, missing error checks)
- Each issue has a SC#### code with explanation
- Fix or add `# shellcheck disable=SC####` with justification

**Cost**: <1s per script. Very low overhead.

**Why not gate-blocking**: Minor style issues shouldn't block development. Run before forensics-critical changes.

#### actionlint

**When to use**:
- Before committing changes to `.github/workflows/*`
- When debugging workflow failures
- When adding new CI jobs

**Command**:
```bash
actionlint
actionlint .github/workflows/ci.yml
```

**How to interpret**:
- Validates workflow syntax and common mistakes
- Catches typos in action names, invalid shell commands, missing required fields
- Provides line numbers and clear error messages

**Cost**: <2s. Very fast.

**Why not gate-blocking**: Workflows are already validated by GitHub on push. This is for early feedback.

#### typos

**When to use**:
- Before releases
- After documentation updates
- When adding new public APIs (catches identifier typos)

**Command**:
```bash
typos
typos --write-changes  # Auto-fix
typos docs/
typos --format brief
```

**How to interpret**:
- Reports potential typos in files
- Can produce false positives on domain terms (add to `.typos.toml`)
- Useful for catching embarrassing typos before public release

**Cost**: <5s for full repo scan.

**Why not gate-blocking**: High false positive rate on technical terms. Useful for polish, not correctness.

**Configuration**: Create `.typos.toml` to add project-specific dictionary:
```toml
[default.extend-words]
perl-lsp = "perl-lsp"  # Don't flag our project name
tokei = "tokei"        # Tool names
```

---

## 5. Forensics Tools

For PR archaeology and dossier generation.

| Tool | Script | Purpose |
|------|--------|---------|
| **pr-harvest** | `scripts/forensics/pr-harvest.sh` | Pull PR facts |
| **temporal-analysis** | `scripts/forensics/temporal-analysis.sh` | Commit topology |
| **telemetry-runner** | `scripts/forensics/telemetry-runner.sh` | Base->head deltas |
| **render-dossier** | `scripts/forensics/render-dossier.sh` | Generate full PR dossier |

### pr-harvest

**Purpose**: Extract structured facts from a merged PR for forensics analysis.

**Command**:
```bash
./scripts/forensics/pr-harvest.sh 259
./scripts/forensics/pr-harvest.sh 259 -o pr-259-facts.yaml
```

**Output**: YAML with PR metadata, commits, change surface, dependency delta, check runs.

**Requirements**: `gh`, `git`, `jq`

### temporal-analysis

**Purpose**: Analyze commit patterns to understand how a feature converged.

**Command**:
```bash
./scripts/forensics/temporal-analysis.sh origin/master..HEAD
./scripts/forensics/temporal-analysis.sh --pr 251
```

**Output**: YAML with sessions, friction heatmap, oscillations, convergence pattern.

**Metrics computed**:
- Session detection (gap > 30 min = new session)
- Grind detection (same files touched repeatedly)
- Friction score (commits * churn per file)
- Convergence pattern (linear, oscillating, chaotic)

### telemetry-runner

**Purpose**: Compute hard probe deltas between base and head commits.

**Command**:
```bash
./scripts/forensics/telemetry-runner.sh <base-sha> <head-sha>
./scripts/forensics/telemetry-runner.sh --pr 123
./scripts/forensics/telemetry-runner.sh --quick HEAD~5 HEAD
```

**Lane 1 metrics** (always available):
- `tokei`: LOC by module
- `cargo clippy`: warning count
- `cargo test`: test counts (passed/failed/ignored)
- `Cargo.toml`: dependency count

**Lane 2 metrics** (if tools installed):
- `cargo geiger`: unsafe block count
- `cargo tree -d`: duplicate dependencies

**Output**: YAML to stdout, artifacts to `./artifacts/telemetry/`

---

## 6. Security/Policy Tools

Optional but recommended for releases and security-sensitive changes.

| Tool | When | What it catches |
|------|------|-----------------|
| **gitleaks** | Releases | Secrets in history |
| **cargo deny** | If policy needed | License/ban violations |
| **semgrep** | If patterns needed | Path traversal, injection |

### cargo-audit (Vulnerability Scanning)

| Attribute | Value |
|-----------|-------|
| **Command** | `cargo audit` |
| **Lane** | Security/Policy |
| **Trigger** | Label `ci:audit`, releases, or dependency updates |
| **What it measures** | Known vulnerabilities in dependency tree |
| **Failure meaning** | Dependency has known CVE |
| **Installation** | `cargo install cargo-audit --locked` |

**How to interpret**:

| Finding | Meaning | Action |
|---------|---------|--------|
| **No advisories** | Dependencies secure | Continue |
| **Informational** | Low-impact advisory | Review and decide |
| **Warning** | Moderate vulnerability | Update or mitigate |
| **Error** | Critical vulnerability | Immediate update required |

**Cost/benefit**: ~5s. Low cost, should run frequently.

### cargo-deny (Supply Chain Policy)

| Attribute | Value |
|-----------|-------|
| **Command** | `cargo deny check` |
| **Lane** | Security/Policy |
| **Trigger** | Label `ci:strict`, releases |
| **What it measures** | License compliance, duplicate dependencies, banned crates, advisories |
| **Installation** | `cargo install cargo-deny --locked` |
| **Config** | `deny.toml` |

**Key sections in `deny.toml`**:
- **Licenses**: MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Unicode-3.0 allowed
- **Bans**: `multiple-versions = "warn"`, `wildcards = "allow"`
- **Advisories**: Default RustSec database
- **Sources**: crates.io only, no git sources

### gitleaks (Secrets Detection)

**When to use**: Before releases, after potential secret exposure.

**Command**:
```bash
gitleaks detect --source . --verbose
gitleaks detect --source . --log-opts="--all"  # Full history
```

**How to interpret**:
- Any finding requires investigation
- May need history rewrite if secrets found

**Status**: Aspirational - not yet integrated into CI

### semgrep

**When to use**: Custom security pattern detection.

**Command**:
```bash
semgrep --config=auto .
semgrep --config=p/rust .
```

**How to interpret**: Rule-dependent. Review each finding for applicability.

---

## 7. Tool Installation

### Always-On (included with rustup)

```bash
rustup component add rustfmt clippy
```

### Exhibit-Grade

```bash
cargo install cargo-audit cargo-semver-checks cargo-geiger cargo-llvm-cov cargo-mutants
```

### Research Tier

```bash
# Rust analysis tools
cargo install cargo-modules --locked
cargo install rust-code-analysis-cli --locked
cargo install cargo-bloat --locked
cargo install cargo-udeps --locked
cargo install cargo-outdated --locked
cargo install cargo-expand --locked
cargo install cargo-depgraph --locked

# Script and config linters
cargo install typos-cli --locked
# shellcheck: apt install shellcheck (or brew install shellcheck)
# actionlint: download from https://github.com/rhysd/actionlint/releases
```

### Full Installation

```bash
# Required for ci-gate
cargo install just

# Exhibit-grade analysis
cargo install cargo-audit --locked
cargo install cargo-semver-checks --locked
cargo install cargo-geiger --locked
cargo install cargo-llvm-cov --locked
cargo install cargo-mutants --locked
cargo install cargo-nextest --locked

# Forensics
cargo install tokei --locked
# Plus: gh, jq (from system packages)

# Research tier
cargo install cargo-modules --locked
cargo install rust-code-analysis-cli --locked
cargo install cargo-bloat --locked
cargo install cargo-udeps --locked
cargo install cargo-outdated --locked
cargo install cargo-expand --locked
cargo install cargo-depgraph --locked
cargo install typos-cli --locked
# shellcheck: apt install shellcheck (or brew install shellcheck)
# actionlint: download from https://github.com/rhysd/actionlint/releases

# Security
cargo install cargo-deny --locked
# gitleaks: download from https://github.com/gitleaks/gitleaks/releases
# semgrep: pip install semgrep
```

### Required Tools Verification

These **must** be installed for `just ci-gate` to work.

| Tool | Installation | Verification |
|------|--------------|--------------|
| `cargo` | Via rustup | `cargo --version` |
| `rustfmt` | Via rust-toolchain.toml | `cargo fmt --version` |
| `clippy` | Via rust-toolchain.toml | `cargo clippy --version` |
| `just` | `cargo install just` or Nix | `just --version` |
| `python3` | System package | `python3 --version` |

### Optional Tools (Graceful Degradation)

| Tool | Installation | Used By | Fallback |
|------|--------------|---------|----------|
| `cargo-nextest` | `cargo install cargo-nextest --locked` | Nix checks | Use `cargo test` |
| `cargo-deny` | `cargo install cargo-deny --locked` | Security lane | Skip deny checks |
| `cargo-audit` | `cargo install cargo-audit --locked` | Security audits | Skip audit |
| `cargo-semver-checks` | `cargo install cargo-semver-checks --locked` | API compat | Skip semver checks |
| `cargo-mutants` | `cargo install cargo-mutants --locked` | Mutation testing | Skip mutation analysis |
| `cargo-llvm-cov` | `cargo install cargo-llvm-cov --locked` | Coverage | Skip coverage |
| `tokei` | `cargo install tokei --locked` | LOC metrics | Manual counting |

---

## 8. Adding New Tools

Checklist before adding a tool to the stack:

### 8.1 Evaluation Questions

1. **What rot does it prevent?**
   - Name the specific failure mode it catches
   - Provide an example of a bug it would have caught

2. **What's the false positive rate?**
   - Run on current codebase
   - Count actionable vs noise findings
   - Target: <5% false positives for always-on

3. **What's the runtime cost?**
   - Measure on clean build
   - Measure on incremental build
   - Target: <30s for always-on

4. **Where does it fit?**
   - Always-on: <30s, <5% false positives, catches real bugs
   - Exhibit-grade: Higher cost/depth, selective application
   - Forensics: Post-hoc analysis only

5. **How do we interpret failures?**
   - Document clear pass/fail criteria
   - Document common false positives
   - Document fix patterns

### 8.2 Integration Steps

1. Add to appropriate section in this document
2. Add installation instructions to Section 7
3. If always-on, add to `ci-gate` in `justfile`
4. If exhibit-grade, document selection criteria
5. Update CLAUDE.md quick reference if needed

### 8.3 Removal Criteria

Remove a tool from always-on if:
- False positive rate exceeds 10%
- Runtime exceeds 60s
- Duplicates another tool's coverage
- No bugs caught in last 3 months

---

## 9. See Also

- [QUALITY_SURFACES.md](QUALITY_SURFACES.md) - The four quality surfaces
- [ANALYZER_FRAMEWORK.md](ANALYZER_FRAMEWORK.md) - Specialist analyzers for deep forensics
- [DEVLT_ESTIMATION.md](DEVLT_ESTIMATION.md) - Decision-weighted budget estimation
- [MUTATION_TESTING_METHODOLOGY.md](MUTATION_TESTING_METHODOLOGY.md) - Mutation testing guide
- [CLAUDE.md](../CLAUDE.md) - Quick reference for development commands
- [CURRENT_STATUS.md](CURRENT_STATUS.md) - Computed metrics and project health

---

## Quick Reference

### Running Gates Locally

```bash
# Required before every push
nix develop -c just ci-gate

# Full CI (for large changes)
just ci-full

# MSRV validation (proves 1.89 compatibility)
just ci-gate-msrv
```

### Label-Triggered CI Jobs

| Label | What it runs |
|-------|--------------|
| `ci:mutation` | Mutation testing |
| `ci:bench` | Benchmarks |
| `ci:strict` | Strict clippy, doc hygiene |
| `ci:semver` | SemVer compatibility |
| `ci:coverage` | Code coverage |
| `ci:audit` | Security audit |
| `ci:determinism` | Test determinism |

### Health Check Commands

```bash
just health              # Show codebase metrics
just status-check        # Verify computed metrics are current
just bugs                # Show bug queue status
```

---

*Last Updated: 2026-01-07*

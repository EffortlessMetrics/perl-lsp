# Contributing to Perl LSP

Thank you for your interest in contributing to Perl LSP! This is a **public alpha** — the core feature set is solid, but there are rough edges and plenty of room to help. Whether you are fixing a bug, improving Perl parsing coverage, or adding an LSP feature, you are welcome here.

> **Public alpha means:** things move fast, APIs may change between minor versions, and your early feedback shapes the 1.0 design. See [STABILITY.md](docs/reference/STABILITY.md) for the stability policy.

## Quick Start

Clone, check the environment, and run the fast local gate:

```bash
git clone https://github.com/EffortlessMetrics/perl-lsp.git
cd perl-lsp
nix develop                   # Recommended: Rust 1.95.0 + all repo tools
just devex
just pr-fast
```

No Nix? Install Rust via [rustup](https://rustup.rs/) (MSRV 1.95, toolchain `1.95.0` pinned in `rust-toolchain.toml`), install `just`, then run the same commands:

```bash
cargo install just
just devex
just pr-fast
```

## Getting Started

### Prerequisites

- **Rust** toolchain (pinned via `rust-toolchain.toml`, MSRV 1.95, channel `1.95.0`)
- **Nix** (recommended, not required) for a fully reproducible dev environment — `nix develop` drops you into a shell with all tools present
- **just** — task runner used for all build/test/lint commands (`cargo install just` or via Nix)

### First-Time Setup Checklist

Run through these five steps once after cloning. Each step has a clear success signal — if you see something different, check the note below it.

**1. Clone and enter the repository**
```bash
git clone https://github.com/EffortlessMetrics/perl-lsp.git
cd perl-lsp
```

**2. Enter the dev environment**
```bash
nix develop          # Recommended: pins Rust 1.95.0 + all tools
# No Nix? Install rustup and then: cargo install just
```
Success: your shell prompt changes (or you see the nix shellHook banner listing available commands).

**3. Verify the environment**
```bash
just devex           # Required tools and Rust components
just doctor          # Workspace health, hooks, branch/worktree state
```
Success: every required check passes. If something fails, the output explains what's missing and how to install it. Optional-tool warnings are safe to skip for basic contribution work.

**4. Validate everything compiles and tests pass**
```bash
just pr-fast         # ~1-2 min
```
Success: ends with `PR-fast gate complete` and no `FAILED` lines.

**5. Install the pre-push git hook**
```bash
bash scripts/install-githooks.sh
```
Success: prints `Installed pre-push hook`. The hook runs `just pr-fast` automatically before every `git push`.

Once all five steps succeed, you're ready to make changes. Before pushing, run `just ready` to combine the workspace doctor with the fast PR gate.

### Setup

```bash
git clone https://github.com/EffortlessMetrics/perl-lsp.git
cd perl-lsp
nix develop          # Recommended: reproducible environment with all tools

# Or without Nix -- just ensure Rust and just are installed:
# curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# cargo install just
```

Install the pre-push git hook so the gate runs automatically before every push:

```bash
bash scripts/install-githooks.sh
```

### Agent Quickstart

If you are contributing from an agentic coding environment, use the repo's bounded build profiles so target directories and caches do not grow inside disposable worktrees:

```bash
just agent-preflight
just agent-check
just agent-test
just agent-clippy
just agent-pr-fast
```

For Codex Desktop specifically:

1. Open the repository root as the workspace.
2. Run checks through a POSIX shell (`bash`) even on Windows.
3. Prefer `just agent-check`, `just agent-test`, and `just agent-clippy` for large compile/test work.
4. Use `just pr-fast` or `just ready` before handing a PR to review.

```bash
# Fast inner-loop validation
just pr-fast

# Canonical pre-PR gate (matches merge expectations)
nix develop -c just ci-gate
```

If your desktop environment does not use Nix, run `just ci-gate` directly.
For stale toolchains or workspace drift, run `just devex` and `just doctor`.

### Build and Test

```bash
cargo build -p perl-lsp-rs --release  # Build the LSP server binary
cargo test --workspace --lib          # Run all library tests
```

If something looks broken, `just doctor` diagnoses common environment issues (missing tools, stale worktrees, drift between generated files and source):

```bash
just doctor
```

## Project Structure

The workspace contains many crates organized into families. Key crates:

| Crate | Purpose |
|-------|---------|
| `perl-parser` | Main parser (v3 recursive descent) |
| `perl-lsp-rs` | LSP server binary |
| `perl-dap` | Debug Adapter Protocol server |
| `perl-lexer` | Context-aware tokenizer |

Crate families: `perl-module-*` (module resolution), `perl-lsp-*` (LSP providers), `perl-dap-*` (DAP), `perl-workspace-*` (workspace discovery).

For the full crate map, key paths, and architecture details, see [CLAUDE.md](CLAUDE.md).

## Finding Issues to Work On

- Look for issues labeled **`good first issue`** for beginner-friendly tasks
- **`help wanted`** marks issues where maintainer input is available
- **`parser`** issues improve Perl parsing coverage
- **`lsp`** issues add or fix editor features
- Browse [open issues](https://github.com/EffortlessMetrics/perl-lsp/issues) or check the [roadmap](docs/project/ROADMAP.md) for larger goals

## Development Workflow

### 1. Branch

```bash
git checkout -b feature/your-feature-name
```

### 2. Check the environment (optional but useful)

```bash
just devex      # Verifies that required tools are available and in a good state
just doctor     # Deeper workspace health check — finds drift, stale files, config issues
```

### 3. Iterate locally

```bash
just pr-fast    # Fastest checks: fmt + clippy + tests (~1-2 min). Run this often.
```

### 4. Run the canonical merge gate

This is the explicit full validation step before merge. It runs the same checks CI runs:

```bash
nix develop -c just ci-gate   # Recommended: reproducible env (~3-5 min)
# or, without Nix:
just ci-gate
```

If you install the pre-push git hook, it runs the faster Tier A gate automatically on push:

```bash
bash scripts/install-githooks.sh
```

That hook runs `nix develop -c just pr-fast` (or `just pr-fast` without Nix).
It is a quick push guard, not the full merge gate.

### Command Decision Table

| Situation | Command | Why |
|---|---|---|
| New checkout | `just doctor` | Verifies workspace health, hooks, branch state, and common drift. |
| Tool/env check | `just devex` | Checks required tools, Rust components, and local setup. |
| Before push | `just ready` | Runs doctor plus the fast PR gate. |
| Fast PR loop | `just pr-fast` | Cheapest useful proof while iterating. |
| Agent compile/test | `just agent-check` / `just agent-test` | Uses cargo-safe agent profiles and bounded build directories. |
| Agent lint | `just agent-clippy` | Runs clippy through the cargo-safe agent profile. |
| Agent PR proof | `just agent-pr-fast` | Runs the PR-fast gate through cargo-safe. |
| Full pre-merge | `just ci-gate` or `nix develop -c just ci-gate` | Canonical local merge gate. |
| Memory lifecycle touched | `cargo xtask check-memory-lifecycle-policy` | Enforces retained-state lifecycle and receipt policy. |
| Retained owner added | `cargo xtask check-memory-retained-owner-drift --base origin/master` | Checks whether long-lived storage/task additions need retained-state inventory coverage. |
| Parser-accuracy metrics touched | `just ci-metrics-ratchet-check parser_accuracy` | Verifies parser-accuracy scorecard floors do not regress. |
| Generated status docs touched | `just status-update` then `just status-check` | Regenerates and verifies `docs/project/status/` outputs. |
| Release/version surfaces touched | `just version-check` then `just release-check` | Verifies version sync and the release-prep gate before tagging/publishing. |
| Need a command map | `just quick-ref` or [Commands Reference](docs/reference/COMMANDS_REFERENCE.md) | Shows the short command decision tree. |
| DevEx docs changed | `cargo xtask check-devex-docs` | Verifies toolchain wording and documented command references stay current. |

### 5. Expand for larger changes or release prep

```bash
just ci-full    # Full pipeline including mutation testing, fuzzing, benchmarks (~15-30 min)
```

### 5a. Verify your Cargo.toml changes won't break publishing

If your PR touches any `Cargo.toml` file (adding/removing deps, adding new crates, modifying
workspace metadata), a **publish dry-run gate** runs automatically in CI. It packages every
allowlisted crate in topological dependency order — the same order and with the same dev-dep
stripping logic used by the actual publish workflow — and fails loudly if any crate cannot be
packaged.

**Why this gate exists:** Two separate publish failures happened within a single session:
- A dev-dep ordering issue where `perl-corpus` dev-depends on `perl-tdd-support` but was
  sorted before it (fixed by Tarjan SCC in the topo sort, #3236).
- An intra-SCC dev-dep cycle where `perl-module-import` dev-depends on `perl-module-token`,
  causing `cargo package` to fail during manifest resolution (#3254).

Both bugs would have been caught by this gate before the breaking PR merged.

**If the gate fails**, your PR would break the `publish-crates.yml` workflow. Fix before merging.
Common causes: adding a dev-dep on an un-allowlisted crate, a normal-dep cycle, or removing a
crate without updating dependents.

**Run it locally:**

```bash
just publish-dry-run
```

This runs in ~5-10 minutes (packaging only, no uploading).

### 6. Keep docs and status in sync (only needed if you changed metrics or generated files)

```bash
just status-update
just status-check
```

If your change introduces or modifies public APIs, also run the documentation
coverage workflow so CI catches missing rustdoc before review:

```bash
just docs-check
just docs-report
```

See [docs/reference/MISSING_DOCUMENTATION_GUIDE.md](docs/reference/MISSING_DOCUMENTATION_GUIDE.md)
for remediation workflow and [docs/reference/API_DOCUMENTATION_STANDARDS.md](docs/reference/API_DOCUMENTATION_STANDARDS.md)
for required rustdoc structure.

### 7. Open a Pull Request

1. Push your branch and open a PR
2. Give the PR a CI-safe title in the form `type(scope): summary (#1234)` — the title format is validated by a CI workflow, so get it right the first time
3. Describe your changes and link related issues in the PR body
4. All PRs run format checks, clippy, and tests automatically in CI

Conventional subject format:

- `feat(scope): imperative summary`
- `fix(scope): imperative summary`
- `fix(scope)!: imperative summary` — include `!` before the colon for breaking changes
- `chore(scope): imperative summary`
- `docs(scope): imperative summary`
- `test(scope): imperative summary`

Example PR titles:

```text
fix(parser): handle here-doc inside ternary (#3052)
feat(lsp): add rename symbol provider (#2980)
docs(contributing): refresh for v0.13.0 public alpha (#3200)
```

Do not rely on PR title defaults (often noisy, e.g. `Merge branch ...` or the GitHub auto-fill), because they fail `validate-title` and break changelog generation.

#### What Happens After You Open a PR

PRs go through a two-pass review before merging:

1. **Standards review** (haiku-tier) — checks formatting, clippy compliance, test coverage, and scope
2. **Deep correctness review** (sonnet-tier) — checks logic, edge cases, and correctness for feature PRs

You will see pipeline labels added to your PR:

| Label | Meaning |
|-------|---------|
| `in-review` | A reviewer has picked up your PR |
| `needs-deep-review` | Standards review done, awaiting deep correctness pass |
| `reviewed-deep` | Both review passes complete |
| `merge-ready` | Approved and ready for merge |

The CI merge gate only runs on `merge-ready` PRs. This keeps the queue clean — do not worry if CI looks quiet on your draft.

#### External-claim verification

If a PR's description or commit message cites an external specification (Perl language semantics, LSP protocol spec, DAP protocol spec) or a third-party crate API, the review process **must** include running the claimed behavior against the reference implementation, not just reading the documentation. For Perl claims, a short `perl -e` snippet against the runtime is authoritative. For LSP/DAP claims, the published spec text is authoritative. For crate APIs, the current `docs.rs` entry is authoritative.

This rule exists because on 2026-04-11, [PR #4090](https://github.com/EffortlessMetrics/perl-lsp/pull/4090) was approved by multiple reviewers on the basis of a false claim about Perl phase-block pragma semantics that every reviewer had independently assumed was true. The false claim was caught only by a research-verifier agent running `perl -e 'BEGIN { use strict; } $x = 1'` and observing the script succeeds — proving `use strict` inside `BEGIN { }` stays lexically scoped to the block and is **not** active at file scope. The resulting revert is tracked in [#4100](https://github.com/EffortlessMetrics/perl-lsp/issues/4100) and the correct positive-direction lint in [#4101](https://github.com/EffortlessMetrics/perl-lsp/issues/4101). See also related process context in [#4062](https://github.com/EffortlessMetrics/perl-lsp/issues/4062).

For code review automation: the `research-verifier` agent should be invoked on any PR whose body references `perlmod`, `perlop`, `LSP 3.`, `DAP`, or a `docs.rs` URL. The `reviewer-deep` agent's definition at `.claude/agents/reviewer-deep.md` will be updated separately to reference this rule (tracked as a follow-up).

For more detail on the CI structure see [docs/project/CI.md](docs/project/CI.md) and [docs/project/CI_TEST_LANES.md](docs/project/CI_TEST_LANES.md).

## Coding Standards

These are the rules that CI enforces. The quick version: no panics, no unwraps, no debug prints in production code. Use `?` for error propagation and `Result<()>` in tests.

For full detail and rationale, see [CLAUDE.md](CLAUDE.md#coding-standards).

### Formatting and Linting

- Run `cargo xtask fmt` before every commit — `just ci-gate` will catch this, but the sooner the better
- Fix all `cargo clippy --workspace` warnings — clippy is treated as errors in CI
- Use [conventional commits](https://www.conventionalcommits.org/): `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, etc.
- Scope-qualify commit subjects: `feat(lsp): ...`, `fix(dap): ...`, `chore(release): ...`

### Banned in Production Code

These constructs cause panics or hide errors. They are checked by `ci-gate` and will fail CI:

| Banned | Use Instead |
|--------|-------------|
| `unwrap()`, `expect()` | `?`, `.ok_or_else()`, pattern matching |
| `panic!()`, `todo!()`, `unimplemented!()` | Return `Result` or `Option` |
| `dbg!()` | `tracing::debug!` |
| `std::process::exit()` | Only in `bin/` and `lifecycle.rs` |

In tests: use `Result<()>` returns or `perl_tdd_support::must` / `must_some` helpers instead of `unwrap()`.

### Style Preferences

- `.first()` over `.get(0)`
- `.push(char)` over `.push_str("x")` for single characters
- `or_default()` over `or_insert_with(Vec::new)`
- Avoid `.clone()` on `Copy` types

### Documentation Anti-Drift

Metrics in this project are **computed, not hand-edited**. Never put exact numeric claims (crate counts, test counts, percentages) in prose files. Link to [CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md) for live metrics instead.

## Testing Guidelines

- Place tests in `tests/` or inline with `#[cfg(test)]`
- Test both success and failure paths
- For parser changes, add edge case tests and run `just cpan-corpus-sweep` to check CPAN coverage

```bash
cargo test -p <crate>                          # Test a specific crate
cargo test -p perl-parser -- test_name --exact # Run an exact test
cargo nextest run                              # Fast parallel runner
```

For LSP tests, control threading to avoid flaky results:

```bash
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs -- --test-threads=2
```

See [COMMANDS_REFERENCE.md](docs/reference/COMMANDS_REFERENCE.md) for the full command catalog.

## SemVer and Breaking Changes

We follow [Semantic Versioning 2.0.0](https://semver.org/). Check for breaking changes before submitting PRs that modify public APIs:

```bash
just semver-check
```

If a breaking change is necessary:
1. Document it in the PR description with a migration guide
2. Label the PR with `breaking-change`
3. Coordinate with maintainers

See [STABILITY.md](docs/reference/STABILITY.md) for our API stability policy.

### Public API Surface Ratchet

The five user-facing facade crates (`perl-lsp-rs`, `perl-parser`, `perl-uri`, `perl-dap`, `perllsp`) have their public API surface locked in text baselines at `.ci/public-api-baselines/`. The nightly CI job fails if the surface changes without a baseline update.

When you intentionally add or remove items from a facade crate's public API:

1. Run `just public-api-update` to regenerate all 5 baselines.
2. Include the updated `.ci/public-api-baselines/*.txt` files in your PR.
3. In your PR description, describe what changed and why.
4. Add the `ci:public-api` label to trigger the surface check in CI.

The check uses `cargo public-api -p <crate> --simplified` (omits blanket-impl noise).

## Release Workflow

### Version Bump

All workspace crates inherit their version from `[workspace.package] version` in the root
`Cargo.toml`. To bump the version across the entire workspace in one command:

```bash
just bump-version 0.13.0
```

This updates every tracked version site in a single pass:
- `[workspace.package] version` in `Cargo.toml`
- All `[workspace.dependencies]` version fields in `Cargo.toml`
- `vscode-extension/package.json` (and `package-lock.json` if present)
- `features.toml` `[meta] version`
- Documentation references in `README.md`, `CLAUDE.md`, and `docs/project/ROADMAP.md`

Individual crate `Cargo.toml` files use `version.workspace = true` and pick up the new
version automatically — they are not touched by the bump script.

After running, review the diff (`git diff`), commit, push, and open a PR.

### Release Sequence

Once the version-bump PR is merged:

1. Tag the release on `master`:
   ```bash
   git tag v0.13.0
   git push origin v0.13.0
   ```
2. Create a GitHub Release from the tag — this triggers the publish workflow automatically.
3. The publish workflow validates that every workspace crate reports the tag version, then
   publishes them to crates.io in topological dependency order.

### Verify Version Consistency

At any time, verify that all version sites agree with the workspace canonical version:

```bash
just version-check
```

This runs as part of `just release-gate` before cutting a release.

## Updating Demo GIFs

The three animated GIFs shown in `README.md` live in `docs/assets/gifs/`. They
are produced from manual screen recordings, not generated automatically.

### When to Re-Record

Re-record a GIF when:
- A menu label, key binding, or status bar message changes.
- The workflow changes enough that the current GIF is misleading.
- The GIF is blurry or hard to read at 960 px wide.

### Recording Process

1. Open `demo_workspace/` in VS Code with the `EffortlessMetrics.perl-lsp-rs`
   extension active and the LSP server running.
2. Set a clean theme (large font, minimal panels visible).
3. Record the interaction using your platform screen-capture tool:
   - Linux: `peek`, `simplescreenrecorder`, or `ffmpeg -f x11grab`
   - macOS: QuickTime Player or ScreenFlow
   - Windows: Xbox Game Bar, OBS Studio, or ShareX
4. Save the raw recording to `docs/assets/recordings/` (gitignored).

The full step-by-step script for each GIF is in
[`docs/assets/gifs/README.md`](docs/assets/gifs/README.md).

### Rendering

After capturing, convert to a compressed GIF:

```bash
python scripts/marketing/render-walkthrough-gif.py \
  --input docs/assets/recordings/goto-definition.mp4 \
  --output docs/assets/gifs/goto-definition.gif \
  --fps 12 \
  --width 960 \
  --max-bytes 3145728
```

Use `--start` and `--duration` to trim dead time. Run `--help` for all options.
Requires `ffmpeg`; `gifsicle` is used automatically if available.

### GIF Inventory

| File | Workflow | Max size |
|------|---------|---------|
| `docs/assets/gifs/install-health.gif` | VS Code install, auto-download, `perllsp --health` | 3 MB |
| `docs/assets/gifs/goto-definition.gif` | Ctrl+Click go-to-def, Find All References | 3 MB |
| `docs/assets/gifs/extract-variable.gif` | Select, light-bulb, Extract Variable | 3 MB |

### Commit Message Convention

```
docs: re-record goto-definition gif for v0.13 navigation changes
```

## Adding New Crates

1. Create the crate under `crates/` using the naming convention of its family
2. Add it to the workspace `members` in the root `Cargo.toml`
3. Follow the structure of a sibling crate in the same family
4. Run `nix develop -c just ci-gate` to verify, and `just ci-full` for larger
   workspace-impacting changes

## Getting Help

Use the right channel for the fastest response:

| Channel | Use for |
|---------|---------|
| [GitHub Discussions - Q&A](https://github.com/EffortlessMetrics/perl-lsp/discussions/categories/q-a) | Editor setup, configuration, how-to questions |
| [GitHub Discussions - Ideas](https://github.com/EffortlessMetrics/perl-lsp/discussions/categories/ideas) | Feature brainstorming before opening a formal issue |
| [GitHub Discussions - Show & Tell](https://github.com/EffortlessMetrics/perl-lsp/discussions/categories/show-and-tell) | Configs, workflows, and integrations to share |
| [GitHub Issues](https://github.com/EffortlessMetrics/perl-lsp/issues) | Bug reports and confirmed feature requests |

> Note: Discussions must be enabled in repository settings before the links above are active.
> See [#2169](https://github.com/EffortlessMetrics/perl-lsp/issues/2169) for the tracking issue.

- **Docs**: See `docs/` for detailed guides -- start with [COMMANDS_REFERENCE.md](docs/reference/COMMANDS_REFERENCE.md)
- **Verification policy**: See [VERIFICATION_LADDER.md](docs/contributing/VERIFICATION_LADDER.md) for claim verification requirements

## Code of Conduct

We follow the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md). Please be respectful and constructive in all interactions.

## License

This project is dual-licensed under [MIT](LICENSE-MIT) and [Apache-2.0](LICENSE-APACHE). By contributing, you agree that your contributions will be licensed under both licenses.

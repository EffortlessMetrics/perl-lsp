# Contributing to Perl LSP

Thank you for your interest in contributing to Perl LSP! Whether you're fixing a bug, improving the parser, or adding an LSP feature, this guide will help you get started.

## Getting Started

### Prerequisites

- **Rust** toolchain (pinned via `rust-toolchain.toml`, MSRV 1.92)
- **Nix** (recommended) for a reproducible dev environment

### Setup

```bash
git clone https://github.com/EffortlessMetrics/perl-lsp.git
cd perl-lsp
nix develop          # Recommended: reproducible environment

# Or without Nix -- just ensure Rust is installed:
# curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Build and Test

```bash
cargo build -p perllsp --release      # Build the public LSP binary
cargo test --workspace --lib          # Run all library tests
```

## Project Structure

The workspace contains many crates organized into families. Key crates:

| Crate | Purpose |
|-------|---------|
| `perl-parser` | Main parser (v3 recursive descent) |
| `perl-lsp` | LSP server binary |
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

### 2. Check the environment

```bash
just devex
```

### 3. Iterate locally

```bash
just pr-fast
```

### 4. Run the canonical pre-push gate

```bash
nix develop -c just ci-gate
# or, without Nix:
just ci-gate
```

### 5. Expand for larger changes or release prep

```bash
just ci-full
```

### 6. Keep docs and status in sync

```bash
just status-update
just status-check
```

### 7. Before a release candidate

```bash
just release-check
```

Install the pre-push hook to run the gate automatically:

```bash
bash scripts/install-githooks.sh
```

### 5. Open a Pull Request

1. Push your branch and open a PR
2. Give the PR a CI-safe title in the form `type(scope): summary (#1234)` so
   the title check passes on the first run
3. Describe your changes and link related issues in the PR body
4. All PRs run format checks, clippy, and tests automatically in CI
5. Merge with a conventional, descriptive squash commit

Once checks are green and reviews are complete, use an explicit conventional commit
subject when squashing so history is release-friendly and changelog-friendly.

```bash
pr=<number>
gh pr merge "$pr" --squash \
  --subject "feat(lsp): ... " \
  --body "PR summary:
- ...
```

Conventional subject format:

- `feat(scope): imperative summary`
- `fix(scope): imperative summary`
- `chore(scope): imperative summary`
- include `!` for breaking changes, e.g. `feat!: ...`

Do not rely on PR title defaults (often noisy, e.g. `(...#NNNN)`), because they
break commit consistency for changelog generation and can fail `validate-title`.

Example PR title:

```text
docs(parser): rewrite the upgrade guide (#3052)
```

If you are driving the work from an agent or a worktree, keep the branch scoped
to one concern and open the PR from that isolated worktree instead of editing
the main checkout.

#### CI Labels and Gates

The current PR smoke, merge gate, and label-gated workflows are documented in
[docs/project/CI.md](docs/project/CI.md) and
[docs/project/CI_TEST_LANES.md](docs/project/CI_TEST_LANES.md).
If you change docs or generated status output, run `just status-update` and
`just status-check` before opening the PR.

## Coding Standards

### Formatting and Linting

- Run `cargo fmt --all` before every commit
- Fix all `cargo clippy --workspace` warnings
- Use [conventional commits](https://www.conventionalcommits.org/): `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, etc.
- For squash merges, prefer scope-qualified forms such as
  `feat(lsp): ...`, `fix(dap): ...`, `chore(release): ...`, etc.

### Banned in Production Code

| Banned | Use Instead |
|--------|-------------|
| `unwrap()`, `expect()` | `?`, `.ok_or_else()`, pattern matching |
| `panic!()`, `todo!()`, `unimplemented!()` | Return `Result` or `Option` |
| `dbg!()` | `tracing::debug!` |
| `std::process::exit()` | Only in `bin/` and `lifecycle.rs` |

In tests: use `Result<()>` returns or `perl_tdd_support::must` / `must_some` helpers.

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
RUST_TEST_THREADS=2 cargo test -p perl-lsp -- --test-threads=2
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

## Code of Conduct

We follow the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md). Please be respectful and constructive in all interactions.

## License

This project is dual-licensed under [MIT](LICENSE-MIT) and [Apache-2.0](LICENSE-APACHE). By contributing, you agree that your contributions will be licensed under both licenses.

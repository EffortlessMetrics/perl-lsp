# Your First PR

Five minutes to read. Then clone and go.

## 1. Clone

```bash
git clone https://github.com/EffortlessMetrics/perl-lsp.git
cd perl-lsp
```

No Git submodules to initialize. A normal clone is enough; `--recurse-submodules` is optional
and has no effect for this repo.

You can verify this at any time:

```bash
git submodule status
```

Expected output is empty.

The `tree-sitter-perl/` directory is checked-in legacy C code (not a submodule) and is excluded
from the default Rust build, so you can ignore it unless you are working on parser migration tasks.

## 2. Set up the dev environment

**With Nix (recommended — fully reproducible):**

```bash
nix develop
```

**Without Nix — install Rust and `just`, then you are ready:**

```bash
# Install Rust if you don't have it
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install just (task runner)
cargo install just

# Verify tools are present
just doctor
```

Rust toolchain is pinned in `rust-toolchain.toml` (MSRV 1.92). `rustup` picks it up automatically.

**Windows users:** Enable long path support before building (one-time, requires admin terminal):

```
reg add HKLM\SYSTEM\CurrentControlSet\Control\FileSystem /v LongPathsEnabled /t REG_DWORD /d 1 /f
```

Without this, `cargo fmt` may fail with "os error 206" from deep worktree paths. Start a new terminal after setting the key.

Install the pre-push hook so the fast gate runs before every push:

```bash
bash scripts/install-githooks.sh
```

## 3. Find a good first issue

```bash
gh issue list --label "good-first-issue" --state open
```

Good candidates are labeled `size/S` or `size/M`, have a clear acceptance criteria section, and
do not require swarm or architectural context. The issue body will tell you which file to edit.

If you are not sure which issue to pick, read a few. The ones with "Files Affected" or "Root
Cause" sections are the easiest to get started on.

## 4. Build and run the test for your crate

Run ONLY the tests for the crate you are changing, not the full workspace — the workspace has
134 crates and full-workspace runs take minutes. The fast cycle is:

```bash
# Build to confirm it compiles
cargo build -p <crate-name>

# Run just that crate's tests
cargo test -p <crate-name>

# Run one specific test (faster iteration)
cargo test -p <crate-name> -- test_name_here --exact
```

For example, if you are fixing `perl-module-resolution`:

```bash
cargo test -p perl-module-resolution
```

For LSP crates, use threading flags to avoid flaky results:

```bash
RUST_TEST_THREADS=2 cargo test -p perl-lsp -- --test-threads=2
```

## 5. Make your change

Edit, add your test first (TDD is preferred), then implement:

1. Write a test that fails for the right reason.
2. Make the minimal change to pass it.
3. Run the crate test again to confirm green.

**Banned in production code** (CI will catch these):

| Banned | Use instead |
|--------|-------------|
| `unwrap()`, `expect()` | `?`, `.ok_or_else()`, pattern matching |
| `panic!()`, `todo!()`, `unimplemented!()` | Return `Result` or `Option` |
| `dbg!()` | `tracing::debug!` |

In tests: use `Result<()>` return type or `perl_tdd_support::must` / `must_some` helpers instead
of `unwrap()`.

## 6. Verify locally

```bash
just pr-fast
```

This runs `cargo fmt`, `cargo clippy`, and the crate tests in about 1-2 minutes. Fix anything it
reports. If all green, you are ready to push.

## 7. Open a PR

```bash
git checkout -b fix/my-description
git add <files>
git commit -m "fix(scope): what you changed (#NNNN)"
git push -u origin fix/my-description
gh pr create
```

**PR title convention** (enforced by CI — get it right or the `validate-title` check fails):

```
type(scope): imperative summary (#NNNN)
```

The `(#NNNN)` at the end is the issue number. Required. Examples:

```
fix(perl-module-resolution): normalize path separators in use_lib tests (#4154)
docs(perl-lsp-semantic-tokens): correct token counts in CLAUDE.md (#4159)
test(perl-parser): add regression test for postfix deref chain (#4167)
```

Types: `feat`, `fix`, `docs`, `test`, `refactor`, `chore`.

## 8. What reviewers look for

PRs go through two review passes:

1. **Standards review** — fmt, clippy compliance, no banned constructs, test coverage, scope
2. **Deep review** — logic correctness, edge cases (feature PRs)

The review bot will add labels (`in-review`, `needs-deep-review`, `reviewed-deep`, `merge-ready`)
as your PR progresses. You do not need to do anything except respond to comments.

If the reviewer pushes a fix directly to your branch, that is normal. Check and approve it.

## Reference

| Need | Command |
|------|---------|
| Build LSP server | `cargo build -p perl-lsp-rs --release` |
| Run all tests | `cargo test --workspace --lib` |
| Format | `cargo xtask fmt` |
| Lint | `cargo clippy --workspace` |
| Full merge gate | `just ci-gate` |
| Environment check | `just doctor` |
| All commands | [docs/reference/COMMANDS_REFERENCE.md](../reference/COMMANDS_REFERENCE.md) |
| Coding standards | [CLAUDE.md — Coding Standards](../../CLAUDE.md#coding-standards) |
| Full contributor guide | [CONTRIBUTING.md](../../CONTRIBUTING.md) |

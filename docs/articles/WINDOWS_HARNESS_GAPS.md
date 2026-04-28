# Five Windows Harness Gaps in One Session

**Date**: 2026-04-19
**Session**: Wave G1 collapse on perl-lsp (Windows 11 orchestrator)
**Cross-references**: issue #4514 (harness-to-xtask plan), #4456 (MAX_PATH), #4342 (main-branch switch), #4509 (task-tool persistence), #4512 (hook package-name inference)

---

## TL;DR

perl-lsp's agent harness (`.claude/hooks/*.sh`, `.git/hooks/*`, the task-tool backend) was built with Linux conventions. The 2026-04-19 session ran on a Windows 11 orchestrator and surfaced five distinct platform-specific bugs in one 11-hour sitting. Each had a workaround; none is individually alarming. Collectively, they describe a brittle layer of shell scripts and OS-specific assumptions that would benefit from migration into `xtask` (Rust, cross-platform, testable).

---

## The Five

### 1. Pre-push hook infers package name from directory basename

`.git/hooks/pre-push` (via `cargo xtask fmt --check`) invokes `cargo fmt -p <inferred>`, where `<inferred>` is the basename of the crate directory. For `crates/perl-lsp-rs/` this gives `-p perl-lsp` — but the actual package name in `Cargo.toml` is `perl-lsp-rs`. The command fails; the hook rejects the push; two builders in one session bypassed with `--no-verify`.

Every other crate in the workspace has dir-basename matching package-name. The one mismatch was enough to train two agents to `--no-verify`. Tracked as issue #4512. Fix: read the package name via `cargo metadata --no-deps` instead of inferring from the directory.

### 2. `archive/` paths exceed Windows MAX_PATH when nested

`git worktree add` creates a full tree copy at `.claude/worktrees/agent-<id>/`. The project has an `archive/` directory with deeply nested historical crates — paths like `archive/crates/tree-sitter-perl-rs/crates/tree-sitter-perl-rs/benchmark_results/...` that are ~240 characters long at the project root. Inside a worktree, the path becomes ~290 characters and exceeds the 260-char MAX_PATH limit.

Three worktree creations failed outright in this session. Workaround: manually create worktrees at short external paths (e.g. `C:\wt4501`) with `git sparse-checkout set` to exclude `archive/`. This worked — the agent operated inside the sparse tree and never touched `archive/` — but required hand-crafted setup the orchestrator had to remember. Tracked as issue #4456.

### 3. Orchestrator shell pwd drifts into nested worktrees

Each `Agent(isolation: "worktree")` call creates a worktree for the subagent, not for the orchestrator. But the orchestrator's bash shell pwd somehow accumulates the nested paths — at one point in the session, the orchestrator's pwd was `.../worktrees/agent-A/.claude/worktrees/agent-B/.claude/worktrees/agent-C/.claude/worktrees/agent-D`. Each new `git worktree add` attempt from that pwd failed with "cannot create directory … Filename too long."

Workaround: `cd /h/Code/Rust/perl-lsp` periodically to reset the orchestrator shell. Captured as memory: `feedback_nested_worktree_main_switch.md`.

### 4. Non-isolated agents switch the main checkout branch

When an agent runs without `isolation: worktree` and does `git checkout <branch>` on the main tree, it switches the main's branch. The next agent expects master and finds a different branch. Two incidents in this session — once the main was on `impl/4497-public-api-ratchet`, once on `temp-fmt`. Tracked as issue #4342 (pre-existing, reproduced this session).

Workaround: `git worktree list | head -1` to verify main is on master before each routing decision. If not, `git checkout master`.

### 5. Task-tool `TaskUpdate` reports success but state doesn't persist

Not Windows-specific per se, but manifested on a Windows orchestrator. `TaskUpdate` reported `"Updated task #N status"` on every call, but subsequent `TaskGet` returned the old value. Happened on status transitions and subject/description edits equally. The hook ruled itself out via inspection (would have exited non-zero on rejection; didn't). Likely a buffering or race in the harness backend. Tracked as issue #4509.

Workaround: stopped using the task tool as authoritative state; relied on GitHub labels + `git log` instead. The tool still served as scratchpad but its display was out of sync with reality by the end of the session.

## The Common Thread

Four of the five live in shell scripts or shell-invoked commands:
- Hook basename inference (#1)
- MAX_PATH tolerance (#2 — would be same at OS level, but the `archive/` path layout is the proximate cause)
- Pwd drift (#3 — shell state management)
- Branch switch on main tree (#4 — `git checkout` on the shared tree)

The fifth (#5) is harness-backend, not shell — but the fact that it was hard to diagnose from shell-level observations suggests the debugging loop for harness bugs has poor Windows instrumentation.

## Why `xtask` Would Help

`xtask` in Rust projects is a pattern: a small CLI binary living alongside the workspace that handles repo-level tooling (CI invocations, benchmarks, release automation, etc.). The advantages over shell scripts for harness logic:

- **Cross-platform by construction.** Rust's `std::path::Path` handles Windows/POSIX separators uniformly. `std::process::Command` works the same on both. MAX_PATH handling can be explicit via `std::path::PathBuf::components().count()` or library helpers.
- **Testable.** `xtask/tests/` can include fixture workspaces and integration tests against synthetic scenarios. Shell scripts are much harder to test in CI.
- **Uses `cargo_metadata` crate directly.** No basename inference; read `Cargo.toml` via the actual metadata API. Fixes #1 structurally.
- **Error handling.** `Result<T, E>` propagation is clearer than shell `exit 2` with no type information. Unit-testable error paths.
- **Shared types across hooks.** A `pre-push`, `pre-commit`, and `post-merge` hook can share data structures and helpers, rather than each reimplementing YAML parsing or Cargo.toml reading.

The shim remains: `.git/hooks/pre-push` becomes a two-line script that calls `cargo xtask pre-push`. The logic lives in Rust.

## What's Tracked

- **#4514** — plan: migrate `.claude/hooks/*.sh` logic into `xtask` (systemic, this article's direct ask)
- **#4512** — fix: pre-push hook uses `cargo_metadata` instead of basename (tactical, enables #4514's first phase)
- **#4511** — cosmetic: rename `crates/perl-lsp-rs/` → `crates/perl-lsp-rs/` to eliminate the one dir-vs-package mismatch (cosmetic, supplements #4512)
- **#4456** — MAX_PATH audit (existing, ongoing)
- **#4342** — main-branch switch prevention (existing)
- **#4509** — task-tool persistence (harness-backend, likely outside xtask's reach)

## Why It's Worth Naming

Individual platform bugs are boring. But **five distinct ones in one session** is a pattern. The pattern says: the harness was built with a specific OS in mind, and each new OS exposes gaps. The systemic fix — consolidate into `xtask` — is a one-time investment that makes future platform gaps easier to catch and fix (unit tests in `xtask/tests/`) rather than relying on session-level manual workaround discovery.

## Related

- Full session: [../forensics/2026-04-19-wave-g1-collapse-retrospective.md](../forensics/2026-04-19-wave-g1-collapse-retrospective.md) §5
- [COST_ROI.md](COST_ROI.md) — ongoing cost accounting
- Memory: `feedback_harness_to_xtask.md`, `feedback_nested_worktree_main_switch.md`

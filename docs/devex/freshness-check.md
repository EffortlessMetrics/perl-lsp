# Freshness check

Two surfaces protect against the silent **stale-checkout failure mode** —
code-state claims (issue filings, implementation maps, builder briefs)
made against a local checkout behind `origin/master` without warning.

The 2026-05-11 #8485 incident is the canonical case: an orchestrator read
`crates/perl-lsp-rs/src/runtime/language/completion.rs` from a checkout 5
commits behind master, claimed a function did not exist when it did, and
that false claim framed eight issues before architectural review caught it.

Tracking: **#8546**.

---

## Surface 1 — `cargo xtask freshness-check`

Repo-native. Usable by humans, codex, factory-droid, aider, CI preflight
scripts, and any non-Claude agent.

### Command shape

```bash
cargo xtask freshness-check                                            # default: warn mode, base=origin/master
cargo xtask freshness-check --base origin/master
cargo xtask freshness-check --mode warn                                # exit 0 even when stale
cargo xtask freshness-check --mode block                               # exit 1 when stale
cargo xtask freshness-check --json target/devex/freshness.json         # write receipt
cargo xtask freshness-check --allow-historical --reason "reproducing v0.13.4 behavior"
```

### JSON receipt

```json
{
  "schema_version": 1,
  "base_ref": "origin/master",
  "head": "abc123",
  "base_head": "def456",
  "behind_by": 5,
  "fetch_age_seconds": 3600,
  "worktree_dirty": false,
  "safe_for_code_state_claims": false,
  "mode": "warn"
}
```

### Exit code

- `0` — always, in warn mode
- `0` — in block mode when `safe_for_code_state_claims == true`
- `1` — in block mode when stale (and `--allow-historical` not passed)

### Logic

- Identify default branch via `git symbolic-ref refs/remotes/origin/HEAD`.
- `git fetch origin <base>` unless `--no-fetch` passed.
- `behind_by` = `git rev-list --count HEAD..origin/<base>`.
- `fetch_age_seconds` from `.git/FETCH_HEAD` mtime.
- `safe_for_code_state_claims = (behind_by == 0)`.
- `--allow-historical` bypasses block mode but requires `--reason` (recorded in
  the receipt as `bypass_reason`).

---

## Surface 2 — Claude pre-tool hook

Catches stale reads before `Read` / `Grep` / `Edit` operations in Claude
Code sessions. Delegates to `cargo xtask freshness-check` so the staleness
logic has one implementation.

### Files

- `scripts/githooks/warn-stale-checkout.sh` — invokes the xtask, parses
  the JSON receipt, emits a one-line warning to stderr when
  `safe_for_code_state_claims == false`.
- `.claude/settings.local.json` — hook registration.

### Registration

```json
{
  "hooks": {
    "beforeRead": ["scripts/githooks/warn-stale-checkout.sh"]
  }
}
```

### Configurability

`.claude/settings.local.json` may override:

| Key | Default | Effect |
|---|---|---|
| `staleCheckoutWarningThreshold` | `1800` | Min seconds since fetch before warning |
| `staleCheckoutMaxBehind` | `1` | Min commits behind to trigger warning |
| `staleCheckoutWarnMode` | `"warn"` | `"warn"` or `"block"` |

### Behavior

- Does NOT fire on uncommitted working-tree files (no commit history to be
  stale relative to).
- Does NOT fire on a non-master branch by design — staleness is relative
  to that branch's own upstream, not master.
- Does NOT fire when `--allow-historical` was last invoked recently in
  this worktree (cached in `.git/devex/freshness-bypass`).

---

## Issue-scout workflow

Scouts (`scout`, `scout-parser`, `scout-lsp`, `scout-dap`) MUST run
`cargo xtask freshness-check --mode block` (or rely on the Claude hook)
before filing an issue with code-state claims. Code-state claims include:

- Naming a function, struct, or trait as the location of a bug.
- Quoting a line of code or a file path that the issue body asserts is
  "current state."
- Writing an implementation map that names existing symbols to modify.

The receipt JSON should be linked from the issue body (or pasted as a
collapsed `<details>` section) when the issue is filed via an automated
pipeline.

---

## Bypass

For intentional historical work (reproducing past behavior, debugging an
older release), use:

```bash
cargo xtask freshness-check --allow-historical --reason "..."
```

The `--reason` is logged in the receipt. If invoked from a scout filing
an issue, the reason should appear in the issue body.

---

## Related

- **#8546** — tracking issue with the full acceptance criteria.
- **`feedback_stale_checkout.md`** (orchestrator memory) — the upstream
  behavioral rule.
- **`feedback_issue_correction_record.md`** + **#8554** — downstream
  remediation rule for when a stale claim slips through anyway.

## Claim boundary

This document describes the freshness-check surfaces, not their
implementations. The implementation lives in `xtask/src/freshness_check.rs`
(once built) and `scripts/githooks/warn-stale-checkout.sh` (once built).
This doc is the spec; the code is the proof.

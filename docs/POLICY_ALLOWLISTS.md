# Policy allowlists

Policy allowlists are receipts for intentionally retained risk. They are not a
shortcut around the default rule that Rust and `xtask` are the preferred
implementation surfaces and that unchecked panic paths should be burned down.

## Rust 1.95 rollout target

The Rust 1.95 / 0.14.0 rollout map is
[`docs/ci/perl-lsp-rust-1.95-rollout.md`](ci/perl-lsp-rust-1.95-rollout.md). The
planned policy ledgers are staged so each PR tightens one concern at a time:

- Clippy lints and exceptions stay governed by
  [`policy/clippy-lints.toml`](../policy/clippy-lints.toml) and future exception
  ledgers.
- No-panic allowlists and baselines should use exact counted matching before any
  baseline reset.
- Non-Rust file allowlists should identify the owner, reason, surface,
  classification, coverage, creation date, and review date for each entry.
- Companion ledgers for generated, executable, dependency, workflow, process, and
  network surfaces should describe risky behavior separately from where files are
  allowed to exist.

## Allowlist hygiene

Every allowlist entry should answer these questions:

- What exact path, glob, finding, or behavior is allowed?
- Who owns the exception?
- Why is the exception retained?
- What gate, test, document, or operational control covers it?
- When should it be reviewed or expired?

Broad globs need an explicit broad-glob reason. Absolute paths and backslash paths
should be rejected because policy ledgers must be portable across Linux, macOS,
and Windows worktrees.

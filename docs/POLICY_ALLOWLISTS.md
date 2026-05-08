# Policy allowlists

Policy allowlists are receipts for intentional exceptions. They should be narrow,
owned, dated, reviewed, and connected to a gate or companion policy.

## Rust 1.95 rollout target

The Rust 1.95 / `0.14.0` rollout map is
[`docs/ci/perl-lsp-rust-1.95-rollout.md`](ci/perl-lsp-rust-1.95-rollout.md). It
splits allowlist work into separate lanes so the MSRV bump does not also change
panic, file, workflow, process, or network policy.

Planned allowlist surfaces include:

- exact counted no-panic allowlist entries;
- a generated no-panic baseline marked as generated metadata;
- a non-Rust file allowlist with owner, reason, surface, classification, and
  coverage fields;
- companion ledgers for generated files, executable files, dependency surfaces,
  workflow surfaces, process execution, and network access;
- `ripr` suppressions for advisory static oracle-gap findings.

## Required posture

Allowlists are not blanket permission. Broad globs require a `broad_glob_reason`,
entries need review dates, and generated or risky surfaces need companion policy
coverage before they become blocking CI gates.

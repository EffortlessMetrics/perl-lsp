# File policy

Rust and `xtask` are the default implementation surfaces for this repository. Non-Rust
files are allowed when they are intentional, owned, covered, and recorded by policy.

## Rust 1.95 / 0.14.0 rollout target

The docs-first rollout PR does not enforce file policy. It records the target shape so
later PRs can add a non-Rust ledger, proposal tooling, companion risky-surface ledgers,
and CI gate wiring without mixing them into the MSRV or release bumps.

Legitimate non-Rust surfaces include:

- Perl fixtures and corpus files;
- tree-sitter C/native parser bindings;
- VS Code extension files;
- GitHub workflows;
- CI scripts;
- generated docs/status artifacts;
- release metadata.

## Target non-Rust entry shape

Every non-Rust allowlist entry should include:

- `id`
- `glob`
- `kind`
- `language`
- `surface`
- `classification`
- `owner`
- `reason`
- `covered_by`
- `created`
- `review_after`

Broad globs also need `broad_glob_reason`.

## Enforcement ladder

1. Add the ledger and parse it, with no enforcement.
2. Add inventory and proposal commands that write reports under `target/policy/` and do
   not mutate the real ledger.
3. Add advisory and blocking checker modes.
4. Wire blocking allowlist checks into gate receipts once the ledger is populated.

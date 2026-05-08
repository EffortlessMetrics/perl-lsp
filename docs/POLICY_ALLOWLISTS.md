# Policy allowlists

Policy allowlists are review receipts, not escape hatches. The Rust 1.95 / 0.14.0
rollout uses dedicated PRs to add or tighten allowlists so version, lint, no-panic,
file, and CI changes do not collapse into one broad policy change.

## Rollout targets

| Ledger | Current rollout state | Target state |
|---|---|---|
| Clippy lint ledger | Active, tracked, planned, and debt entries already exist in `policy/clippy-lints.toml` and `policy/clippy-debt.toml`. | Rust 1.93 rustc lints active; Rust 1.94/1.95 Clippy ratchets active or receipted. |
| No-panic allowlist | Missing or incomplete for exact counted enforcement. | Deliberately retained findings recorded with snippet and count before no-new-debt reporting. |
| Non-Rust file allowlist | Missing or incomplete. | Every allowed non-Rust surface has owner, reason, classification, coverage, dates, and broad-glob justification when needed. |
| Companion ledgers | Not yet part of the first documentation PR. | Generated, executable, dependency, workflow, process, and network surfaces are allowlisted by risk class. |
| ripr suppressions | Advisory suppressions live separately from branch protection. | ripr remains advisory and routes exposure findings without replacing mutation or gate receipts. |

## Required discipline

- Add allowlist entries in the PR dedicated to that policy surface.
- Prefer narrow entries with owners, reasons, creation dates, review dates, and expiry
  where applicable.
- Do not use allowlists to absorb new debt silently.
- Do not mix no-panic baseline resets, Clippy test-carveout removal, non-Rust inventory,
  and release preparation in the same PR.

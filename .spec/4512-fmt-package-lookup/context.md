# Context: #4512

## Decision log

- **Use xtask subcommand, not inline shell `cargo metadata` in hook**: The hook is Bash; shelling out to `cargo metadata | jq` in Bash is brittle and jq may not be installed. Delegating to `cargo xtask resolve-package-name` keeps the logic in Rust and testable.
- **Reuse `resolve_package_names` from targeted_checks.rs**: This function already exists and does exactly the right thing (serde_json + cargo metadata shell-out). No `cargo_metadata` crate needed; no new dep added.
- **Graceful fallback in hook**: If `cargo xtask` is unavailable (cold build, incomplete checkout), fall back to basename. Degrades to old buggy behaviour rather than blocking all pushes.
- **Do NOT change xtask/src/tasks/fmt.rs**: It already uses `--manifest-path` (not `-p`), so it is unaffected by the dir/package name mismatch.

## Objections addressed

- **"Rename the crate directory instead"**: Tracked as #4511. Complementary fix but does not address the systemic brittleness -- any future dir/package mismatch would break again. This fix hardens the hook generically.
- **"Document --no-verify as acceptable"**: Rejected. Trains against the `feedback_pre_push_hook_windows_race.md` memory rule.

## Research findings

- No research verifier pass run (issue marked `builder-ready` with explicit "skip verification pipeline" note -- tooling-internal change, no external Perl/LSP/crate claims to verify).
- `cargo_metadata` crate is NOT in xtask deps; the existing `serde_json` approach in `targeted_checks.rs` is the correct pattern to follow.
- `tempfile` IS already in xtask/Cargo.toml (workspace dep) -- no new dep needed for tests.

## Related issues

- #4511 -- rename `crates/perl-lsp/` to match package name `perl-lsp-rs` (complementary, scheduled after G1b)
- #4509 -- task-tool persistence hook (unrelated hook issue)
- #4456 -- Windows MAX_PATH bug (separate; previously conflated with this issue)
- #4510 -- PR where G1b builder bypassed with --no-verify due to this bug

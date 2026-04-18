# ADR/Spec Findings — work-aa31d3df

## What This ADR Decides

This ADR documents the architectural and operational decisions arising from the 2026-04-11 end-of-session security sweep (issue #4141). It resolves which findings require immediate action, which belong in the backlog, and which must be struck as factually incorrect.

## Key Decisions

1. **Accept `RUSTSEC-2026-0097` risk** — Add a time-scoped ignore entry in `deny.toml` with a revisit trigger, rather than waiting for a `rand` patch. The risk is theoretical (perl-lsp does not install custom loggers) and the CI gate is currently broken.

2. **Strike SBOM task entirely** — The `sbom-cyclonedx.json` and `sbom-spdx.json` files do not exist in the repository; they are generated on-demand during release. Finding 8 / the "13-day drift" claim was materially incorrect.

3. **Backlog: `run_test_sub` identifier hardening** — Add regex validation for `sub_name` to enforce that it is a valid Perl identifier shape. Existing threat model (run_tests executes arbitrary code) limits urgency.

4. **Backlog: `validate_expression` rename** — Rename to `reject_multiline_expression` with a doc comment clarifying its narrow scope. The DAP debugger context means the blast radius is already bounded.

5. **Investigation: cargo machete** — Do not commit a fix until `cargo machete` is run against the full workspace in CI and the false positive is confirmed.

## Alternatives Considered

**Alternative 1 — Patch `rand` immediately:** Bump `rand` to a patched version once available. Rejected because no patched version exists yet, and the advisory is not exploitable in perl-lsp's usage.

**Alternative 2 — Remove SBOM task from backlog without investigation:** Simply drop the task. Rejected because the research/verification disagreement needed explicit resolution to prevent the task from resurfacing.

**Alternative 3 — Broaden `validate_expression` to deny-list dangerous constructs:** Attempt to reject backticks, `system`, `eval`, etc. via regex. Rejected because Perl is not reliably parseable by regex and this risks false positives on valid expressions.

**Alternative 4 — Skip `deny.toml` ignore, accept CI failures:** Allow `cargo deny check` to fail. Rejected because it breaks the merge gate for all PRs touching `Cargo.lock`.

## Consequences

**Benefits:**
- CI gate for `cargo deny check` is restored without masking a genuine vulnerability
- Backlog items are clearly scoped and prioritized
- Invalid SBOM finding is permanently resolved

**Tradeoffs:**
- `RUSTSEC-2026-0097` ignore must be revisited when `rand` is patched
- `run_test_sub` identifier validation is a partial hardening (regex cannot fully parse Perl)
- `cargo machete` investigation is deferred

## Acceptance Criteria

See `specs.md` for the full specification. Key criteria:
- `cargo deny check advisories` passes after the `deny.toml` change
- `run_test_sub` rejects non-identifier `sub_name` with a clear error
- `validate_expression` is renamed to `reject_multiline_expression` with doc comment
- SBOM regeneration task is removed from all tracking artifacts
- Cargo machete task is flagged as `needs-verification` pending CI confirmation
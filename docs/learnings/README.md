# Learnings Index

**Purpose**: A greppable, keyword-rich index of real incidents that happened in this
repository. Each incident is a separate file with YAML frontmatter for tagging and
search. Future agents: grep for the exact symbol, error string, PR number, or hazard
class you are investigating.

For the portable, repo-agnostic patterns behind these incidents, see
[docs/concepts/](../concepts/).

For spec contracts and hazard-class acceptance criteria, see
[docs/reference/PARSER_CONTRACTS.md](../reference/PARSER_CONTRACTS.md) and
[docs/agents/SPEC_UPDATE_CHECKLIST.md](../agents/SPEC_UPDATE_CHECKLIST.md).

To add a new incident: copy [TEMPLATE.md](TEMPLATE.md) and fill in all sections.

---

## Incidents

| File | Title | Tags | Related PRs |
|------|-------|------|-------------|
| [2026-06-ripr-output-schema-break.md](2026-06-ripr-output-schema-break.md) | ripr 0.9.x output-schema rename broke suppression matching | coverage-integrity, ripr | #1329, #1336 |
| [2026-06-dap-ref-space-collision.md](2026-06-dap-ref-space-collision.md) | DAP variablesReference base 50_000 collided with scope-ref formula | id-collision, dap | #1219, #1246 |
| [2026-06-coverage-gate-measurement.md](2026-06-coverage-gate-measurement.md) | LCOV brace scanner blind to string/char/comment literals | scanner-blindness, coverage-integrity | #1327, #1326 |
| [2026-06-test-encodes-the-bug.md](2026-06-test-encodes-the-bug.md) | Pre-existing test asserted the stale-frames defect as expected | test-encodes-bug, dap | #1337, #964 |
| [2026-06-recreate-over-untangle.md](2026-06-recreate-over-untangle.md) | Multi-agent branch tangle: #1309 re-created fresh as #1337 | multi-agent, re-create | #1309, #1337 |
| [2026-06-merge-cancellation-cascade.md](2026-06-merge-cancellation-cascade.md) | Concurrent merges triggered Codecov upload cancellation cascade | ci, serialization, codecov | #1206, #1230 |
| [2026-06-codecov-false-low.md](2026-06-codecov-false-low.md) | Codecov false-low: --lib profdata only; integration-test lines undercounted | coverage-integrity, codecov | #1282, #1263 |
| [2026-06-shift-left-validated.md](2026-06-shift-left-validated.md) | Shift-left validated: 0-fix deep-review after hazard invariants front-loaded | shift-left, validation | #1246, #1340 |

---

## Tags reference

| Tag | Hazard class |
|-----|--------------|
| id-collision | ID/reference-space collision (Class 1) |
| bounds | Bounds/overflow (Class 2) |
| protocol-safety | Protocol-safety / invalid input (Class 3) |
| scanner-blindness | Scanner literal/comment blindness (Class 4) |
| test-encodes-bug | Test asserts defect as expected behavior (Class 5) |
| coverage-integrity | Coverage/measurement integrity (Class 6) |
| shift-left | Shift-left pattern / failure-catching rung |
| multi-agent | Multi-agent branch ownership / tangle |
| ci | CI pipeline / check failures |
| serialization | Merge serialization / cancellation cascade |
| dap | Debug Adapter Protocol |
| ripr | ripr gap-gate tool |
| codecov | Codecov patch-coverage gate |

# Learnings Index

**Purpose**: A greppable, keyword-rich index of real incidents that happened in this
repository. Each incident is a separate file with YAML frontmatter for tagging and
search. Future agents: grep for the exact symbol, error string, PR number, or hazard
class you are investigating.

For the portable, repo-agnostic patterns behind these incidents, see
[docs/concepts/](../concepts/). The 2026-06-11→13 campaign's meta-orchestration
learnings (substrate-model, model-conformance, human-corrects-substrate, paced-merges,
lane-relevance, spawn-guards) are distilled into three new concept docs:
`orchestrator-substrate-model.md`, `model-conformance.md`, `human-corrects-substrate.md`.

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
| [2026-06-ripr-suppression-application-gap.md](2026-06-ripr-suppression-application-gap.md) | ripr suppression-application gap: path-check skipped before `continue` on unrecognized classification | coverage-integrity, ripr, gate-logic, suppression | #1346, #1349 |
| [2026-06-deep-review-net-for-novel-gate-logic.md](2026-06-deep-review-net-for-novel-gate-logic.md) | Deep-review remains the net for novel gate/infra logic even after shift-left | coverage-integrity, ripr, deep-review, gate-logic | #1349 |
| [2026-06-merged-before-review-fix-forward.md](2026-06-merged-before-review-fix-forward.md) | PR merged on 3-green before in-flight deep-review completed; fix landed as fix-forward | multi-agent, dap, deep-review, fix-forward | #1240, #1363, #1364 |
| [2026-06-serialize-merges-misframe.md](2026-06-serialize-merges-misframe.md) | "Hold main still" misframe: parallel velocity + rebase-robustness is the correct doctrine | multi-agent, ci, serialization, merge-velocity | #1206, #1230 |
| [2026-06-tagged-range-codec-band-overflow.md](2026-06-tagged-range-codec-band-overflow.md) | Type-enum promotion re-introduced ID-collision through the wire codec (band-overflow, residue-disambiguation) | id-collision, bounds, dap, codec, band-overflow | #1219, #1351, #1430 |
| [2026-06-nodekind-variant-silent-consumer-drop.md](2026-06-nodekind-variant-silent-consumer-drop.md) | New NodeKind variant silently dropped by three non-exhaustive consumers (if-let loop, wildcard arm) | parser, ast, nodekind, exhaustiveness, silent-drop, lsp-feature-gap | #1457, #1362 |

| [2026-06-red-tdd-invalid-red.md](2026-06-red-tdd-invalid-red.md) | Red-TDD produced invalid red: tests that passed immediately or failed for wrong reasons | tdd, red-tdd, verification, stochastic-pipeline, test-validity | #1372, #1445, #1338 |
| [2026-06-substrate-tax-and-red-is-a-smell.md](2026-06-substrate-tax-and-red-is-a-smell.md) | Substrate tax and red-is-a-smell: two recalibrated operating principles from the 2026-06 campaign | substrate, ci, merge, economics, anti-pattern, incident | #651, #1282, #1453, #1458 |

| [2026-06-coverage-job-ran-tests.md](2026-06-coverage-job-ran-tests.md) | Coverage-named checks must not hide test failures — decoupling measurement from validation | ci, coverage, observability, misclassification | #1457, #1470, #1469 |

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
| gate-logic | Gate evaluation loop ordering / cross-cutting policy application |
| suppression | Policy suppression matching |
| deep-review | Deep-review as correctness net |
| fix-forward | Fix-forward recovery after merged-before-review |
| merge-velocity | Merge pacing / rebase-robustness |
| codec | Wire/serialization codec correctness |
| band-overflow | Encoded value crosses its declared wire band |
| tagged-range | Tagged-range codec with disjoint-band design |
| ast | AST node structure / NodeKind variants |
| nodekind | NodeKind enum variant addition or consumer audit |
| exhaustiveness | Rust exhaustiveness checker blind spots (if-let, wildcard arm) |
| silent-drop | Variant or case silently skipped by non-exhaustive consumer |
| lsp-feature-gap | Missing LSP feature (tokens, hover, goto, rename, refs) caused by consumer gap |

| tdd | TDD stage (red-tdd, green-tdd) correctness and validity |
| red-tdd | Red-TDD stage specifically: test validity before builder starts |
| verification | Pipeline artifact verification / ground-truth check |
| stochastic-pipeline | Stochastic-pipeline posture / reliability profile |
| test-validity | Test correctly red/green for the right reason |
| substrate | CI gate scope, required-check list, instrument configuration |
| economics | Token cost, CI cost, amortization, leverage arithmetic |
| anti-pattern | Operating anti-pattern (merge-past-red, routine-override)

| observability | Observable failure / gate output clarity and naming honesty |
| misclassification | Failure routed to wrong subsystem due to check name or output lying |




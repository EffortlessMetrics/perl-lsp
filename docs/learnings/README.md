# Learnings Index

**Purpose**: A greppable, keyword-rich index of real incidents that happened in this
repository. Each incident is a separate file with YAML frontmatter for tagging and
search. Future agents: grep for the exact symbol, error string, PR number, or hazard
class you are investigating.

For the portable, repo-agnostic patterns behind these incidents, see
[docs/concepts/](../concepts/). The 2026-06-11→13 campaign's meta-orchestration
learnings are distilled into concept docs including `orchestrator-substrate-model.md`,
`model-conformance.md`, and `human-corrects-substrate.md`.

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
| [2026-06-rerunning-broken-gates.md](2026-06-rerunning-broken-gates.md) | A gate that fails repeatedly on verified-correct content is the bug | ci, gate-logic, stochastic-pipeline, verification, instrument, observability | #1457, #1470, #1469 |
| [2026-06-agent-claims-vs-ground-truth.md](2026-06-agent-claims-vs-ground-truth.md) | Agent claims must be verified against ground-truth facts before routing | ci, agent-claims, verification, ground-truth, observability, multi-agent, stochastic-pipeline | #1474 |

| [2026-06-red-tdd-invalid-red.md](2026-06-red-tdd-invalid-red.md) | Red-TDD produced invalid red: tests that passed immediately or failed for wrong reasons | tdd, red-tdd, verification, stochastic-pipeline, test-validity | #1372, #1445, #1338 |
| [2026-06-substrate-tax-and-red-is-a-smell.md](2026-06-substrate-tax-and-red-is-a-smell.md) | Substrate tax and red-is-a-smell: two recalibrated operating principles from the 2026-06 campaign | substrate, ci, merge, economics, anti-pattern, incident | #651, #1282, #1453, #1458 |

| [2026-06-coverage-job-ran-tests.md](2026-06-coverage-job-ran-tests.md) | Coverage-named checks must not hide test failures — decoupling measurement from validation | ci, coverage, observability, misclassification | #1457, #1470, #1469 |
| [2026-06-substrate-self-validation-bootstrap.md](2026-06-substrate-self-validation-bootstrap.md) | You cannot validate a gate-fix through the broken gate | ci, substrate, bootstrap, recursion, incident, gate-logic, self-validation | #1469, #1477, #1478, #1479, #1484, #1485 |
| [2026-06-ripr-draft-skip-fails-gate.md](2026-06-ripr-draft-skip-fails-gate.md) | ripr+ New Gap Gate fails on draft PRs when router skips them | ci, gate-logic, ripr, routing, draft-pr | #1578, #1574, #1556, #1555, #1512, #1511, #1558 |
| [2026-06-docs-only-runs-rust-matrix.md](2026-06-docs-only-runs-rust-matrix.md) | Docs-only PRs run full Rust build matrix; pure .md changes fail irrelevantly | ci, routing, workflow-dispatch, build-matrix, documentation | #1558, #1512 |
| [2026-06-validate-title-issue-ref-gap.md](2026-06-validate-title-issue-ref-gap.md) | validate-title fails on agent-generated PRs lacking issue reference in title | ci, pr-metadata, validation, agent-generated, title-check | #1583, #1519 |
| [2026-06-merge-funnel-throughput-constraint.md](2026-06-merge-funnel-throughput-constraint.md) | Merge funnel, not discovery, is the binding throughput constraint | ci, workflow, throughput, merge-velocity, bottleneck, economics | #1578, #1574, #1556, #1555, #1512, #1511, #1558, #1583 |

| [2026-07-config-scope-predicate-drift.md](2026-07-config-scope-predicate-drift.md) | Config-read scope mismatch and predicate drift in VS Code settings integration | config-read-scope, predicate-drift, vscode-config, component-vs-system, external-truth-gate | #3308, #3276 |
| [2026-07-guard-trigger-coverage-and-matching.md](2026-07-guard-trigger-coverage-and-matching.md) | Guard trigger coverage and substring-matching in strict-product-surface scanner | guard-trigger-coverage, substring-matching, scanner-blindness, enforcement-over-doctrine, cross-pr-coupling, strict-surface, product-surface | #3315, #3308, #3319, #3276 |
| [2026-07-external-truth-review-product-docs.md](2026-07-external-truth-review-product-docs.md) | External-truth review of product-surface documentation and CI noise triage | external-truth-gate, product-docs, fact-verification, cross-pr-reference, ci-noise, non-required-checks, flaky-gates | #3319, #3315, #3308, #3324, #3276 |

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



| instrument | Measurement instrument failure / scope misconfiguration |
| agent-claims | Agent output reliability / claims verification |
| ground-truth | Ground-truth fact verification / trust-but-verify |
| bootstrap | Bootstrap recursion / self-validating layer circularity |
| self-validation | Layer validating its own correctness or fixes |
| routing | Workflow dispatch routing; gate-selection logic |
| draft-pr | Draft PR state handling in CI gates |
| workflow-dispatch | GitHub Actions workflow dispatch routing |
| build-matrix | Multi-target/multi-version build matrix |
| documentation | Documentation-only changes (no code) |
| pr-metadata | PR title, body, labels metadata validation |
| validation | Validation check (gate, form, contract) |
| agent-generated | PRs created by agents, not humans |
| title-check | PR title validation / format enforcement |
| throughput | Merge funnel throughput / cycle time |
| bottleneck | Pipeline bottleneck / binding constraint |
| config-read-scope | Config API scope mismatch in integration points |
| predicate-drift | Change-detection predicate diverges from writer parser |
| vscode-config | VS Code configuration API integration |
| component-vs-system | Component passes tests; system fails in integration |
| external-truth-gate | User-visible facts verified against code/spec/oracle |
| guard-trigger-coverage | Enforcement gate trigger condition too narrow |
| substring-matching | Text scanner false-negative from substring vs word matching |
| enforcement-over-doctrine | Mechanical gate required, not doctrine-only rule |
| cross-pr-coupling | Feature or fix reaches across PR boundaries |
| strict-surface | Strict product-surface scanner / policy enforcer |
| product-surface | User-visible product API surface |
| product-docs | User-facing documentation of features/capabilities |
| fact-verification | Fact-checking docs against source |
| cross-pr-reference | Doc references feature in in-flight/unmerged PR |
| ci-noise | Non-actionable CI output / noise vs signal triage |
| non-required-checks | Advisory CI checks; do not block merge |
| flaky-gates | Stochastic gate failures on stable code |

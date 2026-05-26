# Acceptance Criteria: Green-CI Cancel Classification

## Behavioral Cases (4 test rows)

| Case | Input | Expected Output | File |
|------|-------|-----------------|------|
| **INFRA-NOISE** | `conclusion: cancelled`, `started_at == completed_at` (zero-duration) | Mark check INFRA-NOISE; exclude from RED count; include in GREEN verdict | green-ci-check.md step 5a |
| **DEVELOPER-CANCEL** | `conclusion: cancelled`, `completed_at - started_at > 5s` | Mark check DEVELOPER-CANCEL; include in RED count; list in verdict details | green-ci-check.md step 5a |
| **FAILURE-ALWAYS-RED** | `conclusion: failure` | Mark check RED; ignore any cancel-related log content; always include in RED count | green-ci-check.md step 5a |
| **SUCCESS-NO-CHANGE** | `conclusion: success` | Existing behavior unchanged; include in GREEN verdict | green-ci-check.md step 5a |

## File Targets

- [ ] `/home/user/perl-lsp-swarm/.claude/commands/green-ci-check.md` — step 5 replaced with 5a (classify) + 5b (verdict) logic
- [ ] `/home/user/perl-lsp-swarm/.claude/agents/green-ci.md` — Verdicts section includes new INFRA-NOISE verdict
- [ ] `/home/user/perl-lsp-swarm/.claude/agents/green-ci.md` — "What you do NOT check" section includes concurrency-group cancellation exclusion

## Verification Method

Empirical: Next GitHub Actions concurrency-group cancellation event will validate:
1. Zero-duration cancellations are classified as INFRA-NOISE
2. Green verdict is issued if no RED checks remain
3. DEVELOPER-CANCEL (>5s) are still listed in failure details
4. No spurious RED verdicts from concurrency kills

## Success Criteria

- Green verdict issued on next concurrency-cancellation event (was previously RED)
- Manual review of PR verdict comment confirms "INFRA-NOISE" language used
- No regression: failure cases still classified as RED

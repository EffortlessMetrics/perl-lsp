# Source Sync Receipt: 2026-06-06 promote swarm a87f766ab

## Sync Identity

| Field | Value |
|---|---|
| Swarm repo | `EffortlessMetrics/perl-lsp-swarm` |
| Swarm RC branch | `release/next-rc` |
| Swarm RC SHA | `a87f766ab60da513833dfff47349384be96fdae2` |
| Source repo | `EffortlessMetrics/perl-lsp` |
| Source target branch | `master` |
| Source base SHA | `a539017f88477901db11f73f568c86d6063046ce` |
| Merge-base (swarm ∩ source) | `151c5ecee69ef465836d2e7e173c310690391574` |
| Sync direction | swarm → source (deliberate release-prep promotion) |
| Source sync PR | `EffortlessMetrics/perl-lsp#9909` |
| Swarm receipt PR | This PR; merge SHA recorded by GitHub after merge |

## Included Areas

| Area | Files | Decision |
|---|---|---|
| `crates/` | 115 | INCLUDE — all shared Rust crates (parser, lexer, LSP, DAP, providers, workspace, diagnostics, inline completion, code actions) |
| `vscode-extension/` | 11 | INCLUDE — user-facing editor extension |
| `xtask/` | 37 | INCLUDE — all xtask tasks present in source; 8 new task files (quality gate, RIPR evidence, semantic inline, ci route, lsp UX smoke, quality baseline, inline completion quality, supported editor inline smoke) |
| `features.toml` | 1 | INCLUDE — canonical LSP capability definition |
| `Cargo.toml` / `Cargo.lock` | 2 | INCLUDE — workspace manifest (see version concern below) |
| `docs/` | 21 | INCLUDE — ci/, development/, how-to/, project/status/, reference/, releases/, specs/, swarm/sync-protocol.md |
| `testdata/` | 30 | INCLUDE — UX smoke fixture files for release smoke harness (6 fixtures, 40 requests) |
| `scripts/` | 64 of 71 | INCLUDE (7 excluded — see below) |
| `.github/workflows/` | 14 (+ 1 new) | INCLUDE — all 14 changed workflows; `lsp-318-claim-guard.yml` is new in swarm |
| `.github/ISSUE_TEMPLATE/` | 1 new | INCLUDE — candidate_issue.yml (discovery desk template) |
| `.github/PULL_REQUEST_TEMPLATE.md` | 1 | INCLUDE |
| `.ci/` | 7 | INCLUDE — gate-policy, required-checks, coverage-packs (new), droid schema (new), blockers, README-coverage, receipts registry |
| `policy/` | 6 | INCLUDE — quality-gate-exceptions (new), ripr-suppressions, ci-budget, ci-lane-whitelist, ci-lanes, ci-risk-packs |
| `justfile` | 1 | INCLUDE |
| `CHANGELOG.md` | 1 | INCLUDE — release history docs |
| `RELEASE_HISTORY.md` | 1 | INCLUDE — release ledger |
| `codecov.yml` | 1 | INCLUDE — coverage configuration |
| `CLAUDE.md` | 1 | INCLUDE — MAINTAINER_AGENT_DOCTRINE link added |
| `.gitattributes` | 1 | INCLUDE — CRLF fixture annotation |
| `badges/` | 1 | INCLUDE — ripr-plus.json refresh |

## Excluded Areas

| Excluded | Reason |
|---|---|
| `.claude/` (18 files) | Swarm agent harness, commands, hooks — internal to swarm orchestration pipeline only |
| `scripts/agent-cleanup.ps1` | Swarm agent lifecycle management — absent from source |
| `scripts/agent-preflight.ps1` | Swarm agent safety gate (PS1) — absent from source; `agent-preflight.sh` remains |
| `scripts/swarm-clean` | Swarm ops tooling — absent from source |
| `scripts/swarm-doctor` | Swarm ops tooling — absent from source |
| `scripts/tests/test_swarm_clean.sh` | Tests for swarm-only tool |
| `scripts/tests/test_swarm_doctor.sh` | Tests for swarm-only tool |
| `scripts/tests/test-swarm-summary-wrapper.sh` | Tests for swarm-only tool |
| `docs/swarm/source-syncs/` receipt files | Swarm-side sync artifacts — not mirrored to source |
| `.ops-perl-lsp/` | Swarm ops metrics — no delta in this promotion |

## Included Swarm PRs (notable)

Release-blocker batch: #1190, #1191, #1192, #1193, #1194

Feature content: #1188, #1187, #1189, #1184, #1179, #1176, #1175, #1167, #1165, #1166, #1163, #1007

Inline completion roadmap: #449–#579 (semantic context, constructor style, loop bindings, DBI receiver, guard condition, lexical return, test assertions, self-receiver, Moo accessors, module reachability, ghost text corpus)

LSP 3.18 conformance: #459–#480 (textDocumentContent wire contracts, 3.18 boundary spec, claim guard, debug messages)

Parser fixes: #775, #711, #455, #454

Quality/proof-lane (#8197): quality gate enforcement, RIPR new-gap gating, patch coverage gate, CI route, RIPR evidence reporting, semantic inline receipts, merge-ready required contexts

## Verification Commands Run

```bash
# Source branch compilation check
export CARGO_TARGET_DIR="/tmp/sync-promote-target"
cd H:/Code/Rust/perl-lsp-swarm-sync-promote
cargo check -p perl-lsp-rs --locked    # → Finished dev profile (0 errors)
cargo check -p xtask --locked          # → Finished dev profile (0 errors)
git diff --check                        # → 0 whitespace issues

# Swarm RC verification (performed before this sync)
# pr-fast 10/10 PASS at a87f766ab
# Product smoke: 6 fixtures / 40 requests green
# Quality-gate exceptions: ripr-total-burndown, project-coverage-burndown
#   (policy/quality-gate-exceptions.toml, both expire 2026-09-30)
```

## Known Concerns

1. **Workspace version concern**: Cargo.toml workspace version in swarm is `0.15.0` (swarm development baseline). Source was at `0.15.2` before this sync. The 0.15.1 and 0.15.2 version bumps happened in source but were never mirrored to swarm. After merging this sync PR, a version bump to `>=0.15.3` (or `0.16.0` for the feature batch) is required before the next `cargo publish`. Do NOT publish without an explicit version bump.

2. **No history preservation**: This is a content-state mirror. Commit history from swarm is not replicated to source. `EffortlessMetrics/perl-lsp` remains the commit-history and release-lineage authority.

## Claim Boundary

This is a deliberate release-preparation promotion from perl-lsp-swarm (development source of truth) into perl-lsp (release lineage repo). It does not make perl-lsp a full-history mirror of perl-lsp-swarm, and it does not repair GitHub contributor graph provenance. perl-lsp remains the commit-history and release-lineage authority until a separate history-preserving mirror/fork decision replaces the current content sync model.

## Invariant Check

Per sync-protocol.md Hard Invariant: After this sync, `perl-lsp/master` (which will include swarm content through a87f766ab) will not be ahead of `perl-lsp-swarm/main` (currently at a87f766ab). The invariant is preserved. Specifically, perl-lsp/master will receive a strict subset of the swarm content at a87f766ab (some swarm-only infrastructure is excluded), so source will be at or behind swarm.

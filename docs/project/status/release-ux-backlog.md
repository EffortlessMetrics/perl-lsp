# Release UX Backlog

> Human-owned routing snapshot. Refresh the GitHub queries below before acting.
> This page does not authorize release work, provider promotion, PR closure, or
> publishing-repo development by itself.

Snapshot collected: 2026-06-04.

## Purpose

This page keeps the release-shaped UX lane focused on user trust instead of
random queue drain. The near-term question is:

> Can a real Perl user install `perl-lsp`, open a normal Perl project, and get
> quiet, correct, useful editor behavior across setup, module resolution,
> diagnostics, links, docs, completion, and safe code actions?

Use this page to choose the next narrow PR. It is a backlog map, not a metric
source of truth and not a merge queue.

## Refresh Commands

```bash
gh pr list -R EffortlessMetrics/perl-lsp-swarm --state open --limit 200 \
  --json number,title,isDraft,headRefName,mergeStateStatus,updatedAt,url,labels

gh pr list -R EffortlessMetrics/perl-lsp --state open --limit 200 \
  --json number,title,isDraft,headRefName,updatedAt,url,labels

gh pr view 684 -R EffortlessMetrics/perl-lsp-swarm \
  --json number,title,isDraft,mergeStateStatus,files,body,url,labels

gh pr view 930 -R EffortlessMetrics/perl-lsp-swarm \
  --json number,title,isDraft,mergeStateStatus,files,body,url,labels

gh pr view 991 -R EffortlessMetrics/perl-lsp-swarm \
  --json number,title,isDraft,mergeStateStatus,files,body,url,labels
```

Current snapshot shape:

- `perl-lsp-swarm`: 75 open PRs; 70 non-draft, 5 draft; 63 `BEHIND`, 11
  `DIRTY`, 1 `BLOCKED`.
- `perl-lsp`: 43 open PRs; 36 draft, 7 non-draft.
- `perl-lsp` PL410 duplicate cluster: 33 open PRs by title.

Do not treat these counts as durable project metrics. They are a triage receipt
for this snapshot only.

## Ready / Near-Ready User-Facing PRs

1. `perl-lsp-swarm#684` - `fix(lsp): correct document link offsets (#0000)`
   - State: non-draft, `BLOCKED`, labels: `codex`, `needs-issue-link`.
   - User value: document links stop pointing at wrong ranges in CRLF-prefixed
     documents and local quoted-file cases.
   - Current diff: `crates/perl-lsp-rs/src/features/lsp_document_link.rs` plus
     `crates/perl-lsp-rs/tests/lsp_document_link_provider_coverage.rs`.
   - Next action: validate against current `main`, add or link the canonical
     issue, re-run focused proof, then merge if clean.

2. Document-link coverage cluster: `#674`, `#683`, and `#684`.
   - User value is concentrated in `#684`; the other document-link PRs should be
     checked for unique regressions before any carry-forward work.
   - Do not merge multiple overlapping document-link PRs without proving they
     cover distinct behavior.

3. Editor-shaped LSP tests: `#670`, `#672`, and `#673`.
   - These are potentially useful for the later smoke harness, but should follow
     the document-link correctness fix so the receipt pack does not lock in
     known bad link ranges.

## Duplicate Clusters

1. Publishing-repo PL410 quick-fix storm.
   - `EffortlessMetrics/perl-lsp` has 33 open PRs whose titles describe the same
     undefined loop-control-label quick fix.
   - Representative range in the snapshot: `#9610`, `#9615`, `#9618`, `#9625`,
     `#9630`, `#9635` through `#9648`, `#9650` through `#9655`, `#9662`
     through `#9670`.
   - Correct lane action: choose the best implementation idea, apply one
     canonical PL410 quick-fix PR in `perl-lsp-swarm`, then close publishing
     duplicates only with maintainer-approved pointers to the canonical swarm
     PR or issue.

2. Leaf coverage swarms.
   - URI/path coverage: `#663`, `#665`, `#667`, `#669`, `#679`.
   - Lexer coverage: `#659`, `#660`, `#661`, `#662`.
   - Regex coverage: `#632`, `#633`, `#634`, `#635`.
   - Tree-sitter facade/grammar coverage: `#636`, `#637`, `#638`, `#639`,
     `#642`, `#643`, `#644`.
   - These may contain useful tests, but they are queue-noise until grouped by
     unique behavior and current-main applicability.

3. Refactor split cluster.
   - `#617` through `#627` are mostly SRP/refactor slices across xtask, CI,
     critic, perltidy, inline, and code actions.
   - Release UX should only carry a refactor from this cluster if it directly
     unblocks a focused user-facing proof.

## CI / Proof Reliability PRs

1. `#1085` - security scan report with 0 findings.
   - Release relevance: useful operational receipt if still current.
   - Next action: verify it is not superseded by a newer scan before carrying.

2. `#680`, `#641`, `#625`, `#626`, and related CI-hygiene splits.
   - Release relevance: possible queue-drag reduction.
   - Boundary: do not broaden CI architecture in this lane. Carry only focused
     checks that reduce false queue state or improve proof quality.

3. Fuzz/property PRs: `#675`, `#676`, `#681`, `#682`, `#686`, `#690`.
   - Release relevance: useful after user-facing correctness gaps are handled.
   - Boundary: assign each to fast, slow, nightly, or manual lanes before merge;
     do not flood default PR checks.

## Long-Lived Lane Branches To Split

1. `#930` - NodeKind Fidelity lane.
   - State: draft, `DIRTY`.
   - Value: shared `NodeKind` classification could prevent DAP, semantic
     tokens, document symbols, and scope analysis from growing divergent local
     heuristics.
   - Current problem: the branch now spans parser, DAP, LSP providers, xtask,
     status docs, and tests. That is too broad for release-shaped review.
   - Split target: extract only the `perl-ast` classification contract and its
     tests first. Consumer migration should be later PRs.

2. `#991` - DAP Debugger Trust lane.
   - State: draft, `DIRTY`.
   - Value: debugger protocol trust is part of release feel.
   - Current problem: stdio framing, lifecycle matrix, variables reference
     hardening, and a build fix are stacked together.
   - Split order: stdio malformed/partial/EOF coverage, lifecycle matrix,
     variablesReference hardening, then stale placeholder/fake capability
     follow-up.

3. Discovery desk drafts: `#939`, `#966`, `#967`.
   - Value: may improve future queue intake.
   - Boundary: not first-order release UX. Do not let discovery process work
     displace document links, PL410, startup, `@INC`, or editor setup receipts.

## Dependency PRs To Batch

Open Dependabot PRs in this snapshot:

- `#1130` - TypeScript group in `vscode-extension`.
- `#1131` - `tar` in `vscode-extension`.
- `#1132` - `@types/vscode`.
- `#1134` - Cargo dependency group with 3 updates.
- `#1135` - `serial_test`.
- `#1136` - `taiki-e/install-action`.
- `#1137` - `docker/setup-qemu-action`.

Batch if the updated surfaces are independent and the focused proof passes:

```bash
cargo check --workspace --all-targets --profile agent --locked
cargo test -p xtask --profile agent --locked
npm --prefix vscode-extension ci
npm --prefix vscode-extension test
git diff --check
./scripts/storage-doctor
```

Close superseded bot PRs only after the batch PR exists and with maintainer
approval if lane ownership is unclear.

## Publishing-Repo PRs To Close Or Port Back To Swarm

`EffortlessMetrics/perl-lsp` is the release, history, and package-lineage repo.
It is not the development queue.

Current source-repo patterns to route:

1. PL410 duplicate drafts.
   - Port one canonical implementation into `perl-lsp-swarm`.
   - Close duplicates only after the canonical swarm PR or issue exists.

2. Source-repo Dependabot/security PRs.
   - Non-draft source PRs in this snapshot include dependency and security
     updates. Keep them release-lineage only unless they block swarm or release
     work.

3. New source-repo feature drafts.
   - Do not merge routine feature work from `perl-lsp`.
   - If a feature is valuable, port one semantic slice to `perl-lsp-swarm` and
     preserve the original source PR number in the swarm PR body.

## Release UX Gaps With No PR Yet

These are the missing receipts that make the next release user-testable:

1. Local release smoke fixtures:
   - `minimal_script`
   - `lib_project`
   - `local_lib_project`
   - `crlf_links`
   - `diagnostics_quickfix`
   - `perldoc_links`
   - `moose_or_moo`
   - `test2_project`
   - `mojolicious_app` when practical

2. One command for the first-day editor loop:
   - target command: `cargo xtask lsp-ux-smoke --fixture testdata/ux/release_smoke`
   - expected coverage: initialize, initialized, didOpen, diagnostics,
     documentLink, codeAction, completion when stable, hover/docs when stable,
     shutdown.

3. Quiet startup receipt:
   - no panic
   - no scary warning for normal optional-tool absence
   - clear status for unavailable Perl/perltidy/perlcritic
   - workspace root detected
   - server handles initialize/didOpen/shutdown

4. Effective `@INC` release receipt:
   - same include context respected by goto definition, module completion,
     workspace symbols, document links where applicable, and diagnostics.
   - Existing source of truth: [module_resolution.md](module_resolution.md).

5. Quick-fix release receipt:
   - PL410 undefined loop-control label.
   - Existing stable strict/warnings or import/use quick fixes if they already
     have deterministic behavior.
   - Negative cases that prove no unsafe action appears.

6. Editor setup receipts:
   - VS Code managed-binary path.
   - JetBrains/LSP4IJ supported path.
   - Do not make raw-command/manual descriptor setup the primary JetBrains path.

7. Next-release UX readiness receipt:
   - install channels
   - known-good editor paths
   - known gaps
   - smoke fixture paths
   - `@INC`, document-link, quick-fix, startup, and DAP status
   - explicit non-claims

## Recommended Next PR Order

1. Finish or rework `#684` document-link offset correctness.
2. Add the canonical PL410 quick-fix in `perl-lsp-swarm`.
3. Add release smoke fixture pack under `testdata/ux/release_smoke/`.
4. Add `cargo xtask lsp-ux-smoke`.
5. Add startup, `@INC`, quick-fix, and editor setup receipts.
6. Split NodeKind and DAP trust lanes into reviewable PRs.
7. Batch low-risk dependency PRs.
8. Publish the next-release UX readiness receipt.

## Acceptance For This Backlog Page

```bash
cargo xtask check-support-claims
cargo xtask check-devex-docs
cargo xtask doc-claims
git diff --check
./scripts/storage-doctor
```

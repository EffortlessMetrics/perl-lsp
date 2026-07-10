# Spec-Governance Implementation Plan

Status: planned
Owner: perl-lsp maintainers
Linked proposal: [PLSP-PROP-0003](../../docs/proposals/PLSP-PROP-0003-spec-governance.md)
Linked spec: none yet (S2/S3 will add a `spec-system` contract spec once the
markdown-adapter and native-bundle shapes are settled)
Goal manifest: none yet — deferred to S1 (see "Cutover/rollback" below for why)
Tracker: [#3586](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/3586)

## Purpose

Define the phase train that migrates structural validation of `perl-lsp`'s
spec system (identity, typed links, lifecycle, ownership, closeout edges) onto
`cargo-allow`'s opt-in `spec-system` profile, while keeping content authoring
and repo-specific semantics exactly where they already work. This plan is a
routing map, not a product-claim document: S0 (this PR) plans the migration
and does not execute it.

## Grounded Tool Facts (verified 2026-07-10, this worktree)

- `cargo-allow` v0.1.10 is installed (`cargo allow --version`). Its default
  job is the source-exception ledger (`policy/allow.toml`); it also ships an
  opt-in `spec-system` profile: `cargo-allow check --profile spec-system`
  (modes: audit/no-new/strict/release; formats include json/sarif/markdown;
  writes receipts). `init`/`doctor`/`worklist`/`explain` all accept
  `--profile spec-system`.
- `cargo-allow doctor --profile spec-system` currently reports `Ready: false`
  with `.allow/profiles/spec-system.toml` missing (config provenance =
  `built_in_default`). That config file is created at S1, not in this PR.
- `cargo-allow check --profile spec-system --mode audit` runs today against
  built-in default roots: 0 artifacts, 0 links, 2 findings (1 blocking-eligible
  — `doc_artifact_ledger_missing` — and 1 advisory — missing
  `docs/status/SUPPORT_TIERS.md`).
- The exact `.allow/profiles/spec-system.toml` shape must come from
  `cargo-allow init --profile spec-system --dry-run` at S1 time. This plan
  does not hardcode an assumed shape — the dry-run output is the source of
  truth when S1 opens.
- `cargo-allow init --profile spec-system --dry-run` was run (read-only,
  this worktree) to check whether its defaults fit the actual repo layout.
  They do not: the dry-run would create `.allow\goals\` (not the existing
  `.perl-lsp/goals/`), `docs\status\SUPPORT_TIERS.md` (not the existing
  `docs/project/status/SUPPORT_TIERS.md`), and a competing `docs\templates\`
  tree (canonical templates already live in `docs/reference/`). Its
  `.allow\artifacts\doc-artifacts.toml` default does match a sensible
  ledger location. This is why S1 must explicitly configure roots rather
  than accept built-in defaults — see "S1 repo roots" below.
- See the doctor/check raw output appendix at the bottom of this plan.

## Objective

`cargo-allow`'s `spec-system` profile becomes the canonical **structural**
graph validator for the spec system: identity, typed links, lifecycle,
ownership, and closeout edges. `spec-planner`/`spec-builder.js` keep
**authoring** content (hazards, acceptance criteria, API shape, test grid,
blast radius). `xtask` keeps owning **repo-specific semantics** (lane IDs, WIP
caps, permitted-lane rules, hazard-class completeness). CI executes proof
(tests, clippy, RIPR, corpus, receipts) exactly as it does today.

## Responsibility Split

| Concern | Owner | Notes |
|---|---|---|
| Problem analysis, hazards, API-shape, test-grid, blast-radius | Claude spec-planner + `spec-builder.js` | Unchanged — content authoring stays repo-side |
| Stable IDs, kinds, statuses, ownership, typed graph links | `cargo-allow` `spec-system` profile | New responsibility, phased in S1-S3 |
| Lane IDs, WIP caps, permitted-lane rules | Existing `xtask` checks | Unchanged — repo-specific semantics stay in xtask |
| Test execution, clippy, RIPR, corpus, receipts | `cargo`/`xtask`/CI | Unchanged — `cargo-allow` never executes proof |
| Live PR/issue state | GitHub | Unchanged — `cargo-allow` never queries GitHub |
| "What landed" | Closeout artifact + merge/reachability receipt | New artifact type added in S5 |

## What Does NOT Migrate (stays repo-specific)

- All six `acceptance.md` sections (Behavior, Hazards, Contracts, API-Shape,
  Test-Grid, Blast-Radius)
- Hazard-class completeness checking
- LSP/DAP/parser contract reasoning
- API duplicate/caller searches
- Lane IDs and WIP caps
- RTK-prefix command policy
- Tests-exist-and-pass verification
- Spec/test/code semantic three-way agreement (owned by `spec-test-code-match`)
- Live GitHub state (PR status, CI checks, labels)

`cargo-allow` does not lint prose and does not execute proof. Its job is
structural: does the graph parse, are IDs unique, do links resolve, are
lifecycle transitions valid.

## Two Independent Axes: Scan Mode vs. Enforcement Posture

This plan uses two axes that sound similar but are not the same knob:

- **`cargo-allow` command/scan mode** (`audit` / `no-new` / `strict` /
  `release`) — a per-invocation flag to `cargo-allow check` that controls how
  strictly *that single run* evaluates the currently-registered graph (e.g.
  `audit` reports everything, `no-new` fails only on newly-introduced
  findings relative to a baseline, `strict`/`release` tighten further). This
  is chosen per CI job or per local run.
- **Profile enforcement posture** (advisory -> shadow -> narrow-block) — the
  phase-train-level decision (S1 through S6) about whether *any* finding
  from the `spec-system` profile is allowed to fail a build at all, and if
  so, which finding classes. This is a property of the profile config and
  CI wiring, changed rarely, one finding class at a time (see S6).

A CI job can run `cargo-allow check --profile spec-system --mode audit` (scan
mode: report everything) while the job itself stays advisory (enforcement
posture: never fails the build) — that is exactly the S1-S5 setup. Only in
S6 does a specific finding class, evaluated under whatever scan mode the CI
job uses, get to fail the build. Do not conflate "audit mode" with
"advisory posture" — a `strict`-mode scan can still run inside an
advisory-posture CI job.

## Phase Train S0-S6

S1-S6 are **explicitly deferred** until #3579 lands / a separate WIP slot
opens. (Note: #3579 — `refactor(lsp): centralize parsed state in
generation-tagged ParsedSnapshot` — merged to `main` as `b30fa1831` before
this plan was written; the landing condition is satisfied, but S1 still
requires its own dedicated PR/WIP slot rather than riding on this one.)

### S0 — Migration contract (this PR)

Goal: record the phase train, responsibility split, and governance clause as
a durable, planned reliability train.

Stop condition: this plan, the linked proposal, and tracker #3586 are merged
and cross-linked. No code, `.allow/` config, or `.perl-lsp/goals/` manifest
changes ship. No generated status is altered.

### S1 — Advisory bootstrap

Goal: run `cargo-allow init --profile spec-system --dry-run`, then create
`.allow/profiles/spec-system.toml` (the modern `.allow/` namespace, not the
legacy `policy/*.toml` used by the source-exception ledger). Register a
**small** first graph: Real Perl Editor Trust proposal/spec/plan, the
compiler-program plan + goal manifest, Fresh Facts Fast plan + goal manifest,
the support-tier surface, and the source-exception policy ledger. Add an
advisory CI job separate from the existing source-exception check.

**S1 repo roots** — the dry-run's built-in defaults create *new* files that
compete with paths that already exist in this repo (see "Grounded Tool
Facts"). S1 must explicitly point the profile config at the real layout
instead of accepting those defaults:

```toml
goals = ".perl-lsp/goals"
support_tiers = "docs/project/status/SUPPORT_TIERS.md"
artifact_ledger = ".allow/artifacts/doc-artifacts.toml"
```

**S1 requirements posture** — start from these settings, adjusted to match
whatever the actual `cargo-allow init --profile spec-system --dry-run`
output supports at S1 time (this is a starting point, not a locked
contract):

```toml
ledger_required = true
templates_required = false   # do not create a competing docs/templates/
                              # tree — canonical templates live in
                              # docs/reference/
support_tiers_required = true
active_goal_required = false # enable after the small graph resolves clean
closeout_required_for_done_items = false  # enable after closeouts are
                                           # dogfooded (S5)
```

**S1 legacy `.spec` import** — the historical `.spec` tree is registered as
a read-only *import root*, not folded into the canonical ledger:

```toml
[[import_roots.entries]]
id = "legacy-perl-lsp-specs"
path = ".spec"
ecosystem = "generic-spec"
role = "legacy"
```

Findings from this import root stay advisory; individual imported files are
never force-registered into the canonical ledger by this phase (see "Legacy
`.spec` posture" below for the backfill criteria).

Stop condition: `cargo-allow doctor --profile spec-system` reports
`Ready: true` for the registered roots; the advisory CI job runs and reports
findings without blocking merge; no more than the named small graph is
registered; the legacy `.spec` import root produces only advisory findings.

Explicit non-goal: do NOT hand-register the whole historical `.spec` tree
into the canonical ledger — it is imported read-only, advisory-only.

### S2 — Generic markdown adapter improvements (upstream)

Goal: improve `cargo-allow`'s generic markdown adapter upstream
(`EffortlessMetrics/cargo-allow`) so that: imported markdown with front-matter
uses the declared `id`/`kind` over path-derived identity; `linked_*`
front-matter normalizes into typed edges; explicit identity gets
high-confidence scoring; imports are never rewritten; duplicate IDs are
diagnosed. Ship and pin the release before any native cutover in S3.

`cargo-allow` is a separately-installed CLI, not a workspace dependency —
pinning it does not touch `Cargo.lock`. Stop condition: publish the
upstream release; pin the exact `cargo-allow` version in the repo's CI/
tool-bootstrap surface via `cargo install cargo-allow --version <VERSION>
--locked`; assert `cargo-allow --version` reports that pin in CI; rerun the
S1 graph against that version and confirm it re-validates cleanly. (Only if
`cargo-allow` is ever intentionally made a workspace dependency would
`Cargo.lock` be the right pin surface — it is not, as of this plan.)

### S3 — Native bundles for new `.spec` work

Goal: new `.spec` bundles become `cargo-allow`-native: `context.md` ->
`proposal`, `acceptance.md` -> `spec`, `checklist.md` -> `implementation_plan`,
add `closeout.md` -> `closeout`. Globally-unique IDs follow the pattern
`PLSP-SWARM-{PROP,SPEC,PLAN,CLOSEOUT}-<issue>-<run>`. Update
`docs/reference/SPEC_TEMPLATE.md`, `docs/agents/SPEC_UPDATE_CHECKLIST.md`,
the `spec-planner` agent + commands, `spec-builder.js`, the
builder/red-tdd prompts, and `spec-test-code-match` to know about the new
`closeout.md` file and the native ID scheme.

Stop condition: a newly created `.spec/<issue#>-<slug>/` bundle round-trips
through `cargo-allow check --profile spec-system` with zero structural
findings, and all named docs/agents/commands are updated to reference the
native shape. The `spec-system` contract spec's PLSP-SPEC ID is allocated
when this phase actually creates the file (checked against the current
`docs/specs/` numbering at that time) — not reserved in advance by S0.

Explicit non-goal: this phase does not touch the historical `.spec` tree
(see "Legacy `.spec` posture" below).

### S4 — Fresh Facts Fast as first native dogfood graph

Goal: Fresh Facts Fast (proposal -> spec -> plan -> goal -> phase closeouts)
becomes the first fully `cargo-allow`-native graph. This phase dogfoods **at
least one `closeout.md`** for a completed Fresh Facts Fast phase and
validates that `cargo-allow` links it back to its spec/plan — but
`closeout_required_for_done_items` stays `false` here; closeouts are proven
optional-but-working in S4, not yet enforced (enforcement is S5's job, so
S4 and S5 do not both gate on the same flag at the same time). Only after
all linked artifacts are registered and `cargo-allow explain` shows the
intended graph with no unknown-link findings does this phase set
`active_goal_required = true`.

Stop condition: `cargo-allow explain --profile spec-system` renders the
complete Fresh Facts Fast graph (proposal through at least one closeout)
with zero unknown-link findings; the dogfooded closeout links to its
spec/plan without a manual patch; `active_goal_required = true` is set only
after that check passes.

Explicit non-goal: do NOT move `.perl-lsp/goals` to `.allow/goals` in the same
PR as this dogfood cutover — that is a separate, later migration once the
dogfood graph has run clean for a full lane cycle. Do NOT set
`closeout_required_for_done_items = true` in this phase.

### S5 — Closeouts become part of "done"

Goal: update the authoring workflow (spec-planner/builder/wisdom prompts and
commands) so a `closeout.md` at merge time (merged SHA, production
entrypoint, tests/receipts, reachable behavior, fallback remaining, deferred
work, rollback, uncertainty) is produced for every newly-completed work item,
then flip `closeout_required_for_done_items = true` in the profile config so
`cargo-allow` enforces it going forward. A work item is `done` only once its
closeout exists and is linked. This phase depends on S4 having already
proven the closeout shape and linkage mechanics work (S4 dogfoods, S5
enforces — not the reverse).

Stop condition: the authoring workflow reliably produces closeouts for newly
completed work; `closeout_required_for_done_items = true` is set and
`cargo-allow check --profile spec-system` blocks (at least in shadow mode)
a newly-completed work item that lacks a linked closeout; the closeout
fields above are populated from real merge data, not placeholders, across
more than one lane.

### S6 — Shadow, then narrowly block

Goal: promote **only** objective structural findings to blocking: duplicate
IDs, missing artifact files, invalid kind/status, a registered file missing
its ID, unknown linked IDs, and malformed profile/ledger config. Everything
else (stale goals, missing closeouts, support-tier completeness, README/
release coverage) stays advisory for longer.

Stop condition: the advisory CI job has run clean (or with only advisory
findings) across at least the S4 dogfood graph for a full lane cycle before
any finding class is promoted to blocking; promotion is done finding-class by
finding-class, not as a single flag flip.

## Legacy `.spec` Posture

Assigned to **S1**: the historical `.spec` tree is registered as a
**read-only import root** (`id = "legacy-perl-lsp-specs"`, `path = ".spec"`,
`ecosystem = "generic-spec"`, `role = "legacy"` — see "S1 legacy `.spec`
import" above), not folded into the canonical ledger. Its findings stay
advisory permanently, independent of the S6 promotion train for the
canonical graph. Individual files backfill into the canonical ledger only
when:

- Currently-active specs (an open PR or in-flight build references them)
- Specs linked by an active goal manifest
- Frequently-referenced specs (cited by more than one other durable artifact)
- Historical specs already being materially edited for an unrelated reason

No mass rewrite of the old tree is in scope at any phase of this train.

## Governance Clause (long-running operating goal)

> `cargo-allow`'s `spec-system` profile is the canonical structural graph
> validator for the perl-lsp spec system. Claude still authors artifacts.
> **After S1 merges**, run `doctor`/`check`/`worklist` for the `spec-system`
> profile at session start. New governed artifacts carry stable IDs, kinds,
> owners, statuses, and typed links. A work item is not consolidated until
> its closeout is linked (enforced from S5 onward). `cargo-allow` findings
> never substitute for proof execution or live GitHub state. Legacy `.spec`
> bundles stay read-only until touched.
>
> Before S1 merges, `.allow/profiles/spec-system.toml` does not exist and
> `cargo-allow doctor --profile spec-system` reports `Ready: false` by
> design — that red readiness output is migration **evidence** for this
> plan, not a session-start gate to act on. Do not treat pre-S1 doctor
> output as an actionable failure.

## Cutover/Rollback

Promotion order: **advisory -> shadow -> narrow-block**, one finding class at
a time (see S6). Rollback at any phase: revert the `.allow/profiles/
spec-system.toml` config change, or drop the CI job back to advisory mode.
No phase depends on an irreversible data migration — the graph is derived
from markdown front-matter, not a separate database, so reverting the profile
config is sufficient to roll back validation without touching the underlying
`.spec`/`docs/` content.

## Reserved IDs

- Proposal: `PLSP-PROP-0003` (allocated by this PR — next free ID after
  `PLSP-PROP-0002`; see `docs/proposals/PLSP-PROP-0003-spec-governance.md`)
- Spec: not allocated by this PR. A prose-only ID reservation for a
  not-yet-created file is exactly the kind of invisible convention this
  migration exists to eliminate — see "Two Independent Axes" and the
  Objective above: `cargo-allow` owns identity, not chat/plan prose. When
  S2/S3 actually creates the `spec-system` contract spec, that phase
  allocates its `PLSP-SPEC-####` ID by checking `docs/specs/` numbering at
  that time (see the S3 stop condition above).

## Appendix: `cargo-allow doctor --profile spec-system` (read-only, this worktree)

```
# cargo-allow doctor --profile spec-system

**Result:** advisory
Mode: `advisory`
Status: `passed`
Profile: `spec-system`
Config: `default spec-system roots`
Config provenance: `built_in_default`

## Setup Readiness

Mode: `advisory`
Ready: `false`

| Check | Status | Path | Message |
|---|---|---|---|
| `profile_config` | `missing` | `.allow/profiles/spec-system.toml` | spec-system profile config does not exist |
| `artifact_root` | `ready` | `docs/proposals` | artifact root docs/proposals exists |
| `artifact_root` | `ready` | `docs/specs` | artifact root docs/specs exists |
| `artifact_root` | `ready` | `docs/adr` | artifact root docs/adr exists |
| `artifact_root` | `ready` | `plans` | artifact root plans exists |
| `artifact_root` | `missing` | `.codex/goals` | artifact root .codex/goals is missing |
| `artifact_ledger` | `missing` | `policy/doc-artifacts.toml` | failed to read doc artifact ledger (os error 2) |
| `support_tiers` | `missing` | `docs/status/SUPPORT_TIERS.md` | failed to read support-tier file (os error 3) |
| `active_goal` | `missing` | `.codex/goals/active.toml` | cannot be validated until doc artifact ledger parses |
| `templates` | `missing` | `docs/templates` | missing spec-system templates (proposal.md, spec.md, adr.md, implementation-plan.md, plan-item.md, closeout.md, pr-body.md) |
| `federation_ledgers` | `ready` | `.allow/config.toml` | federation ledger registry is not configured |

| Metric | Count |
|---|---:|
| Artifacts | 0 |
| Links | 0 |
| Support-tier rows | 0 |
| Findings | 2 |
| Blocking-eligible findings | 1 |
| Advisory findings | 1 |
```

This appendix is read-only evidence captured 2026-07-10 in this worktree; it
is not a claim that any of these findings are fixed or fixable by this PR.

## Work Item: spec-governance-s0-migration-contract

Status: active (this PR)
Lane: control-plane
Linked proposal: [PLSP-PROP-0003](../../docs/proposals/PLSP-PROP-0003-spec-governance.md)
Blocks: S1 (advisory bootstrap)
Blocked by: none (the #3579 timing note above is informational, not a hard
dependency of S0 itself)

Goal

Author the migration contract (proposal + this plan + tracker) so S1 can open
in its own PR/WIP slot without re-deriving the phase train from chat history.

Production delta

Docs only: two new files (`docs/proposals/PLSP-PROP-0003-spec-governance.md`,
`plans/spec-governance/implementation-plan.md`), one README index update
(`docs/proposals/README.md`), one tracking issue (#3586). No code, no
`.allow/` config, no `.perl-lsp/goals/` manifest, no generated status update,
no provider behavior change.

Non-goals

No `.allow/profiles/spec-system.toml`, no registered spec-graph artifacts, no
CI job, no native bundle mapping, no closeout template, no goal manifest.

Acceptance

Proposal and plan exist, are cross-linked, and reference tracker #3586. Proof
commands pass without workspace errors.

Proof commands

```bash
git diff --check
cargo xtask ci-hygiene check-doc-paths docs/proposals
cargo xtask ci-hygiene check-doc-paths plans/spec-governance
```

Rollback

Revert this PR. No other artifact depends on it yet — S1 has not opened.

## Future Work Items (not yet open)

- `spec-governance-s1-advisory-bootstrap` — `.allow/profiles/spec-system.toml`
  + small first graph + advisory CI job
- `spec-governance-s2-adapter-upstream` — generic markdown adapter
  improvements in `cargo-allow` upstream
- `spec-governance-s3-native-bundles` — native `.spec` bundle mapping +
  tooling updates
- `spec-governance-s4-fresh-facts-fast-dogfood` — Fresh Facts Fast native
  graph + `active_goal_required` cutover
- `spec-governance-s5-closeout-required` — closeout artifact required for
  "done"
- `spec-governance-s6-narrow-block` — shadow-then-block promotion of
  objective structural findings

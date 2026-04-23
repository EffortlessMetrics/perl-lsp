# CI Budget Discipline Archaeology
## How The Repo Turned CI Spend Into An Engineering Constraint

This note traces a specific pattern in the project docs: CI is not treated as a neutral background service. It is treated as a bounded resource that must be designed, budgeted, and defended.

The evidence is spread across the validation, cost-tracking, quality, and performance docs:

- [`docs/project/CI_TEST_LANES.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/CI_TEST_LANES.md)
- [`docs/project/CI_COST_TRACKING.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/CI_COST_TRACKING.md)
- [`docs/project/CI_LOCAL_VALIDATION.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/CI_LOCAL_VALIDATION.md)
- [`docs/project/QUALITY_INFRASTRUCTURE.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/QUALITY_INFRASTRUCTURE.md)
- [`docs/reference/PERFORMANCE_MONITORING.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/reference/PERFORMANCE_MONITORING.md)

Taken together, they show a repo that made CI budget discipline part of its operating model.

---

## 1. CI Is Structured As Lanes, Not One Blob

[`CI_TEST_LANES.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/CI_TEST_LANES.md) is the clearest statement of the lane model.

The default lanes are intentionally small:

- `Core` for fast essential tests
- `LSP` for default integration tests

Heavier lanes are gated:

- `Stress` for long-running stability checks
- `Extras` for optional protocol features
- `Security` for hangs and malformed-input edge cases

That split is an engineering decision, not a convenience choice. The repo is saying that the default merge path should stay cheap and predictable, while expensive validation exists as an explicit opt-in surface.

The same file makes the local-first posture explicit:

- `just ci-gate` is the required pre-push gate
- `just ci-full` is the deeper path for larger changes
- `nix develop -c just ci-gate` is the canonical local gate

The archaeology point is simple: the repo does not wait for CI to discover whether a change is valid. It expects validation to happen before the push.

---

## 2. Cost Tracking Is Part Of Design

[`CI_COST_TRACKING.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/CI_COST_TRACKING.md) turns the abstract lane model into a cost model.

It does three important things:

- assigns a cost to runner minutes
- estimates per-PR essential and optional job spend
- defines the savings from cancellation, path filters, and local validation

That document is unusually explicit about the operational tradeoffs:

- missing concurrency cancellation wastes minutes on abandoned runs
- expensive jobs should not run on every PR
- docs-only changes should not pay for full code validation
- local validation is the primary defense against wasted CI spend

The key historical signal is that the repo quantifies CI as a budget problem. It does not just say “be efficient.” It shows where the money goes and where the waste comes from.

The docs even frame Issue `#211` as a cost-optimization target, with savings broken down across local validation, concurrency cancellation, label gating, path filters, and caching.

---

## 3. Local-First Validation Is The Default Discipline

[`CI_LOCAL_VALIDATION.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/CI_LOCAL_VALIDATION.md) makes the philosophy explicit:

- CI is a confirmation step, not the iteration loop
- validation should run locally before pushing
- fast feedback is worth more than “discover it in GitHub Actions”
- deterministic local commands reduce avoidable CI churn

That file is important because it turns cost discipline into habit. The repo is not merely optimizing CI after the fact. It is shaping developer behavior so bad pushes are filtered out before they become remote work.

The canonical commands reinforce the point:

- `nix develop -c just ci-gate`
- `just ci-gate`
- `just ci-full` when the change is larger or riskier

This is the cheapest place to catch failure, and the repo knows it.

---

## 4. Quality Infrastructure Turns Spend Into A System

[`QUALITY_INFRASTRUCTURE.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/QUALITY_INFRASTRUCTURE.md) shows why the budget discipline exists in the first place.

The repo is enforcing more than ordinary test correctness:

- no-fatal-constructs policy
- mutation testing
- fuzz testing
- corpus validation
- supply-chain checks
- technical-debt budgets
- declarative gate policy

This matters because CI spend is only worth managing if the checks actually buy confidence. The repo’s answer is to tie cost to quality surfaces:

- fast gates catch obvious mistakes
- expensive gates are reserved for deeper confidence
- nightly lanes handle mutation, fuzz, benchmark, and coverage work
- receipts and baselines make the results durable

The architecture is cost-aware because the validation stack is expensive enough to matter. The repo therefore splits cheap confidence from expensive confidence and treats both as necessary.

---

## 5. Performance Monitoring Is Budget Discipline In Another Form

[`PERFORMANCE_MONITORING.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/reference/PERFORMANCE_MONITORING.md) extends the same design logic into runtime behavior.

The performance model is:

- run benchmarks on labeled PRs
- compare against committed baselines
- emit alerts when regressions appear
- gate on critical regressions if needed

This is the same pattern as the CI lane design:

- default path stays cheap
- heavier checks are opt-in or scheduled
- the repo uses baselines instead of ad hoc judgment
- alerts are structured artifacts, not just human memory

The archival lesson is that CI budget discipline and performance regression discipline are the same family of problem. Both require a split between everyday validation and deliberately expensive scrutiny.

---

## 6. The Repo Treats Spend As An Input To Architecture

The deeper pattern across these docs is that budget pressure shaped architecture choices:

- lane separation keeps default PR validation small
- concurrency cancellation prevents paying for stale runs
- `paths-ignore` keeps docs-only work from burning code-test minutes
- label gating reserves heavy jobs for changes that need them
- local-first validation reduces wasted remote runs
- nightly and optional lanes absorb expensive confidence checks

That is not incidental housekeeping. It is the operating model of a repo that expects a lot of machine-generated change and needs to keep review and CI sustainable.

The result is a design principle this codebase repeats in different forms:

1. make the cheap path cheap
2. make the expensive path explicit
3. make the proof durable
4. make the waste visible

That is how CI spend becomes an engineering problem instead of an afterthought.

---

## Evidence Pointers

- [`docs/project/CI_TEST_LANES.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/CI_TEST_LANES.md)
- [`docs/project/CI_COST_TRACKING.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/CI_COST_TRACKING.md)
- [`docs/project/CI_LOCAL_VALIDATION.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/CI_LOCAL_VALIDATION.md)
- [`docs/project/QUALITY_INFRASTRUCTURE.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/QUALITY_INFRASTRUCTURE.md)
- [`docs/reference/PERFORMANCE_MONITORING.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/reference/PERFORMANCE_MONITORING.md)

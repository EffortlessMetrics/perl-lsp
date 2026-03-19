# Alpha Readiness Archaeology
## How March 2026 Framed Public-Alpha Readiness Without Collapsing Truth Into Planning

This note tracks a narrow question: how the repository talked about
`v0.12.0`-era public-alpha readiness in March 2026, what counted as evidence,
what was treated as a blocker, and what was explicitly non-blocking.

The important distinction is between:

- release truth: what the repo says is true right now
- milestone planning: what the repo is trying to land next

The repo keeps those separate on purpose, and the March 2026 readiness
material follows that split.

All claims below are bounded to the evidence available in:

- `Cargo.toml`
- `docs/project/CURRENT_STATUS.md`
- `docs/project/ROADMAP.md`
- `features.toml`
- March 2026 GitHub issues and PRs relevant to release readiness
- the local March 19, 2026 readiness scout note as a lead, not as authority

---

## 1. Release Truth Stays On `v0.11.0`

The canonical truth sources still separate shipped reality from the next
milestone.

[`Cargo.toml`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/Cargo.toml)
still declares the workspace version as `0.11.0`.

[`docs/project/CURRENT_STATUS.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/CURRENT_STATUS.md)
calls the current release line `v0.11.0` public alpha and names `v0.12.0`
only as the active hardening sprint.

[`docs/project/ROADMAP.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/ROADMAP.md)
keeps the same boundary:

- current release line: `v0.11.0` public alpha
- active milestone: `v0.12.0` public-alpha hardening sprint

That separation is the first readiness signal. The repo does not pretend the
next milestone is already the shipped release.

---

## 2. Evidence Meant Receipts, Coverage, And Ratchets

March 2026 readiness was framed around evidence, not vibes.

The status doc requires evidence from:

- `Cargo.toml` for the current release line
- `nix develop -c just ci-gate` for merge-readiness
- `bash scripts/ignored-test-count.sh` for tracked test debt
- `features.toml`, capability snapshots, or targeted tests for capability truth

The current status page then anchors readiness in concrete receipts:

- `just ci-gate` is the merge gate
- parser audit receipts are explicitly named
- CPAN baseline receipts are explicitly named
- coverage baseline receipts are explicitly named
- the docs are current only if `just status-update` and `just status-check` agree

`features.toml` also matters here because it is the capability catalog, and the
computed status table shows the repo treating feature completeness as a
generated fact rather than a manual claim.

The readiness scout mirrors that structure by pointing at:

- CPAN corpus baseline
- feature completeness
- test coverage
- release status
- issue priority breakdown
- release quality gates

That is the pattern: readiness is an evidence bundle, not a slogan.

---

## 3. The Blockers Were Narrow And Specific

The readiness material treats the March 2026 blockers as parser/corpus and
version truth, not as a vague general-quality problem.

The scout note identifies the critical path as six parser issues:

- `#2189` `unexpected_rbrace_expr`
- `#2187` `expected_left_paren`
- `#2186` `expected_colon`
- `#2184` `expected_import_item`
- `#2188` `unexpected_arrow_expr`
- `#2149` `expected_left_brace`

It also says those six issues could plausibly move the corpus from `72.1%`
clean toward `80%+`.

That aligns with the roadmap: parser robustness and corpus ratchets are the
hardening sprint, and `90%+` CPAN clean parse rate is the release target.

Version truth is also a blocker until it is updated. PR `#2035` explicitly
describes the version bump as a release blocker because `perl-lsp --version`
was still reporting `0.11.0`.

So the blocker set is not broad:

- parser fixes on the critical path
- corpus ratchet progress
- version synchronization

That is a tighter definition than “everything must be perfect.”

---

## 4. Non-Blockers Stayed Explicitly Non-Blocking

The same readiness material is careful about what does not block the alpha.

[`docs/project/ROADMAP.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/ROADMAP.md)
labels several items as post-alpha or later work:

- diagnostic hardening
- refactoring reliability
- DAP hardening beyond preview
- performance, security, and API stabilization toward `v1.0.0`

The scout note adds a more specific March 2026 split:

- parser edge cases are roadmap work, not alpha-blocking
- DAP hardening is preview work with bridge mode available
- refactoring hardening is post-alpha
- diagnostic hardening is next-cycle work
- documentation articles are great-to-have, not blockers

That last item is useful because it keeps the launch-article work in the right
place. The article and archaeology work is valuable, but it is not the thing
that blocks release.

Issues `#2195`, `#2196`, and `#2197` reinforce the same boundary: they are
article issues, not release blockers.

---

## 5. Readiness Was Framed As A Burndown, Not A Pretend Ship

The readiness scout lands on a simple recommendation: ready for public alpha,
provided the parser merge wave lands, version is bumped, corpus is ratcheted,
and docs are synchronized.

That is the right framing for the repo in March 2026:

- public alpha is credible
- the repo knows which work is blocking the alpha
- the repo knows which work is post-alpha
- the repo does not confuse readiness with milestone naming

PR `#2069` is useful here because it turns release readiness into an explicit
checklist artifact. It is not the release itself. It is a release-gate
instrument.

PR `#2035` is the corresponding version-truth artifact. It is the line where
the repo says the release state itself must move, not just the roadmap.

Together they show the split clearly:

- readiness checklist: what must be true before release
- version bump: when the shipped line actually changes

---

## 6. What This Note Actually Proves

This archaeology note does not prove that 0.12.0 was already shipped in March
2026. It proves something narrower and more defensible:

- the repo treated `v0.11.0` as the truth line
- the repo treated `v0.12.0` as the active hardening milestone
- readiness evidence centered on gates, corpus, tests, and version sync
- parser fixes and corpus ratchets were the blockers
- docs/articles and other polish work were not blockers
- the repo kept release truth separate from milestone planning

That separation is one of the stronger signs that the repo is acting like an
operating system for releases, not just a set of docs about a release.

---

## Evidence Pointers

- [`Cargo.toml`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/Cargo.toml)
- [`docs/project/CURRENT_STATUS.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/CURRENT_STATUS.md)
- [`docs/project/ROADMAP.md`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/ROADMAP.md)
- [`features.toml`](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/features.toml)
- [PR #2069](https://github.com/EffortlessMetrics/perl-lsp/pull/2069)
- [PR #2035](https://github.com/EffortlessMetrics/perl-lsp/pull/2035)
- [Issue #2195](https://github.com/EffortlessMetrics/perl-lsp/issues/2195)
- [Issue #2196](https://github.com/EffortlessMetrics/perl-lsp/issues/2196)
- [Issue #2197](https://github.com/EffortlessMetrics/perl-lsp/issues/2197)

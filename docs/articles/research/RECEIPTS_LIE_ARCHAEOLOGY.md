# Receipts Lie Archaeology
## How PR #209 Turned Proof Into A First-Class Concern

This note traces one of the most important historical lessons in the repository:
proof bundles are necessary, but they are only as strong as the instrumentation
behind them.

The clearest original example is PR `#209`. It is also the example the
maintainer points to in the Q3 swarm talk and the casebook framing. The lesson
is not that receipts are fake. The lesson is that a technically true receipt can
still be operationally misleading if it measures the wrong thing or stops too
early.

---

## 1. The Principle Appears In The Talk Before It Hardens In Repo Policy

[Q3_SWARM_TALK_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/Q3_SWARM_TALK_ARCHAEOLOGY.md)
already records the warning in plain language:

- trusted change matters more than cheap code generation
- receipts outrank prose claims
- receipts can still lie if the instrument is weak

The same framing survives in
[CASEBOOK.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/CASEBOOK.md),
which treats scar stories and evidence quality as part of the repo's operating
memory rather than as embarrassing exceptions.

That gives the right lens for reading PR `#209`: not as a failure of evidence,
but as the first major case where the repo learned that evidence itself needed
governance.

---

## 2. Why PR `#209` Is The Canonical Original Example

[PR #209](https://github.com/EffortlessMetrics/perl-lsp/pull/209) merged on
`2025-10-09` as:

- `feat(dap): Phase 1 DAP support - Bridge to Perl::LanguageServer (#207)`

Its receipt surface was unusually strong for the time:

- `248` changed files
- `69,505` additions
- closes [issue #207](https://github.com/EffortlessMetrics/perl-lsp/issues/207)
- rich PR body with tests, benchmarks, policy, docs, and security claims
- Q3-era label stack including `review:stage:intake`, `flow:integrative`,
  `merge-ready`, `gate:tests (pass)`, `gate:security (clean)`, and
  `gate:policy (clear)`

The PR body made ambitious claims such as:

- `53` tests passing
- benchmark targets exceeded by large margins
- security and policy validation complete
- nearly `1,000` lines of documentation

And the surrounding commit trail shows the repo explicitly building a proof
bundle around the merge:

- `63aa3050d` `chore(governance): contract review validation for PR #209 (Issue #207)`
- `5445b566d` `feat: Add comprehensive security and test validation receipts for PR #209`
- `9ecf3acc8` `feat: Add comprehensive mutation testing summary for PR #209`

This is what makes `#209` historically distinctive. It is not just a large PR.
It is an early attempt to make a large PR mergeable by surrounding it with a
dense proof envelope.

---

## 3. What Lied Was Not The Receipt, But The Confidence It Invited

The Q3 talk's benchmark example is the concrete warning:

- the benchmark surface said the work was dramatically faster
- the measured workload was too shallow to justify the implied confidence

In other words, the receipt was technically true about the chosen benchmark and
still operationally weak as a readiness claim.

That distinction matters. The repository does not respond by abandoning
receipts. It responds by demanding better receipts:

- stronger workloads
- clearer reproduction
- machine-readable outputs
- explicit gates instead of narrative confidence

That is why `#209` belongs in the repo's casebook logic. It is the first clear
moment where "proof attached" stopped being enough by itself.

---

## 4. The Immediate Follow-Up Was Governance

The strongest proof that `#209` changed the repo is the very next policy move:
[issue #210](https://github.com/EffortlessMetrics/perl-lsp/issues/210),
created on `2025-10-13`.

Its title is explicit:

- `Formalize Merge-Blocking Gates, Receipts, and Check-Run Lifecycle for perl-lsp`

And its requested deliverables are even more explicit:

- merge-blocking CI gates
- deterministic scenario harnesses
- machine-readable `receipt.json`
- artifact uploads
- check-run lifecycle updates
- local reproduction commands

That is the critical historical step. The repo did not read `#209` and decide
"receipts worked." It read `#209` and decided "receipts need to become policy."

So the sequence is:

1. large proof bundle in `#209`
2. realization that proof surfaces need stronger structure
3. formal governance request in `#210`

That is exactly what the talk warned about.

---

## 5. March 2026 Repeats The Same Lesson On Different Surfaces

By March 2026, the repo has far better status accounting and much stronger
receipt culture. But the same lesson appears again.

[CURRENT_STATUS.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/CURRENT_STATUS.md)
shows a mature evidence posture, yet the period still produces follow-up work on
the validity of the measuring surfaces themselves:

- [PR #1950](https://github.com/EffortlessMetrics/perl-lsp/pull/1950)
  `test(perl-tdd-support): add test coverage for helper utilities`
- `2d7a5c26e` `fix(parser): accept complex expressions in use/no import lists (#2184)`
- `6c438af25` `fix(parser): keep spaced field calls callable`

And the March learning issues around parser work make the failure mode explicit:

- [issue #2190](https://github.com/EffortlessMetrics/perl-lsp/issues/2190)
- [issue #2191](https://github.com/EffortlessMetrics/perl-lsp/issues/2191)

Those issues document that `assert_clean_parse()` was missing `(ERROR ` in its
marker list while a shared `ERROR_MARKERS` constant already existed nearby in
[crates/perl-parser-core/tests/cpan_test_helpers/mod.rs](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/crates/perl-parser-core/tests/cpan_test_helpers/mod.rs).

That is the same structural lesson as `#209`, expressed on a different surface:

- the repo had evidence
- the evidence was not fabricated
- the measuring helper still had a blind spot

So "receipts can lie" evolves from a benchmark story into a broader validator
story.

---

## 6. The Historical Meaning

The lineage is not anti-receipt. It is pro-instrumentation.

This repository keeps moving toward structured proof, but it learns that proof
itself must be tested, versioned, and audited.

Read together, the record shows a stable progression:

1. the Q3 talk states the principle
2. PR `#209` creates the first canonical scar story
3. issue `#210` turns that scar into governance
4. March 2026 helper and parser repairs show the same lesson recurring at
   validator level

That is why this repo is unusually interesting historically. It does not merely
generate receipts. It keeps discovering where its receipts are too weak and
promoting those discoveries into better control surfaces.

---

## Evidence Pointers

- [Q3_SWARM_TALK_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/Q3_SWARM_TALK_ARCHAEOLOGY.md)
- [TRUSTED_CHANGE_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/TRUSTED_CHANGE_ARCHAEOLOGY.md)
- [PROVENANCE_RECEIPTS_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/PROVENANCE_RECEIPTS_ARCHAEOLOGY.md)
- [CASEBOOK.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/CASEBOOK.md)
- [CURRENT_STATUS.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/CURRENT_STATUS.md)
- [PR #209](https://github.com/EffortlessMetrics/perl-lsp/pull/209)
- [Issue #207](https://github.com/EffortlessMetrics/perl-lsp/issues/207)
- [Issue #210](https://github.com/EffortlessMetrics/perl-lsp/issues/210)
- [PR #1950](https://github.com/EffortlessMetrics/perl-lsp/pull/1950)
- [cpan_test_helpers/mod.rs](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/crates/perl-parser-core/tests/cpan_test_helpers/mod.rs)
- `63aa3050d`, `5445b566d`, `9ecf3acc8`, `2d7a5c26e`, `6c438af25`

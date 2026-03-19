# Validator Blind Spot Archaeology
## How The Repo Kept Repairing The Thing That Measured Correctness

The important lesson after PR `#209` was not only that proof matters. It was
that proof surfaces can have their own blind spots.

[RECEIPTS_LIE_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/RECEIPTS_LIE_ARCHAEOLOGY.md)
already records `#209` as the canonical scar story: a dense proof bundle can be
technically true and still operationally weak if the measurement surface is too
shallow. This note traces what happened next. The repository repeatedly had to
repair the validator/helper layer itself.

---

## 1. PR `#209` And Issue `#210` Move Proof Into Governance

[PR #209](https://github.com/EffortlessMetrics/perl-lsp/pull/209) is the first
large proof envelope in the repository:

- `63aa3050d` `chore(governance): contract review validation for PR #209 (Issue #207)`
- `5445b566d` `feat: Add comprehensive security and test validation receipts for PR #209`
- `9ecf3acc8` `feat: Add comprehensive mutation testing summary for PR #209`

That sequence matters because it is not just code plus tests. It is code plus
an explicit proof stack.

[Issue #210](https://github.com/EffortlessMetrics/perl-lsp/issues/210) then
translates that experience into governance:

- merge-blocking gates
- deterministic scenario harness
- `receipt.json`
- artifact uploads
- check-run lifecycle
- local reproduction commands

And this is not merely aspirational text left behind in the tracker. The repo
later grows the exact surfaces issue `#210` asked for:

- [.ci/gate-policy.yaml](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.ci/gate-policy.yaml)
- [.ci/receipt.schema.json](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.ci/receipt.schema.json)
- [xtask/src/tasks/gates.rs](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/xtask/src/tasks/gates.rs)
- [xtask/src/main.rs](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/xtask/src/main.rs)

Historically, that is the key shift: the repo starts treating the measuring
surface as code and policy, not as prose around a PR.

---

## 2. The Corpus Gate Becomes A Hardened Measurement Surface

One of the clearest later examples is:

- `d8c1ac325` `feat(infra): add common corpus zero-error gate`

That commit is important because it is not a parser fix. It is a fix to how the
repo measures parser reality:

- manifest-driven corpus selection
- strict `0 unreadable`, `0 errors`, and `0 ERROR nodes`
- profile-aware receipts
- explicit gate wiring

The evidence surfaces are concrete:

- `.ci/common-corpus-manifest.txt`
- `.ci/gate-policy.yaml`
- `xtask/src/tasks/parser_corpus_sweep.rs`
- `xtask gates` receipt flow

This is exactly the post-`#210` pattern: the repo is not satisfied with "run
some corpus checks." It keeps formalizing what a valid corpus measurement looks
like.

---

## 3. Helper Utilities Start Getting Tested As A Surface

Another clear step is:

- `21fccfac7` `test(perl-tdd-support): add test coverage for helper utilities (#1950)`

That change matters because `perl-tdd-support` is part of the testing
infrastructure itself. The repo stops treating helper utilities as trusted by
default and starts testing them directly:

- `must`, `must_some`, `must_err`
- panic formatting
- `#[track_caller]` behavior
- workflow-state transitions
- governance/coverage helper edges

That is a measurement-surface repair, not just more tests. It narrows the gap
between "helpers are used everywhere" and "helpers are themselves verified."

---

## 4. Parser Test Helpers Improve, But The Blind Spot Remains Visible

The most literal example of a validator blind spot is in:

- `f5b449c22` `test(parser-core): add paren recovery test coverage (#1948)`

That commit improves the shared parser test helper surface in
[cpan_test_helpers/mod.rs](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/crates/perl-parser-core/tests/cpan_test_helpers/mod.rs):

- it adds a shared `ERROR_MARKERS` constant
- it adds `assert_has_error()`
- the shared constant includes uppercase `(ERROR `

But the same file still reveals the problem:

- `assert_clean_parse()` keeps its own local marker list
- that local list still omits uppercase `(ERROR `

That is why the March 2026 learning issues are so valuable:

- [issue #2190](https://github.com/EffortlessMetrics/perl-lsp/issues/2190)
- [issue #2191](https://github.com/EffortlessMetrics/perl-lsp/issues/2191)

They document that the validator was partially repaired and still not fully
aligned. The repo had already created the better shared marker list, but the
clean-parse path had not been wired to use it.

This is the strongest single example of the repository debugging its own
measurement layer.

---

## 5. Test Assertions And Baselines Keep Getting Tightened

The same pattern appears in smaller but still meaningful changes:

- `06d1dcd18` `refactor(tests): trim imports and harden assertions (#1995)`
- `7038ba51b` `perf: establish performance baselines for 0.12.0 (#1654)`

These are different kinds of validator hardening:

- `06d1dcd18` narrows and hardens test surfaces so they are less permissive
- `7038ba51b` promotes benchmark results into an explicit baseline document and
  repeatable comparison surface

The benchmark side matters because it echoes the original `#209` lesson. If
proof can overstate readiness, then benchmark categories and baselines need to
be explicit too.

The relevant documented surfaces include:

- [PERFORMANCE_BASELINES.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/PERFORMANCE_BASELINES.md)
- [CURRENT_STATUS.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/CURRENT_STATUS.md)
- [QUALITY_INFRASTRUCTURE.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/QUALITY_INFRASTRUCTURE.md)

---

## 6. The Historical Pattern

The useful pattern is not "the repo got perfect at validation." The pattern is
that the repo kept finding new places where validation itself was incomplete:

1. PR `#209` makes proof highly visible
2. issue `#210` turns proof into policy and code surfaces
3. corpus gates get hardened
4. helper utilities get direct coverage
5. parser test helpers improve but still expose a blind spot
6. benchmark baselines get formalized instead of implied

That means the repository is not only debugging parser behavior, LSP behavior,
or DAP behavior. It is also debugging the instruments that claim to measure
those behaviors.

That is one of the more distinctive curiosities of this codebase: it keeps
promoting "the validator was wrong or too weak" into first-class engineering
work.

---

## Evidence Pointers

- [RECEIPTS_LIE_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/RECEIPTS_LIE_ARCHAEOLOGY.md)
- [TRUSTED_CHANGE_ARCHAEOLOGY.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/articles/research/TRUSTED_CHANGE_ARCHAEOLOGY.md)
- [QUALITY_INFRASTRUCTURE.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/QUALITY_INFRASTRUCTURE.md)
- [CURRENT_STATUS.md](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/docs/project/CURRENT_STATUS.md)
- [.ci/gate-policy.yaml](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.ci/gate-policy.yaml)
- [.ci/receipt.schema.json](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/.ci/receipt.schema.json)
- [xtask/src/tasks/gates.rs](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/xtask/src/tasks/gates.rs)
- [cpan_test_helpers/mod.rs](/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/crates/perl-parser-core/tests/cpan_test_helpers/mod.rs)
- [PR #209](https://github.com/EffortlessMetrics/perl-lsp/pull/209)
- [issue #210](https://github.com/EffortlessMetrics/perl-lsp/issues/210)
- `d8c1ac325`, `06d1dcd18`, `21fccfac7`, `f5b449c22`, `7038ba51b`

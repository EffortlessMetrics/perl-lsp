# Validator Blind Spot Archaeology
## How the repo kept repairing the thing that measured correctness

PR `#209` and issue `#210` are the starting point for this history. PR `#209`
showed that the repo could assemble a dense proof bundle around a large change;
issue `#210` then turned that experience into a request for merge-blocking gate
policy, deterministic harnesses, receipt files, artifact uploads, and local
reproduction. The useful historical pattern is what followed: the repo kept
discovering that the measuring surfaces themselves also needed repair.

This note records concrete examples where the validator/helper layer was
strengthened, or where a blind spot in that layer became visible.

---

## 1. PR `#209` Turned Proof Into A First-Class Surface

[PR #209](https://github.com/EffortlessMetrics/perl-lsp/pull/209) merged on
`2025-10-09` with commit trail entries such as:

- `63aa3050d` `chore(governance): contract review validation for PR #209 (Issue #207)`
- `5445b566d` `feat: Add comprehensive security and test validation receipts for PR #209`
- `9ecf3acc8` `feat: Add comprehensive mutation testing summary for PR #209`

That sequence matters because it shows the first large proof envelope in the
repo history: security receipts, test receipts, and mutation summaries were
attached to one change. The lesson was not that the proof was fake. The lesson
was that proof could be technically true and still too narrow if the measuring
surface was shallow.

[Issue #210](https://github.com/EffortlessMetrics/perl-lsp/issues/210),
created on `2025-10-13`, made that lesson explicit by asking for formal merge
gates, `receipt.json`, artifact uploads, check-run lifecycle handling, and
reproduction commands.

---

## 2. The Corpus Gate Was Hardened Into A Shared Measurement Surface

On `2026-03-10`, `d8c1ac325` added the common corpus zero-error gate.
The commit message is unusually explicit about the hardening:

- manifest mode for a pinned list of Perl modules
- strict enforcement of `0 unreadable`, `0 errors`, and `0 ERROR nodes`
- profile-aware receipt naming
- wiring into `ci-gate` and gate policy as `common_corpus_clean`

That is a clear example of the measurement surface itself being repaired. The
repo did not just keep checking the corpus; it changed the gate so the corpus
check became manifest-driven, strict, and receipt-backed.

Relevant evidence:

- `d8c1ac325` `feat(infra): add common corpus zero-error gate`
- `.ci/common-corpus-manifest.txt`
- `.ci/gate-policy.yaml`
- `xtask/src/tasks/parser_corpus_sweep.rs`

---

## 3. Parser Test Helpers Were Tightened Instead Of Reused Blindly

On `2026-03-18`, `06d1dcd18` refactored the parser test suite to trim imports
and harden assertions. The concrete change was not cosmetic: many test files
stopped carrying their own broad import sets, and the assertions were made
less permissive.

That kind of cleanup matters historically because it reduces the chance that
the helper layer itself is masking problems. It is a small but real shift from
ad hoc test scaffolding toward narrower, more explicit checks.

Relevant evidence:

- `06d1dcd18` `refactor(tests): trim imports and harden assertions (#1995)`
- `crates/perl-parser-core/tests/parser_tests.rs`
- `crates/perl-parser-core/tests/parse_error_tests.rs`

---

## 4. Helper Utilities Were Tested As A Surface, Not Just Used As One

On `2026-03-19`, `21fccfac7` added 70 tests for `perl-tdd-support` helper
utilities. The scope is important because it covers the helpers that other
tests rely on:

- `must`, `must_some`, and `must_err`
- panic formatting and `#[track_caller]` behavior
- TDD workflow state transitions
- coverage tracker boundaries
- governance scoring and validation edge cases

This is another measurement-surface repair. Instead of assuming the helper
library was trustworthy, the repo started treating the helpers themselves as a
thing worth testing.

Relevant evidence:

- `21fccfac7` `test(perl-tdd-support): add test coverage for helper utilities (#1950)`
- `crates/perl-tdd-support/tests/test_helper_coverage.rs`

---

## 5. Error-Detection Helpers Got Centralized, But The Blind Spot Stayed Visible

On `2026-03-19`, `f5b449c22` added a shared `ERROR_MARKERS` constant and a new
`assert_has_error()` helper in `crates/perl-parser-core/tests/cpan_test_helpers/mod.rs`.
That change hardened the parser test helper surface in two ways:

- it centralized the marker list for error-node detection
- it added an explicit inverse helper for malformed input
- it included uppercase `(ERROR ` in the shared marker list

The same file still shows why this note exists. `assert_clean_parse()` keeps a
local marker list that does not use the shared constant, and that local list
still omits uppercase `(ERROR `. In other words, the helper layer improved, but
the blind spot remained visible in the cleaner path.

Relevant evidence:

- `f5b449c22` `test(parser-core): add paren recovery test coverage (#1948)`
- `crates/perl-parser-core/tests/cpan_test_helpers/mod.rs`

This is the most literal example of the repository learning that a validator
can be partially repaired and still not be fully aligned.

---

## 6. Benchmark Baselines Were Promoted Into Documented History

Also on `2026-03-19`, `7038ba51b` established performance baselines for
`0.12.0`, with new benchmark categories for completion and navigation and a
new `docs/project/PERFORMANCE_BASELINES.md` file. That is a measurement-surface
hardening in the same family as the corpus gate:

- benchmark categories became explicit
- measured numbers were documented instead of implied
- the repo gained a baseline file for later comparison

Relevant evidence:

- `7038ba51b` `perf: establish performance baselines for 0.12.0 (#1654)`
- `docs/project/PERFORMANCE_BASELINES.md`
- `benchmarks/scripts/run-benchmarks.sh`

---

## Historical Reading

The useful pattern is not "the repo got perfect at measurement." It is the
opposite. The history shows a sequence of repairs where the repo discovered
that the measurement layer itself could be incomplete:

1. PR `#209` made proof a visible asset.
2. Issue `#210` turned that into gate policy.
3. Later work hardened corpus gates, benchmark baselines, parser helpers, and
   helper utilities.
4. The `assert_clean_parse()`/`ERROR_MARKERS` split shows that helper repair
   can still leave a blind spot in place.

That is the recurring archaeology lesson: the repo does not only debug parser
behavior. It also debugs the tools that claim to measure parser behavior.

---

## Evidence Pointers

- [RECEIPTS_LIE_ARCHAEOLOGY.md](RECEIPTS_LIE_ARCHAEOLOGY.md)
- [QUALITY_INFRASTRUCTURE.md](../../project/QUALITY_INFRASTRUCTURE.md)
- [CURRENT_STATUS.md](../../project/CURRENT_STATUS.md)
- `87f56b754`, `63aa3050d`, `5445b566d`, `9ecf3acc8`
- `d8c1ac325`, `06d1dcd18`, `21fccfac7`, `f5b449c22`, `7038ba51b`

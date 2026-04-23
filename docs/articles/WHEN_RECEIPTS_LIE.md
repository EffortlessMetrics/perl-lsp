# When Receipts Lie

A structured evidence culture is supposed to protect you from drift,
confabulation, and silent failure. You invest in CI gates, receipt schemas,
ratchets, computed status documents, and machine-readable proof. You establish
a rule: no artifact, no claim.

And then you discover that the artifacts themselves can be wrong.

Not forged. Not malicious. Just quietly incomplete, measuring the wrong thing,
or drifting out of sync with the system they claim to describe. The receipt
says green. The reality is red. And because you trusted the receipt, nobody
looked.

This is the story of six times that happened in the perl-lsp codebase, and
what we learned about verifying verification itself.

---

## The Promise of Receipts

perl-lsp is developed by an agentic swarm -- dozens of AI agents working in
parallel, each in its own worktree, producing PRs that flow through review,
CI, and merge queues. At scale, you cannot rely on humans to read every line.
You need structured evidence: gate receipts that prove a PR passed clippy,
test receipts that prove assertions ran, corpus manifests that prove the
parser handles real-world Perl.

The project invested heavily in this infrastructure. Gate policies are declared
in `.ci/gate-policy.yaml`. Receipts are emitted as machine-readable JSON
conforming to `.ci/receipt.schema.json`. Status documents are computed from
source, not hand-edited. Corpus baselines are ratcheted -- they can only move
forward, never backward.

This receipt culture works. It catches real regressions, prevents broken code
from merging, and lets a human maintainer trust that 50 concurrent agents are
producing valid work.

But it has a blind spot: it assumes the instruments are correct.

---

## Case 1: The Silent Tests

**56 tests that could not fail.**

The `assert_clean_parse()` helper is used across dozens of parser tests. Its
job is simple: parse a Perl snippet, convert the AST to an S-expression, and
check that the output contains no error markers. If the S-expression contains
an error node, the parse failed and the test should fail.

The helper checked for `(error` and `(Error`.

The parser emits `(ERROR`.

Uppercase. The one form the helper did not check.

This meant that 56 tests were silently passing despite the parser producing
error nodes in their output. The tests ran. They reported green. The CI gate
saw green and let the PRs through. The receipt said "tests pass." The tests
did pass -- they just were not testing what they claimed to test.

The flaw was discovered by a scout agent on March 19, 2026, during a routine
sweep of parser test infrastructure. PR #2238 fixed the helper by aligning it
with the shared `ERROR_MARKERS` constant that already existed in the same test
module. Issue #2239 documents the 56 newly-exposed failures that had been
invisible for weeks.

The irony: the codebase already had the correct marker list. Commit `f5b449c22`
had introduced a shared `ERROR_MARKERS` constant that included `(ERROR `.
But `assert_clean_parse()` was never wired to use it. The fix existed. It
just was not connected.

**What lied**: The test receipt. Every test run reported 56 additional
successes that were not real.

**Why it lied**: The validator had a case-sensitivity blind spot. Nobody
tested the test helper.

---

## Case 2: The Phantom Metrics

**An error bucket that no code path generates.**

The CPAN corpus sweep categorizes parser errors into semantic buckets -- named
categories like `unexpected_token_in_expr` or `expected_semicolon` -- so the
team can prioritize fixes by impact. The `SEMANTIC_BUCKETS` mapping classified
approximately 83 corpus files into a bucket.

The bucket did not correspond to any error the parser actually emits.

The mapping was correct syntactically. The bucket name was valid. The files
were real. But the classification was based on a misunderstanding of which
error strings the parser generates. The 83 files were being counted toward a
problem category that does not exist in the parser's error vocabulary.

The result: the project's error metrics were inflated by 2-3%. The team was
planning work to fix errors in a bucket that could never be fixed because the
errors were not what the bucket said they were.

**What lied**: The metrics dashboard. Error counts included a phantom category.

**Why it lied**: The bucket mapping was never validated against the parser's
actual error emission sites. It was plausible, so it was accepted.

---

## Case 3: The Meaningless Benchmark

**Technically correct. Operationally useless.**

An agent wrote a benchmark for a parser subsystem. The benchmark compiled.
It ran without errors. It produced timing numbers. Those numbers were
consistent across runs and showed impressive performance.

The benchmark was measuring struct construction, not parsing.

The setup phase -- creating the parser configuration and input data --
dominated the measurement. The actual parsing operation was either trivial
or absent. The benchmark was timing how long it takes Rust to allocate and
initialize a struct, which is fast and not interesting.

The CI gate saw a passing benchmark. The PR description cited the numbers.
A reviewer would have had to trace the benchmark's code path line by line to
notice that the measured operation was not the operation described in the
benchmark's name.

The flaw was caught when a human asked: "What exactly are we measuring here?"

**What lied**: The benchmark receipt. The numbers were real measurements of a
real operation. The operation just was not the one claimed.

**Why it lied**: Benchmarks are not self-describing. A benchmark named
`parse_complex_expression` that actually measures struct allocation will
produce valid, reproducible, meaningless numbers.

---

## Case 4: The Stale Version

**Everything said 0.11.0. We were shipping 0.12.0.**

Version strings appear in `Cargo.toml` files, documentation, changelogs,
README badges, and status documents. When the project moved from 0.11.0 to
0.12.0, the workspace `Cargo.toml` was updated. Many of the downstream
references were not.

No CI gate caught it.

The project had gates for clippy, tests, formatting, corpus sweeps, and
receipt schema compliance. It did not have a gate that checked whether the
version string in the README matched the version string in `Cargo.toml`.
The version was a claim made in prose, and prose claims were not computed.

The mismatch persisted across multiple PRs and multiple review cycles. Every
agent that read the README saw "0.11.0" and had no reason to question it. The
version was not wrong in a way that broke anything -- the binary still worked,
the tests still passed. It was wrong in the way that erodes trust: a user
reading the documentation would see a version that does not match the
installed binary.

**What lied**: Documentation. The README, changelog references, and status
documents all claimed a version that was no longer current.

**Why it lied**: Version strings were scattered across files with no
single-source-of-truth enforcement. The CI gate verified that the code was
correct, not that the documentation about the code was correct.

---

## Case 5: The Free Wins Nobody Claimed

**249 corpus files. Zero code changes needed.**

The CPAN corpus ratchet tracks which Perl modules parse without errors. When
a parser fix lands, newly-clean modules are supposed to be added to the
baseline manifest. The ratchet then ensures the baseline never regresses.

After a wave of parser fixes, 249 corpus files were parsing cleanly. They
were not in the manifest. The baseline said the corpus pass rate was 3-4%
lower than reality.

No code change was needed to claim this improvement. The parser already
handled these files correctly. The only missing step was running
`just cpan-corpus-ratchet` to update the manifest.

The gap persisted because the ratchet update is a manual step. Agents that
fixed parser bugs ran the parser tests and the CI gate, but the corpus
ratchet was not part of the standard verification flow. The improvement was
real, sitting on the table, and invisible to every metric the team tracked.

**What lied**: The corpus baseline. It understated the parser's actual
capability by 249 files.

**Why it lied**: The ratchet measured what had been claimed, not what was
true. The claiming step was manual and easily forgotten.

---

## Case 6: The Status Update Trap

**Agents add tests. Metrics go stale. PRs get blocked.**

The `CURRENT_STATUS.md` document is computed from test counts, corpus results,
and crate metrics. A CI gate (`policy_checks`) verifies that this document
matches computed reality. If the document is stale, the gate fails.

When an agent adds new tests, the test count changes. `CURRENT_STATUS.md`
now has the old count. The policy check fails. The PR is blocked -- not
because the code is wrong, but because the code changed a metric that a
computed document tracks, and the document was not regenerated.

This created a recurring pattern: agents would produce correct parser fixes,
add proper tests, pass clippy and all test suites, and then be blocked at the
final gate because they did not know they needed to run
`update-current-status.py` after adding tests.

The fix is to add the status update to the agent's task list. But the failure
mode is instructive: a gate designed to prevent stale documentation was
blocking correct code. The receipt (the policy check) was doing exactly what
it was designed to do. The problem was that its design created a coupling
between unrelated concerns -- parser correctness and documentation freshness.

**What lied**: Nothing, technically. The gate was correct that the document
was stale. But the gate's design made it appear that the PR had a problem
when the actual problem was a missing automation step.

**Why it trapped**: The gate conflated "documentation is current" with "code
is correct." Both are valid concerns. Coupling them in a single blocking gate
turned a documentation-refresh task into a code-review blocker.

---

## Why Receipts Lie

These six cases share a common structure:

1. **The receipt is technically correct.** The test passes. The benchmark
   produces numbers. The gate checks what it was told to check.

2. **The receipt is operationally misleading.** The test does not test the
   right thing. The benchmark measures the wrong operation. The gate checks
   the wrong invariant.

3. **The gap is invisible to automation.** No existing gate catches the
   discrepancy because the discrepancy is between what the gate measures and
   what it claims to measure. You cannot automate your way out of a
   measurement that measures the wrong thing -- unless you have a second
   measurement that checks the first.

The root cause is not carelessness. It is the natural consequence of building
verification infrastructure at speed. When dozens of agents are producing
tests, benchmarks, and documentation in parallel, the instruments accumulate
faster than the instruments-about-instruments. You get a test helper that
checks for two of three error patterns. You get a benchmark that measures
setup instead of the target operation. You get a ratchet that requires a
manual claiming step.

Each of these is a small gap. Together, they create a system where the
receipt culture -- which genuinely works and catches real bugs -- also
generates a false sense of completeness.

---

## The Fix

There is no single fix. But there are patterns that narrow the gap.

### Mutation Testing

Traditional test coverage answers: "did the test execute this line?" Mutation
testing answers: "if I change this line, does a test fail?" The benchmark
that measures struct allocation would survive mutations to the parsing code --
revealing that it proves nothing about parsing. The `assert_clean_parse()`
helper that ignores `(ERROR` would let mutants survive that inject error
nodes -- revealing that the assertion has a blind spot.

perl-lsp runs mutation testing via `cargo-mutants` and maintains dedicated
regression harnesses (`mutation_hardening_tests`,
`parser_boolean_logic_mutation_hardening`, `mutation_survivors_elimination`)
that target previously-surviving mutants. This is the closest thing to
"testing the tests."

### Oppositional Validation

Every receipt should have a corresponding negative test: a case where the
receipt *should* fail. `assert_clean_parse()` should be tested with input
that contains `(ERROR` nodes to verify that it actually catches them. A
corpus ratchet should be tested with a file removed from the manifest to
verify that regression detection works. If you cannot demonstrate that your
validation fails when it should, you do not know that it works when it does.

### Ratchets with Automation

A ratchet that requires a manual step is a ratchet with a gap. The corpus
ratchet should be updated automatically when parser tests pass, or at minimum,
the agent's task list should include the update step as a required
verification action. The status document should be regenerated as part of the
CI pipeline, not as a manual pre-merge step.

### Instrument Audits

Periodically, someone (human or agent) should ask: "What are our validators
actually checking?" The `assert_clean_parse()` bug was found by a scout
doing exactly this -- a routine sweep of test infrastructure, not a response
to a known failure. The phantom error bucket was found by tracing bucket
names back to parser emission sites. These are cheap investigations with
high-value returns.

### Decouple Orthogonal Concerns

When a single gate blocks on both code correctness and documentation
freshness, a documentation-refresh failure looks like a code problem. Gates
should be granular enough that a failure clearly indicates what is actually
wrong. The policy check should report "documentation is stale" separately
from "tests fail," even if both block the merge.

---

## Implications

The uncomfortable truth is that receipt culture creates its own failure mode.
The more you invest in structured evidence, the more you trust that evidence,
and the less likely you are to question it. A green CI gate feels like proof.
It is not proof. It is proof that the gate's checks passed. Whether those
checks measure what matters is a separate question.

This is not an argument against receipts. perl-lsp's receipt infrastructure
catches real regressions every day. Without it, the project could not operate
at the scale it does. The argument is that receipts are necessary but not
sufficient.

The lesson from these six cases:

- **Test your tests.** If a test helper is used by 56 tests, it is
  infrastructure. Test it like infrastructure.
- **Validate your validators.** If a metric bucket classifies 83 files,
  trace it back to the code that emits the error. If the error does not
  exist, neither does the bucket.
- **Name what you measure.** A benchmark named "parse expressions" should
  measure expression parsing, not struct initialization. The name is a
  claim. Verify the claim.
- **Automate the claiming step.** If an improvement is real but unclaimed,
  the gap is in the automation, not the code.
- **Audit the instruments.** The most valuable kind of review is not
  reviewing code -- it is reviewing the things that review code.

Receipts work. But they work the way locks work: they keep honest systems
honest. If the lock itself is broken, the door is open and nobody notices,
because the lock still looks locked.

Trust your receipts. Then verify them. Then verify the verification.

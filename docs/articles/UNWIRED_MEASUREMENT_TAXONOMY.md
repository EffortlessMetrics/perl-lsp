# The Unwired Measurement Taxonomy

Four ways a test, metric, or CI gate can pass every local check, be valid at
every inspection point, and still deliver nothing at all.

This article names a failure-mode family that surfaced repeatedly during the
2026-04-11 perl-lsp session. In a single wave, four distinct bugs came in
wearing four different masks, and it took us most of the day to notice they
were all the same shape underneath. Each was a concrete defect; each had a
specific owner and a specific fix. But none of them was an isolated mistake.
They were members of a category.

The category: **measurement infrastructure exists, passes its most local
check, and is not connected to the thing it claims to measure.** The test
file is on disk. The feature gate compiles. The assertion runs. The tool is
invoked. And in every case, the contextual gluing is missing, so the CI
signal is green on nothing.

We are naming the family *unwired measurement*. This article gives it a
taxonomy of four modes, each illustrated by a 2026-04-11 incident with a
specific PR or issue as evidence, and proposes a single coordinated guard
framework -- a `cargo xtask audit reality` recipe -- to catch all four.

---

## TL;DR

Four modes of unwired measurement were observed in one session:

1. **Code-level**: a test file exists on disk but is not declared with
   `mod <name>;` anywhere -- it never compiles. (PR #4079,
   `unclosed_block_recovery_tests.rs`, dormant for weeks.)
2. **Build-system level**: a test file compiles and asserts correctly but
   is gated behind `#[cfg(feature = "X")]` and no CI recipe activates that
   feature -- the test is never executed. (PR #3984 multi-root workspace
   tests: 1020 lines, 0 CI runs.)
3. **Semantic level**: a test runs, a test is green, but the assertion
   encodes a false premise about external behavior -- CI is measuring
   wrongness. (Issue #4100: `lint_pipeline_strict_inside_begin/end/init`
   asserted that `BEGIN { use strict; }` propagates strict to file scope. It
   doesn't.)
4. **CI-invocation level**: the tool supports the flag you need, but the CI
   recipe doesn't pass it -- the tool runs, but the data the scorecard needs
   is not produced, or is produced and then thrown away. (Issue #4070:
   `cargo mutants` invoked in `.github/workflows/ci-nightly.yml` without any
   artifact-upload step; nightly mutation data never leaves the runner.)

The shared root cause is that **CI going green is a necessary but not
sufficient condition for measurement to deliver value**. Each mode produces
a green signal while delivering zero information. The shared fix direction
is a coordinated reality-guard framework that audits the gluing layers, not
the tests themselves.

---

## Why This Article Exists

On a typical day, CI-bug post-mortems are one-offs. A test was wrong, a fix
went in, the team moves on. The 2026-04-11 session was not typical. Four
distinct incidents surfaced inside one working day, across four different
parts of the codebase, with four different discovering agents, each using a
different verification lens. Treating them as isolated would have missed
the pattern. Taxonomizing them turned four bug reports into one category.

The category matters because every mode is invisible to the obvious gate.
If a test file does not compile, the obvious gate is "tests pass" -- and
the tests do pass, because the orphaned file never enters the build graph.
If a feature-gated test is never activated, the obvious gate is "tests
pass" -- and the tests do pass, because the test never runs. If an
assertion encodes a false premise, the obvious gate is "tests pass" -- and
the tests do pass, because the assertion is satisfied by the buggy code.
If a CI invocation omits a flag, the obvious gate is "the tool ran" -- and
it did run, just without producing the output the scorecard wanted.

CI-as-feedback-loop can silently drift over months without anyone
noticing. This article is a first attempt to make that drift namable,
classifiable, and auditable.

---

## Mode 1: The File That Never Compiled

**Incident**: PR #4079, "test(parser): wire orphaned unclosed-block
recovery tests and add edge cases (#3496)". Merged 2026-04-11 09:42 UTC.

Inside `crates/perl-parser-core/src/engine/parser/` sat a file named
`unclosed_block_recovery_tests.rs`. 488 lines at the time of the fix, but
the original dormant version contained 6 well-crafted tests against the
parser's EOF recovery behavior. The tests had real assertions. They were
internally consistent. They looked like the kind of tests you would point
to in a code review and say "the parser team is on top of error recovery."

They had never compiled.

The reason was a 2-line omission. The parent `mod.rs` had never added
`#[cfg(test)] mod unclosed_block_recovery_tests;`. Without that
declaration, Rust does not treat the file as part of the crate. It sits on
disk; `cargo build` never reads it; `cargo test` never runs it. The tests
exist in the file-system sense and in no other sense.

PR #4079's builder discovered this while investigating the #3496 ticket.
The builder expected to fix a parser bug; instead the "fix" turned out to
be the 2-line mod declaration. The PR body reads, in part: "this file
existed on disk with 6 well-written tests but was never registered with
`#[cfg(test)] mod unclosed_block_recovery_tests;` and has never compiled
or run." On merge, the PR wired the module and added 6 additional
edge-case tests for a final count of 13 passing assertions.

For an unknown duration before the fix, every CI run reported "parser
tests pass" while the parser's EOF recovery coverage included six dormant
paper tigers. The signal was green. The evidence was false.

**How to catch it.** A scanning recipe that enumerates every `.rs` file
under `crates/*/src/` containing `#[test]` or `#[cfg(test)]` and verifies
a corresponding `mod` declaration in a parent `mod.rs` or `lib.rs`. This
is Guard A of issue #4102, and a scaffold already exists as
`xtask/src/tasks/check_test_wiring.rs`. It is almost entirely mechanical
and would have caught this incident on day one.

---

## Mode 2: The Feature Gate Nobody Flipped

**Incident**: the multi-root workspace integration tests added by
PR #3984 (merged 2026-04-10), discovered dormant during the #4068
scoping pass on 2026-04-11.

`crates/perl-lsp-rs/tests/multi_root_workspace_tests.rs` is 1020 lines of
integration tests covering multi-root workspace support for issue #3513.
Per-folder TOML configuration loading. Cross-folder module navigation.
Same-name symbol ambiguity resolution. Workspace folder removal. Hover
and definition consistency across folders. Eight named test blocks, each
one a regression anchor for a piece of functionality that the project
cares about enough to have built.

Every test block in the file is wrapped in
`#[cfg(all(feature = "workspace", feature = "expose_lsp_test_api"))]`.

No justfile recipe activates that feature combination. No
`.github/workflows/*` CI job activates it either. `just ci-gate`,
`just pr-fast`, the nightly workflow -- none of them runs
`cargo test --features "workspace,expose_lsp_test_api"` for the
`perl-lsp` crate. The file compiles to an empty test binary under the
default feature set. The binary runs. It reports "0 tests, 0 failed." CI
is green. Multi-root workspace has no regression protection.

This was discovered during the #4068 scoping pass when a plan-reviewer
was stress-testing a scout's claim that the workspace-substrate
scorecard had "4 of 6 metric coverage." The plan-reviewer went to verify
the anchors. The plan-reviewer found that the anchors existed but were
never exercised. Coverage was actually 2.5 of 6. A builder subsequently
added a `ci-workspace-multiroot` recipe to the nightly gate (commit
`a705e97e`) to unflag the dormant work. The tests now run.

The subtlety of this mode: the tests themselves are not wrong. They are
excellent tests. The scout who originally wrote them did the hard work of
designing correct assertions, setting up realistic fixtures, covering
edge cases, and structuring the code well. All of that investment was
real. What was missing was the four-line entry in the CI recipe that
tells the build system to actually include the feature flags. An hour of
work that, for weeks, was the difference between "we have multi-root
regression protection" and "we believe we have multi-root regression
protection."

**How to catch it.** A feature-flag exercise audit that cross-references
every `#[cfg(feature = "X")]` test gate against the justfile recipes and
CI workflow files. If a test is feature-gated, at least one CI recipe
must activate that feature for the test's containing crate. Flag any
feature-gated test with no CI path that runs it. This is Guard B of
issue #4102.

---

## Mode 3: The Green Light On Wrongness

**Incident**: `lint_pipeline_strict_inside_begin/end/init`, a trio of
integration tests in `perl-lsp-diagnostics`. The false-premise fix
landed in PR #4052 (merged 2026-04-11 10:22 UTC). The revert was
filed as issue #4100 and shipped as PR #4108 (merged 2026-04-11 11:39
UTC).

This mode is nastier than modes 1 and 2 because the test is not dormant
and the gate is not missing. The test runs. It runs fast. It runs on
every PR. It reports green. And the green means nothing because the
assertion encodes a claim about Perl's runtime semantics that is false.

The three tests were authored on the belief that `BEGIN { use strict; }`
propagates strict-mode to the surrounding file. That is, the author
believed:

```perl
BEGIN { use strict; }
$x = 1;   # bareword assignment
```

would be rejected by Perl under the propagated strict pragma. The tests
were written to assert that the LSP diagnostics layer matched this
behavior: a file with only `use strict;` inside a BEGIN block should
**not** trigger the `PL100` "missing strict" lint.

Direct verification against Perl 5.38.2 refutes the premise:

```bash
$ perl -e 'BEGIN { use strict; } $x = 1; print "ok: strict not active\n"'
ok: strict not active

$ perl -e 'use strict; $x = 1; print "ok\n"'
Global symbol "$x" requires explicit package name at -e line 1.
```

Perl phase blocks (`BEGIN`, `END`, `INIT`, `CHECK`, `UNITCHECK`) scope
pragmas **lexically to the body of the block**. `use strict;` inside
`BEGIN { }` applies to the BEGIN block's own body only. It does not
propagate to file scope.

The tests had been failing on master after the #4052 rebase exposed the
underlying pragma-scope behavior. Rather than ask "is the test
correct?", the builder added a `walk_node` workaround in
`crates/perl-lsp-diagnostics/src/lints/strict_warnings.rs` that scans
PhaseBlock bodies and propagates pragmas outward -- faking, at the LSP
layer, the propagation Perl does not perform. The workaround made the
tests pass. The tests were green. The fix merged. The false premise was
now baked into both the assertion and the code the assertion claimed to
verify, which is the worst possible combination, because now the test
and the code agreed with each other and neither agreed with reality.

This was caught on PR #4090 by a research-verifier (agent `a184125b`)
that ran `perl -e` directly on the claim. The verifier took about 30
minutes to produce the two-line transcript above. The orchestrator
re-verified. Issue #4100 was filed to revert the workaround and rewrite
the tests with inverted assertions. PR #4108 landed the revert. The tests
now assert the opposite of what they originally asserted, and the code no
longer carries the workaround.

The mode-3 lesson: **internal consistency is not a substitute for
external verification**. The tests were internally consistent with the
code. The code was internally consistent with the tests. The whole
subsystem was self-consistent and wrong. No number of cargo test runs
would have found this. The only tool that would find it was the Perl
interpreter itself.

**How to catch it.** Research-verifier dispatch, mandatory for any PR
whose body cites external runtime semantics -- Perl documentation, the
LSP specification, the DAP specification, external crate API behavior.
Issue #4111 tracks the process change to the reviewer-deep skill to make
this dispatch mandatory rather than optional. Unlike modes 1, 2, and 4,
this mode cannot be fully mechanized. External ground truth has to be
queried. But the trigger for the query can be mechanized -- scan the PR
body for citation keywords and dispatch the verifier if any match.

---

## Mode 4: The Tool Ran, The Output Vanished

**Incident**: the engineering health scorecard proposed by issue #4070,
specifically the per-crate mutation score row.

Scout agent `a6116a22` observed that `docs/project/status/quality.md`
publishes a single global mutation score and does not break it down by
crate. The fix direction looked cheap: `cargo mutants` already writes a
`mutants.json` file containing per-mutant results, so all a scorecard
generator has to do is parse that JSON and produce a per-crate histogram.
Accuracy-scout `aa50278a` verified the shape of the JSON. Plan was
clear.

Plan-reviewer `a9909643` then opened the nightly CI workflow to see how
the existing `cargo mutants` invocation was set up. The relevant line, in
`.github/workflows/ci-nightly.yml` at line 75:

```yaml
- run: cargo mutants --timeout 60 --no-shuffle || true
```

There is no `--json` flag and no `--output` flag. The tool supports per-
crate machine-readable data. The CI recipe was not asking for it. The
scorecard generator would have run against the last-good cached JSON (or
nothing) and silently reported a placeholder number. CI would stay green.
The engineering health dashboard would display a value that was stale at
best and synthetic at worst. And the team would believe it.

During the subsequent implementation pass, builder `a717174b` discovered
a second layer of the same mode: `--json` in `cargo mutants` actually
applies only to `--list` mode; the full mutation run always writes
`mutants.json` automatically. So the `--json` flag the plan-reviewer
proposed was itself partially wrong. The real fix was different: the
nightly workflow needed an `actions/upload-artifact` step to preserve
`mutants.json` after the job ended. The tool was writing the file. The
file was being deleted when the runner shut down. Neither the CI recipe
nor the scorecard generator would ever see it.

Three layers of verification each corrected the previous layer's partial
truth. The scout was broadly right (per-crate data exists). The accuracy-
scout was narrowly right (the JSON has the right shape). The plan-
reviewer was right about the category of bug (CI is not asking for what
it needs) but wrong about the specific flag. The builder was right about
the specific fix. Every layer caught a real problem and each layer was
necessary.

**How to catch it.** A CI-invocation audit that cross-references every
CI tool invocation against the tool's documented flag and output
behavior, and flags cases where the CI is producing a strict subset of
the tool's available output modes. This is Guard D, a proposed extension
to issue #4102. It is mechanical but requires some tool-specific
knowledge; a first version can simply catch the most common patterns
(missing `--json`, missing `--output`, missing artifact upload steps).

---

## The Shared Root Cause

All four modes have the same underlying shape:

- The measurement investment exists.
- The measurement investment passes its most local check.
- A layer of the production pipeline above the test is not connected.
- The CI signal is green on nothing.

Unpacked:

| Mode | Local check that passes | Layer that is unconnected |
|------|-------------------------|---------------------------|
| 1    | File exists on disk     | Rust build graph (`mod` decl) |
| 2    | File compiles under some feature set | CI recipe for that feature set |
| 3    | Assertion runs and is satisfied | Assertion to external truth |
| 4    | Tool executes successfully | Tool output to consuming scorecard |

Each mode corresponds to a gluing layer that is invisible to the layer
below it. The test file does not know whether it is wired to `mod.rs`.
The feature-gated test does not know whether CI activates the feature.
The assertion does not know whether the premise it tests is true in the
real world. The tool invocation does not know whether its output is
being captured. Each layer happily reports success in its own terms, and
success in its own terms does not propagate upward into success in the
system as a whole.

The shared insight is small and uncomfortable: **green CI is necessary
but not sufficient for measurement to deliver value**. Every mode in this
taxonomy produces green CI while delivering zero measurement. That is why
CI-as-feedback-loop can drift for months.

---

## Why the Session Surfaced All Four in One Wave

This was not coincidence. The 2026-04-11 session had multiple verification
layers active simultaneously across unrelated pieces of work, and each
layer has a different lens.

- **Mode 1** (code level) was caught by a builder investigating an
  unrelated issue. The builder ran `cargo test` and the test file
  compiled for the first time. The builder-level lens is "does the code
  do what the file claims to do?" -- and for a file that does not
  compile, that lens exposes the problem the moment the builder tries to
  run the file.

- **Mode 2** (feature-gate level) was caught by a plan-reviewer stress-
  testing a scout's coverage claim. The plan-reviewer lens is "is the
  claim supported by the actual state of CI?" -- and cross-referencing
  an anchor to an active CI recipe surfaces dormant features.

- **Mode 3** (semantic level) was caught by a research-verifier running
  `perl -e` against the false premise. The research-verifier lens is
  "is the external claim actually true?" -- and no amount of internal
  verification catches this class of bug.

- **Mode 4** (CI invocation level) was caught by a plan-reviewer reading
  the ci-nightly.yml file after an accuracy-scout had verified only the
  tool behavior, and then refined by a builder who discovered that the
  flag the plan-reviewer proposed did not actually exist. The
  plan-reviewer lens here is "does the recipe produce what the scorecard
  consumes?" and the builder lens is "does the flag I am invoking do
  what I think it does?"

Each layer caught a mode that the other layers were structurally blind
to. A single reviewer could not have caught all four. A single test
could not have caught all four. It took the diversity of
verification-lenses from the layered-verification protocol to surface
the full set. The taxonomy is legible today precisely because the
protocol pulled all four modes into view at once.

---

## The Reality Guard Framework

Issue #4102 originally proposed three guards (A, B, C) targeting modes
1, 2, and 3 in isolation. This article proposes rolling them into a
single recipe alongside Mode 4's proposed Guard D, all under a common
name:

```
cargo xtask audit reality
  ├── test-wiring          (Mode 1)
  ├── feature-exercise     (Mode 2)
  ├── assertion-semantics  (Mode 3, trigger only)
  └── ci-invocation-completeness (Mode 4)
```

Each sub-guard corresponds to one mode of the taxonomy:

- **test-wiring**: every `.rs` file under `crates/*/src/` containing
  `#[test]` or `#[cfg(test)]` must have a corresponding `mod`
  declaration in a parent `mod.rs` or `lib.rs`. Scaffold already exists
  at `xtask/src/tasks/check_test_wiring.rs` per issue #4102.

- **feature-exercise**: every `#[cfg(feature = "X")]` test gate in a
  crate's source or tests must have at least one CI recipe (justfile
  recipe or GitHub Actions workflow) that runs `cargo test
  --features "X"` for that crate. Flag any feature-gated test with no
  CI path.

- **assertion-semantics**: scan PR bodies for external-citation
  keywords (perlmod, perlop, LSP spec, DAP spec, docs.rs, crate name).
  If any match, require research-verifier dispatch before merge-ready.
  This is the only sub-guard that cannot be fully mechanized -- it
  triggers a manual verification step rather than performing it. Tracked
  in issue #4111.

- **ci-invocation-completeness**: parse every CI tool invocation and
  cross-reference against the tool's `--help` output, flagging cases
  where the CI is strictly reducing the tool's output modes (no
  `--json`, no `--output`, no artifact upload) despite having a
  downstream consumer that needs the full output.

The framework name matters. **Reality guard** signals that the purpose
is to catch cases where the CI signal has drifted from reality, not to
add more CI gates for their own sake. Not every test file needs a
`mod` declaration, but every test file with tests in it does. Not
every feature-gated test needs a CI recipe, but every feature-gated
test that is intended to measure something does. Not every assertion
needs external verification, but every assertion about external
behavior does. Not every CI invocation needs `--json`, but every
invocation whose output is consumed by a scorecard does.

The guard is not "more strictness." It is "the minimum correctness
check that would prevent this specific green-on-nothing failure
mode."

---

## Open Questions

**Can Mode 3 be further mechanized?** Today's proposal triggers a
research-verifier on keyword match. A stronger version would parse the
PR's claimed behavior and drive an external reference implementation
(Perl, a live LSP, a live DAP) to confirm the claim. This is
experimental agentic territory and may not be worth the cost until we
have two or three more Mode-3 incidents.

**Are there more modes we haven't seen yet?** The taxonomy currently
has four because the 2026-04-11 session surfaced four. There may be a
Mode 5: output exists, is captured, but is not rendered in the
dashboard. Or a Mode 6: dashboard is rendered but nobody reads it. The
framework should be open to extension. If a future incident doesn't fit
any of the four modes, the taxonomy needs to grow.

**How do we prevent the taxonomy itself from becoming unwired
measurement?** This is not a joke. A taxonomy article that names four
guards, proposes a framework, and then sits in `docs/articles/`
unreferenced by any CI recipe would itself be a mode-1 failure at the
documentation layer. The article's claim ("we track four unwired
measurement modes") would be green in the sense that the file exists,
and zero in the sense that the modes are not being tracked. The fix is
to link the article from the guard code, so that anyone adding a new
guard is walked through the taxonomy, and anyone editing the taxonomy
is reminded to update the guards.

---

## Cross-References

- Issue **#4102** -- the original regression-guards issue, proposes
  Guards A, B, C. This article extends to Guard D and names the family.
- Issue **#4111** -- the mandatory research-verifier dispatch for
  external-semantics claims; Mode 3's process-level fix.
- Issue **#4100** -- the revert of the false-premise
  `strict_warnings` workaround; Mode 3's code-level fix.
- Issue **#4070** -- the engineering-health scorecard that surfaced
  Mode 4 during its plan-review pass.
- Issue **#4068** -- the workspace scorecard that surfaced Mode 2
  during its plan-review pass.
- PR **#4079** -- the unclosed-block-recovery wiring; Mode 1's fix.
- PR **#4108** -- the lexical pragma revert landing issue #4100's fix.
- PR **#3984** -- the multi-root workspace tests added 2026-04-10,
  dormant until a day later.
- PR **#4052** -- the diagnostics rebase that landed the false-premise
  workaround later reverted by #4108.
- PR **#4090** -- the false-premise PR caught by research-verifier,
  closed 2026-04-11.
- Protocol: `docs/project/protocols/verification.md` -- the layered
  verification protocol whose diversity of lenses is what surfaced all
  four modes in one wave.

---

The one-sentence version: **tests that can't fail, features CI doesn't
flip, assertions that disagree with reality, and tools whose output is
thrown away are four different bugs with one root cause, and green CI is
not enough to catch any of them**.

The one-word version: *unwired*.

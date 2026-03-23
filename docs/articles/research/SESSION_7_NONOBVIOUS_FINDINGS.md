# Session 7: Non-Obvious Findings

Things that are apparent now with full context but would be missed looking at the git log later.

## 1. Worktree contamination has a specific vector

AGENTIC_ECONOMICS_DATA.md appeared in **3 unrelated PRs** (#2884, #2887, #2894). The vector: builder agents run `git pull` in the main checkout instead of their worktree, picking up commits from other agents. The economics file was committed directly to local master by an earlier agent, then propagated into every worktree that pulled.

**Why:** This isn't a random bug. Agents that run `git pull origin master` in the wrong directory will always contaminate. The fix isn't "be careful" — it's the preflight check in `/agent-preflight` that verifies `pwd` matches the worktree path. Without that check, this will recur every session.

## 2. The "already-fixed" rate reveals pipeline timing

7 issues were found already-fixed during this session. That's not waste — it's evidence that **the pipeline moves faster than the issue tracker**. PRs merge, but the issues stay open because the closing agent hasn't run. Research-verifiers catch this and prevent duplicate builds.

**Pattern**: The optimal number of "already-fixed" findings is NOT zero. Zero means you're not checking. The cost of building something already done (~2% session for a builder) is much higher than the cost of checking (~0.25% for a verifier).

## 3. Deep review improvement rate was literally 100%

Every single deep review this session pushed improvements to the PR branch. Not "found minor style issues" — pushed real code: edge case tests, bug fixes, restored deleted files, strengthened assertions.

**Why this is non-obvious**: In most code review cultures, "LGTM" is the common outcome. Here, 0% of PRs got LGTM. That suggests either (a) builder quality is consistently below what deep review expects, or (b) deep review is finding real depth that builders can't reach in their first pass. Evidence supports (b) — builders produce correct main-path code, but deep review catches edge cases, vacuous tests, and subtle composition bugs.

## 4. The vacuous test pattern is systemic, not accidental

7+ vacuous tests found across 4 unrelated PRs by different builders. The patterns:
- Balanced braces always net zero → integration tests pass without the fix
- `let _: bool` → result discarded, test can never fail
- OR conditions → input payload satisfies assertion, not the template
- Tests pass when feature is disabled → silently test nothing

**Why:** Builders write tests to demonstrate the fix works, not to prove the absence of the fix breaks things. The TDD "red phase" should catch this, but builders often write the test alongside the fix, never seeing it fail. Deep reviewers test deletion scenarios ("would this test fail if I reverted the fix?") which is a fundamentally different verification.

**Recommendation**: Add a `/verify-non-vacuous` step that runs new tests against the pre-fix code. If they pass, they're vacuous.

## 5. Plan-reviewers rejected 100% of scout option-lists

Every plan-review that received a scout spec with "Option A / Option B / Option C" rejected at least one option and often all three. Examples:
- #2090: All 3 options rejected (single-letter triggers would fire on normal typing)
- #2088: Sigil-family linking rejected (would cause live co-editing of wrong tokens)
- #2084: "Missing traversal" framing rejected (it's a wiring problem, traversal already exists)

**Why:** Scouts explore breadth. They enumerate possibilities. Plan-reviewers evaluate depth. They trace code paths and find that Option A would cause a regression, Option B is already done, and Option C requires an architecture change. The plan-review stage isn't optional polish — it's where bad approaches die before they waste builder time.

## 6. Merge conflicts correlate with session velocity

4 merge conflicts occurred this session, all from PRs that were in review while other PRs merged to master. At 92 PRs/session, this is a ~4% conflict rate. The conflicts were all in parser files (which many PRs touch).

**Pattern**: High-velocity sessions need the cherry-pick-to-fresh-branch approach, not rebase-in-place. The hook blocks `git push --force`, so rebasing requires creating a new branch anyway. Accept this and build it into the ops flow.

## 7. The CI "failure" was a governance issue, not a code issue

The CI red that alarmed the user was entirely from the post-merge status regeneration step trying to push directly to master (blocked by branch protection). The actual build/test/clippy gates were green.

**Why this matters**: CI red creates urgency. But the fix was a governance change (make the push non-fatal), not a code fix. Future sessions should check WHICH CI step failed before treating all red as equal.

## 8. Disk economics are non-linear

Worktree creation is cheap (~100MB per worktree for this repo). But **cargo builds in worktrees share target/**, so the first build in any worktree triggers recompilation. With 20+ agents doing `cargo test`, target/ can grow to 15GB+.

The session went from 1.1TB free → 114MB free → worktrees cleaned → 93GB free → builds ran → 30GB free → partial cargo clean → 1.1TB free.

**Lesson**: `cargo clean -p perl-parser -p perl-lsp` between build waves frees ~6GB without losing the full dependency cache. Better than full `cargo clean` (rebuilds everything) or no clean (disk fills).

## 9. The economics data became scope creep itself

The AGENTIC_ECONOMICS_DATA.md file that contaminated 3 PRs was the economics research document from the PREVIOUS session. Ironic: the documentation about agent efficiency caused agent inefficiency. The file was 688 lines committed directly to local master by a session 6 agent that didn't use worktree isolation.

**Meta-lesson**: Documentation agents need the same worktree isolation as code agents. No exceptions.

## 10. The "publish tomorrow" deadline shaped everything

The entire session was organized around one constraint: v0.12.0 ships tomorrow. This is why:
- Discovery was minimal (6 scouts, not 30)
- Plan-review was focused (only launch-relevant issues)
- Building was targeted (close gaps, not explore frontiers)
- Review was thorough (can't ship bugs on launch day)

Without the deadline, this session would have been another exploration cycle. The deadline converted it into a finishing sprint. That's the real economics lesson: **the constraint was the strategy**.

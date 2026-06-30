# Trustworthy scoreboard: encode failure as first-class as success

**Pattern.** A scoreboard you can trust is one that makes the BROKEN as explicit and mandatory as the WORKING. An instrument that only records successes — where "not done" is the implicit default and someone has to remember to mark a thing broken or closed — cannot be trusted, because the gap between the record and reality grows *silently*. The honest instrument is loud about failure.

**Two instruments, opposite trustworthiness (perl-lsp-swarm).**
- The **dogfood test suite** (`crates/perl-lsp-ux-tests/tests/ux_scenario_*`) is ground truth: 213 tests where a working provider behavior is a hard `assert!` regression guard and a broken one is `#[ignore]` + a filed issue (5 such gaps, each tracked — e.g. `#[ignore = "real gap — incomingCalls returns empty for OO method callers; tracking #3093"]`). The negative space is first-class: you cannot have a broken behavior the suite is silent about, because "broken" is an explicit, compiled, listed state.
- The **GitHub issue board** is the least trustworthy instrument in the same repo, for the opposite reason: "done" is implicit and manual, so nobody flips the bit — it lags ~45% (done-but-unclosed). A *clean-looking* board is a *lying* board.

**Why the asymmetry matters.** A success-only instrument fails *invisibly by construction* — you can't see what it forgot to record — so it degrades toward optimism and always looks better than reality. A both-states instrument fails *visibly* — a broken entry is right there in the list — so it degrades toward honesty. When you must choose which instrument to believe, believe the one that's loud about what's broken, even if it's messier than the clean official tracker.

**How to build one.**
- Make BROKEN a compiled, listed, mandatory state — not an absence. In a test suite: working → hard assertion guard; broken/unimplemented → `#[ignore = "<gap>; tracking #N"]` + the issue. The ignored test *is* the tracked gap; you can't lose it, and it shows up every run.
- A passing test that would still pass if the behavior were wrong is the same disease one layer down — a success recorded without verification. Assert the right *content*, with a positive control where the negative would otherwise be vacuous (assert a named thing IS collected, so asserting an anonymous one is NOT becomes discriminating). See `external-truth-gate.md`.
- Treat the loud-about-failure instrument as the backlog: its broken entries are the real, ground-truth work list (the 5 ignored dogfood tests are the actual product gaps), each closeable by fixing the behavior → un-ignore → the guard locks it forever.
- Distrust any "clean" report whose cleanliness depends on someone remembering to annotate. Prefer instruments where state is *derived or forced*, not hand-maintained.

**The deeper point.** This connects to why an aggregate view beats any single signal: you only learn an instrument is lying by holding its claim next to ground truth. The dogfood suite earns trust because it carries both — the green guards *and* the ignored gaps — in one place. A scoreboard that cannot show you its own failures cannot show you anything.

Related: `external-truth-gate.md` (CI verifies consistency, not truth), `enforcement-over-doctrine.md`, `gate-names-must-match-failure-classes.md`, `doctrine-is-a-hypothesis.md`.

# Session 3 Learnings

*Cycle 5, Session 3 -- 2026-03-20. Quick-scan format for future sessions.*

---

## Process Learnings

1. **Research-then-build = 90% success rate.** 40 scouts --> 30 targeted PRs. Unconstrained builders = 50%. Scouts cost 5 min, save 30 min per builder. *Evidence: 11 parser PRs, 12 feature PRs, nearly all merged successfully.*

2. **Parallel lanes beat sequential phases.** Run research+build+merge+document simultaneously from session start. Dynamic ratio: 30/30/20/20. *Evidence: 38 merges + 55 PRs + 15 articles in one session.*

3. **External analysis finds invisible framings.** ChatGPT found "three-layer product" that 40 internal scouts missed. Budget 10 min/session for outside-in review. *Evidence: Three-layer product, quality-before-cheapness, agent/skill split correction.*

4. **False positives from audits are cheaper than missed real issues.** 2 false positives (30 min each) vs 3 critical findings (56 silent tests, 260GB waste, god file). Don't pre-filter. *Evidence: unwrap audit false positive, incremental sync false positive, assert_clean_parse real bug.*

5. **Don't broadcast shutdown.** Broadcasting wind-down to 117 agents consumed 6% of context for zero value. Idle agents are free. Just stop sending messages. *Evidence: Cycle 5 session 2 context waste.*

## Scale Learnings

6. **60 agents are sustainable** when balanced across 4 lanes. Key: don't over-index on any one lane. Builders outpace merge 3:1 (expected). *Evidence: Session sustained 60+ agents for 7 hours.*

7. **The merge queue (3-wide) is the real ceiling**, not agent count. While CI runs (5 min), builders work. While builders code (30 min), scouts complete 6 rounds. *Evidence: PR count peaked at 64 during build wave.*

8. **Scout:builder:reviewer ratio = 4:1:2.** This ratio emerged naturally, not from planning. Matches Steven's stated preference. *Evidence: 40 scouts, 15 builders, mix of reviewers/improvers.*

## Discovery Learnings

9. **"Built but not wired" is the highest-ROI scoutable pattern.** 5 pieces of fully-built infrastructure found disconnected. Each fix = 10-50 lines. *Evidence: Logos lexer, dead code detector, incremental parsing, heredoc detector, Moose resolver.*

10. **Verify audit findings before acting on them.** Wide-net audits at 20% false positive rate are still net positive. A builder discovering "nothing to fix" is a valid, cheap outcome. *Evidence: unwrap count audit, incremental sync audit.*

11. **Validators need validating.** `assert_clean_parse` had a case-sensitivity bug silencing 56 tests. After fixing, the trust envelope for all prior parser correctness claims changed. *Evidence: PR #2238.*

## Architecture Learnings

12. **Three-layer product is the strategic position.** Layer 1: LSP (user-facing). Layer 2: swarm OS (team-facing). Layer 3: memory/evidence (process-facing). Work on any layer is productive. *Evidence: ChatGPT external analysis.*

13. **Quality came before cheapness.** The process was solid before the swarm. The swarm externalized it -- made existing quality cheap, didn't add new quality. *Evidence: Zero-panic policy, 8:1 test ratio, mutation testing at 87%.*

14. **The methodology was always trying to exist.** Era 4's monolithic `/fleet` prompt had all the right ideas. Era 5 decomposed it when Claude Code exposed the primitives. *Evidence: Era archaeology, five-eras analysis.*

## Operational Learnings

15. **Promotion matters more than storage.** Need tighter promotion convention: pitfall --> finding --> issue --> article evidence --> archaeology. *Evidence: 156 memory files, many at wrong promotion tier.*

16. **`gh pr list` defaults to 30 results.** This hid 65 PRs. Always use `--limit 200` or equivalent. *Evidence: Session 3 opening discovery.*

17. **The cheapest improvement is running `just cpan-corpus-ratchet`.** 249 corpus files improved for zero code changes. Free gains from a forgotten manual step. *Evidence: Ratchet run during session.*

18. **Memory is now load-bearing infrastructure.** 156 files encoding institutional knowledge. Future sessions start smarter because past sessions left knowledge behind. Needs quarterly consolidation. *Evidence: Session 3 memory consolidation pass.*

---

## One-Line Thesis

**Research+build+merge+document in parallel with external review, and don't filter audits. The invisible ROI is in the constraints you don't wait on.**

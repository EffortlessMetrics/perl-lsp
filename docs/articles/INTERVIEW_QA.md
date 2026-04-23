# Interview Q&A: perl-lsp and AI-Native Development

*Lightly edited for clarity. Voice preserved.*

---

**Q: How did this start?**

tree-sitter-perl was causing issues for my internal AST context packing engine. I should have just turned Perl off. I don't use Perl. But for some reason I decided to fix it instead. And then fixing tree-sitter-perl turned into writing a parser, and the parser needed tests, and the tests needed a corpus, and the corpus needed real-world Perl, and at some point I had a language server.

---

**Q: Why Perl of all languages?**

Perl is the most popular language that doesn't have proper language tooling. There are millions of lines of Perl in production at major companies, and the developer experience for maintaining those codebases is stuck in the 2000s. No reliable go-to-definition, no real-time diagnostics, no refactoring tools. If you're going to build a language server, Perl is where the gap is widest.

---

**Q: Why do people keep asking "but why Perl?"**

The biggest question I get is "but why Perl?", which is weird, because I don't get that reaction for my COBOL tooling. Nobody says "but why COBOL?" They just nod. With Perl, people assume it's dead, which it isn't -- it's just unfashionable. There's a difference between a language nobody writes anymore and a language nobody talks about at conferences anymore.

---

**Q: How long did tree-sitter last?**

Zero days. I started with Pest, actually. And Pest couldn't handle Perl's undecidability. The language is context-sensitive in ways that break PEG parsers -- the same character means different things depending on parser state, and you can't encode that in a grammar file. So we went to a hand-written recursive descent parser. That's where we are now. It handles the ambiguity natively because you can just... write the logic.

---

**Q: 130 crates -- isn't that extreme?**

Many small focused SRP crates with stable APIs is extreme for a human to maintain. It's simple and searchable and context efficient and well routed for an AI. Each crate has a single responsibility, a stable interface, and tests that verify that interface. An agent working on the lexer doesn't need to load the LSP server into context. An agent working on module resolution doesn't need to know about the DAP adapter. The crate boundaries are the context boundaries.

---

**Q: What accounting principles do developers miss?**

It's not accounting that matters here. It's controls and materiality. In accounting, you don't verify every transaction -- you verify that the controls are sound and focus your attention on what's material. Software has the same structure. You can't review every line an agent writes. But you can verify that the CI gates catch regressions, the test suite covers the important paths, and the review process is adversarial. If your controls are sound, the output is trustworthy even when you haven't read every line.

---

**Q: What does "best Perl LSP" look like?**

I think we're already there, no? At this point it's about finding ways to make a better user experience. The parser handles 85%+ of CPAN. Go-to-definition works. Diagnostics are real-time. The debug adapter connects. The question now isn't "can we parse Perl" -- it's "how do we make maintaining a 200,000-line Perl codebase feel like maintaining a modern TypeScript project."

---

**Q: If you could restart, what would you change?**

I wouldn't. It was a mistake. I should not have started. I don't write Perl. I had no reason to build this. But I still can't put it down. There's something about perl-lsp. Something about making legacy maintenance and maintainership easier. Something about proving that if you build the right tooling, even the languages people have given up on become workable again.

---

**Q: Tell me about the AI development side. How much of this was written by agents?**

All of it. Every line of Rust was written or directed by AI agents under human supervision. I set direction, review output, and make architectural decisions. The agents do the implementation. We've gone through five distinct eras of AI development on this project -- from single-conversation pairing to 100-agent parallel swarms. The git history records all of it.

---

**Q: 100 agents in parallel -- what does that even look like?**

Each agent gets its own git worktree, its own task, and its own verification step. They don't coordinate with each other directly. The microcrate architecture means they rarely touch the same files. An agent fixing a parser ambiguity in the lexer crate is completely isolated from an agent adding a new LSP feature. They produce PRs, the PRs go through review and CI, and the ones that pass get merged. The ones that don't get closed. It's embarrassingly parallel.

---

**Q: What's the failure rate?**

Constrained tasks -- where a scout agent has already identified the exact file, function, and line that needs to change -- succeed at about 90%. Unconstrained tasks -- "fix the unexpected token error bucket" -- succeed at about 50%. That delta is the entire methodology. The difference between a system that works and one that wastes half its compute is the quality of the input specification.

---

**Q: You mentioned scouts. What are those?**

A scout is a read-only agent. It doesn't write code. It spends 60 seconds tracing an error to its root cause -- the exact function, the exact line, the exact failing input. Then it writes a GitHub issue with everything a builder agent needs to implement the fix. The scout's output IS the constraint. We discovered early on that vague instructions produce vague code. Precise instructions produce precise fixes.

---

**Q: What's the hardest part about parsing Perl specifically?**

Larry Wall said "only perl can parse Perl," and he wasn't exaggerating. The character `/` is either division or a regex depending on what the parser just saw. Curly braces could be a hash reference, a block, or a bare block. The word after `->` could be a method call or a hash key. `use constant` changes how identifiers are parsed for the rest of the file. And source filters can rewrite code before the parser even sees it. A static parser -- one that doesn't execute Perl -- can never be 100% correct. The goal is correct enough for real-world IDE features.

---

**Q: How do you measure "correct enough"?**

We parse the CPAN corpus. 4,355 real-world Perl files from published modules. The parse rate ratchets -- CI blocks any change that would lower it. We started at around 50%. We're at 85.4% on the full corpus baseline, and 90.9% clean on the lib-file sweep after recent parser fixes. Every session either improves the number or leaves it unchanged. Regressions are structurally impossible.

*Updated 2026-03-21: baseline 85.4% (3,717/4,355 files), manifest 2,052 clean modules. The March 21 session merged fat-arrow (#2613) and defined/ref (#2626) parser fixes — the lib-file sweep shows 90.9% clean (3,077/3,386). The baseline JSON will reset on next ratchet run.*

---

**Q: What's the broader point here? Why does this project matter beyond Perl?**

The Perl part was the sharp edge. The thing that actually matters is: can you build and maintain production-quality software using AI agents as the primary workforce, with a human directing strategy? We think the answer is yes, but only if you solve the institutional knowledge problem. An agent that starts from scratch every session is expensive and unreliable. An agent that inherits 100+ memory files, a library of verified skills, and hook-based enforcement is fast and predictable. The methodology compounds. That's the thing worth studying.

---

**Q: What surprised you most?**

That the swarm improves itself. We allocate about 20% of capacity to self-improvement -- agents that fix the development process rather than the codebase. They update skills, add enforcement hooks, write memory files. The 50th session is structurally better than the 1st, not because the agents are smarter, but because the environment they work in has been refined by every previous session. That feedback loop is the part nobody talks about.

---

**Q: How is this different from "vibe coding"?**

Vibe coding is prompting an AI, accepting the output, and shipping it. It works for prototypes. It does not work for production software. The failure mode is specific: the code compiles and runs, but nobody verified it handles edge cases or won't regress next week. We're the opposite. Every change goes through formatting, linting, a test suite, a review agent, and CI. Mutation testing verifies that the tests would catch real bugs. At no point does the agent that wrote the code get to decide whether the code is correct.

---

**Q: What would you tell someone starting a similar project?**

Exploration and planning are cheap. Building is expensive. Invest heavily in understanding the problem before you start generating code. Use scouts. Write issues that are precise enough that a builder agent can execute them without guessing. Break features into constraint-shaped slices. And accept that the first version of your process will be wrong -- the point is that it gets better every cycle.

---

*The story of perl-lsp is not "person loved Perl." It is: a broader tooling system hit Perl as the sharp edge, and fixing that sharp edge turned into a parser, then a language server, then a proving ground for AI-native maintainership.*

---

## Session 3 Discoveries: Q36-Q57

*Questions generated from 60+ agents spending a full session scouting, building, and reviewing the codebase. Answers from the follow-up session.*

---

**Q36: The assert_clean_parse bug exposed 56 silently passing tests. How does it feel to discover your own testing infrastructure was lying to you?**

Stepwise orchestration with context-efficient ordering is transformative to both code quality and speed. What that bug revealed is that the problem wasn't the tests themselves -- it was the order of verification. When your receipt infrastructure is wrong, everything downstream inherits that wrongness silently. The fix was trivial. The lesson is that you have to periodically turn your verification tools on themselves.

---

**Q37: 52 orphaned worktrees consuming 218GB. How do you think about infrastructure debt that accumulates invisibly?**

Those were just previous run leftovers from roughly 150 agents. We have a script that runs at the start and end of runs now. But worktrees are honestly the smaller problem. Branches are the real debt -- 1,529 of them right now. That's the invisible accumulation that actually matters. The worktrees are just disk. The branches are state.

---

**Q38: Your agent ratio -- how did you arrive at that? Was it intuitive or measured?**

Building isn't the bottleneck. Understanding and trust outnumber building a hundred to one. If I were optimizing from scratch I'd split something like 2:3:1:1:1:0.5 across scout, plan-review, build, review-and-improve, deep-review-and-improve, and ops. If I only had ten agents, I'd barely run builders at all. The leverage is entirely upstream. Scouts and plan-reviewers are the whole game.

---

**Q39: You said "I think we're already there" for best Perl LSP. But the corpus was 85%, not 100%. Where's the gap between your confidence and the numbers?**

"We were at 50% a couple days ago. We'll be at 95% within days." And we ratcheted to 92% during the session. The remaining gap isn't user-facing Perl -- it's source filters and edge cases that real codebases don't hit. The confidence isn't about the number, it's about whether the tool is useful for the work people actually do. At 85% and climbing, it is.

---

**Q40: The perlcritic integration was a 30-line wiring fix. How often do you find "built but not wired" in your codebase?**

More than I'd like. Scouts find them. The infrastructure accumulates ahead of the wiring because agents build components in isolation and the connection step requires understanding the whole system. It's not enough of a pattern to dedicate a permanent lane to -- but when a scout finds one, it's usually the highest-ROI fix available. Thirty lines, immediate user-visible feature. That ratio doesn't happen with new infrastructure.

---

**Q41: "It was a mistake. I should not have started." But you also said "I still can't put it down." What's the thing that keeps pulling you back?**

Both are true. It's both a mistake and something I genuinely can't put down. It's been driving learning and design insight that I'd never have gotten any other way. Right now I'm thinking about a GLM Max plan with a maintainer-focused harness -- something that takes what we built here and makes it accessible to other projects doing the same thing. The mistake is still running, and I'm still learning from it.

---

**Q42: You said you don't use Perl and don't know any Perl developers personally. Who IS this for, then?**

Initially it was internal tooling. The corpus was the proxy for users I'd never met. But somewhere in the middle it became clear the audience is Perl developers broadly -- the people maintaining those millions of lines at major companies. I infer needs from complaints and research rather than personal experience. The CPAN corpus is real-world user code. If the tool handles what's in the corpus, it handles what users write.

---

**Q43: "The biggest question I get is 'but why Perl?', which is weird, because I don't get that reaction for my COBOL tooling." Why do you think Perl triggers that reaction?**

Perl is the funky cousin of Python and Ruby. The Betamax. The Laserdisc. It lost the cultural war even while staying in production. COBOL never had cultural pretensions -- it was always business infrastructure. Perl had ambitions, and then fell off the conference circuit, and people confuse that with death. Anyway, it doesn't matter much. We're abstracting away from the syntax entirely. The parser is the bottom layer. Everything above it is just IDE features.

---

**Q44: The zero-panic policy came from panics crashing WSL and bringing down 10+ Claude Code sessions at once. How many cascading failures did it take before you made it policy?**

A couple. Cascading failures are effective teachers. You work for six hours, you're ten agents deep into a complex session, and then one panic in one process brings the whole WSL instance down and you lose the session state. That happens twice and you write the policy. "15 sessions crashing six-plus hours in is waste." The ban needs to extend to tests too, not just production code. We're not all the way there yet.

---

**Q45: The 130-crate architecture is designed for AI agents, not humans. Is that intentionally inhospitable to contributors?**

You work one sector at a time. You don't need to hold the whole graph. Nobody designs machine-compiled assembly for human readers -- you design it for the machine. The crate boundaries are context boundaries, and context boundaries are what make parallel agent work tractable. A human contributor working on one crate will find it straightforward. The intimidating thing is the total count, not any individual piece.

---

**Q46: The three-layer product insight (LSP + swarm OS + memory system) -- did you see it as three layers while building, or only in hindsight?**

In stages. The swarm patterns clicked around August. The memory and repository architecture came together in May. Seeing all three layers as a coherent system -- each compounding on the others -- that was September. It's not that each layer was planned. It's that each one solved a real bottleneck as it appeared, and only afterward was it clear they were part the same structure.

---

**Q47: "Way more compute heavy than anyone in the industry is ready for." Can you quantify?**

Shockingly cheap at API pricing. Roughly $5 per flow run, $40 plus CI for a solid PR. CI cost can dwarf token cost -- that was a surprise. We're running about 3% of weekly budget for approximately 30 merged PRs in two hours. The economics work. The surprise isn't the cost, it's that CI becomes the bottleneck, not the agents. Token spend is small. Waiting for green is the expensive part.

---

**Q48: Multiple provider arbitrage across ChatGPT, Copilot, Claude, Codex, Roo Code -- was that strategic or desperation?**

Both. And sustainability. A lot of the struggles are more down to the harness than the model. The multi-provider approach has structural advantages -- different tools have different rate limits, different strengths, different failure modes. But it also emerged from "what credits do I have available right now." What I've learned is that the harness matters more than the model. A good harness with a mid-tier model beats a bad harness with the best model.

---

**Q49: Your framework says "once a developer reaches a mode, they don't step down voluntarily." Have you ever stepped down?**

No. It was platform constraints each time, not voluntary regression. What looked like stepping down was actually waiting for the platform to catch up to where the methodology needed to go. Era 3's deceleration was architecture work, not a mode change. If the tooling had existed in Q3 2025, we never would have had that pause. I could have adapted faster. That's the real lesson.

---

**Q50: ChatGPT said "the methodology was always trying to exist, but kept getting trapped inside one prompt." Is that how you see it?**

Yes and no. The methodology evolved. Breaking into context-efficient chunks has been the pattern since Q3. But ChatGPT's framing is useful -- the structure was trying to emerge before the tooling existed to express it. Each era was a failed attempt to express something that finally became expressible in Era 5. And the answer to "did the methodology work" is: look at the codebase. That's the receipt.

---

**Q51: The parser wall and the SDLC wall are "the same idea at two levels of the stack." Did you see that parallel before it was pointed out?**

Not the parser wall specifically. My parser wall was exploding parse times with backtracking -- a different technical problem. But the structural parallel -- cheap generation, hard downstream trust -- I see it everywhere now that it's been named. Building the parser verification infrastructure and building the agent verification infrastructure are the same problem. The ratchet, the corpus sweep, the CI gate -- they're all about converting "the thing says it works" into "we have evidence it works."

---

**Q52: Your documentation uses both 6-flow and 7-flow models. Which is canonical?**

The 7-stage pipeline is canonical. Demo Swarm was a Q3 recreation as a learning artifact -- it was built to document what the methodology had been, not what it became. There should be even more than 7 stages. The pipeline model keeps revealing new useful gates as the system matures. The 6-flow was early; the 7-flow is current; I expect it to grow.

---

**Q53: 8:1 test-to-code ratio. Was that intentional?**

The 8:1 is skewed by mutation testing hardening crates, which add tests specifically to kill surviving mutants. For the ratio I'm actually aiming for, rough target is 3:1. But what matters more than the ratio is locking in all the behaviors -- 100% of the API and ABI surface you care about. If mutation testing is green and the behavior is locked, the exact ratio matters less than the coverage.

---

**Q54: The community scout found 78% of Perl devs use no LSP at all. Does that change your launch strategy?**

Probably. My instinct is still "just sell the features" -- show what go-to-definition looks like in a real Perl codebase, show real-time diagnostics catching a typo that would have been a bug in production. But I probably should have an LSP education document. People who have never used a language server need to understand what they're getting before they can appreciate why it matters.

---

**Q55: "I should have built and finished my stepwise agentic swarm harness in Q3 2025 instead of dawdling." What would be different today?**

We'd probably have launched by now, with the harness. Maybe half as many crates -- the microcrate explosion happened partly because the architecture was being discovered at the same time it was being built. If the harness had existed, the architecture decisions would have been made earlier and with more information. The codebase would be smaller and more intentional. The corpus would be higher.

---

**Q56: "Receipts work the way locks work: they keep honest systems honest. If the lock itself is broken, the door is open and nobody notices." How do you check if the lock is broken?**

Defence in depth. All of them. Mutation testing, oppositional validation, instrument audits, receipt schema checks. They scale together. No single audit type catches everything, so the answer is layering them. The six cases in WHEN_RECEIPTS_LIE.md were each caught by a different mechanism. The triage logic is: start with the receipt that the most downstream work depends on, and audit that first.

---

**Q57: The CURRENT_STATUS.md gate was blocking correct code because the documentation was stale. How do you feel about coupling documentation freshness to merge eligibility?**

The gate is right in principle but wrong in scope. The design needs tweaking. Probably decouple to per-microcrate status rather than a single global document. When 60 agents are running in parallel, a single shared status file becomes a coordination bottleneck. The freshness gate should be scoped to the crate being changed, not the whole workspace. That's the fix.

---

*These 22 questions were generated by a 60-agent session that spent a full day inside the codebase. The session found the assert_clean_parse blind spot, the 52 orphaned worktrees, and the "built but not wired" perlcritic fix. The questions they generated are better than questions a human interviewer would write -- because they come from evidence, not assumptions.*

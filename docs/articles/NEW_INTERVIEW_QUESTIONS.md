# New Interview Questions: Session 3 Discoveries

These questions were generated after 60+ agents spent a full session scouting, building, and reviewing the perl-lsp codebase. The original 35 questions (`.claude/interview-questions.md`) were written before this session. These questions target angles that only became visible through the session's findings.

Each question includes: the question itself, why it unlocks an interesting story, evidence from the codebase, and a follow-up.

---

## FROM SESSION DISCOVERIES

### Q36: The assert_clean_parse bug exposed 56 silently passing tests. How does it feel to discover your own testing infrastructure was lying to you?

**The Question**: A scout agent found that `assert_clean_parse()` checked for `(error` and `(Error` but the parser emits `(ERROR`. Fifty-six tests were reporting green while the parser was producing error nodes in their output. The shared `ERROR_MARKERS` constant with the correct pattern already existed in the same module -- it just was never wired up.

**Why it's interesting**: This is not a bug in the code. It is a bug in the thing that verifies the code. The irony -- that the fix existed and was never connected -- reveals a class of failure that scales with agent count. Every agent that ran those tests saw green. Nobody had reason to question green.

**Evidence from codebase**:
- PR #2238 fixed the case-sensitivity blind spot in `assert_clean_parse()`
- Issue #2239 tracks the 56 newly-exposed failures
- `WHEN_RECEIPTS_LIE.md` documents this as Case 1: "The Silent Tests"
- The `ERROR_MARKERS` constant (commit `f5b449c22`) already included `(ERROR ` -- the helper just never used it

**Follow-up**: You wrote an entire article about receipts lying. Did writing it change how you think about green CI, or were you already skeptical before this incident?

---

### Q37: 52 orphaned worktrees consuming 218GB. How do you think about infrastructure debt that accumulates invisibly?

**The Question**: A cleanup sweep found 52 stale worktrees -- one per agent session that spawned a worktree and never cleaned it up. 218GB of disk space, invisible to every metric the project tracks, accumulating silently across sessions.

**Why it's interesting**: This is infrastructure debt in its purest form. No CI gate checks for stale worktrees. No metric tracks disk consumption from agent sessions. The debt is real, material, and invisible until someone looks. For a project built on receipts and computed metrics, the fact that this fell through the cracks is instructive.

**Evidence from codebase**:
- Worktree cleanup during session 3 found 52 stale entries
- Memory: `feedback_worktree_contention.md` notes that "Multiple agents sharing worktrees causes branch conflicts"
- CLAUDE.md mandates `isolation: "worktree"` for every agent -- each session spawns many

**Follow-up**: Is there infrastructure debt in your project that you know about but haven't prioritized? What's the threshold where invisible debt becomes a crisis?

---

### Q38: Your 4:1:2 scout:builder:reviewer ratio -- how did you arrive at that? Was it intuitive or measured?

**The Question**: In your answer to the original Q19, you described the scout-constrain-build pattern. But the actual ratio of agent types -- how many scouts per builder, how many reviewers per PR -- is a tuning parameter. What ratio did you converge on and how?

**Why it's interesting**: Most AI development discussions focus on whether to use agents, not how to proportion them. The ratio reveals what the bottleneck actually is. If scouts outnumber builders, the constraint is understanding, not implementation. If reviewers are scarce, the constraint is trust.

**Evidence from codebase**:
- Memory: `feedback_merge_queue_is_bottleneck.md` -- "Merge queue is 3-wide; 75 agents generating 50+ PRs creates backlog. Optimal coding agents ~ 9."
- Memory: `feedback_team_roster_hard_ceiling.md` -- "Team roster hard ceiling ~75"
- Memory: `feedback_agent_success_rate_pattern.md` -- "Constrained tasks ~90% success, unconstrained features ~50%"
- Session 3 ran 60+ agents with structured role assignment

**Follow-up**: If you could only have 10 agents, how would you split them? Does the ratio change by task type (parser work vs. LSP features vs. documentation)?

---

### Q39: You said "I think we're already there" for best Perl LSP. But the corpus is 85%, not 100%. Where's the gap between your confidence and the numbers?

**The Question**: In your interview answers, you said "I think we're already there, no?" when asked what the best Perl LSP looks like. But 85% CPAN coverage means 15% of real-world Perl files produce parse errors. How do you reconcile the confidence with the gap?

**Why it's interesting**: This is a question about what "good enough" means for developer tools. The answer reveals whether the remaining 15% is noise (source filters, exotic syntax nobody uses in production) or signal (common patterns that block real users).

**Evidence from codebase**:
- INTERVIEW_QA.md: "I think we're already there, no? At this point it's about finding ways to make a better user experience."
- CPAN corpus baseline: 85.4% clean (3,717/4,355 files) as of 2026-03-20
- Lib-file sweep after March 21 parser fixes: 90.9% clean (3,077/3,386 files)
- Manifest: 2,052 clean modules explicitly verified
- docs/issues/corpus/gaps/ lists specific gap categories including source filters and exotic syntax
- Memory: `scout_unexpected_token_analysis.md` categorizes 146 failing files into 10 subcategories

**Draft answer**:

"There are two numbers. There's the baseline — 85.4% on 4,355 full corpus files — and there's the sweep, which is 90.9% on the lib-file subset after recent parser fixes. The gap between those two numbers is partly real and partly measurement artifact. The full corpus includes test files, scripts, and edge-case modules that nobody deploys. The lib-file sweep is closer to what you'd actually see in a production Perl codebase.

The remaining ~10% breaks into categories. Source filters — code that rewrites itself before parsing — are structurally unfixable for any static parser. Exotic DSL syntax (some Moose patterns, some DBIx::Class query generation) needs semantic awareness, not just parsing. And then there's a long tail of real bugs we haven't fixed yet. The ratchet tracks all three: it just can't tell them apart.

So when I said 'I think we're already there' — I meant for the IDE features that matter: go-to-definition, real-time diagnostics, hover. Those work. The 10% that doesn't parse cleanly mostly generates a degraded experience, not a broken one. The parser produces partial results, and the LSP does its best with what it has. That's different from saying we're done. We're not done. But 'not done' and 'not useful yet' are different claims."

**Follow-up**: Have you profiled what percentage of Perl code in active production codebases falls in that 15%? Is the gap CPAN-specific or representative of what users actually write?

*Updated 2026-03-21: baseline 85.4% (3,717/4,355), manifest 2,052 modules, 90.9% clean on lib-file sweep. Fat-arrow fix (#2613) and defined/ref fix (#2626) merged this session — estimated 80+ newly clean files not yet reflected in baseline JSON.*

---

### Q40: The perlcritic integration was a 30-line wiring fix -- the infrastructure was 85% built. How often do you find "built but not wired" in your codebase?

**The Question**: PR #2285 wired perlcritic integration -- a feature users would notice immediately -- in roughly 30 lines. The underlying infrastructure (diagnostic pipeline, subprocess management, config plumbing) had been built across earlier PRs. Similarly, PR #2057 was the famous 9-line fix. How much of the codebase is infrastructure waiting to be connected?

**Why it's interesting**: This inverts the usual narrative about feature development. The hard work is already done. The visible work is often trivial. Scouts that look for "built but not wired" patterns might be the highest-ROI agents you can deploy.

**Evidence from codebase**:
- PR #2285: perlcritic integration wiring
- PR #2057: 9-line fix, referenced in memory as highest-ROI change
- Memory: `feedback_wiring_fixes_highest_roi.md` -- "'Built but not wired' is highest ROI"
- The `assert_clean_parse` bug was also a wiring failure: `ERROR_MARKERS` existed but wasn't connected

**Follow-up**: Could you build a scout agent whose entire job is finding infrastructure that's built but not connected? Would that be the highest-leverage use of a single agent?

---

## FROM THE EMOTIONAL CORE

### Q41: "It was a mistake. I should not have started." But you also said "I still can't put it down." What's the thing that keeps pulling you back?

**The Question**: In the same breath, you described perl-lsp as a mistake and as something you can't stop working on. These are contradictory. One of them is rationalizing. Which one?

**Why it's interesting**: Most developer project narratives are tidy -- "I saw a problem, I built a solution." This one is messier and more honest. The answer reveals what motivation actually looks like for a project nobody asked for, in a language you don't use, for a community you're not part of.

**Evidence from codebase**:
- INTERVIEW_QA.md: "I wouldn't [change anything]. It was a mistake. I should not have started. I don't write Perl. I had no reason to build this. But I still can't put it down."
- The project has 2,200+ commits, 130 crates, 6,300+ test functions -- this is not casual interest

**Follow-up**: Is there a version of this project where you walk away? What would make you stop?

---

### Q42: You said you don't use Perl and don't know any Perl developers personally. Who IS this for, then?

**The Question**: Most developer tools are built by people who use the language and feel the pain personally. You don't write Perl. You've said you don't know Perl developers. So who is the intended user, and how do you know what they need?

**Why it's interesting**: Building for a user you've never met is either empathic design at its best or projection at its worst. The CPAN corpus is the proxy for real users, but a corpus can't tell you about workflow, preferences, or frustration points.

**Evidence from codebase**:
- INTERVIEW_QA.md: "I don't use Perl" and "I don't write Perl"
- No evidence of Perl community engagement in the repo (no mailing list references, no r/perl threads, no PerlMonks citations)
- README addresses "Perl developers" -- but the actual user persona is undefined
- CPAN corpus is the closest thing to user research

**Follow-up**: Have any Perl developers tried it yet? What was their reaction? Did they ask for things you hadn't thought of?

---

### Q43: "The biggest question I get is 'but why Perl?', which is weird, because I don't get that reaction for my COBOL tooling." Why do you think Perl triggers that reaction?

**The Question**: COBOL and Perl are both "legacy" languages. Both have millions of lines in production. Both lack modern tooling. But Perl draws skepticism that COBOL doesn't. Why?

**Why it's interesting**: This is about perception vs. reality in the programming language ecosystem. COBOL is "respected legacy." Perl is "joke legacy." The distinction reveals something about how developers assign status to languages and the people who maintain them.

**Evidence from codebase**:
- INTERVIEW_QA.md: direct quote about the Perl vs. COBOL reaction
- The project exists partly as a refutation of the "Perl is dead" narrative

**Follow-up**: Do you think the "why Perl?" reaction will change when people see the LSP working? Or is it a permanent cultural judgment?

---

## FROM ARCHITECTURE INSIGHTS

### Q44: The zero-panic policy came from "panics crashing WSL, bringing down 10+ Claude Code sessions at once." How many cascading failures did it take before you made it policy?

**The Question**: ADR-0012 bans panics in production code. The memory system records that the policy originated from panics in the LSP server process crashing WSL, which brought down all active agent sessions. One panic in one process cascaded to 10+ agents losing their work.

**Why it's interesting**: This is an incident story with real consequences. The policy isn't theoretical -- it was born from lost work. The question probes how many times the failure mode repeated before it became a rule, and what the emotional experience was of watching 10 agents crash simultaneously.

**Evidence from codebase**:
- ADR-0012: "Error Handling Strategy (No Panics Policy)"
- CLAUDE.md bans: `unwrap()`, `expect()`, `panic!()`, `todo!()`, `unimplemented!()`, `std::process::abort()`, `dbg!()`
- ZERO_PANIC.md article exists in docs/articles/
- WSL environment confirmed: OS version shows "microsoft-standard-WSL2"
- All 10+ agents run as separate processes in the same WSL instance

**Follow-up**: Is there a version of the LSP server that's allowed to panic in development? Or is the policy absolute everywhere?

---

### Q45: "Many small focused SRP crates is extreme for a human to maintain. It's well routed for an AI." Does this mean the architecture is INTENTIONALLY inhospitable to human contributors?

**The Question**: You've explicitly said the 130-crate architecture is designed for AI agents, not humans. If a human contributor wanted to submit a PR, they'd face a codebase with 130 crates, a CI gate that checks 15+ things, and a review process designed for agents. Is this a trade-off you've accepted, or a design flaw?

**Why it's interesting**: This touches the "AI-native" question directly. If the codebase is optimized for agents, it might be hostile to the community it claims to serve. Perl developers who want to contribute might be locked out by the architecture that makes agents productive.

**Evidence from codebase**:
- INTERVIEW_QA.md: "Many small focused SRP crates... is extreme for a human to maintain. It's simple and searchable and context efficient and well routed for an AI."
- 130 crate directories in workspace
- ADR-0008: Microcrate architecture decision
- CONTRIBUTING.md exists but targets agent-compatible workflows

**Follow-up**: If a human Perl developer submitted a PR tomorrow, what would their experience be? Would they succeed?

---

### Q46: The three-layer product insight (LSP + swarm OS + memory system) -- did you see it as three layers while building, or only in hindsight?

**The Question**: The KNOWLEDGE_COMPOUNDING article describes three layers: Memory (what we learned), Skills (how we work), and Hooks (what we enforce). These compound across sessions. But did you design this layered architecture, or did it emerge from solving problems one at a time?

**Why it's interesting**: Emergent architecture is more credible than designed architecture -- it means the structure was discovered, not imposed. The answer reveals whether the methodology was planned or evolved.

**Evidence from codebase**:
- KNOWLEDGE_COMPOUNDING.md, Section 2: "Three Layers of Externalized Knowledge"
- Layer 1: Memory files (~100+ across sessions)
- Layer 2: Skills (~32 documented in `.claude/skills/`)
- Layer 3: Hooks (enforcement layer in `.claude/settings.json` or similar)
- INTERVIEW_QA.md: "The methodology compounds. That's the thing worth studying."

**Follow-up**: Which layer came first? Which was the hardest to get right? If you had to drop one layer, which would survive?

---

## FROM THE ECONOMICS

### Q47: "Way more compute heavy than anyone in the industry is ready for." Can you quantify? What's the actual token spend per session?

**The Question**: Running 60-100 agents per session, each reading files, writing code, and communicating, consumes significant compute. What does a typical session cost in tokens, dollars, or API calls?

**Why it's interesting**: Nobody in the AI-assisted development space publishes real cost numbers. The answer sets a benchmark. If a session costs $500 and produces 50 merged PRs, that's $10 per merged change -- cheaper than most human developer hours. If it costs $5,000, the economics look different.

**Evidence from codebase**:
- Memory: `feedback_100_agent_session.md` references 100-agent session scale
- Memory: `feedback_ci_is_the_bottleneck.md` -- "CI is the bottleneck, not agents; optimize for CI throughput"
- Each agent reads multiple files, writes code, runs verification -- significant token volume
- Multiple provider references: Claude, Copilot, Codex, ChatGPT

**Follow-up**: Do you track cost-per-merged-PR? Is there a point where compute cost exceeds the value of the output?

---

### Q48: You described multiple provider arbitrage (ChatGPT, Copilot, Claude, Codex, Roo Code) to spread compute. Is that sustainable or was it desperation?

**The Question**: The git history and branch naming shows work from multiple AI providers: `codex/` prefixed branches, Claude Code agents, Copilot-era PRs. Using multiple providers is unusual. Was this strategic diversification or "whatever credits I had available"?

**Why it's interesting**: Provider arbitrage is a real strategy that nobody talks about publicly. If different providers have different strengths (Codex for bulk, Claude for precision, Copilot for speed), the mix reveals an empirical evaluation of AI capabilities that no benchmark captures.

**Evidence from codebase**:
- Branch prefixes: `codex/`, `claude/`, implicit Copilot branches
- FIVE_ERAS.md documents the Copilot era (Era 4) and Claude Code era (Era 5) explicitly
- Memory: `feedback_codex_duplicate_prs.md` -- "Codex generates near-duplicate PRs"
- The project has used at least 4 distinct AI development tools

**Follow-up**: If you had unlimited budget on a single provider, which would you choose? Or does the multi-provider approach have structural advantages?

---

### Q49: Your blog says "Once a developer reaches a mode, they don't step down voluntarily." Have YOU ever stepped down voluntarily?

**The Question**: The AGENTIC_DEV.md framework describes developer modes -- from manual coding through fully agentic. The claim is that once you reach a higher mode, you don't go back. But the project went through five eras, some of which were retrenchments. Did you ever step DOWN?

**Why it's interesting**: If the answer is yes, it contradicts the framework. If the answer is no, it raises the question of what "stepping down" would even mean for someone orchestrating 100 agents.

**Evidence from codebase**:
- AGENTIC_DEV.md: developer mode progression framework
- Era 3 was an intentional deceleration -- fewer commits, more architecture
- Era 4 to Era 5 was a tooling switch, not clearly "up" or "down"

**Follow-up**: Is there a scenario where going back to manual coding would be the right choice? Or has the methodology made that impossible?

---

## FROM WHAT CHATGPT NOTICED

### Q50: ChatGPT said "the methodology was always trying to exist, but kept getting trapped inside one prompt." Is that how you see it?

**The Question**: An external AI's analysis of the project suggested that the scout-constrain-build pattern, the memory system, and the skill library were all trying to emerge from the beginning, but were trapped in monolithic prompts. The methodology existed before the tooling to express it did.

**Why it's interesting**: This is a question about whether methodology is discovered or invented. If ChatGPT is right, the five eras are not a progression but a series of failed attempts to express the same underlying idea. Era 5 succeeded not because the idea changed, but because the tooling finally matched the idea.

**Evidence from codebase**:
- Five eras show recurring patterns: dedup, quality gates, structured roles
- Memory system didn't exist until Era 5 but the learnings it captures date back to Era 1
- Skills encode patterns that agents were doing informally since Era 2
- FIVE_ERAS.md: each era rediscovers constraints the previous era learned

**Follow-up**: Can you point to a specific prompt from Era 1 or 2 where you see the seed of what the methodology became?

---

### Q51: The parser wall and the SDLC wall are "the same idea at two levels of the stack" -- cheap generation is easy, downstream trust is hard. Did you see that parallel before it was pointed out?

**The Question**: Generating Perl parse trees is easy. Trusting them enough to build IDE features on top is hard. Generating code from AI agents is easy. Trusting it enough to merge to production is hard. These are structurally identical problems at different levels of the stack. The parser wall and the SDLC wall are the same wall.

**Why it's interesting**: This is a meta-insight that connects the product (parser) to the process (swarm). If you saw it, it explains why the receipt culture and the parser verification evolved in parallel. If you didn't, the parallel is still there -- it's just unconscious.

**Evidence from codebase**:
- Parser: 85% CPAN coverage, ratcheted baselines, corpus sweeps -- all trust infrastructure
- Swarm: review agents, CI gates, receipt schemas, mutation testing -- all trust infrastructure
- WHEN_RECEIPTS_LIE.md: "The receipt says green. The reality is red." -- applies to both parser tests and agent output
- Both systems solve: "generation is cheap, verification is expensive"

**Follow-up**: Did building the parser verification infrastructure directly inform how you built the agent verification infrastructure? Or did they evolve independently?

---

### Q52: Your deck uses both 6-flow and 7-flow models (Review is sometimes included, sometimes not). Which is canonical and why did it change?

**The Question**: The pipeline documentation sometimes describes 6 stages (Scout, Issue, Build, Draft, CI, Merge) and sometimes 7 (adding Review between Draft and CI). The inconsistency suggests the role of review evolved during the project.

**Why it's interesting**: Whether review is a distinct stage or folded into CI reveals how you think about trust. If review is separate, you believe automated gates can't catch everything. If review is folded in, you believe the gates are sufficient.

**Evidence from codebase**:
- Memory: `feedback_pipeline_layers.md` -- "7 layers: Scout, Issue, Build, Draft, Review, CI, Merge"
- Memory: `feedback_review_catches_real_bugs.md` -- "Review caught 15+ real bugs; never skip review"
- Some documentation describes the flow without an explicit review step
- SWARM_METHODOLOGY.md and swarm.md may differ on this count

**Follow-up**: Has there ever been a PR that passed review but failed CI, or passed CI but should have failed review? Which direction is the gap more dangerous?

---

## FROM WHAT SURPRISED US

### Q53: High test-to-code ratio. Was that intentional or did it accumulate?

**The Question**: The codebase has an unusually high test count. That is an extreme ratio. Most well-tested projects are 2:1 or 3:1 test-to-code. Was this a design decision, an organic result of mutation testing, or a side effect of how agents write code?

**Why it's interesting**: If intentional, this reveals a philosophy about where trust comes from. If accidental, it reveals something about how agents produce tests -- possibly over-testing as a default behavior. Either way, the ratio is unusual enough to warrant explanation.

**Evidence from codebase**:
- Tier A lib tests (canonical merge-gate): ~2,811 (`cargo test --workspace --lib --exclude tree-sitter-perl -- --list`)
- Doc tests: ~304
- Integration tests: ~18,350
- Total across all test types: ~21,465
- Note: earlier claims of 6,326 were incorrect; the figure came from an unverified audit that conflated scope (see issue #2672 for full root-cause)
- Public function count: ~755 (from stability audit or API surface scan; unverified)
- Mutation testing infrastructure (`cargo-mutants`, dedicated hardening test crates) drives additional test creation
- Agents are instructed to write tests as part of every parser fix (`/parser-fix` skill includes TDD)

**Methodology note**: Use Tier A (lib tests) for the merge-gate count. Use total (~21,465) for the full story. Always specify which scope.

**Follow-up**: Is there a point where more tests become a maintenance burden? Do you ever delete tests? What's the ideal ratio?

---

### Q54: The Perl community engagement scout found 78% of Perl devs use NO LSP at all. Does that change your launch strategy?

**The Question**: A scout researching the Perl developer community found that roughly 78% of Perl developers use no language server protocol integration. They use raw vim, emacs with basic syntax highlighting, or IDEs with no Perl-specific intelligence. The market isn't "switching from a bad LSP" -- it's "using an LSP for the first time."

**Why it's interesting**: This reframes the launch. You're not competing with existing tools. You're introducing a category to people who have never experienced it. The onboarding experience matters more than feature parity with other LSPs. The question is: do these developers even want what you're building?

**Evidence from codebase**:
- Scout findings from session 3 community research
- README targets "Perl developers" broadly but doesn't address LSP-naive users
- VSCode extension exists (easiest onboarding) but many Perl devs use vim/emacs
- No tutorials or "getting started with an LSP" documentation visible

**Follow-up**: Would you build a "what is an LSP and why should you care" introduction for Perl developers? Or do you assume your users already understand LSP?

---

### Q55: "I should have built and finished my stepwise agentic swarm harness in Q3 2025 instead of dawdling." What would be different today if you had?

**The Question**: You expressed regret about not finishing the agentic harness earlier. The implication is that months of development happened without the infrastructure that would have made it faster. What specifically would be different?

**Why it's interesting**: This is a counterfactual question that reveals what the biggest bottleneck was. If the harness had existed in Q3 2025, would the project be at 95% CPAN coverage? Would it have launched already? Would the architecture be different?

**Evidence from codebase**:
- Era 3 (Oct 2025 - Feb 2026) was the "Architectural Sidechain" -- slow and design-focused
- Era 4 (late Feb - early Mar 2026) was the Copilot firehose -- fast and chaotic
- Era 5 (Mar 11-19, 2026) is the structured swarm -- fast and controlled
- The gap between Era 3's end and Era 5's start is the cost of not having the harness

**Follow-up**: Is the agentic harness itself now a product you could release? Would other projects benefit from it?

---

## FROM THE TRUST PROBLEM

### Q56: You wrote "receipts work the way locks work: they keep honest systems honest. If the lock itself is broken, the door is open and nobody notices." How do you check if the lock is broken?

**The Question**: WHEN_RECEIPTS_LIE.md ends with this metaphor. Six cases of broken receipts are documented. But the meta-question remains: how do you know which receipts to distrust? You can't audit everything. What's the triage logic?

**Why it's interesting**: This is the deepest question the article raises and doesn't fully answer. The fix section suggests mutation testing, oppositional validation, and instrument audits. But the prioritization -- which instruments to audit first -- is the real problem at scale.

**Evidence from codebase**:
- WHEN_RECEIPTS_LIE.md: all 6 cases
- Case 1: 56 silent tests (assert_clean_parse case-sensitivity)
- Case 2: phantom error bucket (83 files in a nonexistent category)
- Case 3: benchmark measuring struct construction instead of parsing
- Case 5: 249 clean files not claimed in the ratchet
- Mutation testing infrastructure exists but covers only "critical crates"

**Follow-up**: If you had to pick one category of receipt to audit right now -- tests, benchmarks, metrics, or documentation -- which would you choose and why?

---

### Q57: The `CURRENT_STATUS.md` trap -- a gate designed to prevent stale documentation was blocking correct code. How do you feel about coupling documentation freshness to merge eligibility?

**The Question**: Case 6 in WHEN_RECEIPTS_LIE.md describes agents producing correct parser fixes that were blocked because `CURRENT_STATUS.md` had stale test counts. The gate was doing its job -- the document WAS stale. But it was blocking the wrong thing for the wrong reason.

**Why it's interesting**: This is a design tension between "everything is correct" and "the right things are correct at the right time." The gate conflated two orthogonal concerns. Fixing it requires deciding what's blocking vs. advisory -- and that's a governance question, not a technical one.

**Evidence from codebase**:
- WHEN_RECEIPTS_LIE.md, Case 6
- Memory: `feedback_status_update_trap.md`
- `scripts/update-current-status.py` must be run after adding tests
- Agents that don't know about this step get blocked at the final gate

**Follow-up**: Have you decoupled the gate yet? If not, what's stopping you?

---

## INTERVIEW STRUCTURE

These 22 new questions (Q36-Q57) complement the original 35. For a comprehensive interview using both sets:

**Session Discoveries block (Q36-Q40)**: 15 minutes -- the things this session uncovered that nobody expected
**Emotional Core block (Q41-Q43)**: 10 minutes -- the human motivation behind an "irrational" project
**Architecture block (Q44-Q46)**: 10 minutes -- consequences of designing for AI instead of humans
**Economics block (Q47-Q49)**: 10 minutes -- the real cost of 100-agent sessions
**External Observations block (Q50-Q52)**: 10 minutes -- what other AIs and observers noticed
**Surprise Findings block (Q53-Q55)**: 10 minutes -- data points that challenge assumptions
**Trust Problem block (Q56-Q57)**: 10 minutes -- the deepest question the project raises

**Total: ~75 minutes for the new questions alone, or ~155 minutes combined with the original 35.**

**For a focused 30-minute session using only new questions**: Q36 (silent tests), Q41 (the contradiction), Q47 (compute cost), Q51 (parser wall = SDLC wall), Q56 (checking the lock)

---

## NEW ARTICLE HOOKS

- **"The Lock That Looked Locked: When Testing Infrastructure Lies to You"** (Q36, Q56, Q57)
- **"Building for Users You've Never Met"** (Q42, Q54, Q39)
- **"The Parser Wall and the SDLC Wall"** (Q51, Q46, Q56)
- **"What 100 Agents Cost: The Economics Nobody Publishes"** (Q47, Q48, Q38)
- **"The Architecture Nobody Can Contribute To"** (Q45, Q53, Q42)
- **"It Was a Mistake I Can't Put Down"** (Q41, Q43, Q55)

---

*These questions assume Steven Zimmerman will engage with the contradictions -- between confidence and gaps, between "mistake" and obsession, between AI-native architecture and human contribution. The best interviews surface tensions, not resolutions.*

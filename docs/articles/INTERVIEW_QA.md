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

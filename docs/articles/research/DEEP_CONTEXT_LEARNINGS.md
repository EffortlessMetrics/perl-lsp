# Deep Context Learnings

Insights that only make sense with full knowledge of the developer, the ecosystem, and the session history. Captured 2026-03-20.

## 1. Why perl-lsp Exists

perl-lsp exists because tree-sitter-perl broke Steven's AST context packing engine (likely related to tokmd). The rational choice was to turn off Perl support — it's one language among many. Instead, he fixed it.

- tree-sitter lasted 0 days (C compat, compilation size)
- Pest (v2) couldn't handle Perl's undecidability
- Recursive descent (v3) worked
- Parser grew into LSP, DAP, 132 crates

Steven says: "It was a mistake. Zero ROI. Should have focused elsewhere. But I still can't put it down."

**The non-obvious implication:** The zero-ROI nature is WHY the quality is so high. No business pressure means no corners cut, freedom to experiment with 100-agent sessions, freedom to write 8x more tests than code. This is a luxury project built right. The methodology IS replicable; the luxury isn't.

## 2. perl-lsp in the EffortlessMetrics Ecosystem

perl-lsp is NOT the most complex repo. That's adze (GLR grammar toolchain). perl-lsp is the most DOCUMENTED and STUDIED.

- **adze** — Most complex. General-purpose GLR parser with typed extraction. What you build when you want to parse ANY language.
- **perl-lsp** — Most documented. Specific recursive descent for Perl 5. The proving ground for the swarm methodology.
- **bitnet-rs** — Most frustratingly close. 1-bit LLM inference in Rust. Nearly shipping.
- **tokmd** — 1st pinned repo. Code intelligence platform that started as a tokei shell script.

The trust stack:
- Bottom: parsers that understand data (perl-lsp, adze, copybook-rs, pst-rs, hl7v2-rs)
- Middle: CI sensors that verify each change (covguard, perfgate, lintdiff, etc.)
- Top: orchestration (flow-studio, cockpitctl, demo-swarm)
- Across: intelligence (tokmd, adze)

Revenue comes from AGPL-licensed legacy parsers (copybook-rs, pst-rs, hl7v2-rs). perl-lsp is MIT/Apache — free, proving the methodology that makes paying projects cheaper.

## 3. The "Frustratingly Close" Pattern

bitnet-rs is "frustratingly close." So is perl-lsp's corpus (86.8%, each additional percent harder). So are many features (built but not wired).

The pattern: AI-assisted development excels at 0→90% (generation is cheap). The last 10% requires integration, polish, and judgment — expensive in human attention.

This IS "code is cheap; trusted change is not" in practice. The code is written. The trust (testing, integration, documentation, release) is what's frustrating.

## 4. SDK Bans Shaped the Methodology

Every era's methodology was a RESPONSE to a platform constraint:

| Era | Constraint | Innovation It Forced |
|-----|-----------|---------------------|
| 1 (Opus) | Single conversation limit | Deep context, high quality |
| 2 (Swarms) | No persistent memory | Session-bounded work patterns |
| 3 (Sidechain) | Rate limits + range anxiety | Architecture in browser, code in bursts |
| 4 (Copilot Fleet) | GitHub banning fleet+autopilot | Migration to Claude Code |
| 5 (Agent Teams) | 75-agent ceiling, merge queue | Structured 5-coordinator model |
| Flow Studio | SDK bans | Paused the harness that solves "built but not wired" |

"The methodology was always trying to exist, but kept getting trapped inside platform constraints."

Each constraint forced a workaround that turned out to be an improvement. The workaround IS the innovation.

## 5. The 78% Greenfield Market

78% of Perl developers use NO LSP at all (2025 Perl IDE Survey, 602 respondents). Steven didn't know this stat until a scout found it.

This changes the launch strategy entirely. The competition isn't PerlNavigator (51% of the 22% who DO use an LSP). The competition is "no tooling at all." The zero-dependency Rust binary is the unlock — users who couldn't install PerlNavigator (Node.js + Perl CPAN deps) can install perl-lsp (one binary).

## 6. The "Zero ROI" Tension

Every article frames perl-lsp as a success story. It IS technically excellent. It's also economically irrational — zero revenue, time diverted from paying work.

Both are true simultaneously. The tension is the story.

"This project has zero ROI. It was a mistake to start. Here's why it's the best Perl LSP anyway."

That's more compelling than any success narrative because it's honest, and because it explains why the quality is so high — there's no business pressure to ship prematurely.

## 7. The Real Origin Story

Not: "A Perl fan decided to build a modern Perl LSP."
Actually: "A broader tooling system hit Perl as the sharp edge. Fixing that sharp edge turned into a parser, then a language server, then a proving ground for AI-native maintainership."

Steven doesn't use Perl. Has never read Perl. Built the most comprehensive Perl parser anyway. The motivation was operational, not sentimental.

## 8. The Deepest Throughline

Parser level: "It wants to parse. Then it hits a wall."
SDLC level: "Code is cheap. Trusted change is not."

These are the same idea at two levels of the stack. Both say: the easy first draft is not the hard part. The hard part is what happens downstream — ambiguity, recovery, performance, context, verification, trust, maintainership.

That's the sentence underneath everything in this project.

# Codebase Curiosities and Learnings

*A source-grounded tour of the architectural habits, unusual patterns, and practical lessons hiding in `perl-lsp`.*

This version is intentionally based on the current tree rather than old PR archaeology. It focuses on what the codebase teaches a contributor who reads the implementation today.

---

## Snapshot of the Current Tree

These numbers were derived from the checked-out repository on 2026-03-18.

| Metric | Value | Why it matters |
|---|---:|---|
| Crate directories under `crates/` | 121 | The workspace is deeply decomposed into microcrates. |
| Rust source files (`*.rs`) | 1,536 | The codebase is broad as well as deep. |
| Rust lines | 540,278 | This is far beyond “single-crate side project” scale. |
| Markdown files (`*.md`) | 2,277 | Documentation is a first-class part of the system. |
| Perl fixtures / scripts (`*.pl`, `*.pm`, `*.t`) | 196 | Real Perl examples remain embedded in the Rust workspace. |

Smallest Rust crates by line count in `src/` today:

| Crate | Rust lines |
|---|---:|
| `tree-sitter-perl` | 0 |
| `perl-module-resolution` | 8 |
| `perl-module-token-parser` | 36 |
| `perl-line-index` | 44 |
| `perl-lsp-uri` | 50 |
| `perl-percentile` | 60 |
| `perl-workspace-ignore` | 60 |

Largest Rust files today:

| File | Lines |
|---|---:|
| `crates/perl-workspace-index/src/workspace/workspace_index.rs` | 3,860 |
| `crates/perl-ci-hygiene/src/main.rs` | 3,826 |
| `crates/perl-lexer/src/lib.rs` | 3,286 |
| `crates/perl-refactoring/src/refactor/refactoring.rs` | 3,261 |
| `crates/perl-semantic-analyzer/src/analysis/semantic.rs` | 3,256 |
| `crates/perl-dap/src/debug_adapter/mod.rs` | 3,200 |

### Learning

The project does not optimize for a small crate graph. It optimizes for **named boundaries**. If a concept can be isolated, it usually becomes its own crate.

---

## Curiosity #1: The Workspace Is Extremely Modular — but Not Uniformly Small

At first glance, `perl-lsp` looks like a “microcrate everything” experiment. That impression is correct, but only partially.

- Some crates are tiny wrappers or policy surfaces.
- Some crates are sharply focused utilities.
- A few crates still carry a large amount of system complexity.

This creates an interesting split:

- **Microcrates** express stable seams, ownership, and layering.
- **Large implementation files** still hold the complexity that cannot easily be fragmented without hurting readability or performance.

### Learning

A large Rust workspace does not necessarily imply fine-grained implementation everywhere. In this codebase, the crate graph is used to make **architecture visible**, while a few “gravity wells” retain the most complicated runtime and indexing logic.

---

## Curiosity #2: Dual Indexing Is Not a Detail — It Is a Foundational Rule

One of the clearest patterns in the codebase is the dual indexing strategy for Perl symbol resolution.

When the workspace index sees a function call, it records both:

- the **bare name** (`process_data`)
- the **qualified name** (`Utils::process_data`)

That behavior is documented directly in `workspace_index.rs`, and the implementation stores references under both keys. The tests in `crates/perl-workspace-index/tests/dual_indexing_tests.rs` then treat this as an architectural invariant rather than an implementation accident.

Why this matters in Perl:

- functions may be called unqualified from the current package
- imports can erase qualification at the callsite
- users still expect “find references” to work from either spelling

### Learning

This is a strong example of a codebase choosing **domain truth over purity**. A “cleaner” index might insist on one canonical key, but Perl’s calling conventions reward redundancy. The project accepts extra index entries in exchange for much better navigation behavior.

---

## Curiosity #3: “Never Panic” Turns into Real Design, Not Just Style Guidance

Many repositories say “avoid panics.” This one pushes the idea unusually far.

The most memorable example is `perl-lsp-uri`. Its fallback path tries several hardcoded URIs and, if parsing those ever fails, enters an open-ended loop generating `http://localhost/<n>` until one parses.

That is a funny piece of code, but it also reveals something serious: the project would rather produce a synthetic URI than crash because a supposedly impossible parser behavior changed.

The same philosophy shows up elsewhere:

- routing code explicitly prefers partial answers over hard failure
- workspace features degrade to same-file or open-document fallbacks
- text-based fallback extractors remain available when AST paths are unavailable

### Learning

In this codebase, resilience is treated as a product feature. The guiding question is often not “what is the elegant failure mode?” but rather “what useful result can still be returned safely?”

This oddity is now captured explicitly in [ADR-0037](../adr/0037-guaranteed-valid-uri-fallbacks.md), which documents why malformed URIs degrade to synthetic-but-valid identifiers at the protocol boundary.

---

## Curiosity #4: Degraded Mode Is a First-Class Operating State

The LSP runtime does not treat the workspace index as simply “ready” or “broken.” Instead, it models a lifecycle with explicit states such as:

- `Building`
- `Ready`
- `Degraded`

The routing layer then maps those states to access modes:

- full workspace access
- partial access
- no workspace access

That is notable because many language servers bury this policy inside scattered `if index_ready { ... }` checks. Here, the design is centralized in `runtime/routing.rs`.

The degraded reasons are also explicit:

- parse storm
- I/O error
- scan timeout
- resource limit

### Learning

This is a mature systems pattern: **make partial service explicit**. Contributors do not need to rediscover fallback behavior method by method, because the runtime encodes the operating modes as policy.

---

## Curiosity #5: The LSP Runtime Is a Hybrid, Not a Framework-Default Server

The `perl-lsp` binary uses a nice hybrid shape:

- a blocking reader thread consumes framed messages from stdin
- messages are sent through a Tokio `mpsc` channel
- both stdio and TCP eventually use the same async serving path

That is a practical architecture choice.

It avoids forcing the raw input side into a fully async design while still letting the main dispatch and serving path share one runtime model. The comments in `crates/perl-lsp-rs/src/main.rs` are explicit that stdio and TCP should converge on the same async dispatch path.

### Learning

This codebase repeatedly favors **operational simplicity over framework purity**. Instead of chasing an all-async ideal, it uses a mixed model that matches the realities of LSP framing and transport.

---

## Curiosity #6: Unsafe Exists, but the Safety Story Is Documented Right Next to It

`LspServer` has explicit `unsafe impl Send` and `unsafe impl Sync` because `ParentMap` holds raw pointers into AST nodes. That is the sort of line that should make a reviewer stop.

The good news is that the file immediately explains the invariant:

- access is synchronized through shared locking
- the raw pointers are tied to AST lifetime assumptions
- the surrounding fields are otherwise protected by atomics or mutexes

### Learning

This is a useful model for “necessary unsafe” in Rust infrastructure code:

1. keep the unsafe surface narrow
2. explain exactly why the compiler cannot prove the invariant
3. put the explanation at the unsafe boundary, not in a distant design doc only

---

## Curiosity #7: Tests Are Used as Executable Architecture Notes

The test suite does more than verify correctness. It preserves design intent.

A good example is `dual_indexing_tests.rs`, whose file-level commentary explains the rule before the assertions begin. The tests read like a mini-spec:

- qualified lookups must work
- bare lookups must work
- references must be discoverable from either query shape
- re-indexing must cleanly replace prior symbols

### Learning

This is particularly effective in a fast-moving workspace. When the architecture is spread over many crates, well-named tests become one of the best ways to communicate what must not regress.

---

## Curiosity #8: Documentation Is Not Side Material — It Is Part of the Architecture

The repository currently contains more than two thousand Markdown files. That is not normal overhead; it is a design choice.

The docs are doing several jobs at once:

- contributor onboarding
- architecture explanation
- CI and validation procedure capture
- anti-drift truth sourcing
- project forensics and lessons learned

The interesting part is that this mirrors the code layout itself. The repo likes explicit boundaries in both code and prose.

### Learning

If you contribute here, reading docs is not optional ceremony. It is part of understanding how the project maintains coherence across a very large workspace.

---

## Curiosity #9: The Codebase Likes Specialized Utility Crates More Than Generic Helpers

A recurring pattern is the presence of small crates with very specific jobs:

- URI parsing helpers
- line indexing
- percentile calculations
- workspace ignore handling
- diagnostic types
- module token parsing

These are not broad “utils” modules. They are named capabilities.

### Learning

This is an important maintainability lesson. Generic helper buckets tend to grow into junk drawers. The crate graph here pushes in the opposite direction: give a concept a precise name, then make dependencies talk through that named surface.

---

## Curiosity #10: The Largest File Is the Workspace Index, Which Tells You Where the Real Complexity Lives

The biggest Rust file in the workspace is `workspace_index.rs`. That feels right.

For a Perl language server, the hardest practical problem is often not tokenization or AST construction in isolation, but turning parsed code into useful editor behavior across files, packages, imports, and ambiguous naming patterns.

The file contains:

- lifecycle state modeling
- indexing rules
- symbol and reference lookup
- qualified/bare-name reconciliation
- degradation behavior hooks
- performance-oriented data structures

### Learning

The center of gravity in language tooling is often the **semantic and indexing layer**, not just the parser. This repository makes that visible.

---

## Practical Takeaways for New Contributors

If you are trying to become productive in this repository, these are the main lessons to internalize first:

1. **Expect named boundaries everywhere.** Before adding to a generic module, check whether an existing crate already owns that concept.
2. **Preserve fallback behavior.** The codebase values useful degraded behavior over brittle all-or-nothing correctness.
3. **Treat dual indexing as policy, not optimization.** If you touch navigation or references, verify both qualified and unqualified forms.
4. **Read the runtime routing before changing handlers.** Index state and degraded behavior are centralized on purpose.
5. **Do not dismiss the docs as stale by default.** In this repo, docs are part of the operating model.
6. **Look for the safety invariant when unsafe appears.** The code often documents why an unusual boundary exists.

---

## How This Document Was Derived

This write-up was based on direct inspection of the checked-out tree, especially:

- `crates/perl-workspace-index/src/workspace/workspace_index.rs`
- `crates/perl-workspace-index/tests/dual_indexing_tests.rs`
- `crates/perl-lsp-rs/src/runtime/routing.rs`
- `crates/perl-lsp-rs/src/runtime/mod.rs`
- `crates/perl-lsp-rs/src/main.rs`
- `crates/perl-lsp-uri/src/lib.rs`

It also used lightweight repository-wide counts for crate totals, file counts, line counts, and largest/smallest crate snapshots.

---

## Closing Thought

The strongest impression from the current tree is that `perl-lsp` is not merely a parser plus an editor server. It is a repository that has spent a lot of effort encoding **operational knowledge** into code structure:

- explicit states instead of ambient assumptions
- named crates instead of convenience buckets
- fallbacks instead of brittle hard stops
- tests and docs as architecture carriers

That combination makes the codebase feel unusual, but also surprisingly legible once you understand its core habits.

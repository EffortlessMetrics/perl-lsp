# Codebase Field Notes

*A source-driven tour of curiosities and practical learnings from the perl-lsp workspace.*

This note is intentionally different from the historical and PR-oriented writeups in
[`CODEBASE_CURIOSITIES.md`](CODEBASE_CURIOSITIES.md) and [`LESSONS.md`](LESSONS.md).
It focuses on what the current codebase itself teaches: small patterns that repeat,
constraints that shaped architecture, and implementation details that are easy to miss
when you only skim the top-level crates.

---

## 1. The workspace optimizes for **explicit seams**, not minimal crate count

The repo is large, but its size is not accidental. The dominant pattern is to split
mechanisms into narrow crates with sharply defined responsibilities.

Examples:

- `perl-line-index` exists solely to map byte offsets to line/column pairs.
- `perl-lsp-uri` exists solely to parse and normalize URI handling for LSP-facing code.
- The feature-governance stack is decomposed into catalog parsing, feature IDs,
  capability mapping, profile parsing, policy, grid reporting, and a facade crate.

### Curiosity

The smallest utility crates are *tiny* by normal Rust standards, but they still get the
full treatment: their own package manifest, tests, docs, and lint posture.

### Learning

This codebase treats crate boundaries as a design tool. The benefit is not just compile
parallelism; it also makes hidden coupling visible. If a concept starts needing many
unrelated dependencies, that pressure becomes obvious because the crate stops feeling
"small and pure."

---

## 2. Dual indexing is the architectural answer to Perl's name-resolution ambiguity

One of the clearest recurring ideas in the workspace is that Perl symbol lookup cannot
rely on a single canonical spelling. A function may be invoked as either:

- `function()`
- `Package::function()`

The workspace index handles this by storing references under both the bare name and the
qualified name whenever it sees a function call.

### Curiosity

This is not just a doc claim; the implementation literally writes two entries for the
same usage, then relies on later deduplication at query time.

### Learning

Instead of trying to "solve" Perl's ambiguity with one perfect lookup key, the codebase
solves it with redundancy plus deduplication. That is a useful design lesson for dynamic
languages generally: when the source language is context-heavy, sometimes the robust
solution is to preserve multiple interpretations rather than overcommitting early.

---

## 3. The repo prefers **build-time truth generation** over runtime discovery

The feature-governance subsystem is a good example. `features.toml` is treated as the
single source of truth, and the contracts crate compiles that catalog into generated
Rust during `build.rs`.

### Curiosity

The build script has a fallback path: if catalog loading fails, it still writes a fallback
module rather than breaking downstream consumers in an uncontrolled way.

### Learning

The pattern here is: validate once, generate once, and let the rest of the system depend
on constants. That keeps runtime code simpler and reduces drift between docs, server
capabilities, and test/reporting surfaces.

---

## 4. The codebase is allergic to panics, and that changes the shape of implementations

The workspace lints deny `unwrap`, `expect`, and `panic!` family usage at the workspace
level, and the CI hygiene tooling adds ratchet-style checks on production code.

### Curiosity

The `perl-lsp-uri` crate shows how far this philosophy can go. If several hardcoded
fallback URIs all fail to parse, it enters an open-ended loop generating
`http://localhost/{n}` candidates until parsing succeeds.

That is an unusual choice, but it is deeply consistent with the project's rule: do not
crash LSP-facing code because a fallback path became invalid.

### Learning

The lesson is not "never panic at any cost" so much as: if reliability is the priority,
error-handling policy must become architecture, not style. Once the repo commits to that
policy, even small helpers are designed around graceful degradation.

---

## 5. Lexer hardening is treated as a product feature, not a parser footnote

The lexer has explicit budgets for regex bytes, heredoc bytes, delimiter nesting,
heredoc nesting, and heredoc parsing timeouts.

### Curiosity

The comments make the intent explicit: when limits are exceeded, the lexer degrades into
recoverable tokens instead of hanging. The focus is not just syntactic correctness, but
editor survivability on pathological input.

### Learning

This is a strong IDE-oriented design instinct. In an interactive tool, "doesn't hang on
malicious or bizarre input" matters almost as much as "parses valid input perfectly."
The repo consistently values bounded behavior over theoretical completeness in degenerate
cases.

---

## 6. Even the "small" abstractions carry strong operational intent

`perl-line-index` is a good example. It is only a few dozen lines, but it encodes one
precise invariant: offset/position mapping should be cheap, deterministic, and reusable.

### Curiosity

Because the crate does almost nothing else, its API surface is easy to reason about,
and its implementation is almost self-documenting.

### Learning

The repo shows that a microcrate only works if the abstraction is genuinely crisp. Tiny
crates are not automatically good. They become valuable when each one creates a place
where behavior can be named, tested, and reused without dragging in the world.

---

## 7. Documentation in this repo often describes a **control system**, not just code

A lot of project documentation is operational: feature governance, quality surfaces,
status computation, drift prevention, receipts, CI gates, and architecture decisions.

### Curiosity

That emphasis mirrors the code. The implementation is full of places where metadata,
policies, and generated artifacts act as first-class components.

### Learning

perl-lsp is not just a parser or an LSP server; it is a codebase that keeps trying to
make its own promises machine-checkable. That is one reason there are so many helper
crates and scripts: the system repeatedly converts tribal knowledge into enforceable
surfaces.

---

## 8. A useful mental model: this is a reliability-first tooling workspace

After reading across the code and docs, a recurring pattern emerges:

- explicit lifecycle/state modeling,
- generated truth surfaces,
- dual representations for ambiguous language constructs,
- bounded parsing budgets,
- anti-panic fallbacks,
- narrow utility crates with strong invariants.

### Learning

The codebase is easiest to understand when viewed through that lens. Many unusual choices
make more sense once you assume the primary goal is not elegance in isolation, but stable,
inspectable tooling behavior across messy real-world Perl.

---

## Suggested follow-up reading

If you want to continue this tour from adjacent angles:

- [`CODEBASE_CURIOSITIES.md`](CODEBASE_CURIOSITIES.md) for historical oddities and repo trivia.
- [`LESSONS.md`](LESSONS.md) for mistakes, drift incidents, and process corrections.
- [`FEATURE_GOVERNANCE.md`](FEATURE_GOVERNANCE.md) for the catalog-driven capability system.
- [`CUSTOM_LSP_RUNTIME.md`](CUSTOM_LSP_RUNTIME.md) for why the LSP runtime is custom.
- [`PARSER_EVOLUTION.md`](PARSER_EVOLUTION.md) for the parser design trajectory.

# Codebase Curiosities and Learnings

This note captures architectural patterns that stand out when reading the `perl-lsp` workspace.
It is intentionally qualitative: the goal is to help future contributors quickly understand what is unusual, elegant, or easy to miss.

## 1. The workspace is intentionally microcrate-heavy

A striking property of the repository is how aggressively functionality is split into small crates.
This is not just an organizational preference; it is reflected in the workspace membership list and the publish allowlist tiers in the root `Cargo.toml`.

### What to learn from it

- The repository treats crate boundaries as an architectural tool, not just a packaging concern.
- Many “simple” features are assembled from narrow crates instead of being implemented inside `perl-lsp` directly.
- If you want to add a feature, it is usually worth asking whether it belongs in an existing microcrate, a new microcrate, or only in the application binary.

### Why this is interesting

The publish metadata effectively documents the dependency layering of the system.
That means release policy and architecture are coupled in a deliberate way: the workspace structure itself tells you which crates are foundational and which ones are application-facing.

## 2. Performance constraints are encoded directly in the libraries

Several crates do not merely mention performance in docs; they bake defensive limits and fast paths into the implementation.
The lexer defines explicit scan budgets for regexes, heredocs, and delimiter nesting.
The cancellation crate has dedicated hot-path checks and a small token cache.
The workspace index state machine stores timestamps and coarse state kinds for routing and instrumentation.

### What to learn from it

- This codebase assumes editor-facing workloads where latency matters all the time, not only during optimization passes.
- Guardrails against pathological input are treated as part of correctness.
- When adding new parsing or indexing logic, it is worth thinking in terms of “budgets”, “degradation”, and “observable states”, not just happy-path functionality.

### Why this is interesting

Many language tools talk about performance, but here the design is visibly shaped by it.
The code often prefers bounded behavior and graceful degradation over theoretical completeness in edge cases that could stall the editor.

## 3. UTF-16 correctness is treated as a first-class systems problem

The workspace contains multiple layers for position conversion and line indexing instead of a single one-size-fits-all helper.
There are standalone conversion functions, cache-based helpers, and an owning `LineIndex` type.
This suggests position handling is important enough to justify multiple APIs optimized for different call sites.

### What to learn from it

- LSP position handling is not “just string slicing” in this repository.
- The code assumes Unicode correctness has to survive both ergonomic and performance-sensitive pathways.
- If you touch offsets, ranges, or protocol positions, expect there to be an existing helper that matches your exact use case.

### Why this is interesting

The coexistence of several position helpers is a clue that this area was painful enough, or performance-sensitive enough, to deserve specialization rather than premature unification.

## 4. Security hardening shows up in small defaults, not only in dedicated audits

The security posture is visible in normal runtime code.
For example, workspace path validation rejects traversal and control-character tricks.
Completion-path helpers reject suspicious path forms before directory traversal begins.
Workspace configuration also keeps `use_system_inc` disabled by default and only fetches `@INC` lazily when explicitly enabled.

### What to learn from it

- The secure default is often “off unless explicitly enabled”.
- Input validation and traversal protection are spread through the normal code paths, not bolted on later.
- Feature additions that touch file paths, subprocesses, or editor-supplied input should preserve this bias toward explicit opt-in.

### Why this is interesting

This is a good example of practical hardening: the safest behavior is usually the default behavior, and expensive or risky environment inspection is lazy.

## 5. Feature governance is implemented as data + translation layers + a façade

The LSP feature system is more structured than a typical server-capabilities builder.
A root `features.toml` file acts as the catalog.
Dedicated crates translate between feature IDs and protocol capabilities.
A governance façade then re-exports the profile, contract, grid, and policy APIs behind a single boundary.

### What to learn from it

- The feature list is designed to be machine-readable and reused by multiple consumers.
- Capability advertisement, profile policy, reporting, and runtime selection are intentionally separated concerns.
- If a new LSP feature is added, the “real” work is probably spread across data, mapping, policy, and tests rather than one handler function.

### Why this is interesting

This design turns feature support into a governed catalog rather than a pile of booleans.
That makes it easier to reason about release profiles, compliance reporting, and future automation.

## 6. Documentation is treated as an operational interface

The documentation tree is not just narrative prose.
The docs index names canonical truth sources, and `CURRENT_STATUS.md` explicitly separates machine-updated sections from narrative sections.
That creates a lightweight contract: contributors can tell which numbers are computed, which files are authoritative, and how to refresh them.

### What to learn from it

- Some docs are intended to be read like dashboards, not essays.
- Metrics and status statements are expected to be reproducible from commands.
- When documenting new project-wide facts, it is worth deciding whether they belong in a generated truth source, a curated explanation, or both.

### Why this is interesting

Many repositories accumulate stale status docs.
This one tries to prevent that by making freshness rules explicit.

## 7. The codebase favors stable boundaries over deep re-export leakage

A repeated pattern is to hide internal complexity behind a small public entry point.
The feature-governance crate is a clear example: instead of forcing callers to know which profile, policy, contract, or grid crate to import, it exposes a curated façade.
The configuration crate plays a similar role for runtime settings that might otherwise drift into the main server binary.

### What to learn from it

- A good change often means improving a boundary, not just adding code.
- The workspace tolerates internal decomposition as long as public entry points stay understandable.
- If you introduce a new subsystem, think about its façade early.

## Suggested contributor heuristics

When working in this repository, these questions seem especially valuable:

1. **Is this concern performance-sensitive enough to need a budget or cache?**
2. **Should this behavior be secure-by-default and opt-in for expensive/risky paths?**
3. **Does this belong in a reusable microcrate instead of a top-level binary crate?**
4. **Is there a canonical data source or façade that should own this change?**
5. **Will this affect UTF-16/LSP position correctness in any way?**

## In one sentence

The main learning from the codebase is that `perl-lsp` is not just a parser or server implementation; it is a deliberately decomposed, policy-aware, performance-bounded tooling platform.

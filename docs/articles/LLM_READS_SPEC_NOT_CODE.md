# Your LLM Reads the Spec, Not the Code: A Test-Writing Failure Mode with Growth Data

**Date**: 2026-04-19
**Session**: Wave G1 collapse on perl-lsp
**Cross-references**: issue #4513 (process fix), [forensics/2026-04-19-wave-g1-collapse-retrospective.md](../forensics/2026-04-19-wave-g1-collapse-retrospective.md)

---

## TL;DR

An AI red-TDD agent — one that writes failing tests from a spec before the implementation — consistently writes tests against the **spec's description of the API** rather than the **code's actual public surface**. When the two disagree, the builder later has to "correct" the tests, and each correction costs a round-trip of deep-review scrutiny to verify intent preservation.

On perl-lsp's 2026-04-19 Wave G1 collapse session, this failure mode had measurable growth between two waves of the same collapse pattern:

- **Wave G1a** (PR #4506, 15 crates): **3** API-shape fixes by builder
- **Wave G1b** (PR #4510, 10 crates): **6** API-shape fixes by builder

The per-wave count roughly doubled. Each fix was mechanical (wrong constructor args, wrong generic params, wrong `Default` derive, wrong field names) — not semantic drift. But the trajectory is a process smell: at the same growth rate, a hypothetical G2 would need 12 fixes and G3 would need 24. That scaling is not sustainable.

This article names the failure mode, shows its mechanism, and proposes two complementary fixes. The short version: the red-TDD agent should be instructed to read actual `pub struct` / `pub fn` / `pub use` declarations before writing tests; the spec-planner agent should enumerate public surfaces in `context.md` so the red-TDD agent has a structured source to consume.

---

## The Pattern

Red-TDD is given:
- The issue (scope + acceptance criteria)
- The plan-reviewer's refined spec
- The `.spec/<issue>/` files produced by spec-planner
- The existing repository at current master

Its job: write tests that fail against current master and will pass after the builder implements. Red-TDD's output is a commit of failing tests on the impl branch.

The failure mode: red-TDD reads the spec — which describes, in prose, "what should exist at `<path>`" — and writes tests that try to construct or call those existing-in-the-future types. But the spec is abstract. It doesn't enumerate:

- Whether `Foo::new()` takes arguments or is no-arg
- Whether `Foo` implements `Default`
- Whether a field `index` is `Option<Index>` or plain `Index`
- Whether a public re-export exists at a shorter path
- Whether a constructor method is named `new` or `with_source` or something else
- What lifetime parameters or trait bounds apply

Red-TDD fills in those details with idiomatic-Rust defaults. The default for "constructor" is `::new()`. The default for "simple struct" is `#[derive(Default)]`. The default for "collection parameter" is `Vec<T>`. These defaults often don't match the actual code.

When the builder goes to make the tests pass, it discovers the tests reference APIs that don't exist in the form tested. The builder has two options: (1) change the implementation's public API to match the test's expectation, or (2) correct the test to match the actual API. Option 1 is dangerous — it expands the PR's public surface and can break unrelated consumers. Option 2 is safe but requires a comment annotating that the test was corrected for shape, not for semantics (so deep-review can verify the test's intent is preserved).

On perl-lsp, the builder annotates each such correction with a `NOTE(<wave>-API-fix)` comment on the fixed test. Deep-reviewer then scans for those comments and verifies each one is mechanical, not semantic.

## The Growth Data

The collapse-wave structure gave us natural paired measurements:

### Wave G1a — 15 low-risk provider crates → `perl-lsp-rs-core::providers::*`

Red-TDD wrote 21 failing tests. Builder's post-hoc count of API-shape corrections: **3**.

| Crate | Test's expected API | Actual API |
|---|---|---|
| `perl-lsp-completion-item` | `CompletionItem` wire format | `CompletionItem` domain type |
| `perl-lsp-formatting` | `FormatRange` with different field names | Actual field names |
| `perl-lsp-document-links` | `compute_links(...)` with wrong arity | Different arity |

Each was a mechanical signature mismatch. Builder fixed the tests, added `NOTE(G1a-API-fix)` comments, deep-reviewer verified intent preservation. No semantic drift. PR #4506 merged.

### Wave G1b — 10 medium-risk provider crates → same destination

Red-TDD wrote 40 failing tests (more because wave-G1b is a bigger aggregator absorption). Builder's post-hoc count of API-shape corrections: **6**.

| Fix | Nature |
|---|---|
| `NavigationProvider` facade | Test expected `Default` + no-arg `new()`; actual: takes required `Node` param |
| `FormattingProvider` generic | Test wrote concrete type; actual: generic with a specific bound |
| `OpenAiConfig` no-`Default` | Test constructed via `Default::default()`; actual: no `Default` derive |
| `CompletionProvider` index param | Test omitted index; actual: required |
| `CodeActionsProvider` source param | Test omitted source; actual: required |
| `SignatureHelpProvider` AST param | Test expected `&str`; actual: `Node` |
| `LinkedEditing` u32 args | Test expected a tuple; actual: two separate u32 arguments |

(Seven items because one of the six fixes touched two related tests.)

Again, all mechanical. But **6 fixes on 10 absorbed crates** vs. **3 fixes on 15 absorbed crates** — the per-crate rate of wrong-API-shape went from 20% to 60%. That's a 3x increase on a comparable absorption task. The pattern is not just persistent; it's accelerating.

## Why The Growth?

The candidate explanations, in declining order of likelihood:

1. **More complex APIs.** G1b's 10 crates are medium-risk specifically because they have more interrelated types (aggregators, providers with multiple constructors, types with non-trivial trait bounds). More surface area = more chances to guess wrong.
2. **Less overlap with existing G1a patterns.** G1a was "leaf providers, no cross-deps." G1b is "aggregator + snapshot-heavy + intra-deps." The idiomatic-Rust defaults work less well on the less-idiomatic types.
3. **Red-TDD didn't reference G1a's actual migration patterns.** G1a merged 3 hours before G1b red-TDD ran. The agent could have consulted the merged G1a diff for "this is what absorbed-provider tests look like in practice" — but it was prompted against the spec, not against the recent merge.
4. **Token budget for reading source is implicit.** Red-TDD's prompt doesn't say "read `crates/<crate>/src/lib.rs` for each crate you're testing." It says "write tests that match the spec's acceptance criteria." The agent optimizes for the explicit instruction; the implicit instruction (verify against actual API) is left as a judgment call.

Explanation 4 is the actionable one.

## The Fix

Two complementary changes, filed as issue #4513:

### Fix A — Red-TDD prompt update

Add an explicit pre-test read step to the red-TDD skill:

> Before writing any test that references a symbol in an absorbed crate, read that crate's `src/lib.rs` + any relevant `pub struct` / `pub fn` / `pub use` declarations. Test against THOSE signatures, not inferred ones. If you cannot locate a signature, DO NOT write the test — flag it for the builder with a `// TODO: signature unclear — API shape TBD` comment.

This moves the implicit instruction ("verify against actual API") into explicit territory. The agent is token-budgeted to read the source as part of its core job, not as an optional side-check.

### Fix B — Spec-planner enumerates public surfaces

Update the spec-planner skill to require, for any absorption/refactor issue, a "Public API surfaces" section in `context.md` that enumerates each absorbed crate's constructors, trait impls, and notable type shapes.

Example:

```markdown
## Public API surfaces (for red-TDD consumption)

### perl-lsp-rename → providers::rename
- `pub struct RenameProvider { node: Node, source: String }`
- `RenameProvider::new(node: Node, source: String) -> Self`  — NO no-arg constructor, NO Default
- Public methods: `rename(offset: usize, new_name: &str) -> RenameResult`

### perl-lsp-diagnostics → providers::diagnostics
- [... per-crate enumeration ...]
```

With this, red-TDD reads one clean section of `context.md` instead of navigating 10 crate source trees. The cost is spec-planner does a pre-existing read anyway; formalizing it saves red-TDD a navigation step and standardizes the format.

## Why It's Worth Naming

This is a specific, reproducible failure mode of AI-assisted TDD. It's not a universal "LLMs are bad at code" complaint; it's a targeted observation: **when an LLM reads a spec and a codebase, it optimizes the one in its explicit prompt**. In red-TDD's case, the explicit prompt is the spec. The codebase reading is a judgment call.

The fix is to make the codebase reading an explicit part of the prompt. That's a one-paragraph prompt change. The cost is negligible; the expected benefit — cutting G2 API-shape fixes from a projected 12 to ≤2 — is substantial.

This is also the kind of pattern that only surfaces with **paired measurements**. If we'd run only G1a, we'd see 3 fixes and call it normal noise. If we'd run only G1b, we'd see 6 fixes and call it a big-absorption artifact. The 3 → 6 trajectory is the signal, and it only appeared because the same agent, on the same codebase, did the same kind of task twice in the same day. Collapse-wave programs are unusually well-suited to producing this kind of data.

## Related

- Issue #4513 — process fix filed this session
- [forensics/2026-04-19-wave-g1-collapse-retrospective.md](../forensics/2026-04-19-wave-g1-collapse-retrospective.md) — full session retrospective §3
- [FOUR_WAY_ENSEMBLE_PATTERN.md](FOUR_WAY_ENSEMBLE_PATTERN.md) — related pattern (ensemble verification)
- [KNOWLEDGE_COMPOUNDING.md](KNOWLEDGE_COMPOUNDING.md) — meta-pattern (learning accumulates only when instrumented)

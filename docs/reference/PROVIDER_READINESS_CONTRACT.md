# Provider Readiness Contract

> **Scope:** The doctrine governing how every index-dependent LSP provider behaves
> during workspace indexing, post-edit staleness, and other adverse timing states.
> **Umbrella issue:** [#3099](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/3099).
> **First concrete fix:** [#3097](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/3097)
> (point-query waits for 7 providers).
>
> For the CI gate mechanics that gate these providers, see
> [CI_GATE_PLAYBOOK.md](CI_GATE_PLAYBOOK.md).

---

## The Problem: empty success is a lie

When the workspace index is still building (`IndexState::Building`), an LSP
provider that returns an empty success response — no results, no error, no
explanation — teaches the user not to trust the tool. The editor shows nothing;
the user re-triggers the request; the index finishes; the next request works;
the user concludes the LSP is flaky.

**Empty success during indexing is not a neutral non-answer. It is a false
negative that erodes trust.** The correct contract is: if you cannot give a
reliable answer, say so explicitly (partial, stale, timed-out) or block briefly
until you can.

---

## Policy: readiness is per-provider, not global

Do NOT paste `wait_for_index_ready_if_building()` everywhere. The correct policy
depends on the provider's semantics:

| Provider / operation | Timing class | Policy |
|---|---|---|
| completion, definition, hover, references, signature-help, implementation, call-hierarchy | point query | **WaitBriefly** — bounded wait (≤2 s), then explicit partial/fallback reason |
| diagnostics, semantic-tokens | background stream | **SnapshotOnly** — defer or reschedule on index-ready event; do NOT block the stream |
| rename, code-actions affecting structure | unsafe edit | **FailClosed** — a bad edit is worse than no edit; refuse when stale or partial |
| document-symbols | mostly local | **LocalOnly** — prove it needs the index before adding a wait; default to local-only |
| workspace/symbol, inline-completion (module path) | precedent | already have the wait — confirm contract + regression-lock |

This matrix is authoritative. Deviation requires a comment explaining why the
provider's policy differs from its timing class.

---

## Shared API shape (proposed in #3099)

Rather than ad-hoc `wait_for_index_ready_if_building()` calls scattered across
providers, the contract becomes intentional through a shared enum:

```rust
pub enum IndexReadinessPolicy {
    WaitBriefly,   // point queries: bounded block until Ready
    SnapshotOnly,  // background streams: snapshot what's available, reschedule
    FailClosed,    // unsafe edits: refuse if stale or partial
    LocalOnly,     // document-scoped: skip the index entirely
}

pub enum IndexReadinessOutcome {
    Ready,      // index was Ready before the call
    Waited,     // WaitBriefly: index became Ready during the wait
    Partial,    // WaitBriefly: timed out; result is best-effort
    Stale,      // SnapshotOnly: result is from an earlier generation
    TimedOut,   // WaitBriefly: deadline expired, no index available
}
```

The outcome propagates to the response (partial result token, `result.metadata`,
or an `LSPAny` annotation — exact wire shape TBD in #3099).

---

## Three adverse states to dogfood

The test axis is **"is the provider honest under bad timing?"**, not **"is the
semantic answer correct?"**.

### 1. Request during indexing

Force `IndexState::Building` with a **deterministic barrier** (a latched atomic
or test-injected delay). Do NOT use `sleep` — sleeps produce flaky tests, not
race regression tests.

The `#3069` bug class: a provider returns `success + empty + no explanation`
during `Building`. This is banned. Every scenario_22-class test must assert
a non-null or explicitly partial result, never a silent empty.

Fix coverage: #3097 adds deterministic race-regression tests for 7 providers
(references, hover-inherited, signature-help, implementation, call-hierarchy).

### 2. Post-edit staleness (not yet audited)

Every index-backed answer implicitly carries a generation number: "I answered
from the index as of generation N." If the document has been edited since
generation N, the answer may be wrong.

The correct behavior: if `index_generation != doc_generation`, either
reschedule, return an explicit `Stale` outcome, or refuse the unsafe operation.

**This dimension is NOT yet audited.** It is the next trust failure class after
startup correctness: "I edited the file and the LSP answered from the old world."
Tracked as part of the readiness CONTRACT phase in #3099.

### 3. Malformed / huge / weird input

**AUDITED CLEAN** as of 2026-06 (no panics, no hangs on fuzz sweeps). Revisit
only if a future fuzz sweep surfaces a real panic. Not a current action item.

---

## Test-honesty rule

A test that passes on un-fixed `main` — before the fix lands — is a **capability
guard**, not a race guard. Capability guards are valuable (they confirm a feature
exists), but they do not prove race-safety.

Race-safety tests must:
- issue the request with NO settle delay (no `sleep`, no `wait_for_ready`)
- assert a non-empty or explicitly annotated result
- fail on un-fixed `main` (otherwise they guard nothing)

Mixed proof — a test that passes on both fixed and un-fixed code because it
waits long enough for the index to finish — provides no signal. Reject it.

---

## Validation: use the real VSCode harness

Provider changes in this contract class must be validated via the real LSP
integration test harness (`perl-lsp-ux-tests`), not proxy-only unit tests.

A unit test that directly calls `handle_references_inner(...)` proves the
function logic, not that the production JSON-RPC path (`textDocument/references`
→ dispatcher → handler) reaches the wait correctly. The race lives in the
production path. Verify there.

---

## Related issues and PRs

| # | Description |
|---|-------------|
| [#3099](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/3099) | Readiness CONTRACT umbrella (policy enum + post-edit staleness) |
| [#3097](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/3097) | Point-query waits: 7 providers fixed (in flight) |
| [#3096](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/3096) | `$/progress` indexing UX — surface progress during the wait |
| [#3095](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/3095) | Index readiness dogfood audit |
| [#3080](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/3080) | Diagnostics / `source.organizeImports` code action |

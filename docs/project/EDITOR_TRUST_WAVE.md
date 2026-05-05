# Editor Trust Wave (Issue #7952)

This plan defines the next improvement frame for `perllsp`: prioritize editor trust over broad capability expansion.

## North star

A conservative Perl LSP that:

1. Never hangs.
2. Never lies confidently.
3. Never regresses silently.
4. Recovers while users are typing.
5. Makes every improvement measurable.

## Sequencing discipline

Complete the editor-trust lanes before broad new capability:

1. Security and repository trust.
2. Availability and incremental-state resilience.
3. Completion proof for unknown-receiver fallback.
4. Diagnostics truth with semantic evidence.
5. Stable ratchets and queue hygiene.
6. Architecture boundary cleanup.

## Completed control lanes

Do not reopen these as new implementation work unless a fresh regression proves
they drifted:

- Parser orphan-delimiter hang recovery (#7891).
- Checkpoint anchor correctness after leading edits (#7932).
- Queue review-receipt trust boundary (#7946).
- Bounded unknown-receiver method completion fallback (#7930).
- Real-workspace unknown-receiver fallback baselines (#7961).
- Nightly/label-gated scorecard ratchet wiring (#7945).
- UX receipt routing and tree-sitter query-conformance canonical merges (#8022, #8021).
- Quote-like unclosed diagnostic recovery (#8004).
- Parser coverage risk map advisory baseline (#8005).
- Workspace crate naming drift guard (#7985).

## Completion policy (conservative by design)

Keep candidate generation bounded and focus on ranking quality:

- Exact receiver evidence ranks highest.
- `Foo->new` assignment evidence ranks high.
- Literal `bless {}, "Foo"` evidence ranks medium.
- Unknown receiver fallback remains low-confidence and bounded.
- Dynamic receivers remain fail-closed.
- No all-workspace method dump fallback.

Current completion target: keep the shipped fallback honest with provider-impact
evidence and ranking checks. Any follow-up should verify:

- Useful fallback hits: positive.
- Unrelated method leaks: `0`.
- Dynamic fallback leaks: `0`.
- Exact receiver regressions: `0`.

## Diagnostics policy

Suppress diagnostics only when indexed semantic evidence supports suppression; keep dynamic uncertainty conservative.

Validation priorities:

1. Reconcile diagnostics status docs (#7947).
2. Add order-aware dynamic suppression fixtures (#7948).
3. Add real-workspace semantic baseline (#7949).

Core fixture expectations:

- Generated symbols from `eval` can suppress matching diagnostics.
- Missing symbols remain diagnosed even when nearby dynamic code exists.
- Import order is respected (no premature suppression before import points).
- Legacy behavior remains unchanged when semantic index evidence is absent.

## Parser recovery policy

Target reliability under incomplete code, not only valid files.

Must hold under edit:

- Incomplete interpolation does not collapse top-level parse.
- Missing RHS or malformed local statements recover locally.
- Incomplete deref/method calls preserve symbol and completion usefulness.
- Malformed corpus input never panics.

Adopt and maintain a hard floor via parser panic-free invariant checks (issue #4916).

## UX evidence substrate

Consolidate editor-facing verification through one canonical harness and fixture schema:

`fixtures/editor_ux/*.json` -> shared UX harness -> normalized responses -> scorecard JSON -> status dashboard -> ratchet checks.

Minimum scorecard dimensions:

- Hover correctness.
- Completion relevance (top-1/top-5).
- Definition exact-hit rate.
- Document/workspace symbol quality.
- Incremental stale-symbol eviction.
- Diagnostics false positive/negative counts.
- Request latency p50/p95.

## `@INC` consumer consistency

Enforce one module-resolution policy across completion, definition, hover, diagnostics, and workspace symbols.

Default policy should remain conservative and explicit:

- Workspace root: enabled.
- Configured `includePaths`: enabled.
- Lexical `use lib`: enabled when statically visible.
- System `@INC`: opt-in.
- `PERL5LIB`: opt-in, never implicit.
- Dynamic `require`: evidence-only.

## Ratchet rollout model

Promote metrics in stages:

1. Nightly (broad evidence, non-disruptive).
2. Label-gated PR checks.
3. Merge-blocking only for deterministic, cheap, low-flake floors.

The shared ratchet wiring and operational guide landed in #7945. Future PRs
should promote only proven rows, not all available scorecard output.

Recommended first merge floors:

- Parser panic count = `0`.
- Semantic shadow regressions = `0`.
- Dynamic false-precision count = `0`.
- Exact receiver regression count = `0`.
- Stale-index defect count = `0`.
- UX scenario regressions = `0`.

## Proposed PR sequence

1. `chore(queue): classify open PRs by product value and duplicate risk` (#7950).
2. `docs(status): reconcile live dynamic diagnostics evidence` (#7947).
3. `test(diagnostics): add order-aware dynamic bareword suppression fixtures` (#7948).
4. `test(semantic): add CPAN-style real-workspace baseline` (#7949).
5. `fix(module-resolution): unify include-path behavior across completion, goto, hover, diagnostics` (#7570).
6. `fix(parser): recover incomplete indexed interpolation without ERROR cascade` (#5715).
7. `test(ux): route editor-facing scenarios through canonical harness` (#5306).
8. `feat(ux): add machine-checkable editor UX fixture schema` (#5307).
9. `feat(metrics): promote only stable, cheap scorecard floors using the #7945 ratchet substrate`.
10. `refactor(semantic): clean producer boundaries after the current correctness queue is below threshold`.

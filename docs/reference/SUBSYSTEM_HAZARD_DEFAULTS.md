# Subsystem Hazard-Default Templates

**Purpose**: When spec-planner seeds an `acceptance.md` for a new issue, it should consult
this file to pre-populate the hazard-invariant rows that apply by default for the affected
subsystem. Every row below is a standing obligation — the builder must address it, and the
red-TDD builder must write an adversarial test for it.

This is the repo-specific application of the generic hazard taxonomy.
Generic class definitions (the six canonical classes, when to apply each, and adversarial
test patterns) live in [docs/concepts/hazard-class-invariants.md](../concepts/hazard-class-invariants.md).
The checklist gate that enforces these rows at review time is
[docs/agents/SPEC_UPDATE_CHECKLIST.md §8](../agents/SPEC_UPDATE_CHECKLIST.md#8-hazard-class-invariants).
Repo-specific incidents that motivated these classes are indexed in
[docs/learnings/README.md](../learnings/README.md).

---

## How to use this file

1. Identify the subsystem(s) your change touches (DAP, Parser, LSP, Coverage/CI).
2. Copy the applicable rows verbatim into your `acceptance.md` under a **Hazard invariants** heading.
3. For each row, fill in the `Surface` field with the specific function/struct/file touched.
4. Red-TDD builder writes the corresponding adversarial test before any implementation begins.

A row may be omitted only when the specific surface is provably not touched — note the
reasoning explicitly in `context.md` rather than silently dropping the row.

---

## DAP subsystem

Any change touching `crates/perl-dap/`, `crates/perl-dap-*/`, or the DAP bridge
(`crates/perl-lsp-rs/src/dap*`) should default to the following rows.

### DAP-1: ID / ref-space collision

| Field | Value |
|---|---|
| **Invariant** | All numeric reference spaces (variablesReference, frameId, scope IDs, evaluate-result refs, thread IDs) are provably disjoint. No two allocators share an untyped integer range without a named constant boundary and a compile-time or test-time disjointness proof. |
| **Trigger** | Any newly allocated numeric range or changed allocation formula in DAP state |
| **Required adversarial test** | Allocate one ID from the new range and one from each adjacent existing range; assert they are never equal. Assert that a lookup using an ID from range A into table B returns an error or empty result — not a stale entry from another session. |
| **Motivating incident** | [docs/learnings/2026-06-dap-ref-space-collision.md](../learnings/2026-06-dap-ref-space-collision.md): PR #1219 allocated base 50\_000; existing scope refs used `frame_id*10+scope_type`, colliding at frame\_id=5000. |
| **Ref** | Class 1 in [docs/concepts/hazard-class-invariants.md](../concepts/hazard-class-invariants.md) |

### DAP-2: Bounds / overflow on client-supplied IDs

| Field | Value |
|---|---|
| **Invariant** | All `frameId`, `variablesReference`, `threadId`, and `stackDepth` values originating from a DAP client request are validated before any array subscript or arithmetic. Out-of-range → honest `ErrorResponse`, never a panic or silent wrap. |
| **Trigger** | Any handler that accepts a numeric value from a DAP `Request` body |
| **Required adversarial test** | Send `frameId = u64::MAX`, `frameId = 0` (when no frames exist), and `variablesReference = 9999999` (no matching scope). Assert each returns `ErrorResponse` or equivalent empty result; assert none panic. |
| **Ref** | Class 2 in [docs/concepts/hazard-class-invariants.md](../concepts/hazard-class-invariants.md) |

### DAP-3: Protocol-safety

| Field | Value |
|---|---|
| **Invariant** | Every DAP request handler tolerates unknown command names, missing required fields, empty body, and session IDs that reference a terminated or non-existent session. Response is an honest `ErrorResponse` or empty result — never a crash, never fabricated data. |
| **Trigger** | Any new or modified `handle_*` function in the DAP dispatch layer |
| **Required adversarial test** | Send (a) an unknown command string, (b) a known command with a missing required field, (c) a request whose session ID references a session that was explicitly `.stop()`-ed. Assert each path returns the correct response kind without panicking. |
| **Ref** | Class 3 in [docs/concepts/hazard-class-invariants.md](../concepts/hazard-class-invariants.md) |

### DAP-4: Running-vs-stopped state

| Field | Value |
|---|---|
| **Invariant** | Requests that are only valid when the debuggee is stopped (`stackTrace`, `scopes`, `variables`, `evaluate` in non-repl context) return an honest error when called while the debuggee is running. The handler must check session state before touching frame or variable caches. |
| **Trigger** | Any change to frame/scope/variable retrieval code paths |
| **Required adversarial test** | Call `stackTrace` (or equivalent) on a session whose state is `Running`; assert the response is `ErrorResponse` (or the defined not-stopped error), never stale data from the previous stop. |

### DAP-5: Stale-after-resume (refs from a previous stop are rejected)

| Field | Value |
|---|---|
| **Invariant** | All variablesReferences, frameIds, and scope IDs are invalidated on every `continue` / `next` / `stepIn` / `stepOut` / `reverseContinue`. A client that sends a `variables` request with a ref from before the resume receives `ErrorResponse`, never data from the old stop. |
| **Trigger** | Any change to resume-event handling or session-state transitions |
| **Required adversarial test** | (1) Stop at a breakpoint; record a variablesReference. (2) Resume (`continue`). (3) Immediately send `variables` with the recorded ref. Assert `ErrorResponse`, not stale data. |

### DAP-6: No-active-session behavior

| Field | Value |
|---|---|
| **Invariant** | Any DAP request that arrives when no debug session is active returns an honest `ErrorResponse` with a descriptive message. The handler must not dereference a `None` session or an uninitialized cache. |
| **Trigger** | Any handler that accesses `self.session` or equivalent session state |
| **Required adversarial test** | Send a `stackTrace` request with no active session (before `launch`/`attach` or after `terminated`). Assert `ErrorResponse` is returned; assert no panic. |

### DAP-7: ripr-seam-anticipation

| Field | Value |
|---|---|
| **Invariant** | Inline `#[cfg(test)]` helper functions, predicate closures, or `Mutex`-guard test stubs placed inside production DAP source files (`crates/perl-dap/src/**`) will be flagged by ripr 0.9.x (ripr issues #1428 / #1429) as untested seams. The spec MUST pre-declare how this is handled: either (a) relocate the test-helper code to `crates/perl-dap/tests/` (preferred), or (b) add a pre-planned narrow `#[allow]` suppression citing ripr#1429 and the open xtask suppression-application gap #1346. |
| **Trigger** | Any change that adds `#[cfg(test)]` blocks inside `crates/perl-dap/src/**` files |
| **Required action** | Decision documented in `acceptance.md` before builder starts. CI receipt from `ripr+ New Gap Gate` is the verification artifact — not local ripr output (local may run a different ripr version; CI pins `RIPR_VERSION=0.5.0`). |
| **Ref** | [ripr#1428](https://github.com/nickel-lang/ripr/issues/1428), [ripr#1429](https://github.com/nickel-lang/ripr/issues/1429), xtask gap #1346; RIPR pin in `.github/workflows/ripr.yml` |

---

## Parser / scanner subsystem

Any change touching `crates/perl-parser/`, `crates/perl-lexer/`, `crates/perl-parser-core/`,
or any other crate whose primary job is tokenizing or parsing Perl source text.

### PARSER-1: Literal / comment / raw-string blindness

| Field | Value |
|---|---|
| **Invariant** | Every byte- or char-level scanner in the parser must skip characters inside string literals (`"..."`, `'...'`), heredoc bodies, comment regions (`#...`), and `q{}`/`qq{}`/`qw{}`/`qr{}` quote-like operators. A scanner that is correct on bare source is insufficient. |
| **Trigger** | Any new scanner that counts or matches delimiter characters, brace pairs, or structural tokens |
| **Required adversarial test** | Supply input where the target delimiter appears exclusively inside (a) a double-quoted string, (b) a single-quoted string, (c) a `#` comment, (d) a heredoc body, (e) a `q{}` quote-like. Assert the scanner treats each context as if the character were absent. Also supply input with the delimiter both inside a literal and outside — assert only the outside occurrence is counted. |
| **Motivating incident** | [docs/learnings/2026-06-coverage-gate-measurement.md](../learnings/2026-06-coverage-gate-measurement.md): #1327 LCOV range brace scanner stripped production lines inside string literals |
| **Ref** | Class 4 in [docs/concepts/hazard-class-invariants.md](../concepts/hazard-class-invariants.md) |

### PARSER-2: Delimiter pairing

| Field | Value |
|---|---|
| **Invariant** | Any code that matches opening delimiters (`(`, `[`, `{`, `<`) to closing delimiters must be tested for (a) unbalanced open, (b) unbalanced close, (c) nested same-kind delimiters, and (d) the delimiter appearing inside a string or comment. The parser must not panic on any of these inputs. |
| **Trigger** | Any new or modified delimiter-pairing or brace-counting logic |
| **Required adversarial test** | Feed (a) `({[`, (b) `}])`, (c) `({[{[]}]})`, (d) `"{"` — assert recovery produces a valid (possibly error-tagged) AST node in each case without panicking. |

### PARSER-3: Grammar-ambiguity positive + negative oracles

| Field | Value |
|---|---|
| **Invariant** | Every new grammar rule or disambiguation heuristic must be validated against the real Perl interpreter. For ambiguous constructs, both the positive (Perl accepts) and negative (Perl rejects) test inputs must be confirmed with `perl -MO=Terse` or `perl -cw`. Do not fixture expected output from a single run without a second confirmation. |
| **Trigger** | Any change to parse rules for ambiguous constructs (method calls, indirect object notation, regex vs division, print-with-or-without-parens, etc.) |
| **Required adversarial test** | At minimum one test input that Perl accepts and one that Perl rejects; assert the parser's AST and diagnostic output match the interpreter's verdict. |

### PARSER-4: Recovery honesty (no unreachable variant fixtures)

| Field | Value |
|---|---|
| **Invariant** | Error-recovery test cases must not snapshot AST variants that the parser's current recovery path cannot actually produce. Snapshotting an unreachable variant as "expected" encodes a latent lie — when the recovery code changes, the test silently becomes false. |
| **Trigger** | Any change to error-recovery logic, or any new snapshot test that includes `Error` / `Invalid` / `Malformed` AST nodes |
| **Required adversarial test** | Run the recovery test input through the current parser (not a cached snapshot) and confirm the variant is reachable before treating the snapshot as a positive assertion. Add a comment in the test naming the recovery path that produces each error node. |
| **Ref** | Class 5 (test-encodes-the-bug) in [docs/concepts/hazard-class-invariants.md](../concepts/hazard-class-invariants.md); incident: [docs/learnings/2026-06-test-encodes-the-bug.md](../learnings/2026-06-test-encodes-the-bug.md) |

---

## LSP subsystem

Any change touching `crates/perl-lsp/`, `crates/perl-lsp-rs/`, or `crates/perl-lsp-*/`.

### LSP-1: Request-shape validation (actionable INVALID_PARAMS)

| Field | Value |
|---|---|
| **Invariant** | Every LSP request handler validates required fields before processing. A missing or wrong-type field returns `ErrorCode::InvalidParams` with a message that names the missing field and its expected type. The handler never panics on malformed input. |
| **Trigger** | Any new or modified LSP request handler (`textDocument/*`, `workspace/*`, custom commands) |
| **Required adversarial test** | Send (a) a request with the required position field missing, (b) a request where `textDocument.uri` is null, (c) a request with a field that has an unexpected type. Assert `InvalidParams` is returned for each; assert no panic. |
| **Ref** | Class 3 in [docs/concepts/hazard-class-invariants.md](../concepts/hazard-class-invariants.md) |

### LSP-2: Document lifecycle (didOpen sequencing)

| Field | Value |
|---|---|
| **Invariant** | Any handler that accesses document state must tolerate being called before `textDocument/didOpen` completes (document not yet in the index), after `textDocument/didClose` (document removed), and on a URI that was never opened. The result must be an empty/null response — never stale data from a previously-open document with the same URI. |
| **Trigger** | Any change that reads from the document index (`DocumentStore`, `WorkspaceIndex`, or equivalent) |
| **Required adversarial test** | (a) Call `textDocument/hover` on a URI that has never been opened — assert null/empty response. (b) Open, close, then request — assert the closed document is not returned. (c) Rapid didOpen + didChange before the index settles — assert no panic. |

### LSP-3: URI normalization (cross-platform + UNC)

| Field | Value |
|---|---|
| **Invariant** | All URI handling round-trips correctly for (a) Unix absolute paths (`file:///home/user/foo.pl`), (b) Windows drive-letter paths (`file:///C:/Users/foo.pl`), (c) Windows paths with forward slashes (`file:///C:/Users/foo.pl` vs `file:///C:\Users\foo.pl`), and (d) UNC paths (`file://server/share/foo.pl`). The canonical form must be stable across round-trips. |
| **Trigger** | Any change to URI construction, parsing, or comparison (`uri.rs`, `WorkspaceUri`, `Url`, path-to-uri helpers) |
| **Required adversarial test** | Round-trip each of the four URI forms through the normalizer and assert equality. Also assert that two URIs pointing to the same file but differing only in slash direction compare equal. |
| **Ref** | ADR 0037 (guaranteed-valid-uri-fallbacks) |

### LSP-4: Actionable error guidance

| Field | Value |
|---|---|
| **Invariant** | `ErrorResponse` messages must name what went wrong and, where possible, what the client should do. "Internal error" is not actionable. The message format follows `{what failed}: {specific cause} (hint: {what client can do})` for diagnostic responses. |
| **Trigger** | Any new error path in LSP handlers; any change to error message text |
| **Required adversarial test** | Trigger the error path and assert the response message string contains at minimum the name of the missing/invalid item. |

---

## Coverage / CI subsystem

Any change touching `xtask/`, `.ci/`, `.github/workflows/`, or any coverage-related
transform (`coverage-filter`, `lcov.info` post-processors, ripr configuration, threshold files).

### COV-1: Measurement integrity (transforms never drop production lines)

| Field | Value |
|---|---|
| **Invariant** | Any tool that filters, strips, or annotates `lcov.info` or profdata must never drop lines that originate from production source (`crates/*/src/**`). The filter must be proved correct on a synthetic record containing exactly one production line and one test-only line. |
| **Trigger** | Any change to coverage filters, post-processors, or coverage-routing scripts |
| **Required adversarial test** | Feed a synthetic `lcov.info` containing one `SF: crates/foo/src/lib.rs` entry and one `SF: crates/foo/tests/foo_test.rs` entry through the transform. Assert the production entry survives; assert the test entry is absent; assert line counts match. |
| **Motivating incident** | [docs/learnings/2026-06-coverage-gate-measurement.md](../learnings/2026-06-coverage-gate-measurement.md): #1327 brace scanner in coverage transform stripped production LCOV lines (scanner-blindness + coverage-integrity) |
| **Ref** | Class 6 in [docs/concepts/hazard-class-invariants.md](../concepts/hazard-class-invariants.md) |

### COV-2: Test-only line filter (#1327 pattern)

| Field | Value |
|---|---|
| **Invariant** | Any heuristic that identifies "test-only" lines for exclusion must be scoped to known test patterns (`#[cfg(test)]`, `tests/` directory, `test_` function names). It must not match production lines that contain the substring `test` in a variable name, comment, or string literal. |
| **Trigger** | Any change to coverage exclusion heuristics or `LCOV_EXCL_*` annotation logic |
| **Required adversarial test** | A production source line such as `let test_result = compute();` must survive the filter unchanged. A test-module line `#[cfg(test)] fn test_foo()` must be excluded. Assert both. |

### COV-3: No threshold weakening without receipt

| Field | Value |
|---|---|
| **Invariant** | The `Codecov / Patch 95` threshold (95% patch coverage) must not be lowered. Any PR that reduces a coverage threshold must include a CI receipt showing the current coverage level and a comment from a maintainer approving the reduction. |
| **Trigger** | Any change to `.codecov.yml`, coverage gate config, or `patch_threshold` equivalent |
| **Required action** | If the PR reduces a threshold: attach a CI receipt (run URL) and a maintainer approval comment before the PR can merge. |

### COV-4: Inline-test vs ripr#1428 tension

| Field | Value |
|---|---|
| **Invariant** | Inline `#[cfg(test)]` blocks in production source files (`crates/*/src/**`) count toward `--lib` profdata coverage (boosting Codecov patch numbers) but simultaneously create ripr seams that ripr 0.9.x flags as `ripr#1428` gaps. The spec must declare which outcome is preferred: (a) move test helpers to `crates/*/tests/` to avoid the ripr flag, OR (b) keep inline and add a pre-planned narrow ripr suppression citing ripr#1429 and the open xtask gap #1346. This choice must appear in `acceptance.md` before the builder starts. |
| **Trigger** | Any change that adds `#[cfg(test)]` blocks to production source files in a crate whose ripr gate is currently green |
| **Required action** | Pre-declare the handling strategy in `acceptance.md`. CI receipt from `ripr+ New Gap Gate` (not local ripr output) is the verification artifact — local ripr installs may differ from the CI-pinned `RIPR_VERSION=0.5.0`. |
| **Ref** | [ripr#1428](https://github.com/nickel-lang/ripr/issues/1428), [ripr#1429](https://github.com/nickel-lang/ripr/issues/1429), xtask gap #1346; incident: [docs/learnings/2026-06-ripr-output-schema-break.md](../learnings/2026-06-ripr-output-schema-break.md) |

---

## Cross-subsystem rows (apply to any change)

These rows from SPEC_UPDATE_CHECKLIST.md §8 apply regardless of subsystem and are
reproduced here for convenience. They are NOT subsystem-specific defaults — consult
the generic taxonomy for the canonical definition.

| Class | Apply when |
|---|---|
| **Test-encodes-the-bug** | Any PR modifying an existing test assertion |
| **Scanner literal/comment blindness** | Any byte/char scanner (applies in Parser, Coverage/CI, and occasionally LSP) |

---

## Maintenance

When a new incident motivates a new default row, add it here AND add an entry to
`docs/learnings/` with the incident write-up. Cross-link bidirectionally.

The canonical trigger for updating this file is a deep-review finding that would have
been caught if the hazard row had been seeded in `acceptance.md`.

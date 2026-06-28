# Semantic Snapshot Architecture

**Umbrella contract for the Tranche 1 semantic-model substrate.**

This document defines the durable design invariants that the three implementing PRs must
satisfy:

- **#1600** — collision-proof semantic IDs (`EntityId`)
- **#1598** — complete per-file bundle (`FileSemanticBundle`)
- **#1601** — atomic snapshot publication (`SemanticSnapshot`)

It is a *specification*, not an implementation guide. Claims here describe invariants that
must hold across all present and future implementations. Where current code deviates from
a stated invariant, the implementing PR is responsible for correcting it.

**Related contract indexes**: [PARSER_CONTRACTS.md](PARSER_CONTRACTS.md) (parser
behavioral invariants) | [DAP_CONTRACTS.md](DAP_CONTRACTS.md) (DAP wire-protocol codec).

---

## 1. Ownership Boundary

### `perl-parser` owns (produces)

- Recovery AST: the concrete syntax tree produced by the v3 recursive-descent parser,
  including explicit recovery nodes for malformed input.
- HIR: high-level intermediate representation derived directly from the AST.
- Spans: byte-offset ranges anchored to a single source file.
- Lexical scopes: scope chains resolvable purely from a single file's text.
- Syntactic classification: symbol kind (`Sub`, `Variable`, `Package`, `Label`, …) derived
  from syntax alone.
- Explicit recovery nodes: `ErrorNode` variants that mark the extent of a parse failure
  without discarding surrounding structure.
- Dynamic-boundary nodes: AST nodes that mark a point where static analysis cannot
  determine the program's structure (e.g. runtime `push @ISA`, `eval STRING`, `require`
  with a variable path). These are *represented*, never silently dropped.
- Per-file semantic facts: `SymbolFact`, `ReferenceFact`, `ImportFact`, `ExportFact`,
  `InheritanceFact` records keyed to file-local `EntityId` values (see §2).
- Category hashes: a stable hash over the complete set of facts in each category,
  computed **after** all facts for the file (including generated/eval facts) have been
  appended. The category hash must cover the complete bundle; partial hashes are invalid.

### `perl-parser` must NOT do

- Scan the workspace or any directory outside the current file's path.
- Perform filesystem module resolution (locate `.pm` files, walk `@INC`).
- Read or depend on the ambient `@INC` value at analysis time.
- Spawn a Perl interpreter.
- Produce cross-file symbol rankings or relevance scores.
- Mint IDs that encode cross-file identity (IDs are file-local — see §2).

### `perl-workspace` owns (aggregates)

- Cross-file identity resolution: determining that `Foo::bar` in file A and `Foo::bar` in
  file B refer to the same logical entity.
- The versioned, immutable `SemanticSnapshot` (see §4).
- The `WorkspaceIndex` and all of its constituent indexes (`fact_shards`,
  `semantic_reference_index`, `semantic_import_export_index`, legacy maps).
- Atomic publication: replacing the active snapshot in a single pointer swap.
- Query execution: joining facts from multiple files to answer cross-file queries.
- Open-document overlay: merging in-progress edits on top of the last complete disk
  snapshot without mutating that snapshot.

### Invariant

The boundary is a strict layering: `perl-parser` is a pure function of a single file's
bytes. `perl-workspace` calls `perl-parser` and aggregates results. No fact produced by
`perl-parser` may depend on the workspace; no aggregation performed by `perl-workspace`
may be delegated back to `perl-parser`.

---

## 2. Semantic Identity

### IDs are file-scoped

An `EntityId` is a bare `u64` derived from a hash of `(namespace, name, span)` where the
span is anchored to a specific source file. Two files that each define `Foo::bar` at
different spans produce different `EntityId` values. Cross-file identity is the product of
*resolution*, performed by `perl-workspace`; it is never encoded in the ID itself.

### The collision invariant

**Same qualified name + same span in two different files MUST NOT mint the same `EntityId`.**

The hash function must incorporate a file-identity component (e.g. the canonical file path
or a stable file hash) so that structurally identical symbols in different files diverge.
Implemented by #1600 (merged in #1876, commit 801f507).

Previous state (pre-#1600): `EntityId` was hashed from `(namespace, name, span)` without a
file-identity component — see `perl-semantic-facts/src/lib.rs:22` and
`perl-symbol/src/surface/facts.rs:279-293`. This defect has been corrected by #1600.

### Resolution produces cross-file identity

When `perl-workspace` determines that entity A in file X and entity B in file Y denote the
same program entity (e.g. the same method), it records that association in the workspace
index. The `EntityId` values of A and B remain distinct; the workspace holds the mapping.
No global ID space is needed; cross-file identity is a relation, not a merger.

---

## 3. `FileSemanticBundle`

A `FileSemanticBundle` is the single, complete, self-contained producer result for one
source file at one version of its bytes. Implemented by #1598.

### Completeness invariant

A `FileSemanticBundle` must contain all semantic facts `perl-parser` can derive from that
file's bytes. There must be no post-bundle append path: once the bundle is constructed,
its fact sets are frozen.

### Category-hash ordering invariant

Category hashes are computed **after** all facts for the file — including facts derived
from generated code, `eval STRING` bodies, and heredoc expansions — have been appended to
their respective categories. Computing a category hash over a partial fact set is a
contract violation.

Current state (pre-#1598): category hashes are computed at `semantic/facts.rs:108-112`
and used at `workspace_index.rs:2540-2553` (noted in-source). Whether all facts are
present at hash time must be verified by #1598.

### Immutability

Once published to a `SemanticSnapshot`, a `FileSemanticBundle` is immutable. The workspace
may hold multiple bundles (one per open file version + one per completed disk index);
these are independent values, never mutated in place.

---

## 4. `SemanticSnapshot`

A `SemanticSnapshot` is the workspace-wide, immutable, generation-numbered view of all
semantic facts as of a specific moment. Implemented by #1601.

### Generation invariant

Every `SemanticSnapshot` carries a monotonically increasing `generation: u64`. Generations
are assigned at construction time. A snapshot's generation number never changes after
publication.

### Request-generation invariant

A single LSP request (hover, definition, completion, …) captures exactly one
`Arc<SemanticSnapshot>` at request start. All reads within that request use that snapshot.
A request MUST NOT read from two different generations. This eliminates the class of bugs
where a concurrent re-index changes a fact mid-request.

### Atomic publication invariant

The active snapshot is stored in an `Arc<SemanticSnapshot>` behind a single `ArcSwap` (or
equivalent atomic pointer). Publication is a single pointer swap; there is no multi-step
update sequence that a concurrent reader can observe in a partial state.

Current state (pre-#1601): publication is **not yet atomic**. `index_file` updates legacy maps
and fact shards in one lock block (`workspace_index.rs:1738-1769`) then updates
import/export in a **separate** lock block (`workspace_index.rs:1777-1782`). A concurrent
reader can observe a new shard with stale import visibility. This is the race that #1601
will correct when it ships.

### Off-thread construction

New snapshots are built off-thread (e.g. in a dedicated indexer task). The main LSP
dispatch thread is never blocked waiting for indexing to complete. The previous snapshot
remains active and queryable while a new one is being built.

### Open-document overlay

Open (unsaved) documents are represented as an overlay over the last complete disk
snapshot. The overlay does not mutate the snapshot; it is a separate data structure. When
a request queries a file that has an open-document overlay, the overlay takes precedence.
The underlying disk snapshot remains intact and is used for all other files.

### Query lock order

When a query must hold multiple read locks simultaneously, the lock acquisition order is
fixed: `shards` → `reference` → `import_export`. This order is the only safe order; any
deviation may deadlock. Current code acquires in this order at
`workspace_index.rs:2726-2731`; the `SemanticSnapshot` design eliminates the need for
multiple locks by making the snapshot itself immutable.

---

## 5. `SemanticResult<T>`

Every workspace query returns a `SemanticResult<T>` that bundles the answer with
provenance and quality metadata.

### Shape

```
SemanticResult<T> {
    value:       T,
    generation:  u64,            // snapshot generation this result was read from
    completeness: Completeness,  // how complete the underlying data is
    resolution:  Resolution,     // how the answer was derived
}
```

### `Completeness` variants

| Variant | Meaning |
|---------|---------|
| `Complete` | All relevant files were indexed; the result is authoritative. |
| `Partial` | Some files are pending indexing; the result may be incomplete. |
| `Degraded` | The workspace is in an error state; the result is best-effort only. |

### `Resolution` variants

| Variant | Meaning |
|---------|---------|
| `Exact` | Exactly one candidate matched; result is unambiguous. |
| `Candidates` | Multiple candidates matched; caller must disambiguate. |
| `Dynamic` | The resolution point involves dynamic Perl (see §6); static analysis gives uncertain candidates. |
| `Missing` | No candidate found. |

### Candidate provenance

When `resolution` is `Candidates` or `Dynamic`, each candidate carries:

- **provenance**: the source of the candidate (workspace index, open-document overlay, heuristic).
- **confidence**: a `[0.0, 1.0]` float; 1.0 = statically certain, lower = heuristic or degraded.
- **source anchor**: the `(file, span)` that produced this candidate.
- **workspace root**: the workspace root under which this candidate was found.
- **freshness**: the generation at which this candidate was last confirmed.
- **blockers**: reasons the result could not be promoted to `Exact` (e.g. "unresolved dynamic base class").

### Immutability contract

A `SemanticResult<T>` captures a snapshot of the answer at one generation. The caller may
hold the result indefinitely; it does not become stale (though it may become outdated when
the workspace advances to a new generation).

---

## 6. Dynamic Boundaries

Perl's runtime dynamism (runtime-computed inheritance, `eval STRING`, variable `require`,
`AUTOLOAD`, metaprogramming via `no strict 'refs'`, etc.) creates points where static
analysis cannot determine program structure with certainty. The contract for these points:

### Represent, never drop

Dynamic boundaries MUST be represented as explicit nodes in the AST and/or as explicit
`Dynamic` resolution markers in `SemanticResult`. They MUST NOT be silently dropped,
elided, or treated as `Missing`.

### Examples of dynamic-boundary sites

| Perl construct | Boundary type |
|----------------|---------------|
| `push @ISA, $base` | Dynamic inheritance — base class unknown at analysis time |
| `eval STRING` | Dynamic code — content unknown at analysis time |
| `require $module` | Dynamic module load — path unknown at analysis time |
| `AUTOLOAD` | Dynamic dispatch — method resolution deferred to runtime |
| `no strict 'refs'; $$name->method()` | Symbolic reference — target unknown at analysis time |
| `BEGIN { ... }` modifying `@ISA` | Compile-time dynamic inheritance |

### Uncertainty, not failure

A `SemanticResult` with `resolution: Dynamic` is a valid, informative result. It tells
the caller: "static analysis found candidates but cannot guarantee completeness or
uniqueness because at least one dynamic boundary is in scope." The caller may choose to
surface candidates with lower confidence, or to indicate to the user that results may be
incomplete.

### No silent widening

Dynamic Perl MUST NOT cause the analysis to silently widen its answer set (e.g. by
returning all methods in the workspace when inheritance is dynamic). Widening must be
explicit, bounded, and annotated in the candidate's provenance.

---

## Tranche 1 dependency order

These three implementing PRs must land in order because each builds on the previous:

1. **#1600** (collision-proof IDs) — Fixes `EntityId` to include file identity. All
   subsequent work assumes this invariant holds.
2. **#1598** (complete file bundle) — Introduces `FileSemanticBundle` with the completeness
   and hash-ordering invariants from §3. Depends on corrected IDs from #1600.
3. **#1601** (atomic snapshot) — Introduces `SemanticSnapshot` with generation counters and
   atomic publication, replacing the two-step lock sequence. Depends on complete bundles
   from #1598.

Landing these out of order will produce intermediate states where the contract is partially
satisfied. Do not merge #1598 before #1600, or #1601 before #1598.

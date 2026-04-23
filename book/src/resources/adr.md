# Architecture Decision Records (ADRs)

Architecture Decision Records document significant architectural decisions made in the project.

## Available ADRs

The project currently maintains ADRs in `docs/adr/` across several groups:

- **Legacy foundations**: ADR-0001 through ADR-0002.
- **Historical current-series ADRs**: ADR-001 through ADR-007, preserved for earlier documentation and workflow decisions.
- **Core architecture ADRs**: ADR-0008 through ADR-0030, covering parser architecture, indexing, DAP, UTF-16 position handling, error strategy, rope-backed documents, lifecycle/index routing, and delivery gates.
- **Current direction ADRs**: ADR-0031 through ADR-0037, covering concurrent dispatch, skill/hook enforcement, disposable worktrees, the custom LSP runtime, parent-map traversal, generated feature contracts, and synthetic URI fallbacks.

For the canonical index, including status and date metadata for every ADR, see [`docs/adr/README.md`](../../docs/adr/README.md).

## Suggested starting points

If you are new to the codebase, these ADRs provide a good architectural baseline:

- [ADR-0008: Microcrate Architecture](../../docs/adr/0008-microcrate-architecture.md)
- [ADR-0009: Dual Indexing Strategy](../../docs/adr/0009-dual-indexing-strategy.md)
- [ADR-0010: Incremental Parsing Architecture](../../docs/adr/0010-incremental-parsing-architecture.md)
- [ADR-0012: Error Handling Strategy](../../docs/adr/0012-error-handling-strategy.md)
- [ADR-0020: Rope Document Management](../../docs/adr/0020-rope-document-management.md)
- [ADR-0031: Async Runtime with Concurrent Dispatch](../../docs/adr/0031-async-runtime-concurrent-dispatch.md)
- [ADR-0034: Custom LSP Runtime over Framework Adoption](../../docs/adr/0034-custom-lsp-runtime.md)

## ADR Format

Each ADR generally follows this structure:

1. Context
2. Decision
3. Consequences
4. Status

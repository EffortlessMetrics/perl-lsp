# perl-workspace-index

Workspace-wide symbol indexing and cross-file navigation for Perl LSP tooling.

## When to use this crate

Use `perl-workspace-index` when you need cross-file symbol tracking for Perl:

- workspace-wide definition and reference lookup
- document storage and incremental reindexing
- rename and workspace-symbol operations
- cache-aware coordination for large workspaces

It is a core runtime crate in the
[`perl-lsp`](https://github.com/EffortlessMetrics/perl-lsp) workspace rather
than a standalone end-user package.

## Overview

`perl-workspace-index` provides the indexing engine behind go-to-definition,
find-references, workspace symbol search, and rename refactoring.

## Key Components

- **`WorkspaceIndex`** -- core symbol index with dual indexing (qualified and bare names) for O(1) lookups
- **`DocumentStore`** -- thread-safe in-memory document cache with version tracking
- **`IndexStateMachine`** -- lifecycle state machine (Idle, Initializing, Building, Ready, Degraded, Error)
- **`ProductionIndexCoordinator`** -- production coordinator integrating bounded LRU caches and SLO monitoring
- **`SloTracker`** -- service-level objective tracking with P50/P95/P99 latency percentiles
- **`BoundedLruCache`** -- generic bounded LRU cache with configurable size and TTL

## Public surface

Most consumers enter through the `workspace` module and the `WorkspaceIndex`,
`DocumentStore`, or `ProductionIndexCoordinator` types, depending on whether
they need a simple symbol index or a production-oriented coordinator with cache
and monitoring hooks.

## Features

| Feature | Purpose |
|---------|---------|
| `workspace` | Full workspace support (enables benchmarks) |
| `lsp-compat` | Adds `lsp-types` integration |

## License

Licensed under either of [MIT](https://opensource.org/licenses/MIT) or [Apache-2.0](https://www.apache.org/licenses/LICENSE-2.0) at your option.

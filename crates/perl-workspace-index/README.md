# perl-workspace-index

Workspace-wide symbol indexing and cross-file navigation for Perl LSP tooling.

## Overview

`perl-workspace-index` is a Tier 3 crate in the [tree-sitter-perl-rs](https://github.com/EffortlessMetrics/tree-sitter-perl-rs) workspace. It provides the central indexing engine that powers cross-file operations such as go-to-definition, find-references, workspace symbol search, and rename refactoring.

## Key Components

- **`WorkspaceIndex`** -- core symbol index with dual indexing (qualified and bare names) for O(1) lookups
- **`DocumentStore`** -- thread-safe in-memory document cache with version tracking
- **`IndexStateMachine`** -- lifecycle state machine (Idle, Initializing, Building, Ready, Degraded, Error)
- **`ProductionIndexCoordinator`** -- production coordinator integrating bounded LRU caches and SLO monitoring
- **`SloTracker`** -- service-level objective tracking with P50/P95/P99 latency percentiles
- **`BoundedLruCache`** -- generic bounded LRU cache with configurable size and TTL

## Features

| Feature | Purpose |
|---------|---------|
| `workspace` | Full workspace support (enables benchmarks) |
| `lsp-compat` | Adds `lsp-types` integration |

## License

Licensed under either of [MIT](https://opensource.org/licenses/MIT) or [Apache-2.0](https://www.apache.org/licenses/LICENSE-2.0) at your option.

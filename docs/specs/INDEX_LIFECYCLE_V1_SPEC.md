# Index Lifecycle v1 Specification

**Status**: Draft
**PR Target**: Index Lifecycle v1
**Baseline**: Post-PR #245 (merged 2025-12-29)

---

## Goal

Cross-file features stay correct while files churn, and never block.

---

## IndexState: The Core Contract

```rust
/// Index readiness state - explicit lifecycle management
#[derive(Clone, Debug, PartialEq)]
pub enum IndexState {
    /// Index is being constructed (workspace scan in progress)
    Building {
        /// Current build phase (Idle → Scanning → Indexing)
        phase: IndexPhase,
        /// Files indexed so far
        indexed_count: usize,
        /// Total files discovered
        total_count: usize,
        /// Started at
        started_at: Instant,
    },

    /// Index is consistent and ready for queries
    Ready {
        /// Total symbols indexed
        symbol_count: usize,
        /// Total files indexed
        file_count: usize,
        /// Timestamp of last successful index
        completed_at: Instant,
    },

    /// Index is serving but degraded
    Degraded {
        /// Why we degraded
        reason: DegradationReason,
        /// What's still available
        available_symbols: usize,
        /// When degradation occurred
        since: Instant,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IndexPhase {
    /// No scan has started yet
    Idle,
    /// Workspace file discovery is in progress
    Scanning,
    /// Symbol indexing is in progress
    Indexing,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DegradationReason {
    /// Parse storm (too many simultaneous changes)
    ParseStorm { pending_parses: usize },
    /// IO error during indexing
    IoError { message: String },
    /// Timeout during workspace scan
    ScanTimeout { elapsed_ms: u64 },
    /// Resource limits exceeded
    ResourceLimit { kind: ResourceKind },
}

#[derive(Clone, Debug, PartialEq)]
pub enum ResourceKind {
    MaxFiles,
    MaxSymbols,
    MaxCacheBytes,
}
```

---

## State Transitions

```
          ┌─────────────────────────────────────┐
          │                                     │
          ▼                                     │
    ┌──────────┐   scan complete         ┌─────┴────┐
    │ Building │ ──────────────────────▶ │  Ready   │
    └──────────┘                         └──────────┘
          │                                     │
          │ timeout/error/limits                │ parse storm/IO error
          ▼                                     ▼
    ┌──────────┐                         ┌──────────┐
    │ Degraded │ ◀───────────────────────│ Degraded │
    └──────────┘                         └──────────┘
          │                                     │
          │ recover (re-scan)                   │ recover
          └─────────────────────────────────────┘
```

### Build Phases (Building State)

When the index is in `Building`, it progresses through explicit phases:

```
Idle → Scanning → Indexing → Ready
```

`Degraded` is the error state for early exits, timeouts, IO failures, and resource limits.

### Transition Triggers

| From | To | Trigger |
|------|-----|---------|
| `Building` | `Ready` | Workspace scan completes successfully |
| `Building` | `Degraded` | Scan timeout (>30s), IO error, or resource limit |
| `Ready` | `Building` | Workspace folder change, `didChangeWatchedFiles` |
| `Ready` | `Degraded` | Parse storm (>10 pending), IO error |
| `Degraded` | `Building` | Recovery attempt (manual or after cooldown) |
| `Degraded` | `Ready` | Successful re-scan after recovery |

---

## Invariants

- A single build attempt advances phases monotonically (`Idle` → `Scanning` → `Indexing`).
- `indexed_count` must never exceed `total_count`; callers are responsible for keeping totals current.
- `Ready` and `Degraded` counts are snapshots captured at transition time and should not be mutated by read-only queries.
- State transitions are serialized by the coordinator lock; treat state reads as point-in-time snapshots.

## Failure Modes and Recovery

- **Parse storm**: Enter `Degraded(ParseStorm)` while updates churn; recover once pending parses drain and a re-scan succeeds.
- **IO error**: Enter `Degraded(IoError)`; serve cached results and recover on the next successful scan.
- **Scan timeout / budget exhaustion**: Enter `Degraded(ScanTimeout)`; partial results may be served with a warning.
- **Resource limits exceeded**: Enter `Degraded(ResourceLimit)`; eviction can reduce pressure and allow recovery.

Early-exit events should always be recorded in instrumentation with the reason and elapsed time.

---

## Bounded Resources

### Hard Caps (Configurable)

```rust
pub struct IndexResourceLimits {
    /// Maximum files to index (default: 10,000)
    pub max_files: usize,

    /// Maximum symbols per file (default: 5,000)
    pub max_symbols_per_file: usize,

    /// Maximum total symbols (default: 500,000)
    pub max_total_symbols: usize,

    /// Maximum AST cache size in bytes (default: 256MB)
    pub max_ast_cache_bytes: usize,

    /// Maximum AST cache items (default: 100)
    pub max_ast_cache_items: usize,
}

impl Default for IndexResourceLimits {
    fn default() -> Self {
        Self {
            max_files: 10_000,
            max_symbols_per_file: 5_000,
            max_total_symbols: 500_000,
            max_ast_cache_bytes: 256 * 1024 * 1024,
            max_ast_cache_items: 100,
        }
    }
}
```

### Performance Caps (Soft Budgets)

```rust
pub struct IndexPerformanceCaps {
    /// Initial workspace scan budget in milliseconds (default: 100ms)
    pub initial_scan_budget_ms: u64,
    /// Incremental update budget in milliseconds (default: 10ms)
    pub incremental_budget_ms: u64,
}
```

Early-exit heuristics use these budgets to stop long scans and transition to
`Degraded` with a `ScanTimeout` reason.

---

## Instrumentation

The coordinator tracks:

- **State durations** (Building/Ready/Degraded)
- **Phase durations** (Idle/Scanning/Indexing)
- **Transition counts** (state and phase)
- **Early-exit reasons** (time budget, file limit)

These metrics are exposed via an instrumentation snapshot for tests and logging.

### Eviction Strategy

- **AST Cache**: LRU with content-hash validation, 5-min TTL
- **Symbol Index**: LRU by file access time
- **Eviction is deterministic**: Given same sequence, same items evicted
- **Eviction is tested**: State machine tests verify eviction behavior

---

## Handler Behavior

### Standard Pattern

```rust
impl LspServer {
    fn handle_request(&self, ...) -> Result<Response, Error> {
        // 1. Check index state
        let state = self.index_state();

        // 2. Fast path: Ready state
        if matches!(state, IndexState::Ready { .. }) {
            return self.full_index_query(...);
        }

        // 3. Degraded path: Same-file + open docs only
        match state {
            IndexState::Building { .. } | IndexState::Degraded { .. } => {
                // Log degradation for telemetry
                log::debug!("index_degraded: {:?}", state);

                // Return partial results (same-file, open documents)
                return self.partial_query_open_docs(...);
            }
            _ => {}
        }

        // 4. Never block waiting for index
        Ok(partial_results)
    }
}
```

### Per-Handler Behavior

| Handler | Ready | Building/Degraded |
|---------|-------|-------------------|
| `textDocument/definition` | Full workspace search | Same-file + open docs |
| `textDocument/references` | All workspace refs | Same-file + open docs |
| `workspace/symbol` | Full symbol search | Open doc symbols only |
| `textDocument/rename` | Workspace-wide edits | Reject with message |
| `textDocument/completion` | Full context | Local scope only |
| `textDocument/hover` | Package docs | Same-file symbols |

---

## IndexCoordinator: The Orchestrator

```rust
/// Coordinates index lifecycle, state transitions, and handler queries
pub struct IndexCoordinator {
    /// Current index state (atomic for lock-free reads)
    state: Arc<RwLock<IndexState>>,

    /// The actual index
    index: Arc<WorkspaceIndex>,

    /// Resource limits
    limits: IndexResourceLimits,

    /// Metrics for degradation detection
    metrics: IndexMetrics,
}

pub struct IndexMetrics {
    /// Pending parse operations
    pending_parses: AtomicUsize,

    /// Parse storm threshold
    parse_storm_threshold: usize,

    /// Last successful index time
    last_indexed: AtomicU64,
}

impl IndexCoordinator {
    /// Check current state (lock-free for hot path)
    pub fn state(&self) -> IndexState {
        self.state.read().unwrap().clone()
    }

    /// Query with automatic degradation handling
    pub fn query<T, F>(&self, full_query: F, partial_query: F) -> T
    where
        F: FnOnce(&WorkspaceIndex) -> T,
    {
        match self.state() {
            IndexState::Ready { .. } => full_query(&self.index),
            _ => partial_query(&self.index),
        }
    }

    /// Notify of file change (may trigger state transition)
    pub fn notify_change(&self, uri: &str) {
        self.metrics.pending_parses.fetch_add(1, Ordering::SeqCst);

        // Check for parse storm
        let pending = self.metrics.pending_parses.load(Ordering::SeqCst);
        if pending > self.metrics.parse_storm_threshold {
            self.transition_to_degraded(DegradationReason::ParseStorm {
                pending_parses: pending
            });
        }
    }

    /// Notify parse complete
    pub fn notify_parse_complete(&self, uri: &str) {
        self.metrics.pending_parses.fetch_sub(1, Ordering::SeqCst);

        // Check for recovery from parse storm
        let pending = self.metrics.pending_parses.load(Ordering::SeqCst);
        if pending == 0 {
            if let IndexState::Degraded { reason: DegradationReason::ParseStorm { .. }, .. }
                = self.state()
            {
                self.attempt_recovery();
            }
        }
    }
}
```

---

## Integration Points

### Where Checks Live

```
LspServer
  └── IndexCoordinator (new)
        ├── state: Arc<RwLock<IndexState>>
        ├── index: Arc<WorkspaceIndex>
        └── limits: IndexResourceLimits

Handler Flow:
  1. handle_definition()
  2.   → coordinator.query(full, partial)
  3.     → coordinator.state()  // lock-free read
  4.     → dispatch to full or partial query
```

### Existing Code Changes

| File | Change |
|------|--------|
| `workspace_index.rs` | Add `IndexCoordinator` wrapper |
| `server_impl/mod.rs` | Replace `workspace_index: Option<Arc<WorkspaceIndex>>` with `coordinator: Arc<IndexCoordinator>` |
| `text_sync.rs` | Call `coordinator.notify_change()` / `notify_parse_complete()` |
| `language/navigation.rs` | Use `coordinator.query()` pattern |
| `language/references.rs` | Use `coordinator.query()` pattern |
| `workspace.rs` | Use `coordinator.query()` pattern |

---

## Tests Required

### State Machine Tests

```rust
#[test]
fn test_building_to_ready_transition() {
    let coord = IndexCoordinator::new();
    assert!(matches!(coord.state(), IndexState::Building { .. }));

    coord.complete_initial_scan(100, 5000);
    assert!(matches!(coord.state(), IndexState::Ready { file_count: 100, .. }));
}

#[test]
fn test_ready_to_degraded_on_parse_storm() {
    let coord = IndexCoordinator::new_ready(100, 5000);

    // Trigger parse storm
    for _ in 0..15 {
        coord.notify_change("file.pm");
    }

    assert!(matches!(
        coord.state(),
        IndexState::Degraded { reason: DegradationReason::ParseStorm { .. }, .. }
    ));
}

#[test]
fn test_degraded_recovery() {
    let coord = IndexCoordinator::new_degraded(DegradationReason::ParseStorm { pending_parses: 10 });

    // Clear pending parses
    for _ in 0..10 {
        coord.notify_parse_complete("file.pm");
    }

    // Should attempt recovery
    assert!(matches!(coord.state(), IndexState::Building { .. } | IndexState::Ready { .. }));
}
```

### Degraded Mode Handler Tests

```rust
#[test]
fn test_definition_returns_partial_when_building() {
    let server = test_server_in_state(IndexState::Building { .. });

    let result = server.handle_definition(open_doc_symbol_position());

    // Should return same-file result, not error
    assert!(result.is_ok());
    assert!(result.unwrap().len() == 1); // Only same-file
}

#[test]
fn test_references_returns_partial_when_degraded() {
    let server = test_server_in_state(IndexState::Degraded { .. });

    let result = server.handle_references(open_doc_symbol_position());

    // Should return open-doc refs only
    assert!(result.is_ok());
}

#[test]
fn test_rename_rejects_when_degraded() {
    let server = test_server_in_state(IndexState::Degraded { .. });

    let result = server.handle_rename(rename_request());

    // Should reject with informative message
    assert!(result.is_err());
    assert!(result.unwrap_err().message.contains("index not ready"));
}
```

### Resource Limit Tests

```rust
#[test]
fn test_max_files_triggers_degradation() {
    let limits = IndexResourceLimits { max_files: 10, ..Default::default() };
    let coord = IndexCoordinator::with_limits(limits);

    for i in 0..15 {
        coord.index_file(&format!("file{}.pm", i), "");
    }

    assert!(matches!(
        coord.state(),
        IndexState::Degraded { reason: DegradationReason::ResourceLimit { kind: ResourceKind::MaxFiles }, .. }
    ));
}
```

---

## Rollout Plan

### Phase 1: IndexState + Coordinator (Week 1)
- Add `IndexState` enum and `IndexCoordinator` struct
- Wire into `LspServer` replacing raw `WorkspaceIndex`
- Add state machine tests

### Phase 2: Handler Integration (Week 2)
- Update all handlers to use `coordinator.query()` pattern
- Add degraded-mode fallback logic per handler
- Add handler behavior tests

### Phase 3: Resource Limits (Week 3)
- Add `IndexResourceLimits` configuration
- Implement eviction logic
- Add resource limit tests

### Phase 4: Telemetry + Docs (Week 4)
- Add metrics emission for state transitions
- Document in `docs/INDEX_LIFECYCLE_GUIDE.md`
- Update CLAUDE.md with new patterns

---

## Success Criteria

- [ ] All state transitions are explicit and tested
- [ ] Handlers never block waiting for index
- [ ] Resource limits prevent unbounded growth
- [ ] Degraded mode returns partial results, not errors
- [ ] Recovery from degradation is automatic where possible
- [ ] State is observable (logs, future: metrics endpoint)

---

## Non-Goals (v1)

- Background indexing (synchronous is fine for v1)
- Progress reporting to client (nice-to-have, not blocking)
- Incremental within-file indexing (file-level granularity is fine)
- Distributed/remote indexing

# Latency Caps + SLO Specification

**Status**: Draft
**PR Target**: Latency Caps + SLO
**Baseline**: Post-PR #245

---

## Goal

Define and enforce response time contracts so users never experience blocking operations.

---

## 1. SLO Targets

### Response Time SLOs

| Operation | P95 Target | P99 Target | Hard Cap |
|-----------|------------|------------|----------|
| `textDocument/hover` | 20ms | 50ms | 100ms |
| `textDocument/definition` | 30ms | 75ms | 150ms |
| `textDocument/completion` | 50ms | 100ms | 200ms |
| `textDocument/references` | 100ms | 250ms | 500ms |
| `workspace/symbol` | 50ms | 150ms | 300ms |
| `textDocument/documentSymbol` | 25ms | 50ms | 100ms |
| `textDocument/semanticTokens` | 50ms | 100ms | 200ms |
| `textDocument/formatting` | 500ms | 1000ms | 2000ms |
| `textDocument/rename` | 250ms | 500ms | 1000ms |

### Availability SLOs

| Metric | Target |
|--------|--------|
| Request success rate | ≥99.5% (excluding cancellations) |
| Graceful degradation | 100% (never crash on bad input) |
| Recovery from degradation | <5 seconds |

---

## 2. Result Caps

### Per-Operation Limits

```rust
pub struct ResultCaps {
    /// Maximum completion items
    pub max_completion_items: usize,         // 100

    /// Maximum workspace symbols
    pub max_workspace_symbols: usize,        // 500

    /// Maximum references
    pub max_references: usize,               // 1000

    /// Maximum diagnostics per file
    pub max_diagnostics_per_file: usize,     // 200

    /// Maximum total diagnostics
    pub max_total_diagnostics: usize,        // 1000

    /// Maximum folding ranges
    pub max_folding_ranges: usize,           // 5000

    /// Maximum document symbols
    pub max_document_symbols: usize,         // 1000
}
```

### Pagination Strategy

For operations that may exceed caps:

```rust
impl WorkspaceSymbolHandler {
    fn handle(&self, query: &str) -> Vec<SymbolInformation> {
        let all_matches = self.search(query);

        // Apply cap
        if all_matches.len() > self.caps.max_workspace_symbols {
            // Return top-ranked results
            all_matches
                .into_iter()
                .take(self.caps.max_workspace_symbols)
                .collect()
        } else {
            all_matches
        }
    }
}
```

---

## 3. Time Caps

### Request Deadline Pattern

```rust
pub struct RequestDeadline {
    started: Instant,
    hard_cap: Duration,
}

impl RequestDeadline {
    pub fn new(hard_cap: Duration) -> Self {
        Self {
            started: Instant::now(),
            hard_cap,
        }
    }

    pub fn remaining(&self) -> Duration {
        self.hard_cap.saturating_sub(self.started.elapsed())
    }

    pub fn is_expired(&self) -> bool {
        self.started.elapsed() >= self.hard_cap
    }

    pub fn check(&self) -> Result<(), DeadlineExceeded> {
        if self.is_expired() {
            Err(DeadlineExceeded { elapsed: self.started.elapsed() })
        } else {
            Ok(())
        }
    }
}
```

### Integration with Handlers

```rust
impl LspServer {
    fn handle_references(&self, params: ReferenceParams) -> Result<Vec<Location>> {
        let deadline = RequestDeadline::new(Duration::from_millis(500));

        // Phase 1: Same-file references (always fast)
        let mut results = self.same_file_refs(&params)?;

        // Phase 2: Workspace references (may timeout)
        if let Some(workspace_refs) = self.workspace_refs_with_deadline(&params, &deadline)? {
            results.extend(workspace_refs);
        }

        // Phase 3: Text search fallback (skip if near deadline)
        if deadline.remaining() > Duration::from_millis(100) {
            if let Some(text_refs) = self.text_search_refs(&params, &deadline)? {
                results.extend(text_refs);
            }
        }

        Ok(results)
    }
}
```

---

## 4. Handler Modifications

### Changes Required

| Handler | Change |
|---------|--------|
| `handle_completion` | Cap at 100 items, sort by relevance |
| `handle_workspace_symbol` | Cap at 500 items, sort by match quality |
| `handle_references` | Cap at 1000 refs, deadline-aware scanning |
| `handle_document_symbol` | Cap at 1000 symbols |
| `handle_semantic_tokens` | Deadline-aware, partial on timeout |
| `handle_folding_range` | Cap at 5000 ranges |
| `handle_diagnostics` | Cap at 200 per file, 1000 total |

### Partial Result Pattern

```rust
/// Indicates results may be incomplete
pub struct PartialResults<T> {
    pub results: Vec<T>,
    pub is_complete: bool,
    pub reason: Option<PartialReason>,
}

pub enum PartialReason {
    ResultCapReached,
    DeadlineApproaching,
    IndexDegraded,
}
```

---

## 5. Monitoring & Validation

### Metrics to Track

```rust
pub struct OperationMetrics {
    pub operation: &'static str,
    pub duration_ms: u64,
    pub result_count: usize,
    pub was_capped: bool,
    pub was_partial: bool,
}
```

### Test Assertions

```rust
#[test]
fn test_completion_respects_cap() {
    let server = test_server_with_large_workspace();
    let result = server.handle_completion(params);

    assert!(result.items.len() <= 100);
}

#[test]
fn test_references_respects_deadline() {
    let server = test_server_with_huge_workspace();
    let start = Instant::now();

    let result = server.handle_references(params);

    // Hard cap: 500ms
    assert!(start.elapsed() < Duration::from_millis(600));
}

#[test]
fn test_hover_p95_target() {
    let mut latencies = Vec::new();

    for _ in 0..100 {
        let start = Instant::now();
        server.handle_hover(params.clone());
        latencies.push(start.elapsed());
    }

    latencies.sort();
    let p95 = latencies[94];
    assert!(p95 < Duration::from_millis(20));
}
```

---

## 6. Documentation Deliverable

Create `docs/PERFORMANCE_SLO.md`:

```markdown
# Performance SLO Reference

## Response Time Targets

| Operation | Target | Hard Limit |
|-----------|--------|------------|
| Hover | <20ms | 100ms |
| Definition | <30ms | 150ms |
| Completion | <50ms | 200ms |
| ... | ... | ... |

## Result Limits

| Operation | Limit | Behavior When Exceeded |
|-----------|-------|------------------------|
| Completion | 100 items | Return top-ranked |
| Workspace Symbol | 500 items | Return best matches |
| References | 1000 refs | Return same-file + recent |
| ... | ... | ... |

## Degraded Mode Behavior

When the index is building or degraded, operations return
partial results from open documents only. This ensures
responsiveness even during workspace scanning.

## Configuration

```json
{
  "perl-lsp.performance.maxCompletionItems": 100,
  "perl-lsp.performance.maxReferences": 1000,
  "perl-lsp.performance.requestTimeoutMs": 500
}
```
```

---

## 7. Implementation Phases

### Phase 1: Result Caps (Week 1)
- Add `ResultCaps` struct with defaults
- Apply caps to completion, workspace symbol, references
- Add cap-related tests

### Phase 2: Time Caps (Week 2)
- Add `RequestDeadline` infrastructure
- Integrate into expensive handlers
- Add deadline tests

### Phase 3: Monitoring (Week 3)
- Add operation metrics collection
- Add P95 validation tests
- Create `PERFORMANCE_SLO.md`

---

## Success Criteria

- [ ] No operation exceeds hard cap under any input
- [ ] P95 targets met in benchmark tests
- [ ] Caps are configurable via server settings
- [ ] Partial results clearly indicated (not silent truncation)
- [ ] Documentation describes all limits and behavior

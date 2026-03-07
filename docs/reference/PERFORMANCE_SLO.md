# Performance SLO Reference

This document defines the Service Level Objectives (SLOs) for the Perl LSP server, including response time targets, result limits, and graceful degradation behavior.

## Table of Contents

- [Response Time Targets](#response-time-targets)
- [Result Limits](#result-limits)
- [Deadline Behavior](#deadline-behavior)
- [Degradation Modes](#degradation-modes)
- [Configuration](#configuration)

---

## Response Time Targets

### LSP Operations

| Operation | P95 Target | P99 Target | Hard Limit | Notes |
|-----------|------------|------------|------------|-------|
| `textDocument/hover` | 20ms | 50ms | 100ms | Single symbol lookup |
| `textDocument/definition` | 30ms | 75ms | 150ms | Index + fallback search |
| `textDocument/completion` | 50ms | 100ms | 200ms | Capped at 100 items |
| `textDocument/references` | 100ms | 250ms | 500ms | Multi-phase search |
| `textDocument/documentSymbol` | 25ms | 50ms | 100ms | Single file parsing |
| `textDocument/semanticTokens` | 50ms | 100ms | 200ms | Full semantic analysis |
| `textDocument/formatting` | 500ms | 1000ms | 2000ms | External perltidy |
| `textDocument/rename` | 250ms | 500ms | 1000ms | Cross-file workspace |
| `textDocument/codeAction` | 50ms | 100ms | 200ms | Code action generation |
| `textDocument/codeLens` | 50ms | 100ms | 200ms | Reference counting |
| `textDocument/signatureHelp` | 20ms | 50ms | 100ms | Parameter info |
| `textDocument/inlayHint` | 50ms | 100ms | 200ms | Inline hints |
| `workspace/symbol` | 50ms | 150ms | 300ms | Capped at 200 items |
| `workspace/executeCommand` | 500ms | 1000ms | 2000ms | External tool calls |

### Incremental Parsing

| Operation | Target | Hard Limit | Notes |
|-----------|--------|------------|-------|
| Parse update | 1ms | 5ms | Character-level changes |
| Syntax tree rebuild | 10ms | 50ms | Structural changes |
| Index update | 5ms | 20ms | Symbol extraction |

### Availability Targets

| Metric | Target | Notes |
|--------|--------|-------|
| Request success rate | 99.5% | Excluding client cancellations |
| Graceful degradation | 100% | Never crash on bad input |
| Recovery from degradation | <5s | Index rebuilds |

---

## Result Limits

### Per-Operation Caps

| Operation | Default Cap | Behavior When Exceeded |
|-----------|-------------|------------------------|
| `textDocument/completion` | 100 items | Return top-ranked by relevance |
| `textDocument/references` | 500 refs | Return same-file + recent + indexed |
| `textDocument/documentSymbol` | 500 symbols | Truncate tree depth |
| `textDocument/codeLens` | 100 items | Priority by location |
| `textDocument/inlayHint` | 500 hints | Visible range prioritized |
| `textDocument/diagnostics` | 200 per file | Severity-based priority |
| `workspace/symbol` | 200 items | Best match quality first |

### Index Limits

| Resource | Default Limit | Behavior When Exceeded |
|----------|---------------|------------------------|
| Indexed files | 10,000 | Skip older/less-used files |
| Symbols per file | 5,000 | Truncate deep nesting |
| Total symbols | 500,000 | LRU eviction |
| AST cache entries | 100 | LRU eviction with TTL |
| AST cache TTL | 300s | Automatic expiry |

---

## Deadline Behavior

### Scan Deadlines

| Operation | Default Deadline | Graceful Behavior |
|-----------|------------------|-------------------|
| Workspace folder scan | 30s | Return partial index |
| Single file indexing | 5s | Skip file, log warning |
| Reference search | 2s | Return partial results |
| Regex scan | 1s | Abort scan, use index only |
| Filesystem operation | 500ms | Skip path, continue |

### Timeout Response Pattern

When a deadline is approaching, operations follow a phased approach:

```
Phase 1 (0-50% of deadline): Full operation
Phase 2 (50-80% of deadline): Skip expensive fallbacks
Phase 3 (80-100% of deadline): Return partial results
Phase 4 (>100% of deadline): Immediate return with available results
```

### Partial Result Indication

When results are incomplete due to deadline or cap:

1. **Completion**: `isIncomplete: true` in response
2. **References**: Sorted by confidence, truncated with priority
3. **Workspace symbols**: Best matches returned first
4. **Diagnostics**: Higher severity items prioritized

---

## Degradation Modes

### Index Degradation

Triggered when:
- Parse storm threshold exceeded (>10 pending parses)
- Memory pressure detected
- Workspace scan timeout

Behavior:
- `return_partial_on_timeout: true` (default)
- `include_open_docs_when_degraded: true` (default)
- Reduced result caps automatically applied
- Background index rebuild queued

### Recovery Behavior

| Condition | Recovery Time | Strategy |
|-----------|---------------|----------|
| Parse storm | <1s | Queue draining |
| Index timeout | <5s | Incremental rebuild |
| Memory pressure | <10s | Cache eviction + GC |
| Full rebuild | <30s | Progressive indexing |

### Fallback Chain

For definition resolution:

```
1. Workspace index (fast, may be stale)
   |
   v
2. Open document search (fast, current)
   |
   v
3. Text search fallback (slow, comprehensive)
```

For references:

```
1. Same-file references (always fast)
   |
   v
2. Workspace index search (deadline-aware)
   |
   v
3. Regex fallback search (skip if near deadline)
```

---

## Configuration

### Tuning for Performance

```json
{
  "perl": {
    "limits": {
      "workspaceSymbolCap": 200,
      "referencesCap": 500,
      "completionCap": 100,
      "referenceSearchDeadlineMs": 2000,
      "workspaceScanDeadlineMs": 30000
    }
  }
}
```

### Large Workspace Tuning

For projects with 10K+ files:

```json
{
  "perl": {
    "limits": {
      "maxIndexedFiles": 50000,
      "maxTotalSymbols": 2000000,
      "workspaceScanDeadlineMs": 120000,
      "workspaceSymbolCap": 300,
      "referencesCap": 1000
    }
  }
}
```

### Resource-Constrained Environment

For limited memory/CPU environments:

```json
{
  "perl": {
    "limits": {
      "astCacheMaxEntries": 50,
      "maxIndexedFiles": 5000,
      "maxTotalSymbols": 100000,
      "workspaceScanDeadlineMs": 15000,
      "referenceSearchDeadlineMs": 1000,
      "workspaceSymbolCap": 100,
      "referencesCap": 200
    }
  }
}
```

---

## Monitoring

### Key Metrics

Monitor these for SLO compliance:

| Metric | Target | Alert Threshold |
|--------|--------|-----------------|
| P95 hover latency | <20ms | >50ms |
| P95 completion latency | <50ms | >100ms |
| P95 references latency | <100ms | >250ms |
| Parse update time | <1ms | >5ms |
| Index rebuild time | <30s | >60s |
| Memory usage | <500MB | >1GB |

### Debug Logging

Enable performance logging:

```bash
RUST_LOG=perl_parser::lsp=debug perl-lsp --stdio
```

Latency information appears in logs as:

```
[DEBUG] handle_hover: 15ms (symbol: MyModule::function)
[DEBUG] handle_references: 87ms (refs: 42, phases: 3)
[WARN]  handle_references: deadline approaching, skipping text search
```

---

## Design Philosophy

### Bounded Operations

Every operation has:
1. **Result cap**: Maximum items returned
2. **Time cap**: Maximum execution time
3. **Graceful fallback**: Partial results instead of failure

### No Blocking

The server never blocks indefinitely:
- All I/O has timeouts
- All loops have iteration limits
- All caches have size limits

### Progressive Enhancement

Features degrade gracefully:
1. Full index available: Complete results
2. Partial index: Results from indexed files + open docs
3. No index: Results from open documents only
4. Parse failure: Cached results if available

---

## See Also

- [CONFIG.md](CONFIG.md) - Full configuration reference
- [EDITOR_SETUP.md](EDITOR_SETUP.md) - Editor setup guides
- [THREADING_CONFIGURATION_GUIDE.md](THREADING_CONFIGURATION_GUIDE.md) - Threading options
- [specs/LATENCY_CAPS_SLO_SPEC.md](specs/LATENCY_CAPS_SLO_SPEC.md) - Implementation specification

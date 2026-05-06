# Memory Leak Investigation Findings

**Date**: 2026-05-06  
**Branch**: `claude/investigate-memory-leak-3E8YL`  
**Status**: In Progress

## Summary

Investigation into suspected memory leaks in perl-lsp has confirmed **monotonic memory growth** during document lifecycle churn (open → change → close cycles), even when all documents are properly removed from the index.

- **Test 1 (50 files, 5 changes)**: 12.2 MB → 14.8 MB (+2.6 MB)
- **Test 2 (500 files, 10 changes)**: 12.5 MB → 49.9 MB (+37.4 MB, ~75 KB/file)
- **Test 3 (300 files with workspace/symbol queries)**: 12.7 MB → 39.3 MB (+26.6 MB, ~89 KB/file)

All tests show **linear growth correlated to document count**, even after `didClose` removes documents from indices. This suggests retained Arc references or undiscovered secondary caches.

## Investigation Methodology

Created reproduction harnesses:
- `scripts/repro_lsp_storm.py` — Drives perllsp via stdio with configurable document churn
- Captures RSS before/after each document lifecycle phase
- Calculates slope of final 80% of samples to detect persistent growth

## Key Findings

### Verified Cleanup Paths

These are correctly implemented and being called:

1. **DocumentStore.close()** (line 1805 in workspace_index.rs)
   - ✅ Removes from HashMap via `docs.remove(&key)`
   - ✅ Called from workspace_index.remove_file()

2. **WorkspaceIndex.remove_file()** (lines 1800-1843 in workspace_index.rs)
   - ✅ Removes files HashMap entry
   - ✅ Removes fact shards
   - ✅ Clears semantic reference index
   - ✅ Clears import/export index
   - ✅ Incrementally removes symbols
   - ✅ Removes global references
   - ✅ Called from LspServer.handle_did_close() via coordinator.index().clear_file()

3. **LspServer.documents** (line 993 in text_sync.rs)
   - ✅ Removes from server-level document cache via `documents.remove()`
   - ✅ Called in didClose handler

4. **semantic_analyzer_cache** (line 982 in text_sync.rs)
   - ✅ Retains only entries for other URIs via retain closure
   - ✅ Called in didClose handler

5. **symbol_index** (line 32 in text_sync.rs)
   - ✅ Removes document via `remove_document(uri)`
   - ✅ Called in didClose handler via clear_document_symbols()

### DocumentState Structure (state/document.rs)

DocumentState contains:
- `rope: ropey::Rope` — Rope data structure containing text
- `text: String` — **Duplicate copy** of text content (~2x memory overhead)
- `ast: Option<Arc<perl_parser::ast::Node>>` — Parsed AST
- `parent_map: ParentMap` — Scope traversal map
- `line_starts: LineStartsCache` — Position mapping cache
- `generation: Arc<AtomicU32>` — **Arc reference** (potential GC blocker)
- `parse_errors: Vec<ParseError>`
- `degradation_tier: DegradationTier`

Comment on lines 77-88 explicitly states: "Memory usage: ~2x content size due to dual representation"

### Memory Not Accounted For

Despite all cleanup paths being verified, memory continues to grow. Possible causes:

1. **Arc<AtomicU32> (generation field)** — If captured in closures or shared across async tasks, could prevent DocumentState drop
2. **Undiscovered secondary cache** — May be accumulating symbol/reference data
3. **Rope/String cloning** — Could have escape paths not yet identified
4. **Async task retention** — Background tasks may hold Arc references to generations or documents
5. **Parser cache or compilation artifacts** — Hidden state in parser infrastructure

## Next Steps

### Immediate Actions

1. **Add instrumentation** to document lifecycle:
   - Log creation/destruction events for DocumentState
   - Track Arc<AtomicU32> reference counts
   - Monitor HashMap sizes before/after removal

2. **Search for Arc captures**:
   - Find all places where DocumentState.generation is used
   - Check for closures that might capture it
   - Verify async task cancellation

3. **Run heaptrack/Valgrind**:
   - Use `valgrind --tool=massif` to profile heap growth
   - Identify top allocators during document churn
   - Pinpoint which code path is allocating the 75 KB/file

4. **Audit parser infrastructure**:
   - Check perl-parser for global caches
   - Verify symbol extraction cleanup
   - Ensure no parse trees escape

### Test Scenarios

- [x] Document churn without workspace/symbol (500 files): Leak confirmed
- [x] Document churn with workspace/symbol (300 files): Leak worse
- [ ] Large workspace indexing only (no document churn)
- [ ] Parser-only operations (no LSP handlers)
- [ ] Symbol extraction in isolation

## Files Modified

- `scripts/repro_lsp_storm.py` — LSP stdio reproduction harness
- `scripts/monitor_process_tree.sh` — Process tree RSS monitoring
- `scripts/test_workspace_memory.sh` — Workspace profiling wrapper
- This document — Investigation tracking

## References

- [workspace_index.rs](../../crates/perl-workspace/src/workspace/workspace_index.rs) — Symbol index and removal logic
- [text_sync.rs](../../crates/perl-lsp-rs/src/runtime/text_sync.rs) — LSP request handlers
- [document.rs](../../crates/perl-lsp-rs/src/state/document.rs) — DocumentState struct
- [MEMORY_PATTERNS.md](./MEMORY_PATTERNS.md) — Large workspace memory guidance
- [PROFILING_GUIDE.md](./PROFILING_GUIDE.md) — Profiling methodology

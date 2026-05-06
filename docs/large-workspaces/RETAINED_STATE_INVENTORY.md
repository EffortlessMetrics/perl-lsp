# Retained-State Inventory

This inventory tracks long-lived state that can retain per-document or
per-workspace memory across an LSP session. Every long-lived map, cache, task
queue, or subprocess/session holder must document:

- owner
- key type
- byte-risk
- eviction event
- regression test or receipt

The goal is not to make RSS return to the exact startup value. Allocators and
`HashMap` capacity can retain arenas after work completes. The goal is to make
retained state bounded, explainable, and covered by counters or receipts.

## Lifecycle Semantics

Close and delete are different lifecycle events.

| Lifecycle | Required behavior |
|-----------|-------------------|
| Close-only: `didOpen -> didClose` | Evict editor/session state for the open buffer. Workspace-backed indexes may retain file symbols because the source file still exists. |
| Close and delete: `didOpen -> didClose -> watched-file DELETED` | Evict editor/session state and remove workspace-index state for that file. No index structure should retain the deleted URI after pending work drains. |
| Workspace-folder removal | Evict open documents and workspace-index state under the removed roots only. Unrelated roots must remain intact. |

Use `evict_open_document_session_state(uri)` for close-only state,
`evict_deleted_file_state(uri)` for deleted files, and
`evict_workspace_folder_state(folder_uri)` for folder-scoped cleanup.

Background tasks must not be allowed to repopulate stale state after close,
delete, or folder removal. Indexing tasks need generation or version validation
before committing derived state.

## Current Inventory

| Owner | State | Key type | Byte-risk | Bounds and cleanup | Regression test or receipt |
|-------|-------|----------|-----------|--------------------|----------------------------|
| `LspServer` | Open documents in `documents` | Normalized URI `String` | Raw source text and document metadata | `didClose` uses `evict_open_document_session_state`; delete and folder removal route through stronger helpers | `test_did_close_zeroes_memory_state_snapshot`; folder-removal tests in `workspace.rs` |
| `LspServer` | `ast_cache` | URI `String` with cached content hash | Parsed ASTs for recently used documents | `AstCache::new(100, 300)` in runtime constructors; explicit `AstCache::remove` on close/delete | `ast_cache_remove_evicts_entry_immediately` |
| `LspServer` | `semantic_analyzer_cache` | `(normalized_uri, content_hash)` | Semantic analyzer graphs and derived scope state | Invalidated on `didChange`, close, and delete; hard-clears when the cache reaches 50 entries | semantic analyzer invalidation tests in `text_sync.rs`; `MemoryStateSnapshot` |
| `LspServer` | `parse_cancel_flags` | URI `String` | Per-document cancellation tokens and stale parse coordination | New parses cancel prior tokens; close/delete/folder cleanup trips and removes flags | `test_did_close_cancels_and_removes_flag`; snapshot tests |
| `LspServer` | `pod_cache` | Filesystem `PathBuf` | Parsed POD hover docs | Soft cap 1024 entries, prune target 512; close/delete removes the file path entry | `MemoryStateSnapshot`; hover cache cap should be covered by future POD churn scenario |
| `LspServer` | Pull diagnostics file cache | Filesystem path | Diagnostic result state and external diagnostic reuse | Invalidated on text change, close, and delete through the pull diagnostics orchestrator | lifecycle snapshot tests; future diagnostics churn receipt |
| `LspServer` | Perl::Critic analyzer and warning set | Analyzer config and workspace warning keys | External analyzer cache, profile discovery, warning suppression keys | Analyzer reset on critic configuration changes; file cache invalidated on document changes and eviction | diagnostics tests; future diagnostics churn receipt |
| `StreamSessionManager` | Inline-completion stream sessions | `SessionKey { uri, document_version, line, character }` | Streaming buffers, cancellation flags, per-request session entries | `cancel_for_uri` and `cancel_for_uri_version` cancel and remove entries immediately | stream-session eviction tests; `MemoryStateSnapshot.stream_sessions` |
| `SymbolIndex` | Open-document symbols | Document URI plus symbol name | Per-open-document symbol vectors and lookup maps | Re-index replaces old symbols; close cleanup clears document symbols | `test_did_close_removes_document_symbols_from_index` |
| `WorkspaceIndex` | Files, symbols, references, semantic shards, import/export facts | Normalized URI plus symbol/reference keys | Workspace-wide derived state, potentially proportional to workspace size | Delete and reindex use `remove_file`/`clear_file`; close-only should not remove file-backed symbols | `memory_leak_regression.rs`; close/delete lifecycle tests; memory plateau receipts |
| `DocumentStore` | Workspace document store | Normalized URI `String` | Raw text for workspace-indexed documents | `WorkspaceIndex::remove_file` closes the document store entry | document-store lifecycle tests |
| `NotebookStore` | Notebook documents and cell mapping | Notebook URI and cell URI `String` | Cell document text and notebook-to-cell relations | Notebook close closes cell docs; cell removal clears mapping and document state | notebook lifecycle tests |
| `LspServer` | Pending workspace configuration requests | JSON-RPC request id `i64` | Request parameters and response bookkeeping | Capped to 10; response removes id; configuration and folder changes clear pending entries | workspace configuration lifecycle tests |
| `LspServer` | Progress cancellation maps | Progress token and request id | Cancellation token bookkeeping | Progress cancel removes token and request mapping | progress cancellation tests |
| `CancellationRegistry` | Request cancellation tokens, cleanup contexts, token cache | Request id string | Request-scoped cancellation state and cleanup closures | Dispatch finalization and `RequestCleanupGuard` remove request state | cancellation registry cleanup tests |
| `DiagnosticDebouncer` | Pending diagnostic publish queue | URI `String` | Deferred URI set in worker thread | Expiration removes entries; `Drop` flushes pending entries on shutdown | diagnostic debouncer tests; `RuntimePressureSnapshot.diagnostic_debounce_pending_uris` |
| `FileWatcherDebouncer` | Pending file-watcher URI queue | URI `String` | Deferred URI set during bulk filesystem operations | Expiration removes entries; `Drop` flushes pending entries on shutdown | file watcher debouncer tests; `RuntimePressureSnapshot.file_watcher_pending_uris`; future watcher churn scenario |
| Read scheduler | Queued read requests and latest-sequence map | Request dedup key | Queued request payloads and cancellation tokens | Stale reads are cancelled before worker dispatch; queue drains on channel close | scheduler classification tests; add direct retained-state regression if pressure grows |
| DAP bridge | Debug child process and session state | Debug session/process id | Child process handles, reader tasks, debugger session state | Stop, disconnect, and shutdown paths must terminate or detach process state | DAP lifecycle tests; future `dap_bridge_start_stop_loop` scenario |
| Formatting and external tools | Perltidy/perlcritic subprocess buffers | Request scope and file path | Subprocess output buffers and temp data | Request-scoped by subprocess runtime; no session bag should retain output after completion | formatting/diagnostics tests; future subprocess churn scenario |

## Memory Budgets

| Area | Budget |
|------|--------|
| LSP doc churn PR smoke | 75 files, 5 changes, loose plateau, artifacts retained |
| Nightly doc churn | 500 files, 10 changes, strict plateau |
| Nightly workspace-symbol churn | 300 files, 10 changes, strict plateau |
| POD cache | Soft cap 1024 entries, prune target 512 |
| Semantic analyzer cache | 50 entries before clear |
| Runtime AST cache | 100 entries, 300 second TTL, explicit remove on close/delete |
| Workspace index caches | Use configured workspace resource limits; delete/reindex must not duplicate secondary indexes |

## Regression Surfaces

Memory coverage should stay split by subsystem. A single large memory test is
hard to interpret; focused scenarios make the owner obvious.

| Scenario | Purpose | Current status |
|----------|---------|----------------|
| `lsp_doc_churn_delete` | Baseline open/change/close/delete process RSS plateau | PR smoke and nightly receipts |
| `lsp_workspace_symbol_churn_delete` | Index/query pressure during document churn | Nightly receipts |
| `workspace_index_reindex_same_files` | Secondary-index duplication and remove/reindex cycles | Unit regression coverage |
| `diagnostics_pull_push_churn` | Pull diagnostics, result ids, critic analyzer cache | Follow-up scenario |
| `hover_pod_many_modules` | POD cache cap and path eviction | Follow-up scenario |
| `completion_stream_cancel_storm` | Stream-session cancellation and removal | Unit regression coverage |
| `file_watcher_bulk_create_change_delete` | Watcher debouncer and delete lifecycle | Follow-up scenario |
| `workspace_folder_add_remove_multi_root` | Folder-scoped cleanup without cross-root eviction | Unit coverage, add process scenario if needed |
| `dap_bridge_start_stop_loop` | Debug process/session lifecycle | Follow-up scenario |
| `formatting_perltidy_loop` | Subprocess and output-buffer retention | Follow-up scenario |

## Review Checklist

If a PR adds or changes a long-lived map, cache, task queue, or session holder,
the review must answer:

- What owns it?
- What bounds it?
- What removes entries?
- Is key normalization handled?
- Does close differ from delete?
- Can delayed background work repopulate stale state?
- Is there a regression test?
- Is there a receipt, snapshot counter, or debug counter?

## Investigation Playbook

When memory growth moves again:

1. Identify the growing process: parent LSP, child process, native/FFI, or
   subprocess runtime.
2. Identify the lifecycle: close-only, close/delete, reindex, query churn,
   cancellation, folder removal, DAP session, or external tool loop.
3. Capture counters: documents, stream sessions, parse flags, pending tasks,
   cache entries, workspace index stats, subprocess/session counts.
4. Reproduce with the narrowest harness that triggers the growth.
5. Add a failing regression before changing cleanup logic.
6. Patch the owner or eviction boundary.
7. Add or update a receipt so future runs classify the retained surface.

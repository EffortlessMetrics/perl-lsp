# Why perl-lsp Built a Custom LSP Runtime

*An architectural deep dive into the decision to build a bespoke JSON-RPC runtime instead of using tower-lsp.*

> This narrative overview is now captured as [ADR-0034](../adr/0034-custom-lsp-runtime.md). For the later bounded-concurrency scheduler layered on top of this runtime, see [ADR-0031](../adr/0031-async-runtime-concurrent-dispatch.md).

## Why Most Rust LSPs Use tower-lsp

The Rust ecosystem has a de facto standard for building language servers: [tower-lsp](https://github.com/ebkalderon/tower-lsp). Built on the Tower service abstraction and Tokio async runtime, tower-lsp provides:

- Automatic JSON-RPC message framing and dispatch
- Trait-based handler registration (`LanguageServer` trait)
- Built-in `$/cancelRequest` support
- Async request handling out of the box
- Server capability advertisement helpers

For many projects, tower-lsp is the right choice. It eliminates boilerplate and lets developers focus on language analysis. Projects like `rust-analyzer` (which uses its own `lsp-server` crate) and `taplo` (TOML, uses tower-lsp) demonstrate both approaches in production.

So why did perl-lsp build its own?

## What perl-lsp Built Instead (and Why)

perl-lsp implements a fully custom LSP runtime decomposed across seven dedicated crates:

| Crate | Responsibility |
|-------|---------------|
| `perl-lsp-protocol` | JSON-RPC 2.0 message types, LSP error codes, method constants, capability configuration |
| `perl-lsp-transport` | Content-Length message framing (read/write) |
| `perl-content-length-framing` | Byte-level frame extraction shared with DAP |
| `perl-lsp-cancellation` | Atomic cancellation tokens, registry, cleanup guards |
| `perl-lsp-input-validation` | Request sanitization and path traversal prevention |
| `perl-lsp-feature-governance` | Profile-based capability gating (GA-lock, production, all) |
| `perl-lsp-launcher` | CLI argument parsing, transport mode selection |

The server itself (`perl-lsp`) contains the dispatch loop, lifecycle management, and routing logic in its `runtime/` module. The core loop is remarkably straightforward -- a synchronous read-dispatch-respond cycle:

```rust
pub fn serve(&mut self, reader: &mut dyn BufRead) -> io::Result<()> {
    let mut message_reader = ContentLengthMessageReader::new();
    loop {
        match message_reader.read_next(reader)? {
            Some(request) => {
                if let Some(response) = self.handle_request(request) {
                    let mut output = self.output.lock();
                    write_message(&mut *output, &response)?;
                }
            }
            None => break, // EOF
        }
    }
    Ok(())
}
```

This is not the architecture you get from tower-lsp. There is no async runtime in the message loop, no Tower service layers, no middleware chain. The server is synchronous by design.

### The Historical Record

The git history tells a clear story. tower-lsp was never a production dependency:

- **Cargo.toml**: No `tower-lsp` entry in workspace dependencies.
- **Cargo.lock**: No `tower-lsp` crate resolved.
- **Issue #146** (September 2025): Explicitly removed leftover `tower_lsp::lsp_types` imports from `tdd_workflow.rs`, replacing them with direct `lsp_types` imports. Tests were written to enforce this (`issue_146_unit_tests.rs`, `issue_146_architectural_integrity_tests.rs`).
- **Dependabot config**: References `tower-lsp` in ignore rules and grouping -- this is defensive configuration for a dependency that *might* be added, not evidence it was ever used.

The project appears to have evaluated tower-lsp early in development and opted for a custom runtime from the start, with the ROPE_MIGRATION_GUIDE.md containing a vestigial `use tower_lsp::lsp_types::Position` in example code that was never production code.

## The Custom Runtime Architecture

### Protocol Layer (`perl-lsp-protocol`)

A Tier 1 leaf crate with zero internal dependencies. It defines:

- **`JsonRpcRequest`** / **`JsonRpcResponse`** / **`JsonRpcError`**: Plain serde structs for JSON-RPC 2.0. No tower Service trait, no async, no framework coupling.
- **Method constants** (`methods` module): Every LSP 3.17 method as a `&str` constant, enabling exhaustive match-based dispatch.
- **Error codes**: Full JSON-RPC 2.0 and LSP 3.17 error code catalog with builder functions (`cancelled_response`, `method_not_found`, `internal_error`).
- **Capability configuration**: `BuildFlags`-driven capability generation that integrates directly with the feature governance system.

### Transport Layer (`perl-lsp-transport` + `perl-content-length-framing`)

Two crates handle message framing:

- `perl-content-length-framing` provides the raw byte-level `ContentLengthFramer` -- a stateful parser that handles split headers, split bodies, and multiple messages in a single read. This crate is shared with the DAP (Debug Adapter Protocol) server, which uses the same framing.
- `perl-lsp-transport` wraps the framer with JSON-RPC deserialization, providing `read_message`, `write_message`, and `write_notification` functions.

The transport is synchronous and transport-agnostic. The same framing works over stdio and TCP sockets -- the `main.rs` binary handles the transport selection, converting TCP streams to blocking `Read`/`Write` trait objects.

### Dispatch Layer (`runtime/dispatch/`)

Request dispatch is a single `match` expression over method strings, organized into submodules:

- `text_document.rs` -- Document-level operations (50+ handlers)
- `workspace.rs` -- Workspace operations (symbols, configuration, file events)
- `lifecycle.rs` -- Initialize, shutdown, exit
- `cancellation.rs` -- `$/cancelRequest` processing with enhanced context
- `experimental.rs` -- Test and experimental endpoints

The match arms call directly into handler methods on `LspServer`. There is no dynamic dispatch, no handler registration, no middleware chain. The compiler can see every possible code path.

### Cancellation (`perl-lsp-cancellation`)

This is where the custom runtime diverges most significantly from what tower-lsp provides. The cancellation system implements:

- **Dual-layer design**: A global `CancellationRegistry` (thread-safe `RwLock<HashMap>`) stores `PerlLspCancellationToken` values backed by `AtomicBool` flags.
- **Sub-100-microsecond checks**: `is_cancelled()` is a single `Ordering::Relaxed` atomic load. The `is_cancelled_hot_path()` variant is annotated for branch prediction optimization.
- **Provider-specific cleanup**: Each cancellable request registers a `ProviderCleanupContext` with an optional cleanup callback, enabling resource cleanup when requests are cancelled mid-flight.
- **RAII cleanup guards**: `RequestCleanupGuard` ensures cancellation tokens are removed from the registry even if handlers panic.
- **Performance metrics**: Atomic counters track registration, cancellation, and completion counts for observability.
- **Selective registration**: Only potentially long-running operations (completion, hover, definition, references, workspace symbols, call hierarchy) register cancellation tokens, avoiding overhead for fast operations.

The `early_cancel_or!` macro provides inline cancellation checkpoints before handler execution, achieving the <50ms end-to-end cancellation response target documented in ADR-006.

## Feature Governance Integration

This is the capability that most clearly justifies the custom runtime. tower-lsp cannot do this.

perl-lsp implements a feature governance system spanning six microcrates (`perl-lsp-feature-*`) that controls which LSP capabilities are advertised and available at runtime:

- **Feature profiles**: `GaLock` (conservative), `Production` (standard), `All` (testing). Selected via CLI flag (`--feature-profile`) or compile-time feature gate (`lsp-ga-lock`).
- **Compile-time gating**: `BuildFlags` is a struct of booleans (`formatting`, `signature_help`, `implementation`, `notebook_document_sync`, etc.) that directly controls what `ServerCapabilities` the `initialize` response advertises.
- **Catalog-driven**: `features.toml` is the single source of truth. Each feature has an `id`, `maturity` level, `area`, and `advertised` flag. The governance system reads this catalog to generate capability sets, compliance percentages, and BDD grid reports.
- **Profile-aware dispatch**: The `FeatureProfile` flows from CLI parsing through `LaunchConfig` into `LspServer::new_with_feature_profile()`, where it determines the `advertised_features` and thus the exact set of capabilities returned in the `initialize` response.

In tower-lsp, capability advertisement is a static `ServerCapabilities` struct returned from `initialize`. To implement profile-based governance, you would need to wrap the tower-lsp `LanguageServer` trait implementation with conditional logic in every handler, fight the framework's assumptions about static capability sets, and lose the compile-time capability-to-profile mapping.

With the custom runtime, the feature profile is a first-class concept. The dispatch match arms, the capability builder, and the CLI all share the same governance vocabulary.

## Performance and Control Trade-offs

### What the Custom Runtime Gains

**Synchronous simplicity.** The server processes one request at a time on the main thread. There is no async runtime overhead, no executor scheduling, no `Pin<Box<dyn Future>>` allocations. For a language server where most operations complete in single-digit milliseconds (parsing a single file, looking up a symbol), this is often faster than the async alternative.

**Zero-copy transport.** The `ContentLengthFramer` operates on raw byte buffers and hands `&[u8]` slices directly to `serde_json::from_slice`. There is no intermediate `String` allocation for the message body.

**Compile-time dispatch.** The `match` expression over method strings compiles to a jump table or decision tree. The compiler can inline handler calls, eliminate dead code for disabled features, and optimize the entire dispatch path as a unit.

**Total control over error handling.** The project bans `unwrap()` and `expect()` in production code. Every error path returns `Result` or `Option`. The custom transport gracefully skips malformed frames (logging to stderr) and continues processing. tower-lsp's error handling model would conflict with this policy.

**Testing without process spawning.** `LspServer::with_io()` accepts any `Read + Send` and `Write + Send`, enabling in-process protocol-level tests with `Cursor<Vec<u8>>` buffers. No socket setup, no child process management, no async test runtime.

### What the Custom Runtime Costs

**No automatic concurrency.** The single-threaded message loop means a slow handler blocks all subsequent requests. tower-lsp handles requests concurrently by default. The perl-lsp project mitigates this with cancellation tokens (so clients can abort slow requests) and the AST cache (so repeat parses are fast), but true concurrent request processing would require significant architectural changes.

**More code to maintain.** The seven runtime crates total thousands of lines of transport, framing, and dispatch code that tower-lsp would provide for free. The team must maintain their own Content-Length parser, their own error code catalog, their own capability builder.

**No middleware ecosystem.** Tower's layered middleware model enables pluggable concerns (tracing, rate limiting, metrics). The custom runtime implements these concerns directly in the dispatch loop, which is simpler but less composable.

## Was It Worth It?

The evidence suggests yes, for this project. Three factors make the custom runtime a defensible choice:

1. **Feature governance is a differentiator.** The profile-based capability system -- with its compile-time feature gates, catalog-driven compliance reporting, and BDD grid output -- is deeply integrated into the runtime. Retrofitting this into tower-lsp's `LanguageServer` trait would be fighting the framework rather than building on it.

2. **The synchronous model matches the workload.** Perl source files are parsed individually. The recursive descent parser is fast (sub-millisecond for typical files). Workspace indexing happens incrementally as files are opened. The dominant bottleneck is parsing quality, not I/O concurrency. An async runtime would add complexity without adding throughput.

3. **The microcrate decomposition pays for the maintenance cost.** By splitting the runtime into `perl-lsp-protocol`, `perl-lsp-transport`, `perl-content-length-framing`, and `perl-lsp-cancellation`, the project achieves the same separation of concerns that tower-lsp provides internally. Each crate has focused tests, clear API boundaries, and independent versioning. The `perl-content-length-framing` crate is even shared with the DAP server, a reuse opportunity that would not exist if the framing logic were entangled with tower-lsp's Tower service model.

The Dependabot config still lists `tower-lsp` in its watch patterns -- a pragmatic hedge in case the architecture decision is ever revisited. But with 97 LSP features advertised at GA maturity, a custom cancellation system with sub-100-microsecond check latency, and a feature governance system that no off-the-shelf framework provides, the custom runtime has earned its keep.

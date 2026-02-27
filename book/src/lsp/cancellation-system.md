# LSP Cancellation Protocol Compliance Specification

<!-- Labels: lsp:enhancement, cancellation:protocol-compliance, parser:integration, threading:adaptive -->

## Executive Summary

This specification defines the enhanced LSP cancellation protocol compliance for the Perl Language Server based on finalized Issue #48 requirements. The specification addresses JSON-RPC 2.0 `$/cancelRequest` handling with comprehensive provider integration, thread-safe cancellation tokens, and performance-optimized cancellation overhead (<1ms).

## LSP Protocol Foundation

### JSON-RPC 2.0 Compliance Requirements

**Core Protocol Specification:**
- **Cancellation Notification**: `$/cancelRequest` with proper JSON-RPC 2.0 structure
- **Error Response**: -32800 (RequestCancelled) error code per LSP 3.17+ specification
- **No Response Rule**: Cancellation notifications never produce responses
- **Request ID Matching**: Exact Value comparison for cancellation identification

**Current Implementation Status** ✅ **FUNCTIONAL**:
```rust
// Current implementation in /crates/perl-parser/src/lsp_server.rs:533-537
if request.method == "$/cancelRequest" {
    if let Some(idv) = request.params.as_ref().and_then(|p| p.get("id")).cloned() {
        self.cancel_mark(&idv);
    }
    return None; // Notifications don't get responses
}
```

### Enhanced Protocol Requirements (Issue #48)

#### AC1: JSON-RPC 2.0 Protocol Enhancement

**Requirement**: Enhanced `$/cancelRequest` notification processing with provider context awareness.

**Technical Specification**:
```rust
/// Enhanced cancellation request structure with provider context
#[derive(Debug, Clone)]
pub struct CancellationRequest {
    /// JSON-RPC 2.0 request ID to cancel
    pub id: Value,
    /// LSP method being cancelled for context-aware cleanup
    pub method_context: Option<String>,
    /// Timestamp for cancellation latency tracking
    pub timestamp: std::time::Instant,
    /// Provider-specific cleanup requirements
    pub cleanup_context: ProviderCleanupContext,
}

/// Provider-specific cleanup context for enhanced cancellation
#[derive(Debug, Clone)]
pub enum ProviderCleanupContext {
    /// Completion provider with symbol resolution state
    Completion { workspace_symbols: bool, cross_file: bool },
    /// Workspace symbol search with indexing state
    WorkspaceSymbol { indexing_active: bool, file_count: usize },
    /// References provider with cross-file navigation state
    References { qualified_search: bool, dual_pattern: bool },
    /// Definition provider with incremental parsing state
    Definition { parsing_active: bool, file_uri: Option<String> },
    /// Hover provider with documentation resolution state
    Hover { doc_resolution: bool },
    /// Generic provider without specific cleanup requirements
    Generic,
}
```

#### AC2: Thread-Safe Cancellation Token Architecture

**Requirement**: Thread-safe cancellation tokens with atomic operations and provider integration.

**Enhanced Cancellation Token Design**:
```rust
/// Enhanced cancellation token with Perl LSP context and atomic operations
pub struct PerlLspCancellationToken {
    /// Thread-safe cancellation state using atomic boolean
    cancelled: Arc<AtomicBool>,
    /// Original request ID for tracking and cleanup
    request_id: Value,
    /// LSP provider context for targeted cleanup
    provider_context: ProviderCleanupContext,
    /// Workspace operations to terminate gracefully
    workspace_operations: Arc<Mutex<Vec<WorkspaceOperationId>>>,
    /// Cancellation timestamp for performance tracking
    created_at: std::time::Instant,
    /// Cancellation latency threshold for performance validation
    latency_threshold: std::time::Duration,
}

impl PerlLspCancellationToken {
    /// Create new cancellation token with provider context
    pub fn new(
        request_id: Value,
        provider_context: ProviderCleanupContext,
        latency_threshold: Option<std::time::Duration>,
    ) -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
            request_id,
            provider_context,
            workspace_operations: Arc::new(Mutex::new(Vec::new())),
            created_at: std::time::Instant::now(),
            latency_threshold: latency_threshold.unwrap_or(std::time::Duration::from_millis(1)),
        }
    }

    /// Check cancellation with atomic operation (lock-free)
    /// Returns true if operation should abort immediately
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    /// Check cancellation with performance tracking and context validation
    pub fn is_cancelled_with_context(&self) -> Result<bool, CancellationError> {
        let cancelled = self.cancelled.load(Ordering::Relaxed);

        // Track cancellation check latency for performance validation
        if cancelled {
            let check_latency = self.created_at.elapsed();
            if check_latency > self.latency_threshold {
                return Err(CancellationError::LatencyThresholdExceeded(check_latency));
            }
        }

        Ok(cancelled)
    }

    /// Cancel token with provider-specific cleanup and performance tracking
    pub fn cancel_with_cleanup(&self) -> Result<(), CancellationError> {
        // Mark as cancelled with atomic operation
        self.cancelled.store(true, Ordering::Relaxed);

        // Perform provider-specific cleanup based on context
        match &self.provider_context {
            ProviderCleanupContext::Completion { workspace_symbols, cross_file } => {
                if *workspace_symbols {
                    self.cleanup_workspace_symbol_resolution()?;
                }
                if *cross_file {
                    self.cleanup_cross_file_navigation()?;
                }
            },
            ProviderCleanupContext::WorkspaceSymbol { indexing_active, .. } => {
                if *indexing_active {
                    self.cleanup_workspace_indexing()?;
                }
            },
            ProviderCleanupContext::References { qualified_search, dual_pattern } => {
                if *qualified_search || *dual_pattern {
                    self.cleanup_dual_pattern_search()?;
                }
            },
            ProviderCleanupContext::Definition { parsing_active, .. } => {
                if *parsing_active {
                    self.cleanup_incremental_parsing()?;
                }
            },
            _ => {} // Generic cleanup or no specific cleanup required
        }

        Ok(())
    }

    /// Register workspace operation for cancellation tracking
    pub fn register_workspace_operation(&self, operation_id: WorkspaceOperationId) {
        if let Ok(mut ops) = self.workspace_operations.lock() {
            ops.push(operation_id);
        }
    }

    /// Cleanup functions for provider-specific resource management
    fn cleanup_workspace_symbol_resolution(&self) -> Result<(), CancellationError> {
        // Cancel ongoing workspace symbol indexing and cleanup temporary data
        Ok(())
    }

    fn cleanup_cross_file_navigation(&self) -> Result<(), CancellationError> {
        // Cancel cross-file reference resolution and cleanup file handles
        Ok(())
    }

    fn cleanup_workspace_indexing(&self) -> Result<(), CancellationError> {
        // Interrupt workspace indexing operations and preserve consistency
        Ok(())
    }

    fn cleanup_dual_pattern_search(&self) -> Result<(), CancellationError> {
        // Cancel dual indexing searches (qualified/bare function names)
        Ok(())
    }

    fn cleanup_incremental_parsing(&self) -> Result<(), CancellationError> {
        // Gracefully terminate incremental parsing without corruption
        Ok(())
    }
}
```

#### AC3: Comprehensive LSP Provider Integration

**Requirement**: Integration with all LSP providers including completion, hover, definition, references, workspace symbols, and call hierarchy.

**Provider Integration Schema**:
```rust
/// LSP Provider cancellation integration points
pub trait CancellableProvider {
    /// Provider-specific cancellation handling with context
    fn handle_cancellation(
        &mut self,
        token: &PerlLspCancellationToken
    ) -> Result<(), CancellationError>;

    /// Check if provider supports graceful cancellation
    fn supports_cancellation(&self) -> bool { true }

    /// Get provider-specific cleanup requirements
    fn cleanup_context(&self) -> ProviderCleanupContext;
}

/// Enhanced completion provider with cancellation support
impl CancellableProvider for CompletionProvider {
    fn handle_cancellation(
        &mut self,
        token: &PerlLspCancellationToken
    ) -> Result<(), CancellationError> {
        // Cancel workspace symbol resolution
        self.workspace_index.cancel_symbol_resolution(token)?;

        // Cancel cross-file module resolution
        self.module_resolver.cancel_resolution(token)?;

        // Cleanup temporary completion data
        self.completion_cache.clear_pending();

        Ok(())
    }

    fn cleanup_context(&self) -> ProviderCleanupContext {
        ProviderCleanupContext::Completion {
            workspace_symbols: self.workspace_symbols_enabled,
            cross_file: self.cross_file_enabled,
        }
    }
}

/// Enhanced workspace symbol provider with cancellation support
impl CancellableProvider for WorkspaceSymbolProvider {
    fn handle_cancellation(
        &mut self,
        token: &PerlLspCancellationToken
    ) -> Result<(), CancellationError> {
        // Cancel ongoing file indexing operations
        self.file_indexer.cancel_indexing(token)?;

        // Cancel symbol search across multiple files
        self.symbol_searcher.cancel_search(token)?;

        // Preserve index consistency during cancellation
        self.workspace_index.ensure_consistency()?;

        Ok(())
    }

    fn cleanup_context(&self) -> ProviderCleanupContext {
        ProviderCleanupContext::WorkspaceSymbol {
            indexing_active: self.file_indexer.is_active(),
            file_count: self.workspace_index.file_count(),
        }
    }
}

/// Enhanced references provider with dual-pattern cancellation
impl CancellableProvider for ReferencesProvider {
    fn handle_cancellation(
        &mut self,
        token: &PerlLspCancellationToken
    ) -> Result<(), CancellationError> {
        // Cancel qualified name search (Package::function)
        self.qualified_searcher.cancel_search(token)?;

        // Cancel bare name search (function)
        self.bare_searcher.cancel_search(token)?;

        // Cancel cross-file reference resolution
        self.cross_file_resolver.cancel_resolution(token)?;

        Ok(())
    }

    fn cleanup_context(&self) -> ProviderCleanupContext {
        ProviderCleanupContext::References {
            qualified_search: self.qualified_search_active,
            dual_pattern: self.dual_pattern_enabled,
        }
    }
}
```

#### AC4: Enhanced Error Response Handling

**Requirement**: Proper -32800 error code responses with enhanced error context and performance tracking.

**Error Response Enhancement**:
```rust
/// Enhanced cancellation error with comprehensive context
#[derive(Debug, Clone)]
pub struct CancellationError {
    /// LSP error code (-32800 for RequestCancelled)
    pub code: i32,
    /// Human-readable error message
    pub message: String,
    /// Additional error context for debugging
    pub data: Option<serde_json::Value>,
    /// Provider that was cancelled
    pub provider: String,
    /// Cancellation latency for performance tracking
    pub latency: std::time::Duration,
    /// Request ID that was cancelled
    pub request_id: Value,
}

impl CancellationError {
    /// Create standard RequestCancelled error with context
    pub fn request_cancelled(
        provider: String,
        latency: std::time::Duration,
        request_id: Value,
    ) -> Self {
        Self {
            code: -32800,
            message: format!("Request cancelled in {} provider", provider),
            data: Some(json!({
                "provider": provider,
                "latency_ms": latency.as_millis(),
                "request_id": request_id
            })),
            provider,
            latency,
            request_id,
        }
    }

    /// Create latency threshold exceeded error
    pub fn latency_threshold_exceeded(latency: std::time::Duration) -> Self {
        Self {
            code: -32800,
            message: format!("Cancellation latency exceeded threshold: {}ms", latency.as_millis()),
            data: Some(json!({
                "latency_ms": latency.as_millis(),
                "threshold_exceeded": true
            })),
            provider: "cancellation_system".to_string(),
            latency,
            request_id: json!(null),
        }
    }
}

/// Enhanced error response creation with performance context
pub fn cancelled_response_with_context(
    id: &Value,
    error: CancellationError,
) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id: Some(id.clone()),
        result: None,
        error: Some(JsonRpcError {
            code: error.code,
            message: error.message,
            data: error.data,
        }),
    }
}
```

## Performance Requirements

### AC12: Cancellation Overhead Specification

**Performance Targets**:
- **Cancellation Check Latency**: <100μs per check (well under 1ms requirement)
- **Cancellation Response Time**: <50ms from notification to error response
- **Memory Overhead**: <1MB additional memory usage for cancellation infrastructure
- **Thread Contention**: Zero lock contention for cancellation checks using atomic operations

**Performance Validation Framework**:
```rust
/// Performance tracking for cancellation operations
pub struct CancellationPerformanceTracker {
    /// Total cancellation checks performed
    check_count: AtomicU64,
    /// Total check latency for average calculation
    total_check_latency: AtomicU64,
    /// Maximum observed check latency
    max_check_latency: AtomicU64,
    /// Cancellation response times
    response_times: Arc<Mutex<Vec<std::time::Duration>>>,
}

impl CancellationPerformanceTracker {
    /// Record cancellation check latency
    pub fn record_check_latency(&self, latency: std::time::Duration) {
        self.check_count.fetch_add(1, Ordering::Relaxed);
        self.total_check_latency.fetch_add(latency.as_nanos() as u64, Ordering::Relaxed);

        // Update maximum latency if this exceeds current max
        let latency_nanos = latency.as_nanos() as u64;
        self.max_check_latency.fetch_max(latency_nanos, Ordering::Relaxed);
    }

    /// Calculate average check latency
    pub fn average_check_latency(&self) -> std::time::Duration {
        let count = self.check_count.load(Ordering::Relaxed);
        let total = self.total_check_latency.load(Ordering::Relaxed);

        if count > 0 {
            std::time::Duration::from_nanos(total / count)
        } else {
            std::time::Duration::from_nanos(0)
        }
    }

    /// Get performance metrics for validation
    pub fn get_metrics(&self) -> CancellationMetrics {
        CancellationMetrics {
            total_checks: self.check_count.load(Ordering::Relaxed),
            average_latency: self.average_check_latency(),
            max_latency: std::time::Duration::from_nanos(
                self.max_check_latency.load(Ordering::Relaxed)
            ),
            response_times: self.response_times.lock().unwrap().clone(),
        }
    }
}
```

## Integration with Adaptive Threading (RUST_TEST_THREADS=2)

### AC10: Thread Configuration Compatibility

**Requirement**: Enhanced compatibility with adaptive threading configuration, particularly `RUST_TEST_THREADS=2` environment.

**Adaptive Cancellation Configuration**:
```rust
/// Adaptive cancellation configuration for different threading environments
pub struct AdaptiveCancellationConfig {
    /// Thread count from environment or default
    pub thread_count: usize,
    /// Cancellation check frequency based on thread contention
    pub check_frequency: usize,
    /// Timeout scaling factor for constrained environments
    pub timeout_multiplier: f32,
    /// Cancellation latency threshold adjustment
    pub latency_threshold: std::time::Duration,
}

impl AdaptiveCancellationConfig {
    /// Create configuration from environment variables
    pub fn from_environment() -> Self {
        let thread_count = std::env::var("RUST_TEST_THREADS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(num_cpus::get());

        let (check_frequency, timeout_multiplier, latency_threshold) = match thread_count {
            0..=2 => (
                25,   // More frequent checks in constrained environments
                3.0,  // 3x longer timeouts
                std::time::Duration::from_micros(500), // Higher latency tolerance
            ),
            3..=4 => (
                50,   // Moderate check frequency
                2.0,  // 2x longer timeouts
                std::time::Duration::from_micros(200),
            ),
            _ => (
                100,  // Standard check frequency
                1.0,  // Standard timeouts
                std::time::Duration::from_micros(100), // Strict latency requirement
            ),
        };

        Self {
            thread_count,
            check_frequency,
            timeout_multiplier,
            latency_threshold,
        }
    }

    /// Get cancellation check interval based on thread configuration
    pub fn check_interval(&self, operation_count: usize) -> bool {
        operation_count % self.check_frequency == 0
    }

    /// Get timeout with scaling factor applied
    pub fn scaled_timeout(&self, base_timeout: std::time::Duration) -> std::time::Duration {
        base_timeout.mul_f32(self.timeout_multiplier)
    }
}
```

## Error Handling and Graceful Degradation

### AC5: Multiple Concurrent Cancellation Handling

**Requirement**: Support for cancelling multiple concurrent requests without interference or resource contention.

**Concurrent Cancellation Management**:
```rust
/// Thread-safe cancellation registry for managing multiple concurrent cancellations
pub struct CancellationRegistry {
    /// Active cancellation tokens indexed by request ID
    tokens: Arc<RwLock<HashMap<Value, Arc<PerlLspCancellationToken>>>>,
    /// Performance tracking across all cancellations
    performance_tracker: Arc<CancellationPerformanceTracker>,
    /// Configuration for adaptive threading
    config: AdaptiveCancellationConfig,
}

impl CancellationRegistry {
    /// Register new cancellation token
    pub fn register_token(
        &self,
        request_id: Value,
        provider_context: ProviderCleanupContext,
    ) -> Arc<PerlLspCancellationToken> {
        let token = Arc::new(PerlLspCancellationToken::new(
            request_id.clone(),
            provider_context,
            Some(self.config.latency_threshold),
        ));

        self.tokens.write().unwrap().insert(request_id, token.clone());
        token
    }

    /// Cancel specific request and cleanup resources
    pub fn cancel_request(&self, request_id: &Value) -> Result<(), CancellationError> {
        let start = std::time::Instant::now();

        if let Some(token) = self.tokens.read().unwrap().get(request_id) {
            token.cancel_with_cleanup()?;

            // Remove from registry
            self.tokens.write().unwrap().remove(request_id);

            // Track performance
            let latency = start.elapsed();
            self.performance_tracker.record_check_latency(latency);

            Ok(())
        } else {
            // Request not found or already completed
            Ok(())
        }
    }

    /// Cancel multiple requests efficiently
    pub fn cancel_multiple_requests(
        &self,
        request_ids: &[Value],
    ) -> Vec<Result<(), CancellationError>> {
        request_ids.iter()
            .map(|id| self.cancel_request(id))
            .collect()
    }

    /// Cleanup completed requests to prevent memory leaks
    pub fn cleanup_completed_requests(&self) {
        self.tokens.write().unwrap().retain(|_, token| {
            !token.is_cancelled()
        });
    }
}
```

## Implementation Integration Points

### LSP Server Integration

**Enhanced LSP Server with Cancellation Registry**:
```rust
/// Enhanced LspServer with comprehensive cancellation support
impl LspServer {
    /// Initialize cancellation registry with adaptive configuration
    pub fn new_with_cancellation() -> Self {
        let config = AdaptiveCancellationConfig::from_environment();
        let cancellation_registry = Arc::new(CancellationRegistry::new(config));

        Self {
            // ... existing fields ...
            cancellation_registry,
            // ... other fields ...
        }
    }

    /// Enhanced request handling with provider-aware cancellation
    pub fn handle_request_with_cancellation(
        &mut self,
        request: JsonRpcRequest
    ) -> Option<JsonRpcResponse> {
        let id = request.id.clone();

        // Handle $/cancelRequest with enhanced context
        if request.method == "$/cancelRequest" {
            return self.handle_cancellation_request(request);
        }

        // Register cancellation token for trackable requests
        let cancellation_token = if let Some(ref request_id) = id {
            let provider_context = self.determine_provider_context(&request.method);
            Some(self.cancellation_registry.register_token(
                request_id.clone(),
                provider_context,
            ))
        } else {
            None
        };

        // Check for existing cancellation before processing
        if let Some(ref request_id) = id {
            if let Some(token) = cancellation_token.as_ref() {
                if token.is_cancelled() {
                    return Some(cancelled_response_with_context(
                        request_id,
                        CancellationError::request_cancelled(
                            request.method.clone(),
                            std::time::Duration::from_nanos(0),
                            request_id.clone(),
                        ),
                    ));
                }
            }
        }

        // Process request with cancellation token
        self.process_request_with_token(request, cancellation_token)
    }

    /// Enhanced cancellation request handling
    fn handle_cancellation_request(
        &mut self,
        request: JsonRpcRequest
    ) -> Option<JsonRpcResponse> {
        if let Some(params) = request.params.as_ref() {
            if let Some(cancel_id) = params.get("id") {
                if let Err(error) = self.cancellation_registry.cancel_request(cancel_id) {
                    eprintln!("Cancellation error: {:?}", error);
                }
            }
        }
        None // Notifications don't get responses per LSP spec
    }

    /// Determine provider context from LSP method
    fn determine_provider_context(&self, method: &str) -> ProviderCleanupContext {
        match method {
            "textDocument/completion" => ProviderCleanupContext::Completion {
                workspace_symbols: true,
                cross_file: true,
            },
            "workspace/symbol" => ProviderCleanupContext::WorkspaceSymbol {
                indexing_active: true,
                file_count: self.workspace_index.file_count(),
            },
            "textDocument/references" => ProviderCleanupContext::References {
                qualified_search: true,
                dual_pattern: true,
            },
            "textDocument/definition" => ProviderCleanupContext::Definition {
                parsing_active: true,
                file_uri: None,
            },
            "textDocument/hover" => ProviderCleanupContext::Hover {
                doc_resolution: true,
            },
            _ => ProviderCleanupContext::Generic,
        }
    }
}
```

## Backward Compatibility and Migration

### Compatibility Requirements

**Existing API Preservation**:
- All existing `cancel_mark`, `cancel_clear`, and `is_cancelled` methods remain functional
- `early_cancel_or!` macro continues to work with enhanced token system
- No breaking changes to LSP client interfaces
- Graceful fallback to basic cancellation if enhanced features unavailable

**Migration Strategy**:
1. **Phase 1**: Deploy enhanced cancellation infrastructure alongside existing system
2. **Phase 2**: Migrate LSP providers to use enhanced cancellation tokens
3. **Phase 3**: Enable performance tracking and adaptive threading integration
4. **Phase 4**: Deprecate legacy cancellation methods while maintaining compatibility

## Conclusion

This LSP Cancellation Protocol Compliance Specification provides a comprehensive enhancement framework for Issue #48, addressing all acceptance criteria while maintaining full backward compatibility. The specification ensures <1ms cancellation overhead, thread-safe operations with atomic primitives, and comprehensive LSP provider integration.

Key architectural benefits:
- **Performance**: Atomic operations eliminate lock contention for cancellation checks
- **Scalability**: Adaptive threading configuration optimizes for different environments
- **Reliability**: Provider-specific cleanup ensures graceful resource management
- **Maintainability**: Clear separation of concerns and comprehensive error handling
- **Compatibility**: Full LSP 3.17+ protocol compliance with enhanced context

The enhanced cancellation system integrates seamlessly with the existing Perl LSP ecosystem, preserving ~100% Perl syntax coverage, incremental parsing performance, and security standards while providing robust cancellation capabilities across all LSP providers.
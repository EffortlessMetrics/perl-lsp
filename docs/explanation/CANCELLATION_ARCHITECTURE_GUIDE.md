# Cancellation Architecture Guide - Parser Integration & Dual Indexing

<!-- Labels: architecture:enhancement, parser:integration, lsp:cancellation, performance:optimized -->

## Executive Summary

This guide defines the comprehensive cancellation architecture for the Perl LSP ecosystem, focusing on parser integration requirements, dual indexing compatibility, and workspace navigation cancellation. The architecture ensures seamless integration with incremental parsing (<1ms updates), cross-file navigation, and the enhanced dual indexing strategy (qualified/bare function names) while maintaining ~100% Perl syntax coverage.

## Architectural Foundation

### Core Architecture Components

```
┌─────────────────────────────────────────────────────────────────┐
│                     Perl LSP Cancellation Architecture          │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌──────────────────┐  ┌─────────────────┐ │
│  │   JSON-RPC 2.0  │  │ Cancellation     │  │  Provider       │ │
│  │   Protocol      │◄─┤ Token Registry   ├─►│  Integration    │ │
│  │   ($/cancel)    │  │  (Thread-Safe)   │  │  (11 Providers) │ │
│  └─────────────────┘  └──────────────────┘  └─────────────────┘ │
│           │                     │                      │         │
│           ▼                     ▼                      ▼         │
│  ┌─────────────────┐  ┌──────────────────┐  ┌─────────────────┐ │
│  │  Performance    │  │ Workspace        │  │  Parser         │ │
│  │  Monitoring     │  │ Navigation       │  │  Integration    │ │
│  │  (<1ms checks)  │  │ (Dual Indexing)  │  │  (Incremental)  │ │
│  └─────────────────┘  └──────────────────┘  └─────────────────┘ │
│           │                     │                      │         │
│           └─────────────────────┼──────────────────────┘         │
│                                 ▼                                │
│              ┌──────────────────────────────────────────┐        │
│              │        Adaptive Threading Config         │        │
│              │        (RUST_TEST_THREADS=2)            │        │
│              └──────────────────────────────────────────┘        │
└─────────────────────────────────────────────────────────────────┘
```

### Crate Integration Architecture

**Target Crate Structure** (Based on `/crates/` analysis):
- **perl-parser** (`/crates/perl-parser/`): Core cancellation infrastructure and parser integration
- **perl-lsp** (`/crates/perl-lsp/`): LSP server binary with cancellation protocol handling
- **perl-lexer** (`/crates/perl-lexer/`): Tokenization cancellation integration points
- **perl-corpus** (`/crates/perl-corpus/`): Enhanced test corpus with cancellation scenarios

## Parser Integration Requirements

### AC1 & AC11: Incremental Parsing Cancellation Integration

**Requirement**: Seamless integration with incremental parsing system while preserving <1ms update performance.

**Incremental Parser Cancellation Architecture**:
```rust
/// Enhanced incremental parser with cancellation integration
pub struct IncrementalParserWithCancellation {
    /// Core incremental parser
    parser: IncrementalParser,
    /// Active cancellation token for current parsing operation
    cancellation_token: Option<Arc<PerlLspCancellationToken>>,
    /// Checkpoint manager for safe cancellation points
    checkpoint_manager: CheckpointManager,
    /// Performance metrics for cancellation impact
    performance_tracker: Arc<ParsingPerformanceTracker>,
}

impl IncrementalParserWithCancellation {
    /// Parse with cancellation support and performance preservation
    pub fn parse_with_cancellation(
        &mut self,
        source: &str,
        changes: &[TextChange],
        token: Option<Arc<PerlLspCancellationToken>>,
    ) -> Result<ParseResult, CancellationError> {
        let start = std::time::Instant::now();
        self.cancellation_token = token.clone();

        // Create parsing checkpoints for safe cancellation
        let checkpoint = self.checkpoint_manager.create_checkpoint();

        // Parse with regular cancellation checks
        let result = self.parse_with_checkpoints(source, changes)?;

        // Validate performance preservation
        let parsing_duration = start.elapsed();
        if parsing_duration > std::time::Duration::from_millis(1) {
            eprintln!("Warning: Incremental parsing exceeded 1ms threshold: {:?}", parsing_duration);
        }

        self.performance_tracker.record_parsing_duration(parsing_duration);

        // Clear cancellation token after successful completion
        self.cancellation_token = None;

        Ok(result)
    }

    /// Parse with strategic cancellation checkpoints
    fn parse_with_checkpoints(
        &mut self,
        source: &str,
        changes: &[TextChange],
    ) -> Result<ParseResult, CancellationError> {
        // Strategic cancellation points during parsing
        let cancellation_points = self.calculate_cancellation_points(source, changes);

        for (i, change) in changes.iter().enumerate() {
            // Check cancellation at strategic points (not every iteration)
            if cancellation_points.contains(&i) {
                if let Some(ref token) = self.cancellation_token {
                    if token.is_cancelled()? {
                        // Restore from checkpoint before cancellation
                        self.checkpoint_manager.restore_checkpoint()?;
                        return Err(CancellationError::request_cancelled(
                            "incremental_parser".to_string(),
                            std::time::Duration::from_nanos(0),
                            token.request_id.clone(),
                        ));
                    }
                }
            }

            // Apply incremental change
            self.parser.apply_change(change)?;
        }

        Ok(self.parser.get_result())
    }

    /// Calculate strategic cancellation check points to minimize performance impact
    fn calculate_cancellation_points(
        &self,
        source: &str,
        changes: &[TextChange],
    ) -> Vec<usize> {
        let change_count = changes.len();

        match change_count {
            0..=10 => vec![], // No cancellation checks for small changes
            11..=50 => vec![change_count / 2], // Check once in middle
            51..=200 => vec![change_count / 3, (change_count * 2) / 3], // Check twice
            _ => {
                // For large change sets, check every 100 changes
                (0..change_count).step_by(100).collect()
            }
        }
    }
}

/// Checkpoint manager for safe cancellation during parsing
pub struct CheckpointManager {
    /// Saved parser states for rollback
    checkpoints: Vec<ParserCheckpoint>,
    /// Maximum number of checkpoints to retain
    max_checkpoints: usize,
}

#[derive(Clone)]
struct ParserCheckpoint {
    /// Parser state at checkpoint
    parser_state: ParserState,
    /// Timestamp of checkpoint creation
    timestamp: std::time::Instant,
    /// Source position at checkpoint
    source_position: usize,
}

impl CheckpointManager {
    /// Create new checkpoint before risky operations
    pub fn create_checkpoint(&mut self) -> Result<(), CancellationError> {
        let checkpoint = ParserCheckpoint {
            parser_state: self.capture_parser_state()?,
            timestamp: std::time::Instant::now(),
            source_position: self.get_current_position(),
        };

        self.checkpoints.push(checkpoint);

        // Limit checkpoint history to prevent memory growth
        if self.checkpoints.len() > self.max_checkpoints {
            self.checkpoints.remove(0);
        }

        Ok(())
    }

    /// Restore from most recent checkpoint
    pub fn restore_checkpoint(&mut self) -> Result<(), CancellationError> {
        if let Some(checkpoint) = self.checkpoints.pop() {
            self.restore_parser_state(checkpoint.parser_state)?;
            Ok(())
        } else {
            Err(CancellationError::request_cancelled(
                "checkpoint_manager".to_string(),
                std::time::Duration::from_nanos(0),
                json!("no_checkpoints"),
            ))
        }
    }

    fn capture_parser_state(&self) -> Result<ParserState, CancellationError> {
        // Implementation would capture current parser state
        Ok(ParserState::new())
    }

    fn restore_parser_state(&mut self, state: ParserState) -> Result<(), CancellationError> {
        // Implementation would restore parser to previous state
        Ok(())
    }

    fn get_current_position(&self) -> usize {
        // Implementation would return current parsing position
        0
    }
}
```

### AC3: Workspace Indexing Cancellation Integration

**Requirement**: Integration with workspace indexing operations including dual indexing strategy and cross-file navigation.

**Workspace Index Cancellation Architecture**:
```rust
/// Enhanced workspace index with cancellation-aware operations
pub struct CancellableWorkspaceIndex {
    /// Core workspace index
    index: WorkspaceIndex,
    /// File indexing operations tracking
    indexing_operations: Arc<Mutex<HashMap<String, Arc<PerlLspCancellationToken>>>>,
    /// Dual indexing state for cancellation coordination
    dual_indexing_state: Arc<RwLock<DualIndexingState>>,
    /// Performance metrics for indexing operations
    performance_tracker: Arc<IndexingPerformanceTracker>,
}

impl CancellableWorkspaceIndex {
    /// Index file with cancellation support and dual pattern preservation
    pub fn index_file_with_cancellation(
        &mut self,
        file_path: &str,
        content: &str,
        token: Arc<PerlLspCancellationToken>,
    ) -> Result<(), CancellationError> {
        let start = std::time::Instant::now();

        // Register indexing operation
        self.indexing_operations.lock().unwrap()
            .insert(file_path.to_string(), token.clone());

        // Parse file with cancellation support
        let ast = self.parse_file_cancellable(content, &token)?;

        // Index functions using dual pattern strategy with cancellation
        self.index_functions_dual_pattern(&ast, file_path, &token)?;

        // Index variables and other symbols with cancellation
        self.index_symbols_with_cancellation(&ast, file_path, &token)?;

        // Track performance
        let duration = start.elapsed();
        self.performance_tracker.record_indexing_duration(file_path, duration);

        // Cleanup operation tracking
        self.indexing_operations.lock().unwrap().remove(file_path);

        Ok(())
    }

    /// Index functions using dual pattern strategy (qualified + bare names) with cancellation
    fn index_functions_dual_pattern(
        &mut self,
        ast: &AstNode,
        file_path: &str,
        token: &PerlLspCancellationToken,
    ) -> Result<(), CancellationError> {
        let functions = self.extract_functions(ast, token)?;

        for (i, function) in functions.iter().enumerate() {
            // Check cancellation every 50 functions
            if i % 50 == 0 && token.is_cancelled()? {
                return Err(CancellationError::request_cancelled(
                    "dual_pattern_indexing".to_string(),
                    std::time::Duration::from_nanos(0),
                    token.request_id.clone(),
                ));
            }

            let symbol_ref = SymbolReference {
                name: function.name.clone(),
                location: Location {
                    uri: file_path.to_string(),
                    range: function.range,
                },
                symbol_kind: SymbolKind::Function,
            };

            // Index under bare name (legacy compatibility)
            self.index.references
                .entry(function.name.clone())
                .or_default()
                .push(symbol_ref.clone());

            // Index under qualified name (Package::function)
            if let Some(package) = &function.package {
                let qualified_name = format!("{}::{}", package, function.name);
                self.index.references
                    .entry(qualified_name)
                    .or_default()
                    .push(symbol_ref);
            }
        }

        Ok(())
    }

    /// Extract functions from AST with cancellation support
    fn extract_functions(
        &self,
        ast: &AstNode,
        token: &PerlLspCancellationToken,
    ) -> Result<Vec<FunctionInfo>, CancellationError> {
        let mut functions = Vec::new();
        let mut node_count = 0;

        self.traverse_ast_cancellable(ast, &mut |node| {
            node_count += 1;

            // Check cancellation every 1000 nodes
            if node_count % 1000 == 0 && token.is_cancelled().unwrap_or(false) {
                return Err(CancellationError::request_cancelled(
                    "ast_traversal".to_string(),
                    std::time::Duration::from_nanos(0),
                    token.request_id.clone(),
                ));
            }

            if let AstNodeType::FunctionDef = node.node_type {
                functions.push(FunctionInfo::from_ast_node(node));
            }

            Ok(())
        })?;

        Ok(functions)
    }

    /// Find references with dual pattern matching and cancellation support
    pub fn find_references_with_cancellation(
        &self,
        symbol_name: &str,
        token: &PerlLspCancellationToken,
    ) -> Result<Vec<Location>, CancellationError> {
        let start = std::time::Instant::now();
        let mut locations = Vec::new();

        // Search exact match first
        if let Some(refs) = self.index.references.get(symbol_name) {
            for (i, reference) in refs.iter().enumerate() {
                // Check cancellation every 100 references
                if i % 100 == 0 && token.is_cancelled()? {
                    return Err(CancellationError::request_cancelled(
                        "reference_search".to_string(),
                        start.elapsed(),
                        token.request_id.clone(),
                    ));
                }
                locations.push(reference.location.clone());
            }
        }

        // If qualified name, also search bare name (dual pattern matching)
        if let Some(idx) = symbol_name.rfind("::") {
            let bare_name = &symbol_name[idx + 2..];
            if let Some(refs) = self.index.references.get(bare_name) {
                for (i, reference) in refs.iter().enumerate() {
                    // Check cancellation every 100 references
                    if i % 100 == 0 && token.is_cancelled()? {
                        return Err(CancellationError::request_cancelled(
                            "bare_name_search".to_string(),
                            start.elapsed(),
                            token.request_id.clone(),
                        ));
                    }
                    locations.push(reference.location.clone());
                }
            }
        }

        // Deduplicate based on URI + Range
        locations.sort_by(|a, b| {
            a.uri.cmp(&b.uri).then_with(|| {
                a.range.start.line.cmp(&b.range.start.line)
                    .then_with(|| a.range.start.character.cmp(&b.range.start.character))
            })
        });
        locations.dedup_by(|a, b| a.uri == b.uri && a.range == b.range);

        self.performance_tracker.record_search_duration(symbol_name, start.elapsed());
        Ok(locations)
    }

    /// Cancel all ongoing indexing operations
    pub fn cancel_all_indexing(&mut self) -> Result<(), CancellationError> {
        let operations = self.indexing_operations.lock().unwrap();

        for (file_path, token) in operations.iter() {
            if let Err(e) = token.cancel_with_cleanup() {
                eprintln!("Failed to cancel indexing for {}: {:?}", file_path, e);
            }
        }

        Ok(())
    }
}

/// Dual indexing state for coordination during cancellation
#[derive(Debug)]
struct DualIndexingState {
    /// Files currently being indexed
    active_files: HashSet<String>,
    /// Qualified name indexing progress
    qualified_progress: HashMap<String, usize>,
    /// Bare name indexing progress
    bare_progress: HashMap<String, usize>,
}
```

## Cross-File Navigation Cancellation

### AC1 & AC3: Enhanced Cross-File Definition Resolution

**Requirement**: Cancellable cross-file navigation with enhanced definition resolution and 98% success rate preservation.

**Cross-File Navigation Architecture**:
```rust
/// Enhanced cross-file navigation with comprehensive cancellation support
pub struct CancellableNavigationProvider {
    /// Workspace index for symbol lookup
    workspace_index: Arc<CancellableWorkspaceIndex>,
    /// File system operations manager
    file_manager: Arc<FileManager>,
    /// Navigation cache for performance optimization
    navigation_cache: Arc<RwLock<HashMap<String, NavigationResult>>>,
    /// Performance tracking for navigation operations
    performance_tracker: Arc<NavigationPerformanceTracker>,
}

impl CancellableNavigationProvider {
    /// Find definition with multi-tier fallback and cancellation support
    pub fn find_definition_with_cancellation(
        &self,
        uri: &str,
        position: Position,
        token: Arc<PerlLspCancellationToken>,
    ) -> Result<Vec<Location>, CancellationError> {
        let start = std::time::Instant::now();

        // Check cache first (fastest path)
        let cache_key = format!("{}:{}:{}", uri, position.line, position.character);
        if let Some(cached_result) = self.navigation_cache.read().unwrap().get(&cache_key) {
            if !cached_result.is_expired() {
                return Ok(cached_result.locations.clone());
            }
        }

        // Multi-tier definition resolution with cancellation
        let mut locations = Vec::new();

        // Tier 1: Local file symbol resolution (fastest)
        token.check_cancelled_or_continue()?;
        if let Some(local_locations) = self.find_local_definition(uri, position, &token)? {
            locations.extend(local_locations);
        }

        // Tier 2: Workspace index lookup (fast)
        token.check_cancelled_or_continue()?;
        if locations.is_empty() {
            if let Some(workspace_locations) = self.find_workspace_definition(uri, position, &token)? {
                locations.extend(workspace_locations);
            }
        }

        // Tier 3: Cross-file text search (slower, only if needed)
        token.check_cancelled_or_continue()?;
        if locations.is_empty() {
            if let Some(text_search_locations) = self.find_text_search_definition(uri, position, &token)? {
                locations.extend(text_search_locations);
            }
        }

        // Cache result for future requests
        let result = NavigationResult::new(locations.clone(), std::time::Duration::from_secs(300));
        self.navigation_cache.write().unwrap().insert(cache_key, result);

        // Track performance
        let duration = start.elapsed();
        self.performance_tracker.record_definition_lookup(uri, duration);

        Ok(locations)
    }

    /// Find definition in local file with cancellation
    fn find_local_definition(
        &self,
        uri: &str,
        position: Position,
        token: &PerlLspCancellationToken,
    ) -> Result<Option<Vec<Location>>, CancellationError> {
        token.check_cancelled_or_continue()?;

        // Parse current file and extract symbol at position
        let content = self.file_manager.read_file(uri)?;
        let ast = self.parse_file_cancellable(&content, token)?;
        let symbol = self.extract_symbol_at_position(&ast, position, token)?;

        if let Some(symbol_name) = symbol {
            // Search for definition within the same file
            let definitions = self.find_symbol_definitions_in_ast(&ast, &symbol_name, token)?;
            if !definitions.is_empty() {
                return Ok(Some(definitions.into_iter().map(|range| Location {
                    uri: uri.to_string(),
                    range,
                }).collect()));
            }
        }

        Ok(None)
    }

    /// Find definition using workspace index with dual pattern matching
    fn find_workspace_definition(
        &self,
        uri: &str,
        position: Position,
        token: &PerlLspCancellationToken,
    ) -> Result<Option<Vec<Location>>, CancellationError> {
        token.check_cancelled_or_continue()?;

        // Extract symbol name from position
        let content = self.file_manager.read_file(uri)?;
        let ast = self.parse_file_cancellable(&content, token)?;
        let symbol = self.extract_symbol_at_position(&ast, position, token)?;

        if let Some(symbol_name) = symbol {
            // Use workspace index with dual pattern matching
            let locations = self.workspace_index
                .find_references_with_cancellation(&symbol_name, token)?;

            // Filter for definitions (not references)
            let definitions: Vec<Location> = locations.into_iter()
                .filter(|loc| self.is_definition_location(loc))
                .collect();

            if !definitions.is_empty() {
                return Ok(Some(definitions));
            }
        }

        Ok(None)
    }

    /// Find definition using cross-file text search (fallback)
    fn find_text_search_definition(
        &self,
        uri: &str,
        position: Position,
        token: &PerlLspCancellationToken,
    ) -> Result<Option<Vec<Location>>, CancellationError> {
        token.check_cancelled_or_continue()?;

        // Extract symbol name
        let content = self.file_manager.read_file(uri)?;
        let ast = self.parse_file_cancellable(&content, token)?;
        let symbol = self.extract_symbol_at_position(&ast, position, token)?;

        if let Some(symbol_name) = symbol {
            // Search across all Perl files in workspace
            let workspace_files = self.file_manager.get_perl_files()?;
            let mut definitions = Vec::new();

            for (i, file_path) in workspace_files.iter().enumerate() {
                // Check cancellation every 10 files
                if i % 10 == 0 {
                    token.check_cancelled_or_continue()?;
                }

                if let Some(file_definitions) = self.search_file_for_definition(file_path, &symbol_name, token)? {
                    definitions.extend(file_definitions);
                }
            }

            if !definitions.is_empty() {
                return Ok(Some(definitions));
            }
        }

        Ok(None)
    }
}

/// Enhanced cancellation token with continuation checking
impl PerlLspCancellationToken {
    /// Check cancelled or continue - ergonomic helper for navigation
    pub fn check_cancelled_or_continue(&self) -> Result<(), CancellationError> {
        if self.is_cancelled()? {
            Err(CancellationError::request_cancelled(
                "navigation_provider".to_string(),
                self.created_at.elapsed(),
                self.request_id.clone(),
            ))
        } else {
            Ok(())
        }
    }
}
```

## Performance Integration and Monitoring

### AC12: Performance Preservation with Cancellation

**Requirement**: Maintain <1ms cancellation overhead while preserving existing parsing and navigation performance.

**Performance Monitoring Architecture**:
```rust
/// Comprehensive performance tracking for cancellation impact analysis
pub struct CancellationPerformanceMonitor {
    /// Parsing performance metrics
    parsing_metrics: Arc<RwLock<ParsingMetrics>>,
    /// Navigation performance metrics
    navigation_metrics: Arc<RwLock<NavigationMetrics>>,
    /// Indexing performance metrics
    indexing_metrics: Arc<RwLock<IndexingMetrics>>,
    /// Cancellation overhead metrics
    cancellation_metrics: Arc<RwLock<CancellationMetrics>>,
}

#[derive(Debug, Default)]
pub struct ParsingMetrics {
    /// Parse durations without cancellation
    baseline_durations: Vec<std::time::Duration>,
    /// Parse durations with cancellation support
    cancellation_durations: Vec<std::time::Duration>,
    /// Number of cancelled parsing operations
    cancelled_operations: u64,
    /// Average overhead introduced by cancellation support
    average_overhead: std::time::Duration,
}

#[derive(Debug, Default)]
pub struct NavigationMetrics {
    /// Definition lookup times
    definition_lookup_times: Vec<std::time::Duration>,
    /// Reference search times
    reference_search_times: Vec<std::time::Duration>,
    /// Cross-file navigation success rate
    success_rate: f64,
    /// Cancellation impact on success rate
    cancellation_success_impact: f64,
}

impl CancellationPerformanceMonitor {
    /// Record parsing performance with cancellation support
    pub fn record_parsing_performance(
        &self,
        baseline_duration: std::time::Duration,
        cancellation_duration: std::time::Duration,
        was_cancelled: bool,
    ) {
        let mut metrics = self.parsing_metrics.write().unwrap();

        metrics.baseline_durations.push(baseline_duration);
        metrics.cancellation_durations.push(cancellation_duration);

        if was_cancelled {
            metrics.cancelled_operations += 1;
        }

        // Calculate overhead
        if cancellation_duration > baseline_duration {
            let overhead = cancellation_duration - baseline_duration;
            metrics.average_overhead = self.calculate_running_average(
                metrics.average_overhead,
                overhead,
                metrics.cancellation_durations.len(),
            );
        }
    }

    /// Validate performance preservation requirements
    pub fn validate_performance_requirements(&self) -> PerformanceValidationResult {
        let parsing_metrics = self.parsing_metrics.read().unwrap();
        let navigation_metrics = self.navigation_metrics.read().unwrap();
        let cancellation_metrics = self.cancellation_metrics.read().unwrap();

        let mut violations = Vec::new();

        // Validate <1ms incremental parsing requirement
        if let Some(max_parsing_duration) = parsing_metrics.cancellation_durations.iter().max() {
            if *max_parsing_duration > std::time::Duration::from_millis(1) {
                violations.push(format!(
                    "Incremental parsing exceeded 1ms: {:?}",
                    max_parsing_duration
                ));
            }
        }

        // Validate <100μs cancellation check overhead
        if cancellation_metrics.average_check_latency > std::time::Duration::from_micros(100) {
            violations.push(format!(
                "Cancellation check overhead exceeded 100μs: {:?}",
                cancellation_metrics.average_check_latency
            ));
        }

        // Validate 98% navigation success rate preservation
        if navigation_metrics.success_rate < 0.98 {
            violations.push(format!(
                "Navigation success rate below 98%: {:.2}%",
                navigation_metrics.success_rate * 100.0
            ));
        }

        PerformanceValidationResult {
            passed: violations.is_empty(),
            violations,
            metrics_summary: self.generate_metrics_summary(),
        }
    }

    /// Generate comprehensive performance metrics summary
    fn generate_metrics_summary(&self) -> MetricsSummary {
        let parsing = self.parsing_metrics.read().unwrap();
        let navigation = self.navigation_metrics.read().unwrap();
        let cancellation = self.cancellation_metrics.read().unwrap();

        MetricsSummary {
            parsing_overhead: parsing.average_overhead,
            cancellation_check_latency: cancellation.average_check_latency,
            navigation_success_rate: navigation.success_rate,
            total_cancelled_operations: parsing.cancelled_operations,
            performance_impact: self.calculate_overall_impact(),
        }
    }

    fn calculate_running_average(
        &self,
        current_average: std::time::Duration,
        new_value: std::time::Duration,
        count: usize,
    ) -> std::time::Duration {
        if count == 1 {
            new_value
        } else {
            let current_nanos = current_average.as_nanos() as f64;
            let new_nanos = new_value.as_nanos() as f64;
            let average_nanos = (current_nanos * (count - 1) as f64 + new_nanos) / count as f64;
            std::time::Duration::from_nanos(average_nanos as u64)
        }
    }

    fn calculate_overall_impact(&self) -> f64 {
        // Implementation would calculate overall performance impact percentage
        0.02 // Example: 2% impact
    }
}

#[derive(Debug)]
pub struct PerformanceValidationResult {
    pub passed: bool,
    pub violations: Vec<String>,
    pub metrics_summary: MetricsSummary,
}

#[derive(Debug)]
pub struct MetricsSummary {
    pub parsing_overhead: std::time::Duration,
    pub cancellation_check_latency: std::time::Duration,
    pub navigation_success_rate: f64,
    pub total_cancelled_operations: u64,
    pub performance_impact: f64,
}
```

## Thread-Safe Integration with Adaptive Threading

### AC10: RUST_TEST_THREADS=2 Compatibility Enhancement

**Integration with Existing Threading Configuration**:
```rust
/// Enhanced adaptive threading integration for cancellation system
pub struct AdaptiveCancellationThreading {
    /// Base adaptive configuration from existing system
    base_config: AdaptiveCancellationConfig,
    /// Thread pool for cancellation operations
    cancellation_pool: Arc<rayon::ThreadPool>,
    /// Contention detection and adjustment
    contention_detector: Arc<ContentionDetector>,
}

impl AdaptiveCancellationThreading {
    /// Create from environment with existing threading patterns
    pub fn from_environment() -> Result<Self, CancellationError> {
        let base_config = AdaptiveCancellationConfig::from_environment();

        // Create thread pool sized for cancellation workload
        let cancellation_pool = Arc::new(
            rayon::ThreadPoolBuilder::new()
                .num_threads(base_config.thread_count.max(2)) // Minimum 2 threads
                .thread_name(|i| format!("perl-lsp-cancel-{}", i))
                .build()
                .map_err(|e| CancellationError::thread_pool_error(e))?
        );

        Ok(Self {
            base_config,
            cancellation_pool,
            contention_detector: Arc::new(ContentionDetector::new()),
        })
    }

    /// Execute cancellation with adaptive threading
    pub fn execute_cancellation<F, R>(
        &self,
        operation: F,
    ) -> Result<R, CancellationError>
    where
        F: FnOnce() -> Result<R, CancellationError> + Send,
        R: Send,
    {
        let start = std::time::Instant::now();

        // Detect current thread contention
        let contention_level = self.contention_detector.detect_contention();

        // Adjust execution strategy based on contention
        let result = match contention_level {
            ContentionLevel::Low => {
                // Execute directly in current thread (fastest)
                operation()
            },
            ContentionLevel::Medium => {
                // Use thread pool with normal priority
                self.execute_in_pool_with_priority(operation, Priority::Normal)
            },
            ContentionLevel::High => {
                // Use thread pool with high priority and longer timeout
                self.execute_in_pool_with_priority(operation, Priority::High)
            },
        };

        // Track execution performance
        let duration = start.elapsed();
        self.contention_detector.record_execution_time(duration);

        result
    }

    fn execute_in_pool_with_priority<F, R>(
        &self,
        operation: F,
        priority: Priority,
    ) -> Result<R, CancellationError>
    where
        F: FnOnce() -> Result<R, CancellationError> + Send,
        R: Send,
    {
        let timeout = match priority {
            Priority::Normal => self.base_config.scaled_timeout(std::time::Duration::from_secs(2)),
            Priority::High => self.base_config.scaled_timeout(std::time::Duration::from_secs(5)),
        };

        // Execute with timeout in thread pool
        let (sender, receiver) = std::sync::mpsc::channel();

        self.cancellation_pool.spawn(move || {
            let result = operation();
            let _ = sender.send(result);
        });

        receiver.recv_timeout(timeout)
            .map_err(|_| CancellationError::timeout_exceeded(timeout))?
    }
}

/// Contention detection for adaptive cancellation threading
pub struct ContentionDetector {
    /// Recent execution times for trend analysis
    execution_times: Arc<Mutex<VecDeque<std::time::Duration>>>,
    /// Thread utilization tracking
    thread_utilization: Arc<AtomicU64>,
    /// Contention level determination
    current_contention: Arc<AtomicU8>,
}

#[derive(Debug, Clone, Copy)]
enum ContentionLevel {
    Low = 0,
    Medium = 1,
    High = 2,
}

#[derive(Debug, Clone, Copy)]
enum Priority {
    Normal,
    High,
}
```

## Integration Testing Architecture

### Comprehensive Integration Test Framework

**Test Architecture for Cancellation Integration**:
```rust
/// Comprehensive integration test framework for cancellation architecture
pub struct CancellationIntegrationTestFramework {
    /// Test LSP server instance
    test_server: TestLspServer,
    /// Mock workspace with various Perl files
    test_workspace: TestWorkspace,
    /// Performance validation toolkit
    performance_validator: PerformanceValidator,
    /// Cancellation scenario generator
    scenario_generator: CancellationScenarioGenerator,
}

impl CancellationIntegrationTestFramework {
    /// Test incremental parsing cancellation integration
    pub fn test_incremental_parsing_cancellation(&self) -> TestResult {
        // AC1 & AC11: Test incremental parsing with cancellation
        let scenarios = vec![
            CancellationScenario::CancelDuringParsing,
            CancellationScenario::CancelBetweenChanges,
            CancellationScenario::CancelAfterCheckpoint,
        ];

        let mut results = Vec::new();
        for scenario in scenarios {
            let result = self.execute_parsing_cancellation_scenario(scenario);
            results.push(result);
        }

        TestResult::aggregate(results)
    }

    /// Test workspace indexing cancellation with dual pattern preservation
    pub fn test_workspace_indexing_cancellation(&self) -> TestResult {
        // AC3: Test dual indexing cancellation
        let large_workspace = self.test_workspace.create_large_workspace(100); // 100 files

        let indexing_token = self.create_test_token("workspace_indexing");

        // Start indexing operation
        let indexing_handle = self.test_server.start_workspace_indexing(&large_workspace);

        // Cancel after partial completion
        std::thread::sleep(std::time::Duration::from_millis(100));
        indexing_token.cancel();

        // Verify graceful cancellation
        let result = indexing_handle.join_timeout(std::time::Duration::from_secs(5));
        assert!(result.is_cancelled() || result.is_completed_with_partial_data());

        // Verify index consistency after cancellation
        let index_state = self.test_server.get_workspace_index_state();
        assert!(index_state.is_consistent());
        assert!(index_state.dual_pattern_integrity_maintained());

        TestResult::passed("Workspace indexing cancellation maintains integrity")
    }

    /// Test cross-file navigation cancellation
    pub fn test_cross_file_navigation_cancellation(&self) -> TestResult {
        // AC1 & AC3: Test cross-file definition resolution cancellation
        let complex_workspace = self.test_workspace.create_complex_perl_project();

        let navigation_scenarios = vec![
            NavigationScenario::LocalFileDefinition,
            NavigationScenario::WorkspaceIndexLookup,
            NavigationScenario::CrossFileTextSearch,
        ];

        let mut results = Vec::new();
        for scenario in navigation_scenarios {
            let token = self.create_test_token(&format!("navigation_{:?}", scenario));
            let result = self.execute_navigation_cancellation_scenario(scenario, token);
            results.push(result);
        }

        TestResult::aggregate(results)
    }

    /// Test performance preservation during cancellation
    pub fn test_performance_preservation(&self) -> TestResult {
        // AC12: Validate <1ms overhead requirement
        let baseline_metrics = self.measure_baseline_performance();
        let cancellation_metrics = self.measure_cancellation_performance();

        let performance_comparison = self.performance_validator
            .compare_performance(baseline_metrics, cancellation_metrics);

        assert!(performance_comparison.parsing_overhead < std::time::Duration::from_millis(1));
        assert!(performance_comparison.cancellation_check_latency < std::time::Duration::from_micros(100));
        assert!(performance_comparison.navigation_success_rate >= 0.98);

        TestResult::passed("Performance preservation requirements met")
    }
}
```

## Conclusion

This Cancellation Architecture Guide provides comprehensive integration specifications for enhancing the Perl LSP cancellation system while preserving performance, maintaining dual indexing integrity, and ensuring seamless parser integration. The architecture addresses all finalized Issue #48 requirements with particular focus on:

**Key Architectural Achievements**:
- **Parser Integration**: Seamless cancellation integration with incremental parsing, preserving <1ms update performance
- **Dual Indexing Compatibility**: Enhanced workspace indexing with cancellation support while maintaining qualified/bare function name dual pattern strategy
- **Cross-File Navigation**: Comprehensive cancellation support across multi-tier definition resolution (local → workspace → text search)
- **Performance Preservation**: <100μs cancellation check overhead with atomic operations and strategic checkpoint placement
- **Thread Safety**: Complete integration with adaptive threading configuration (RUST_TEST_THREADS=2) including contention detection
- **Comprehensive Testing**: Integration test framework validating all cancellation scenarios with performance metrics

The architecture maintains full backward compatibility with existing LSP infrastructure while providing enhanced cancellation capabilities across all providers, ensuring production-grade reliability for the Perl Language Server ecosystem.
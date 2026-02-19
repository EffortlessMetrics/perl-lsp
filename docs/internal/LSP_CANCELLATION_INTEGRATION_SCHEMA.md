# LSP Cancellation Integration Schema - Dual Indexing & Workspace Navigation

<!-- Labels: integration:schema, dual-indexing:compatibility, workspace:navigation, cancellation:architecture -->

## Executive Summary

This integration schema defines the comprehensive architecture for integrating enhanced LSP cancellation capabilities with the Perl LSP dual indexing strategy and workspace navigation systems. The schema ensures seamless cancellation support across qualified/bare function name patterns, cross-file navigation, and workspace symbol resolution while maintaining 98% reference resolution success rate and enterprise-grade performance.

## Dual Indexing Integration Architecture

### Core Dual Indexing Cancellation Schema

**Enhanced Dual Pattern Storage with Cancellation Support**:
```rust
/// Enhanced workspace index with cancellation-aware dual pattern storage
pub struct CancellableDualIndex {
    /// Qualified name index (Package::function) with cancellation tracking
    qualified_index: Arc<RwLock<HashMap<String, CancellableSymbolSet>>>,
    /// Bare name index (function) with cancellation tracking
    bare_index: Arc<RwLock<HashMap<String, CancellableSymbolSet>>>,
    /// Cross-reference tracking for consistency during cancellation
    cross_reference_map: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// Indexing operation tracking for graceful cancellation
    active_operations: Arc<Mutex<HashMap<String, CancellationContext>>>,
    /// Performance metrics for dual pattern operations
    dual_pattern_metrics: Arc<DualPatternMetrics>,
}

impl CancellableDualIndex {
    /// Index function with dual pattern strategy and cancellation support
    pub fn index_function_dual_pattern(
        &mut self,
        function_info: &FunctionInfo,
        file_uri: &str,
        token: &PerlLspCancellationToken,
    ) -> Result<(), CancellationError> {
        let operation_id = format!("index_{}_{}", file_uri, function_info.name);
        let context = CancellationContext::new(&operation_id, token.clone());

        // Register operation for tracking
        self.active_operations.lock().unwrap()
            .insert(operation_id.clone(), context);

        // Check cancellation before expensive operations
        token.check_cancelled_or_continue()?;

        let symbol_ref = SymbolReference {
            name: function_info.name.clone(),
            location: Location {
                uri: file_uri.to_string(),
                range: function_info.range,
            },
            symbol_kind: SymbolKind::Function,
            package_context: function_info.package.clone(),
        };

        // Phase 1: Index under bare name with cancellation check
        self.index_bare_name_with_cancellation(&symbol_ref, token)?;

        // Phase 2: Index under qualified name with cancellation check
        if let Some(package) = &function_info.package {
            self.index_qualified_name_with_cancellation(&symbol_ref, package, token)?;
        }

        // Phase 3: Update cross-reference mapping
        self.update_cross_reference_mapping(&symbol_ref, token)?;

        // Cleanup operation tracking
        self.active_operations.lock().unwrap().remove(&operation_id);

        Ok(())
    }

    /// Index bare name with atomic cancellation support
    fn index_bare_name_with_cancellation(
        &mut self,
        symbol_ref: &SymbolReference,
        token: &PerlLspCancellationToken,
    ) -> Result<(), CancellationError> {
        token.check_cancelled_or_continue()?;

        let mut bare_index = self.bare_index.write().unwrap();
        let symbol_set = bare_index
            .entry(symbol_ref.name.clone())
            .or_insert_with(|| CancellableSymbolSet::new(token.clone()));

        // Add symbol with cancellation awareness
        symbol_set.add_symbol_with_cancellation(symbol_ref.clone(), token)?;

        Ok(())
    }

    /// Index qualified name with atomic cancellation support
    fn index_qualified_name_with_cancellation(
        &mut self,
        symbol_ref: &SymbolReference,
        package: &str,
        token: &PerlLspCancellationToken,
    ) -> Result<(), CancellationError> {
        token.check_cancelled_or_continue()?;

        let qualified_name = format!("{}::{}", package, symbol_ref.name);
        let mut qualified_index = self.qualified_index.write().unwrap();

        let symbol_set = qualified_index
            .entry(qualified_name.clone())
            .or_insert_with(|| CancellableSymbolSet::new(token.clone()));

        // Add qualified symbol with cancellation awareness
        symbol_set.add_symbol_with_cancellation(symbol_ref.clone(), token)?;

        // Update cross-reference mapping
        self.cross_reference_map.write().unwrap()
            .entry(symbol_ref.name.clone())
            .or_default()
            .push(qualified_name);

        Ok(())
    }

    /// Enhanced dual pattern search with cancellation support
    pub fn find_references_dual_pattern(
        &self,
        symbol_name: &str,
        token: &PerlLspCancellationToken,
    ) -> Result<Vec<Location>, CancellationError> {
        let start = std::time::Instant::now();
        let mut locations = Vec::new();
        let mut dedup_set = HashSet::new();

        // Phase 1: Search exact match (bare or qualified)
        token.check_cancelled_or_continue()?;
        self.search_exact_match(&mut locations, &mut dedup_set, symbol_name, token)?;

        // Phase 2: Search dual pattern alternatives
        token.check_cancelled_or_continue()?;
        self.search_dual_pattern_alternatives(&mut locations, &mut dedup_set, symbol_name, token)?;

        // Phase 3: Cross-reference expansion if needed
        token.check_cancelled_or_continue()?;
        self.expand_cross_references(&mut locations, &mut dedup_set, symbol_name, token)?;

        // Track performance metrics
        let search_duration = start.elapsed();
        self.dual_pattern_metrics.record_search_performance(
            symbol_name,
            search_duration,
            locations.len(),
            token.is_cancelled().unwrap_or(false),
        );

        Ok(locations)
    }

    /// Search exact match with cancellation checkpoints
    fn search_exact_match(
        &self,
        locations: &mut Vec<Location>,
        dedup_set: &mut HashSet<LocationKey>,
        symbol_name: &str,
        token: &PerlLspCancellationToken,
    ) -> Result<(), CancellationError> {
        // Search in qualified index first
        if let Some(qualified_set) = self.qualified_index.read().unwrap().get(symbol_name) {
            let symbols = qualified_set.get_symbols_with_cancellation(token)?;
            self.add_locations_with_dedup(locations, dedup_set, symbols, token)?;
        }

        // Search in bare index if not found or for completeness
        if let Some(bare_set) = self.bare_index.read().unwrap().get(symbol_name) {
            let symbols = bare_set.get_symbols_with_cancellation(token)?;
            self.add_locations_with_dedup(locations, dedup_set, symbols, token)?;
        }

        Ok(())
    }

    /// Search dual pattern alternatives with cancellation
    fn search_dual_pattern_alternatives(
        &self,
        locations: &mut Vec<Location>,
        dedup_set: &mut HashSet<LocationKey>,
        symbol_name: &str,
        token: &PerlLspCancellationToken,
    ) -> Result<(), CancellationError> {
        // If qualified name provided, search bare alternatives
        if let Some(idx) = symbol_name.rfind("::") {
            let bare_name = &symbol_name[idx + 2..];
            if let Some(bare_set) = self.bare_index.read().unwrap().get(bare_name) {
                let symbols = bare_set.get_symbols_with_cancellation(token)?;
                self.add_locations_with_dedup(locations, dedup_set, symbols, token)?;
            }
        }

        // If bare name provided, search qualified alternatives using cross-reference map
        else {
            if let Some(qualified_alternatives) = self.cross_reference_map.read().unwrap().get(symbol_name) {
                for (i, qualified_name) in qualified_alternatives.iter().enumerate() {
                    // Check cancellation every 10 alternatives
                    if i % 10 == 0 {
                        token.check_cancelled_or_continue()?;
                    }

                    if let Some(qualified_set) = self.qualified_index.read().unwrap().get(qualified_name) {
                        let symbols = qualified_set.get_symbols_with_cancellation(token)?;
                        self.add_locations_with_dedup(locations, dedup_set, symbols, token)?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Add locations with deduplication and cancellation support
    fn add_locations_with_dedup(
        &self,
        locations: &mut Vec<Location>,
        dedup_set: &mut HashSet<LocationKey>,
        symbols: Vec<SymbolReference>,
        token: &PerlLspCancellationToken,
    ) -> Result<(), CancellationError> {
        for (i, symbol) in symbols.iter().enumerate() {
            // Check cancellation every 50 symbols
            if i % 50 == 0 {
                token.check_cancelled_or_continue()?;
            }

            let location_key = LocationKey::from(&symbol.location);
            if dedup_set.insert(location_key) {
                locations.push(symbol.location.clone());
            }
        }
        Ok(())
    }
}

/// Cancellable symbol set with atomic operations
#[derive(Debug)]
pub struct CancellableSymbolSet {
    /// Thread-safe symbol storage
    symbols: Arc<RwLock<Vec<SymbolReference>>>,
    /// Associated cancellation token for this set
    cancellation_token: Arc<PerlLspCancellationToken>,
    /// Set creation timestamp for cleanup
    created_at: std::time::Instant,
}

impl CancellableSymbolSet {
    /// Create new cancellable symbol set
    pub fn new(token: Arc<PerlLspCancellationToken>) -> Self {
        Self {
            symbols: Arc::new(RwLock::new(Vec::new())),
            cancellation_token: token,
            created_at: std::time::Instant::now(),
        }
    }

    /// Add symbol with cancellation check
    pub fn add_symbol_with_cancellation(
        &self,
        symbol: SymbolReference,
        token: &PerlLspCancellationToken,
    ) -> Result<(), CancellationError> {
        token.check_cancelled_or_continue()?;

        self.symbols.write().unwrap().push(symbol);
        Ok(())
    }

    /// Get symbols with cancellation support
    pub fn get_symbols_with_cancellation(
        &self,
        token: &PerlLspCancellationToken,
    ) -> Result<Vec<SymbolReference>, CancellationError> {
        token.check_cancelled_or_continue()?;

        Ok(self.symbols.read().unwrap().clone())
    }
}

/// Location key for deduplication
#[derive(Debug, Hash, PartialEq, Eq, Clone)]
struct LocationKey {
    uri: String,
    start_line: u32,
    start_character: u32,
    end_line: u32,
    end_character: u32,
}

impl From<&Location> for LocationKey {
    fn from(location: &Location) -> Self {
        Self {
            uri: location.uri.clone(),
            start_line: location.range.start.line,
            start_character: location.range.start.character,
            end_line: location.range.end.line,
            end_character: location.range.end.character,
        }
    }
}
```

## Workspace Navigation Integration Schema

### AC1 & AC3: Cross-File Navigation with Cancellation

**Enhanced Multi-Tier Navigation Architecture**:
```rust
/// Enhanced workspace navigation with comprehensive cancellation support
pub struct CancellableWorkspaceNavigation {
    /// Dual index for symbol resolution
    dual_index: Arc<CancellableDualIndex>,
    /// File manager for content access
    file_manager: Arc<FileManager>,
    /// Navigation cache with cancellation awareness
    navigation_cache: Arc<CancellableNavigationCache>,
    /// Multi-tier resolver with cancellation checkpoints
    multi_tier_resolver: Arc<MultiTierResolver>,
    /// Performance tracking for navigation operations
    navigation_metrics: Arc<NavigationMetrics>,
}

impl CancellableWorkspaceNavigation {
    /// Enhanced definition resolution with multi-tier fallback and cancellation
    pub fn resolve_definition_with_cancellation(
        &self,
        uri: &str,
        position: Position,
        token: Arc<PerlLspCancellationToken>,
    ) -> Result<DefinitionResult, CancellationError> {
        let start = std::time::Instant::now();
        let operation_id = format!("definition_{}:{}:{}", uri, position.line, position.character);

        // Register operation for tracking
        token.register_workspace_operation(WorkspaceOperationId::Definition(operation_id.clone()));

        // Multi-tier resolution with cancellation support
        let result = self.multi_tier_resolver
            .resolve_with_cancellation(uri, position, token.clone())?;

        // Update navigation cache
        if let Ok(ref def_result) = result {
            self.navigation_cache
                .cache_result_with_cancellation(uri, position, def_result.clone(), token.clone())?;
        }

        // Track performance
        let duration = start.elapsed();
        self.navigation_metrics.record_definition_resolution(
            uri,
            duration,
            result.as_ref().map(|r| r.locations.len()).unwrap_or(0),
            token.is_cancelled().unwrap_or(false),
        );

        result
    }

    /// Enhanced reference finding with dual pattern matching and cancellation
    pub fn find_references_with_cancellation(
        &self,
        uri: &str,
        position: Position,
        include_declaration: bool,
        token: Arc<PerlLspCancellationToken>,
    ) -> Result<Vec<Location>, CancellationError> {
        let start = std::time::Instant::now();

        // Extract symbol name from position
        let symbol_name = self.extract_symbol_at_position(uri, position, &token)?;

        if let Some(symbol) = symbol_name {
            // Use dual index with cancellation
            let mut references = self.dual_index
                .find_references_dual_pattern(&symbol, &token)?;

            // Filter out declaration if not requested
            if !include_declaration {
                token.check_cancelled_or_continue()?;
                references = self.filter_declarations(references, &token)?;
            }

            // Sort and validate results
            token.check_cancelled_or_continue()?;
            self.sort_and_validate_references(&mut references, &token)?;

            let duration = start.elapsed();
            self.navigation_metrics.record_reference_search(
                &symbol,
                duration,
                references.len(),
                token.is_cancelled().unwrap_or(false),
            );

            Ok(references)
        } else {
            Ok(Vec::new())
        }
    }

    /// Extract symbol at position with cancellation support
    fn extract_symbol_at_position(
        &self,
        uri: &str,
        position: Position,
        token: &PerlLspCancellationToken,
    ) -> Result<Option<String>, CancellationError> {
        token.check_cancelled_or_continue()?;

        // Read file content
        let content = self.file_manager.read_file_with_cancellation(uri, token)?;

        // Parse to extract symbol
        let ast = self.parse_content_with_cancellation(&content, token)?;

        // Find symbol at position
        let symbol = self.find_symbol_at_position(&ast, position, token)?;

        Ok(symbol)
    }

    /// Parse content with cancellation checkpoints
    fn parse_content_with_cancellation(
        &self,
        content: &str,
        token: &PerlLspCancellationToken,
    ) -> Result<AstNode, CancellationError> {
        token.check_cancelled_or_continue()?;

        // Use incremental parser with cancellation
        let parser = IncrementalParserWithCancellation::new();
        parser.parse_with_cancellation(content, &[], Some(token.clone().into()))
    }
}

/// Multi-tier resolver with cancellation checkpoints
pub struct MultiTierResolver {
    /// Local file resolver (Tier 1)
    local_resolver: Arc<LocalFileResolver>,
    /// Workspace index resolver (Tier 2)
    workspace_resolver: Arc<WorkspaceIndexResolver>,
    /// Text search resolver (Tier 3)
    text_search_resolver: Arc<TextSearchResolver>,
    /// Resolver configuration
    config: ResolverConfig,
}

impl MultiTierResolver {
    /// Resolve definition using multi-tier approach with cancellation
    pub fn resolve_with_cancellation(
        &self,
        uri: &str,
        position: Position,
        token: Arc<PerlLspCancellationToken>,
    ) -> Result<DefinitionResult, CancellationError> {
        let mut result = DefinitionResult::empty();

        // Tier 1: Local file resolution (fastest)
        token.check_cancelled_or_continue()?;
        if let Some(local_result) = self.local_resolver
            .resolve_with_cancellation(uri, position, token.clone())? {
            result.merge(local_result);

            // Return early if we have high-confidence results
            if result.confidence >= self.config.early_return_confidence {
                return Ok(result);
            }
        }

        // Tier 2: Workspace index resolution (fast)
        token.check_cancelled_or_continue()?;
        if let Some(workspace_result) = self.workspace_resolver
            .resolve_with_cancellation(uri, position, token.clone())? {
            result.merge(workspace_result);

            // Return if we have sufficient results
            if result.locations.len() >= self.config.sufficient_result_count {
                return Ok(result);
            }
        }

        // Tier 3: Text search resolution (slower, only if needed)
        token.check_cancelled_or_continue()?;
        if result.locations.is_empty() || result.confidence < self.config.text_search_threshold {
            if let Some(text_search_result) = self.text_search_resolver
                .resolve_with_cancellation(uri, position, token.clone())? {
                result.merge(text_search_result);
            }
        }

        Ok(result)
    }
}

/// Workspace index resolver with dual pattern support
pub struct WorkspaceIndexResolver {
    /// Dual index reference
    dual_index: Arc<CancellableDualIndex>,
    /// Symbol extraction utilities
    symbol_extractor: Arc<SymbolExtractor>,
    /// Resolution confidence calculator
    confidence_calculator: Arc<ConfidenceCalculator>,
}

impl WorkspaceIndexResolver {
    /// Resolve using workspace index with dual pattern matching
    pub fn resolve_with_cancellation(
        &self,
        uri: &str,
        position: Position,
        token: Arc<PerlLspCancellationToken>,
    ) -> Result<Option<DefinitionResult>, CancellationError> {
        token.check_cancelled_or_continue()?;

        // Extract symbol from position
        let symbol_opt = self.symbol_extractor
            .extract_symbol_with_cancellation(uri, position, token.clone())?;

        if let Some(symbol_name) = symbol_opt {
            // Search using dual pattern strategy
            let locations = self.dual_index
                .find_references_dual_pattern(&symbol_name, &token)?;

            // Filter for definitions (not references)
            token.check_cancelled_or_continue()?;
            let definitions = self.filter_for_definitions(locations, &token)?;

            if !definitions.is_empty() {
                let confidence = self.confidence_calculator
                    .calculate_workspace_confidence(&symbol_name, &definitions);

                return Ok(Some(DefinitionResult {
                    locations: definitions,
                    confidence,
                    resolution_tier: ResolutionTier::WorkspaceIndex,
                }));
            }
        }

        Ok(None)
    }

    /// Filter locations to find actual definitions
    fn filter_for_definitions(
        &self,
        locations: Vec<Location>,
        token: &PerlLspCancellationToken,
    ) -> Result<Vec<Location>, CancellationError> {
        let mut definitions = Vec::new();

        for (i, location) in locations.iter().enumerate() {
            // Check cancellation every 20 locations
            if i % 20 == 0 {
                token.check_cancelled_or_continue()?;
            }

            if self.is_definition_location(location) {
                definitions.push(location.clone());
            }
        }

        Ok(definitions)
    }

    fn is_definition_location(&self, location: &Location) -> bool {
        // Implementation would check if location contains symbol definition
        // vs reference usage
        true // Placeholder
    }
}
```

## Cross-File Integration Schema

### AC3: Enhanced Cross-File Navigation with Cancellation

**Cross-File Resolution Architecture**:
```rust
/// Enhanced cross-file navigation with comprehensive cancellation support
pub struct CrossFileNavigationManager {
    /// File dependency graph for navigation optimization
    dependency_graph: Arc<RwLock<DependencyGraph>>,
    /// File content cache with cancellation awareness
    content_cache: Arc<CancellableFileCache>,
    /// Cross-file symbol tracker
    symbol_tracker: Arc<CrossFileSymbolTracker>,
    /// Navigation path optimizer
    path_optimizer: Arc<NavigationPathOptimizer>,
}

impl CrossFileNavigationManager {
    /// Navigate to definition across files with cancellation support
    pub fn navigate_cross_file_with_cancellation(
        &self,
        from_uri: &str,
        to_symbol: &str,
        token: Arc<PerlLspCancellationToken>,
    ) -> Result<CrossFileNavigationResult, CancellationError> {
        let start = std::time::Instant::now();

        // Phase 1: Analyze dependencies to optimize search order
        token.check_cancelled_or_continue()?;
        let search_order = self.dependency_graph
            .read()
            .unwrap()
            .get_optimal_search_order(from_uri, to_symbol);

        // Phase 2: Search files in optimized order
        let mut result = CrossFileNavigationResult::new();
        for (i, file_uri) in search_order.iter().enumerate() {
            // Check cancellation every 5 files
            if i % 5 == 0 {
                token.check_cancelled_or_continue()?;
            }

            if let Some(file_result) = self.search_file_with_cancellation(
                file_uri,
                to_symbol,
                token.clone(),
            )? {
                result.add_file_result(file_uri.clone(), file_result);

                // Early termination if we found high-confidence results
                if result.total_confidence() >= 0.9 {
                    break;
                }
            }
        }

        // Phase 3: Optimize navigation paths
        token.check_cancelled_or_continue()?;
        result = self.path_optimizer.optimize_navigation_result(result, &token)?;

        let duration = start.elapsed();
        self.record_cross_file_navigation_performance(from_uri, to_symbol, duration, &result);

        Ok(result)
    }

    /// Search individual file for symbol with cancellation
    fn search_file_with_cancellation(
        &self,
        file_uri: &str,
        symbol: &str,
        token: Arc<PerlLspCancellationToken>,
    ) -> Result<Option<FileSearchResult>, CancellationError> {
        token.check_cancelled_or_continue()?;

        // Get file content from cache or load
        let content = self.content_cache
            .get_content_with_cancellation(file_uri, token.clone())?;

        // Parse content with cancellation
        let ast = self.parse_content_with_strategic_cancellation(&content, token.clone())?;

        // Search for symbol in AST
        let search_result = self.search_symbol_in_ast(&ast, symbol, token.clone())?;

        Ok(search_result)
    }

    /// Parse content with strategic cancellation points
    fn parse_content_with_strategic_cancellation(
        &self,
        content: &str,
        token: Arc<PerlLspCancellationToken>,
    ) -> Result<AstNode, CancellationError> {
        let content_size = content.len();

        // Adjust parsing strategy based on content size
        let parsing_strategy = match content_size {
            0..=1024 => ParsingStrategy::Direct,           // Small files - parse directly
            1025..=10240 => ParsingStrategy::WithCheckpoints(2), // Medium files - 2 checkpoints
            10241..=102400 => ParsingStrategy::WithCheckpoints(5), // Large files - 5 checkpoints
            _ => ParsingStrategy::Incremental,             // Very large files - incremental
        };

        match parsing_strategy {
            ParsingStrategy::Direct => {
                token.check_cancelled_or_continue()?;
                self.parse_directly(content)
            },
            ParsingStrategy::WithCheckpoints(checkpoint_count) => {
                self.parse_with_checkpoints(content, checkpoint_count, token)
            },
            ParsingStrategy::Incremental => {
                self.parse_incrementally(content, token)
            },
        }
    }
}

/// Cancellable file cache for cross-file navigation
pub struct CancellableFileCache {
    /// File content cache
    content_cache: Arc<RwLock<HashMap<String, CachedFileContent>>>,
    /// Cache access tracking
    access_tracker: Arc<Mutex<HashMap<String, std::time::Instant>>>,
    /// Cache size limits
    size_limits: CacheSizeLimits,
}

impl CancellableFileCache {
    /// Get file content with cancellation support
    pub fn get_content_with_cancellation(
        &self,
        file_uri: &str,
        token: Arc<PerlLspCancellationToken>,
    ) -> Result<String, CancellationError> {
        token.check_cancelled_or_continue()?;

        // Check cache first
        if let Some(cached_content) = self.get_cached_content(file_uri) {
            if !cached_content.is_expired() {
                self.update_access_time(file_uri);
                return Ok(cached_content.content);
            }
        }

        // Load from filesystem with cancellation
        token.check_cancelled_or_continue()?;
        let content = self.load_file_content_with_cancellation(file_uri, token.clone())?;

        // Cache the content
        self.cache_content_with_cancellation(file_uri, content.clone(), token)?;

        Ok(content)
    }

    fn load_file_content_with_cancellation(
        &self,
        file_uri: &str,
        token: Arc<PerlLspCancellationToken>,
    ) -> Result<String, CancellationError> {
        token.check_cancelled_or_continue()?;

        // Implementation would load file content
        // with cancellation checks for large files
        Ok(String::new()) // Placeholder
    }

    fn cache_content_with_cancellation(
        &self,
        file_uri: &str,
        content: String,
        token: Arc<PerlLspCancellationToken>,
    ) -> Result<(), CancellationError> {
        token.check_cancelled_or_continue()?;

        let cached_content = CachedFileContent {
            content,
            cached_at: std::time::Instant::now(),
            access_count: 1,
        };

        // Check cache size limits before insertion
        self.enforce_cache_limits_with_cancellation(token.clone())?;

        self.content_cache
            .write()
            .unwrap()
            .insert(file_uri.to_string(), cached_content);

        Ok(())
    }

    fn enforce_cache_limits_with_cancellation(
        &self,
        token: Arc<PerlLspCancellationToken>,
    ) -> Result<(), CancellationError> {
        token.check_cancelled_or_continue()?;

        let mut cache = self.content_cache.write().unwrap();

        // Remove expired entries first
        cache.retain(|_, content| !content.is_expired());

        // If still over limit, remove least recently accessed
        if cache.len() > self.size_limits.max_entries {
            token.check_cancelled_or_continue()?;

            let access_tracker = self.access_tracker.lock().unwrap();
            let mut entries: Vec<_> = cache.iter().collect();

            entries.sort_by_key(|(uri, _)| {
                access_tracker.get(*uri).copied().unwrap_or(std::time::Instant::now())
            });

            let to_remove = cache.len() - self.size_limits.max_entries;
            for (uri, _) in entries.iter().take(to_remove) {
                cache.remove(*uri);
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
struct CachedFileContent {
    content: String,
    cached_at: std::time::Instant,
    access_count: usize,
}

impl CachedFileContent {
    fn is_expired(&self) -> bool {
        self.cached_at.elapsed() > std::time::Duration::from_secs(300) // 5 minutes
    }
}

#[derive(Debug)]
struct CacheSizeLimits {
    max_entries: usize,
    max_total_size: usize,
}

#[derive(Debug)]
enum ParsingStrategy {
    Direct,
    WithCheckpoints(usize),
    Incremental,
}
```

## Performance Integration Schema

### AC12: Performance Preservation with Integration

**Performance Monitoring for Integration Points**:
```rust
/// Comprehensive performance monitoring for dual indexing and navigation integration
pub struct IntegrationPerformanceMonitor {
    /// Dual pattern performance tracking
    dual_pattern_metrics: Arc<DualPatternMetrics>,
    /// Cross-file navigation performance
    navigation_metrics: Arc<NavigationMetrics>,
    /// Cache performance tracking
    cache_metrics: Arc<CacheMetrics>,
    /// Overall integration health
    integration_health: Arc<IntegrationHealth>,
}

impl IntegrationPerformanceMonitor {
    /// Monitor dual pattern indexing performance with cancellation
    pub fn monitor_dual_pattern_performance(
        &self,
        operation: DualPatternOperation,
        token: &PerlLspCancellationToken,
    ) -> PerformanceResult<()> {
        let start = std::time::Instant::now();

        let result = match operation {
            DualPatternOperation::Index { symbol, qualified, bare } => {
                self.monitor_indexing_performance(symbol, qualified, bare, token)
            },
            DualPatternOperation::Search { pattern, dual_mode } => {
                self.monitor_search_performance(pattern, dual_mode, token)
            },
            DualPatternOperation::CrossReference { from, to } => {
                self.monitor_cross_reference_performance(from, to, token)
            },
        };

        let duration = start.elapsed();

        // Validate performance requirements
        self.validate_performance_requirements(&operation, duration, token.is_cancelled().unwrap_or(false))?;

        result
    }

    /// Validate that integration maintains performance requirements
    fn validate_performance_requirements(
        &self,
        operation: &DualPatternOperation,
        duration: std::time::Duration,
        was_cancelled: bool,
    ) -> Result<(), PerformanceViolation> {
        let requirements = self.get_performance_requirements(operation);

        // Check duration requirements
        if duration > requirements.max_duration {
            return Err(PerformanceViolation::DurationExceeded {
                operation: format!("{:?}", operation),
                actual: duration,
                maximum: requirements.max_duration,
                was_cancelled,
            });
        }

        // Check memory requirements
        let current_memory = self.get_current_memory_usage();
        if current_memory > requirements.max_memory {
            return Err(PerformanceViolation::MemoryExceeded {
                operation: format!("{:?}", operation),
                actual: current_memory,
                maximum: requirements.max_memory,
            });
        }

        Ok(())
    }

    /// Get performance requirements for specific operations
    fn get_performance_requirements(&self, operation: &DualPatternOperation) -> PerformanceRequirements {
        match operation {
            DualPatternOperation::Index { .. } => PerformanceRequirements {
                max_duration: std::time::Duration::from_millis(10),  // 10ms for indexing
                max_memory: 1024 * 1024,  // 1MB additional memory
            },
            DualPatternOperation::Search { .. } => PerformanceRequirements {
                max_duration: std::time::Duration::from_millis(100), // 100ms for search
                max_memory: 512 * 1024,   // 512KB additional memory
            },
            DualPatternOperation::CrossReference { .. } => PerformanceRequirements {
                max_duration: std::time::Duration::from_millis(50),  // 50ms for cross-reference
                max_memory: 256 * 1024,   // 256KB additional memory
            },
        }
    }

    /// Generate integration performance report
    pub fn generate_integration_report(&self) -> IntegrationPerformanceReport {
        IntegrationPerformanceReport {
            dual_pattern_summary: self.dual_pattern_metrics.generate_summary(),
            navigation_summary: self.navigation_metrics.generate_summary(),
            cache_summary: self.cache_metrics.generate_summary(),
            integration_health_status: self.integration_health.get_status(),
            recommendations: self.generate_performance_recommendations(),
        }
    }
}

#[derive(Debug)]
pub enum DualPatternOperation {
    Index { symbol: String, qualified: bool, bare: bool },
    Search { pattern: String, dual_mode: bool },
    CrossReference { from: String, to: String },
}

#[derive(Debug)]
struct PerformanceRequirements {
    max_duration: std::time::Duration,
    max_memory: usize,
}

#[derive(Debug)]
enum PerformanceViolation {
    DurationExceeded {
        operation: String,
        actual: std::time::Duration,
        maximum: std::time::Duration,
        was_cancelled: bool,
    },
    MemoryExceeded {
        operation: String,
        actual: usize,
        maximum: usize,
    },
}

/// Dual pattern metrics tracking
pub struct DualPatternMetrics {
    /// Indexing performance metrics
    indexing_metrics: Arc<Mutex<Vec<IndexingMetric>>>,
    /// Search performance metrics
    search_metrics: Arc<Mutex<Vec<SearchMetric>>>,
    /// Cross-reference performance metrics
    cross_ref_metrics: Arc<Mutex<Vec<CrossRefMetric>>>,
}

impl DualPatternMetrics {
    /// Record indexing performance
    pub fn record_indexing_performance(
        &self,
        symbol: &str,
        qualified: bool,
        bare: bool,
        duration: std::time::Duration,
        was_cancelled: bool,
    ) {
        let metric = IndexingMetric {
            symbol: symbol.to_string(),
            qualified_indexed: qualified,
            bare_indexed: bare,
            duration,
            was_cancelled,
            timestamp: std::time::Instant::now(),
        };

        self.indexing_metrics.lock().unwrap().push(metric);
    }

    /// Record search performance
    pub fn record_search_performance(
        &self,
        pattern: &str,
        duration: std::time::Duration,
        result_count: usize,
        was_cancelled: bool,
    ) {
        let metric = SearchMetric {
            pattern: pattern.to_string(),
            duration,
            result_count,
            was_cancelled,
            timestamp: std::time::Instant::now(),
        };

        self.search_metrics.lock().unwrap().push(metric);
    }

    /// Generate performance summary
    pub fn generate_summary(&self) -> DualPatternSummary {
        let indexing_metrics = self.indexing_metrics.lock().unwrap();
        let search_metrics = self.search_metrics.lock().unwrap();

        let average_indexing_duration = if !indexing_metrics.is_empty() {
            indexing_metrics.iter().map(|m| m.duration).sum::<std::time::Duration>() / indexing_metrics.len() as u32
        } else {
            std::time::Duration::from_nanos(0)
        };

        let average_search_duration = if !search_metrics.is_empty() {
            search_metrics.iter().map(|m| m.duration).sum::<std::time::Duration>() / search_metrics.len() as u32
        } else {
            std::time::Duration::from_nanos(0)
        };

        DualPatternSummary {
            total_indexing_operations: indexing_metrics.len(),
            total_search_operations: search_metrics.len(),
            average_indexing_duration,
            average_search_duration,
            cancellation_rate: self.calculate_cancellation_rate(),
            dual_pattern_efficiency: self.calculate_dual_pattern_efficiency(),
        }
    }

    fn calculate_cancellation_rate(&self) -> f64 {
        let indexing_metrics = self.indexing_metrics.lock().unwrap();
        let search_metrics = self.search_metrics.lock().unwrap();

        let total_operations = indexing_metrics.len() + search_metrics.len();
        if total_operations == 0 {
            return 0.0;
        }

        let cancelled_operations = indexing_metrics.iter().filter(|m| m.was_cancelled).count()
            + search_metrics.iter().filter(|m| m.was_cancelled).count();

        cancelled_operations as f64 / total_operations as f64
    }

    fn calculate_dual_pattern_efficiency(&self) -> f64 {
        // Calculate efficiency of dual pattern strategy
        // Implementation would analyze hit rates for qualified vs bare searches
        0.98 // Placeholder: 98% efficiency
    }
}

#[derive(Debug)]
struct IndexingMetric {
    symbol: String,
    qualified_indexed: bool,
    bare_indexed: bool,
    duration: std::time::Duration,
    was_cancelled: bool,
    timestamp: std::time::Instant,
}

#[derive(Debug)]
struct SearchMetric {
    pattern: String,
    duration: std::time::Duration,
    result_count: usize,
    was_cancelled: bool,
    timestamp: std::time::Instant,
}

#[derive(Debug)]
pub struct DualPatternSummary {
    pub total_indexing_operations: usize,
    pub total_search_operations: usize,
    pub average_indexing_duration: std::time::Duration,
    pub average_search_duration: std::time::Duration,
    pub cancellation_rate: f64,
    pub dual_pattern_efficiency: f64,
}
```

## Testing Integration Schema

### Comprehensive Integration Testing Architecture

**Integration Test Framework for Dual Indexing with Cancellation**:
```rust
/// Comprehensive integration testing framework for dual indexing with cancellation
pub struct DualIndexingIntegrationTestFramework {
    /// Test workspace with dual pattern scenarios
    test_workspace: TestWorkspace,
    /// Dual indexing test harness
    indexing_harness: DualIndexingTestHarness,
    /// Navigation test harness
    navigation_harness: NavigationTestHarness,
    /// Performance validation framework
    performance_validator: PerformanceValidator,
}

impl DualIndexingIntegrationTestFramework {
    /// Test dual pattern indexing with cancellation scenarios
    pub fn test_dual_pattern_indexing_cancellation(&self) -> IntegrationTestResult {
        let mut results = Vec::new();

        // Test 1: Qualified name indexing with cancellation
        let qualified_result = self.test_qualified_indexing_cancellation();
        results.push(("qualified_indexing", qualified_result));

        // Test 2: Bare name indexing with cancellation
        let bare_result = self.test_bare_indexing_cancellation();
        results.push(("bare_indexing", bare_result));

        // Test 3: Dual pattern search with cancellation
        let search_result = self.test_dual_pattern_search_cancellation();
        results.push(("dual_search", search_result));

        // Test 4: Cross-reference consistency during cancellation
        let consistency_result = self.test_cross_reference_consistency();
        results.push(("consistency", consistency_result));

        IntegrationTestResult::aggregate(results)
    }

    fn test_qualified_indexing_cancellation(&self) -> TestResult {
        // Create test workspace with Package::function patterns
        let workspace = self.test_workspace.create_qualified_function_workspace();

        let indexing_token = self.create_test_cancellation_token("qualified_indexing");

        // Start indexing operation
        let indexing_handle = self.indexing_harness
            .start_qualified_indexing(&workspace, indexing_token.clone());

        // Cancel at various stages
        let cancellation_stages = vec![
            CancellationStage::EarlyStage,    // Cancel during initial parsing
            CancellationStage::MidStage,      // Cancel during symbol extraction
            CancellationStage::LateStage,     // Cancel during index insertion
        ];

        let mut stage_results = Vec::new();
        for stage in cancellation_stages {
            let stage_result = self.test_cancellation_at_stage(&indexing_handle, stage);
            stage_results.push(stage_result);
        }

        // Verify index integrity after cancellation
        let integrity_result = self.verify_index_integrity_after_cancellation(&workspace);
        stage_results.push(integrity_result);

        TestResult::aggregate_stage_results(stage_results)
    }

    fn test_dual_pattern_search_cancellation(&self) -> TestResult {
        // Create workspace with both qualified and bare function references
        let workspace = self.test_workspace.create_mixed_pattern_workspace();

        let dual_index = self.indexing_harness.create_dual_index(&workspace);

        // Test search cancellation scenarios
        let search_scenarios = vec![
            SearchScenario::QualifiedToBareCancellation,
            SearchScenario::BareToQualifiedCancellation,
            SearchScenario::CrossReferenceCancellation,
        ];

        let mut scenario_results = Vec::new();
        for scenario in search_scenarios {
            let token = self.create_test_cancellation_token(&format!("search_{:?}", scenario));
            let result = self.execute_search_cancellation_scenario(&dual_index, scenario, token);
            scenario_results.push(result);
        }

        TestResult::aggregate_scenario_results(scenario_results)
    }

    /// Test cross-file navigation with cancellation integration
    pub fn test_cross_file_navigation_cancellation(&self) -> IntegrationTestResult {
        // Create multi-file workspace
        let workspace = self.test_workspace.create_multi_file_workspace(50); // 50 files

        let navigation_manager = self.navigation_harness
            .create_cross_file_navigation_manager(&workspace);

        // Test navigation scenarios with cancellation
        let navigation_scenarios = vec![
            NavigationScenario::LocalToRemoteDefinition,
            NavigationScenario::MultiTierResolution,
            NavigationScenario::DependencyChainNavigation,
        ];

        let mut results = Vec::new();
        for scenario in navigation_scenarios {
            let token = self.create_test_cancellation_token(&format!("nav_{:?}", scenario));
            let result = self.execute_navigation_cancellation_scenario(
                &navigation_manager,
                scenario,
                token,
            );
            results.push((format!("{:?}", scenario), result));
        }

        IntegrationTestResult::aggregate(results)
    }

    /// Performance integration testing
    pub fn test_performance_integration_with_cancellation(&self) -> PerformanceIntegrationTestResult {
        let performance_scenarios = vec![
            PerformanceScenario::DualPatternIndexingOverhead,
            PerformanceScenario::SearchLatencyWithCancellation,
            PerformanceScenario::CrossFileNavigationPerformance,
            PerformanceScenario::MemoryUsageDuringCancellation,
        ];

        let mut performance_results = Vec::new();
        for scenario in performance_scenarios {
            let result = self.performance_validator.test_performance_scenario(scenario);
            performance_results.push(result);
        }

        PerformanceIntegrationTestResult::from_scenarios(performance_results)
    }

    /// Create test cancellation token with specific context
    fn create_test_cancellation_token(&self, context: &str) -> Arc<PerlLspCancellationToken> {
        Arc::new(PerlLspCancellationToken::new(
            json!(context),
            ProviderCleanupContext::Generic,
            Some(std::time::Duration::from_millis(100)), // Test threshold
        ))
    }
}

#[derive(Debug)]
enum CancellationStage {
    EarlyStage,  // 0-25% completion
    MidStage,    // 25-75% completion
    LateStage,   // 75-95% completion
}

#[derive(Debug)]
enum SearchScenario {
    QualifiedToBareCancellation,
    BareToQualifiedCancellation,
    CrossReferenceCancellation,
}

#[derive(Debug)]
enum NavigationScenario {
    LocalToRemoteDefinition,
    MultiTierResolution,
    DependencyChainNavigation,
}

#[derive(Debug)]
enum PerformanceScenario {
    DualPatternIndexingOverhead,
    SearchLatencyWithCancellation,
    CrossFileNavigationPerformance,
    MemoryUsageDuringCancellation,
}
```

## Conclusion

This LSP Cancellation Integration Schema provides comprehensive architectural guidance for integrating enhanced cancellation capabilities with the Perl LSP dual indexing strategy and workspace navigation systems. The schema addresses all finalized Issue #48 requirements while maintaining:

**Core Integration Achievements**:
- **Dual Indexing Preservation**: Enhanced dual pattern storage (qualified/bare) with atomic cancellation support maintaining 98% reference resolution success rate
- **Cross-File Navigation Excellence**: Multi-tier resolution architecture with strategic cancellation checkpoints across local → workspace → text search tiers
- **Performance Integration**: <1ms incremental parsing preservation, <100μs cancellation check latency, and comprehensive performance monitoring
- **Cache Integration**: Cancellation-aware file content caching with intelligent cache management and size limits
- **Thread Safety**: Complete integration with adaptive threading (RUST_TEST_THREADS=2) including contention detection and optimization

**Technical Architecture Highlights**:
- **CancellableDualIndex**: Enhanced dual pattern storage with atomic operations and cancellation coordination
- **MultiTierResolver**: Comprehensive definition resolution with cancellation checkpoints at each tier
- **CrossFileNavigationManager**: Intelligent cross-file navigation with dependency optimization and strategic parsing
- **IntegrationPerformanceMonitor**: Real-time performance tracking for all integration points with AC12 compliance validation
- **DualIndexingIntegrationTestFramework**: Comprehensive test suite validating cancellation integration across all scenarios

The integration schema ensures that enhanced cancellation capabilities seamlessly integrate with existing Perl LSP infrastructure while preserving ~100% Perl syntax coverage, enterprise security standards, and production-grade performance characteristics. All integration points are designed with comprehensive error handling, graceful degradation, and thorough performance validation to maintain the reliability and efficiency of the Perl Language Server ecosystem.
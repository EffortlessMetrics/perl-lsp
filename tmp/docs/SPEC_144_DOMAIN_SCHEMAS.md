# SPEC-144: Domain Schemas for Ignored Test Resolution

## Overview

This document defines comprehensive domain schemas for the systematic ignored test resolution implementation, providing concrete data structures and API contracts for the Perl LSP ecosystem.

## 1. Test Categorization Schema

### 1.1 Core Test Metadata Structure

```rust
/// Comprehensive test metadata for ignored test management
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IgnoredTestMetadata {
    /// Unique test identifier
    pub test_id: String,
    /// File path relative to crate root
    pub file_path: PathBuf,
    /// Test function name
    pub test_name: String,
    /// Test category classification
    pub category: TestCategory,
    /// Implementation priority (1=highest, 4=lowest)
    pub priority: u8,
    /// Current ignore reason
    pub ignore_reason: String,
    /// Estimated implementation complexity
    pub complexity: ComplexityLevel,
    /// Target implementation timeline
    pub target_timeline: Duration,
    /// Dependencies on other tests or features
    pub dependencies: Vec<String>,
    /// Success criteria for re-enabling
    pub success_criteria: Vec<String>,
    /// LSP workflow stage integration
    pub workflow_integration: LspWorkflowStage,
    /// Performance requirements if applicable
    pub performance_requirements: Option<PerformanceRequirements>,
    /// Last assessment date
    pub last_assessed: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TestCategory {
    /// Critical LSP protocol compliance and core functionality
    CriticalLsp {
        subcategory: CriticalLspSubcategory,
        lsp_feature: LspFeature,
    },
    /// Infrastructure stability and performance validation
    Infrastructure {
        subcategory: InfrastructureSubcategory,
        component: InfrastructureComponent,
    },
    /// Modern Perl syntax feature support
    AdvancedSyntax {
        subcategory: AdvancedSyntaxSubcategory,
        perl_feature: PerlFeature,
    },
    /// Edge case handling and specialized scenarios
    EdgeCases {
        subcategory: EdgeCaseSubcategory,
        scenario_type: ScenarioType,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ComplexityLevel {
    /// Simple implementation, minimal dependencies
    Low,
    /// Moderate complexity, some integration required
    Medium,
    /// High complexity, significant architectural changes
    High,
    /// Very high complexity, major system modifications
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LspWorkflowStage {
    Parse,
    Index,
    Navigate,
    Complete,
    Analyze,
    /// Cross-cutting concerns affecting multiple stages
    CrossCutting,
}
```

### 1.2 Category-Specific Schemas

```rust
/// Critical LSP category subdivisions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CriticalLspSubcategory {
    /// Malformed frame handling and recovery
    FrameProcessing,
    /// Error response accuracy and context
    ErrorHandling,
    /// Concurrent request management
    ConcurrentRequests,
    /// Diagnostic publication system
    DiagnosticPublishing,
    /// Protocol compliance validation
    ProtocolCompliance,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LspFeature {
    Initialize,
    TextDocumentSync,
    Completion,
    Hover,
    SignatureHelp,
    GotoDefinition,
    FindReferences,
    DocumentSymbol,
    WorkspaceSymbol,
    CodeAction,
    Formatting,
    DiagnosticPublication,
    ExecuteCommand,
    Cancellation,
}

/// Infrastructure category subdivisions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InfrastructureSubcategory {
    /// Performance benchmarking and baselines
    PerformanceTesting,
    /// Threading and concurrency validation
    ThreadingSafety,
    /// Memory usage and leak prevention
    MemoryManagement,
    /// CI/CD pipeline integration
    ContinuousIntegration,
    /// Monitoring and observability
    Monitoring,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InfrastructureComponent {
    LspServer,
    Parser,
    Indexer,
    ThreadingSystem,
    CancellationSystem,
    DiagnosticSystem,
    PerformanceMonitor,
}

/// Advanced syntax category subdivisions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AdvancedSyntaxSubcategory {
    /// Modern subroutine signatures
    SubroutineSignatures,
    /// Postfix dereferencing operators
    PostfixDereferencing,
    /// State variable declarations
    StateVariables,
    /// Unicode identifier support
    UnicodeIdentifiers,
    /// Complex operator parsing
    AdvancedOperators,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PerlFeature {
    /// Perl 5.20+ subroutine signatures
    SubroutineSignatures,
    /// Perl 5.20+ postfix dereferencing
    PostfixDereferencing,
    /// Perl 5.10+ state variables
    StateVariables,
    /// Unicode identifier support
    UnicodeIdentifiers,
    /// Complex heredoc scenarios
    ComplexHeredocs,
    /// Advanced regex features
    AdvancedRegex,
}

/// Edge case category subdivisions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EdgeCaseSubcategory {
    /// Complex heredoc parsing scenarios
    HeredocEdgeCases,
    /// Unicode and emoji edge cases
    UnicodeEdgeCases,
    /// Formatting integration edge cases
    FormattingEdgeCases,
    /// Parser recovery scenarios
    ParserRecovery,
    /// Performance edge cases
    PerformanceEdgeCases,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ScenarioType {
    /// Malformed input handling
    MalformedInput,
    /// Resource exhaustion scenarios
    ResourceExhaustion,
    /// Concurrent access edge cases
    ConcurrentAccess,
    /// Security validation scenarios
    SecurityValidation,
    /// Integration boundary conditions
    IntegrationBoundaries,
}
```

## 2. Performance Baseline Schema

### 2.1 Performance Measurement Framework

```rust
/// Comprehensive performance baseline definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceBaseline {
    /// Unique baseline identifier
    pub baseline_id: String,
    /// LSP operation being measured
    pub operation: LspOperation,
    /// Target performance criteria
    pub target: PerformanceTarget,
    /// Measurement methodology
    pub measurement_method: MeasurementMethod,
    /// Test conditions and environment
    pub test_conditions: TestConditions,
    /// Regression detection thresholds
    pub regression_thresholds: RegressionThresholds,
    /// Historical measurements
    pub measurements: Vec<PerformanceMeasurement>,
    /// Baseline establishment date
    pub established: DateTime<Utc>,
    /// Last validation date
    pub last_validated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LspOperation {
    /// Server initialization sequence
    Initialization {
        workspace_size: usize,
        file_count: usize,
    },
    /// Document parsing and analysis
    Parsing {
        file_size_kb: usize,
        syntax_complexity: SyntaxComplexity,
    },
    /// Diagnostic computation and publication
    DiagnosticPublication {
        diagnostic_count: usize,
        file_count: usize,
    },
    /// Symbol resolution across files
    SymbolResolution {
        workspace_size: usize,
        symbol_depth: usize,
    },
    /// Code completion computation
    Completion {
        context_size: usize,
        completion_items: usize,
    },
    /// Document formatting
    Formatting {
        file_size_kb: usize,
        formatting_complexity: FormattingComplexity,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceTarget {
    /// Target latency (e.g., "<1000ms", "<500μs/KB")
    pub latency: String,
    /// Target throughput if applicable
    pub throughput: Option<String>,
    /// Memory usage target
    pub memory_usage: Option<String>,
    /// CPU usage target
    pub cpu_usage: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConditions {
    /// Hardware specifications
    pub hardware: HardwareSpec,
    /// Software environment
    pub environment: EnvironmentSpec,
    /// Workload characteristics
    pub workload: WorkloadSpec,
    /// Concurrency settings
    pub concurrency: ConcurrencySpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareSpec {
    /// CPU cores available
    pub cpu_cores: usize,
    /// Memory limit in MB
    pub memory_limit_mb: usize,
    /// Storage type (SSD/HDD)
    pub storage_type: StorageType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMeasurement {
    /// Measurement timestamp
    pub timestamp: DateTime<Utc>,
    /// Measured latency in microseconds
    pub latency_us: u64,
    /// Memory usage in bytes
    pub memory_usage_bytes: u64,
    /// CPU usage percentage
    pub cpu_usage_percent: f64,
    /// Measurement environment
    pub environment: EnvironmentSpec,
    /// Additional metrics
    pub additional_metrics: HashMap<String, f64>,
}
```

### 2.2 Regression Detection Schema

```rust
/// Regression detection and alerting system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionThresholds {
    /// Latency increase threshold percentage
    pub latency_threshold_percent: f64,
    /// Memory usage increase threshold percentage
    pub memory_threshold_percent: f64,
    /// CPU usage increase threshold percentage
    pub cpu_threshold_percent: f64,
    /// Statistical significance requirements
    pub statistical_significance: StatisticalRequirements,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatisticalRequirements {
    /// Minimum sample size for regression detection
    pub min_sample_size: usize,
    /// Confidence level (e.g., 0.95 for 95%)
    pub confidence_level: f64,
    /// Number of consecutive measurements above threshold
    pub consecutive_threshold_breaches: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionAlert {
    /// Alert identifier
    pub alert_id: String,
    /// Affected operation
    pub operation: LspOperation,
    /// Regression type
    pub regression_type: RegressionType,
    /// Severity level
    pub severity: AlertSeverity,
    /// Detection timestamp
    pub detected_at: DateTime<Utc>,
    /// Current vs baseline comparison
    pub comparison: PerformanceComparison,
    /// Recommended actions
    pub recommended_actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RegressionType {
    Latency,
    Memory,
    CpuUsage,
    Throughput,
    Combined,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
    Emergency,
}
```

## 3. LSP Enhancement Schema

### 3.1 Enhanced Error Handling Schema

```rust
/// Enhanced LSP error response with comprehensive context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedLspError {
    /// Standard LSP error code
    pub code: i32,
    /// Human-readable error message
    pub message: String,
    /// Request correlation information
    pub request_info: RequestInfo,
    /// Perl-specific error context
    pub perl_context: Option<PerlErrorContext>,
    /// Error severity and category
    pub error_metadata: ErrorMetadata,
    /// Recovery suggestions
    pub recovery_suggestions: Vec<RecoverySuggestion>,
    /// Diagnostic correlation
    pub diagnostic_correlation: Option<DiagnosticCorrelation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestInfo {
    /// Original request ID
    pub request_id: RequestId,
    /// Request method
    pub method: String,
    /// Request timestamp
    pub timestamp: DateTime<Utc>,
    /// Request processing duration
    pub processing_duration: Duration,
    /// LSP workflow stage where error occurred
    pub workflow_stage: LspWorkflowStage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerlErrorContext {
    /// Source file location if applicable
    pub source_location: Option<Range>,
    /// Perl syntax context
    pub syntax_context: Option<SyntaxContext>,
    /// Parser state information
    pub parser_state: Option<ParserState>,
    /// Symbol resolution context
    pub symbol_context: Option<SymbolContext>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMetadata {
    /// Error category for metrics and handling
    pub category: ErrorCategory,
    /// Error severity level
    pub severity: ErrorSeverity,
    /// Whether error is recoverable
    pub recoverable: bool,
    /// Error frequency classification
    pub frequency: ErrorFrequency,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ErrorCategory {
    /// JSON-RPC protocol errors
    Protocol,
    /// Parsing and syntax errors
    Parsing,
    /// Symbol resolution errors
    SymbolResolution,
    /// File system and I/O errors
    FileSystem,
    /// Performance and timeout errors
    Performance,
    /// Security and validation errors
    Security,
    /// Internal system errors
    Internal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ErrorSeverity {
    /// Informational, no action required
    Info,
    /// Warning, degraded functionality
    Warning,
    /// Error, feature unavailable
    Error,
    /// Critical, system instability
    Critical,
    /// Fatal, service unavailable
    Fatal,
}
```

### 3.2 Request Correlation Schema

```rust
/// Comprehensive request correlation and tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestCorrelationContext {
    /// Request correlation ID
    pub correlation_id: String,
    /// Request lifecycle tracking
    pub lifecycle: RequestLifecycle,
    /// Resource usage tracking
    pub resource_usage: ResourceUsageTracking,
    /// Cancellation support
    pub cancellation: CancellationContext,
    /// Performance metrics
    pub performance_metrics: RequestPerformanceMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestLifecycle {
    /// Request received timestamp
    pub received_at: DateTime<Utc>,
    /// Processing started timestamp
    pub processing_started_at: Option<DateTime<Utc>>,
    /// Processing completed timestamp
    pub processing_completed_at: Option<DateTime<Utc>>,
    /// Response sent timestamp
    pub response_sent_at: Option<DateTime<Utc>>,
    /// Current request state
    pub state: RequestState,
    /// Request timeout configuration
    pub timeout: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RequestState {
    /// Request received, pending processing
    Pending,
    /// Request being processed
    Processing,
    /// Request processing completed
    Completed,
    /// Request failed with error
    Failed,
    /// Request was cancelled
    Cancelled,
    /// Request timed out
    TimedOut,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancellationContext {
    /// Cancellation token if applicable
    pub token: Option<String>,
    /// Cancellation requested timestamp
    pub cancellation_requested_at: Option<DateTime<Utc>>,
    /// Cancellation acknowledged timestamp
    pub cancellation_acknowledged_at: Option<DateTime<Utc>>,
    /// Cancellation reason
    pub cancellation_reason: Option<CancellationReason>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CancellationReason {
    /// Client requested cancellation
    ClientRequested,
    /// Server initiated cancellation (timeout, resource limits)
    ServerInitiated,
    /// Cancellation due to newer request superseding
    Superseded,
    /// Cancellation due to system shutdown
    SystemShutdown,
}
```

## 4. Parser Integration Schema

### 4.1 Modern Perl Syntax Support Schema

```rust
/// Comprehensive modern Perl syntax support definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModernPerlSyntaxSupport {
    /// Subroutine signature support
    pub subroutine_signatures: SubroutineSignatureSupport,
    /// Postfix dereferencing support
    pub postfix_dereferencing: PostfixDereferencingSupport,
    /// State variable support
    pub state_variables: StateVariableSupport,
    /// Unicode identifier support
    pub unicode_identifiers: UnicodeIdentifierSupport,
    /// Advanced operator support
    pub advanced_operators: AdvancedOperatorSupport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubroutineSignatureSupport {
    /// Basic parameter parsing
    pub basic_parameters: bool,
    /// Default value support
    pub default_values: bool,
    /// Slurpy parameters (@, %)
    pub slurpy_parameters: bool,
    /// Named parameters (:$name)
    pub named_parameters: bool,
    /// Type annotations
    pub type_annotations: bool,
    /// Constraint validation
    pub constraint_validation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostfixDereferencingSupport {
    /// Array postfix deref (->@*)
    pub array_deref: bool,
    /// Hash postfix deref (->%*)
    pub hash_deref: bool,
    /// Scalar postfix deref (->$*)
    pub scalar_deref: bool,
    /// Code postfix deref (->&*)
    pub code_deref: bool,
    /// Chained dereferencing
    pub chained_deref: bool,
    /// Precedence handling
    pub precedence_support: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateVariableSupport {
    /// Basic state declaration
    pub basic_state: bool,
    /// State with initialization
    pub state_initialization: bool,
    /// State in subroutine context
    pub subroutine_state: bool,
    /// State scope tracking
    pub scope_tracking: bool,
    /// State persistence validation
    pub persistence_validation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnicodeIdentifierSupport {
    /// Basic Unicode identifier parsing
    pub basic_unicode: bool,
    /// Emoji identifier support
    pub emoji_identifiers: bool,
    /// Unicode normalization
    pub normalization: bool,
    /// Security validation
    pub security_validation: bool,
    /// Mixed encoding support
    pub mixed_encoding: bool,
}
```

### 4.2 Parser Enhancement API Schema

```rust
/// Parser enhancement API contracts
pub trait ModernPerlParser {
    /// Parse subroutine signature
    fn parse_subroutine_signature(&mut self, input: &str) -> ParseResult<SubroutineSignature>;

    /// Parse postfix dereferencing expression
    fn parse_postfix_deref(&mut self, input: &str) -> ParseResult<PostfixDeref>;

    /// Parse state variable declaration
    fn parse_state_variable(&mut self, input: &str) -> ParseResult<StateVariable>;

    /// Parse Unicode identifier
    fn parse_unicode_identifier(&mut self, input: &str) -> ParseResult<UnicodeIdentifier>;

    /// Validate modern syntax support
    fn validate_modern_syntax(&self, feature: ModernPerlFeature) -> ValidationResult;
}

/// Parser integration with LSP workflow
pub trait LspParserIntegration {
    /// Parse with LSP workflow context
    fn parse_with_context(&mut self, input: &str, context: LspContext) -> ParseResult<SyntaxTree>;

    /// Incremental parsing with change tracking
    fn parse_incremental(&mut self, changes: &[TextChange], tree: &SyntaxTree) -> ParseResult<SyntaxTree>;

    /// Error recovery with LSP diagnostics
    fn parse_with_recovery(&mut self, input: &str) -> (ParseResult<SyntaxTree>, Vec<Diagnostic>);

    /// Performance-aware parsing with limits
    fn parse_with_limits(&mut self, input: &str, limits: &PerformanceLimits) -> ParseResult<SyntaxTree>;
}
```

## 5. CI Guardrail Schema

### 5.1 Ignored Test Monitoring Schema

```rust
/// Comprehensive ignored test monitoring and governance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IgnoredTestGovernance {
    /// Current ignored test inventory
    pub inventory: IgnoredTestInventory,
    /// Baseline tracking and limits
    pub baseline_management: BaselineManagement,
    /// Quality gates and validation
    pub quality_gates: QualityGates,
    /// Reporting and alerting
    pub reporting: ReportingConfiguration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IgnoredTestInventory {
    /// Total ignored test count
    pub total_count: usize,
    /// Count by category
    pub by_category: HashMap<TestCategory, usize>,
    /// Count by crate
    pub by_crate: HashMap<String, usize>,
    /// Count by priority
    pub by_priority: HashMap<u8, usize>,
    /// Last updated timestamp
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineManagement {
    /// Established baseline count
    pub baseline_count: usize,
    /// Maximum allowed deviation
    pub max_deviation: usize,
    /// Deviation percentage threshold
    pub deviation_threshold_percent: f64,
    /// Baseline establishment date
    pub baseline_date: DateTime<Utc>,
    /// Next baseline review date
    pub next_review_date: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityGates {
    /// Pre-commit validation rules
    pub pre_commit: PreCommitValidation,
    /// CI pipeline validation
    pub ci_validation: CiValidation,
    /// Quality metrics tracking
    pub metrics_tracking: MetricsTracking,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreCommitValidation {
    /// Require justification for new ignored tests
    pub require_justification: bool,
    /// Maximum allowed new ignored tests per commit
    pub max_new_ignored_per_commit: usize,
    /// Required documentation for ignored tests
    pub documentation_requirements: DocumentationRequirements,
}
```

## 6. Success Validation Schema

### 6.1 Implementation Success Criteria

```yaml
# Success validation schema for ignored test resolution
success_criteria:
  quantitative_metrics:
    ignored_test_reduction:
      target: "49 → ≤25 tests"
      minimum_reduction_percent: 49
      measurement_method: "automated_test_count"

    feature_validation_coverage:
      target: "95% of claimed ~91% LSP functionality"
      measurement_method: "feature_matrix_validation"
      validation_framework: "comprehensive_e2e_testing"

    performance_baseline_achievement:
      target: "100% baseline establishment"
      measurement_method: "automated_performance_testing"
      regression_detection: "enabled"

    ci_stability:
      target: "100% test pass rate maintenance"
      measurement_method: "ci_pipeline_monitoring"
      stability_window: "30_days"

    quality_assurance:
      target: "Zero regression introduction"
      measurement_method: "quality_gate_validation"
      regression_prevention: "mandatory"

  qualitative_metrics:
    lsp_protocol_compliance:
      target: "Full LSP 3.17 specification adherence"
      validation_method: "protocol_compliance_testing"

    error_handling_robustness:
      target: "Graceful degradation for all error scenarios"
      validation_method: "fault_injection_testing"

    modern_perl_support:
      target: "Comprehensive Perl 5.20+ feature coverage"
      validation_method: "syntax_feature_matrix"

    developer_experience:
      target: "Enhanced debugging and error reporting"
      validation_method: "user_acceptance_testing"

    maintenance_burden:
      target: "Simplified ignored test management"
      validation_method: "operational_complexity_assessment"

  phase_specific_criteria:
    phase_1_critical_lsp:
      target_reduction: "24 → 5 tests (79% reduction)"
      success_criteria:
        - "LSP 3.17 compliance validation"
        - "Zero race conditions in concurrent tests"
        - "Graceful degradation for malformed frames"
        - "Enhanced error context availability"

    phase_2_infrastructure:
      target_reduction: "5 → 2 tests (60% reduction)"
      success_criteria:
        - "All performance benchmarks passing"
        - "100% test pass rate under load"
        - "Zero memory leaks in stress tests"
        - "CI guardrail system operational"

    phase_3_advanced_syntax:
      target_reduction: "20 → 12 tests (40% reduction)"
      success_criteria:
        - "Complete subroutine signature syntax coverage"
        - "All postfix dereferencing operator combinations"
        - "State variable scope lifecycle validation"
        - "Unicode security validation passing"

    phase_4_edge_cases:
      target_reduction: "4 → 3 tests (25% reduction)"
      success_criteria:
        - "Complex heredoc context parsing"
        - "Unicode edge case security validation"
        - "Formatting integration graceful fallback"
        - "Comprehensive monitoring system operational"
```

---

**Document Status**: Complete
**Dependencies**: SPEC_144_IGNORED_TESTS_ARCHITECTURAL_BLUEPRINT.md
**Validation Required**: Schema consistency, API contract completeness, performance requirement feasibility
**Next Steps**: Technical validation and implementation planning
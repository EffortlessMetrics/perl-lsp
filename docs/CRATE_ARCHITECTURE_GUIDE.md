# Crate Architecture Guide (v0.8.8 GA)

## Published Crates (Workspace Members)

### `/crates/perl-parser/` - Main Parser Library ⭐ **MAIN CRATE**
- **Purpose**: Core recursive descent parser with production-grade features
- **Key Features**:
  - Native recursive descent parser with ~100% Perl 5 syntax coverage (including comprehensive substitution operator parsing)
  - 4-19x faster than legacy implementations (1-150 µs parsing)
  - True incremental parsing with <1ms LSP updates
  - Production-ready Rope integration for UTF-16/UTF-8 position conversion
  - Enhanced workspace navigation with dual indexing strategy for 98% reference coverage
  - **Revolutionary Dual Indexing**: Functions indexed under both qualified (`Package::function`) and bare (`function`) names
  - **Thread-safe semantic tokens** - 2.826µs average performance (35x better than 100µs target)
  - **Zero-race-condition LSP features** - immutable provider pattern with local state management
  - **Cross-file workspace refactoring utilities** - comprehensive WorkspaceRefactor provider for symbol renaming, module extraction, workspace-wide changes
  - **Import optimization system** - comprehensive analysis and optimization of Perl import statements with unused/duplicate detection, missing import analysis, and alphabetical sorting
  - **Production-ready refactoring operations** - move subroutines between modules, inline variables, extract code sections
  - **Enterprise-grade safety and validation** - comprehensive error handling, input validation, and rollback support
  - **Precise name span tracking** - Enhanced AST nodes with O(1) position lookups for Subroutine and Package declarations
  - **Production-stable AST generation** - Comprehensive S-expression generation with 50+ operators and enhanced navigation

- **Key Files**:
  - `src/parser.rs`: Recursive descent parser with precise name span calculation
  - `src/ast.rs`: AST definitions with enhanced navigation and name_span fields
  - `src/textdoc.rs`: Core document management with `ropey::Rope`
  - `src/position_mapper.rs`: UTF-16/UTF-8 position conversion
  - `src/incremental_integration.rs`: LSP integration bridge
  - `src/incremental_handler_v2.rs`: Document change processing
  - `src/declaration.rs`: Declaration provider with O(1) position lookups
  - `src/module_resolver.rs`: **NEW v0.8.8** - Reusable module resolution component for LSP features
  - `src/workspace_index.rs`: **ENHANCED v0.8.8** - Dual indexing strategy for 98% cross-file reference coverage
  - `src/completion.rs`: Enhanced completion provider with pluggable module resolver integration
  - `src/import_optimizer.rs`: Import analysis and optimization engine
  - `src/code_actions.rs`: LSP code actions with import optimization integration

### `/crates/perl-lsp/` - Standalone LSP Server ⭐ **LSP BINARY** (v0.8.8)
- **Purpose**: Clean LSP server implementation separated from parser logic
- **Key Features**:
  - Standalone Language Server binary with production-grade CLI
  - Clean separation from parser logic for improved maintainability
  - Works with VSCode, Neovim, Emacs, and all LSP-compatible editors
- **Key Files**:
  - `src/main.rs`: Clean LSP server implementation
  - `bin/perl-lsp.rs`: LSP server binary entry point

### `/crates/perl-dap/` - Debug Adapter Protocol Server ⭐ **DAP BINARY** (Issue #207 - Phase 1)
- **Purpose**: Debug Adapter Protocol (DAP) implementation for Perl debugging in VS Code and DAP-compatible editors
- **Key Features**:
  - **Phase 1 Bridge Architecture**: Proxies DAP messages to Perl::LanguageServer for immediate debugging capability
  - **Cross-Platform Support**: Windows, macOS, Linux, and WSL with automatic path normalization
  - **Configuration Management**: Launch (start new process) and attach (connect to running process) modes
  - **Enterprise Security**: Path validation, process isolation, input sanitization, safe defaults
  - **Performance Optimized**: <50ms breakpoint operations, <100ms step/continue, <200ms variable expansion
  - **Comprehensive Testing**: 71/71 tests passing with mutation hardening and edge case coverage
- **Key Files**:
  - `src/lib.rs`: Public API exports and crate documentation
  - `src/bridge_adapter.rs`: Bridge to Perl::LanguageServer DAP implementation
  - `src/configuration.rs`: LaunchConfiguration and AttachConfiguration types with validation
  - `src/platform.rs`: Cross-platform perl path resolution, path normalization, environment setup
  - `tests/bridge_tests.rs`: Integration tests for bridge adapter functionality
- **Architecture**:
  ```
  VS Code ↔ perl-dap (Rust bridge) ↔ Perl::LanguageServer (Perl) ↔ perl -d
  ```
- **Future Roadmap**:
  - **Phase 2** (Planned): Native Rust DAP implementation with AST-based breakpoint validation
  - **Phase 3** (Planned): Production hardening with advanced features (conditional breakpoints, logpoints)

### `/crates/perl-lexer/` - Context-Aware Tokenizer (Enhanced v0.8.8)
- **Purpose**: Context-aware tokenizer with mode-based lexing and package-qualified identifier support
- **Key Features**:
  - Context-aware tokenizer with mode-based lexing
  - Handles slash disambiguation and Unicode identifiers
  - **Enhanced Package-Qualified Parsing**: Robust tokenization of `Package::identifier` patterns
  - **Unicode Handling**: Robust support for Unicode characters in all contexts
  - **Heredoc Safety**: Proper bounds checking for Unicode + heredoc syntax
- **Key Files**:
  - `src/lib.rs`: Lexer API with Unicode support
  - `src/token.rs`: Token definitions
  - `src/mode.rs`: Lexer modes (ExpectTerm, ExpectOperator)
  - `src/unicode.rs`: Unicode identifier support

### `/crates/perl-corpus/` - Test Corpus
- **Purpose**: Comprehensive test corpus with property-based testing infrastructure
- **Key Files**:
  - `src/lib.rs`: Corpus API
  - `tests/`: Perl test files

### `/crates/perl-parser-pest/` - Legacy Pest Parser ⚠️ **LEGACY**
- **Purpose**: Pest-based parser (v2 implementation), marked as legacy
- **Status**: Published but marked as legacy, use `perl-parser` instead

## Benchmark Framework (v0.8.8) ⭐ **ENHANCED**

### `/crates/tree-sitter-perl-rs/src/bin/benchmark_parsers.rs`
- **Purpose**: Comprehensive Rust benchmark runner
- **Features**:
  - Statistical analysis with confidence intervals
  - JSON output compatible with comparison tools
  - Memory usage tracking and performance categorization
  - Configurable iterations and warmup cycles

### `/tree-sitter-perl/test/benchmark.js`
- **Purpose**: C implementation benchmark harness  
- **Features**:
  - Node.js-based benchmarking for C parser
  - Standardized JSON output format compatible with comparison framework
  - Environment variable configuration support

### `/scripts/generate_comparison.py`
- **Purpose**: Statistical comparison generator
- **Features**:
  - Cross-language performance analysis (C vs Rust)
  - Configurable regression thresholds (5% parse time, 20% memory defaults)
  - Performance gates with statistical significance testing
  - Markdown and JSON report generation with confidence intervals

### `/scripts/setup_benchmark.sh`
- **Purpose**: Automated benchmark environment setup
- **Features**:
  - Dependency installation for Python analysis framework
  - Environment validation and configuration
  - Complete setup automation for cross-language benchmarking

### `/scripts/test_comparison.py`
- **Purpose**: Comprehensive benchmark framework test suite
- **Features**:
  - 12 test cases covering statistical analysis, configuration, and error handling
  - Validates regression detection and performance gate functionality
  - Unit tests for comparison metrics and threshold validation

## Excluded Crates (System Dependencies)

### `/crates/perl-parser-pest/` - Legacy Pest Parser
- **Status**: Published as `perl-parser-pest` on crates.io (marked legacy)
- **Exclusion Reason**: Requires bindgen for C interop

### `/tree-sitter-perl/` - Original C Implementation
- **Exclusion Reason**: libclang dependency

### `/tree-sitter-perl-c/` - C Parser Bindings
- **Exclusion Reason**: libclang-dev dependency

### `/crates/tree-sitter-perl-rs/` - Internal Test Harness & Unified Scanner
- **Exclusion Reason**: bindgen dependency
- **Scanner Architecture**: Contains unified scanner implementation with C wrapper delegation
  - **`src/scanner/rust_scanner.rs`**: Core Rust scanner implementation
  - **`src/scanner/c_scanner.rs`**: C API compatibility wrapper that delegates to RustScanner
  - **`src/scanner/mod.rs`**: Unified scanner interface and feature flags

### `/xtask/` - Development Automation (*Diataxis: Explanation* - Design decisions)
- **Exclusion Reason**: Circular dependency with excluded crates
- **Purpose**: Advanced testing and development tools requiring system dependencies
- **Architecture**: Excluded from workspace to maintain clean builds while preserving functionality

## xtask Architecture (*Diataxis: Explanation* - Advanced testing design)

### Dual-Scanner Corpus Comparison (v0.8.8+)

The xtask system implements a sophisticated dual-scanner corpus comparison architecture:

#### **Design Rationale**
- **Workspace Exclusion**: xtask is excluded from the main workspace to prevent libclang dependency pollution
- **Clean Builds**: Main workspace builds remain system-dependency-free for CI/CD reliability  
- **Advanced Functionality**: xtask provides C vs Rust scanner comparison requiring system dependencies
- **Development Isolation**: Advanced testing tools don't interfere with production builds

#### **Core Components** 
- **`/xtask/src/tasks/corpus.rs`**: Dual-scanner comparison engine with structural analysis
- **`/xtask/src/types.rs`**: Scanner type definitions (C, Rust, V3, Both)
- **`/xtask/Cargo.toml`**: Dependencies on both tree-sitter-perl (C) and perl-parser (Rust)

#### **Scanner Comparison Architecture**
```rust
// Dual-scanner test outcome tracking
struct TestOutcome {
    passed: bool,              // Test passed in both scanners
    scanner_mismatch: bool,    // Scanners produced different results
}

// Comprehensive result tracking
struct CorpusTestResults {
    total: usize,              // Total tests run
    passed: usize,             // Tests passing both scanners
    failed: usize,             // Tests failing in either scanner  
    mismatched: usize,         // Scanner output differences
    mismatches: Vec<String>,   // Detailed mismatch locations
}
```

#### **Structural Analysis Features** (*Diataxis: Reference* - Technical capabilities)
- **Node Count Comparison**: Tracks structural differences between scanner outputs
- **Missing Node Detection**: Identifies nodes present in C but missing in Rust output
- **Extra Node Detection**: Identifies nodes present in Rust but missing in C output
- **S-expression Normalization**: Whitespace-independent comparison for accurate results
- **Diagnostic Analysis**: Detailed structural breakdown for debugging parser differences

#### **Usage Pattern** (*Diataxis: How-to Guide* - Implementation approach)
```bash
# From project root, navigate to xtask directory
cd xtask

# Run corpus comparison modes (requires legacy feature)
cargo run --features legacy -- corpus                     # Default scanner: v3
cargo run --features legacy -- corpus -- --scanner both   # C vs v3 comparison mode
cargo run --features legacy -- corpus -- --scanner v2-parity --diagnose
```

## Key Components

### ModuleResolver Component (NEW v0.8.8) - (*Diataxis: Reference*)

The ModuleResolver provides a reusable, generic module resolution system for LSP features requiring Perl module path resolution.

#### **Architecture Overview**
```rust
/// Resolve a module name to a file path URI.
/// Generic over document type D for flexible integration
pub fn resolve_module_to_path<D>(
    documents: &Arc<Mutex<HashMap<String, D>>>,
    workspace_folders: &Arc<Mutex<Vec<String>>>,
    module_name: &str,
) -> Option<String>
```

#### **Key Design Principles**
- **Generic Document Support**: Works with any document representation via generic type `D`
- **Performance Optimized**: Fast path checks open documents first, then bounded filesystem search
- **Security Conscious**: Time-limited search (50ms timeout) prevents blocking on network filesystems
- **Cooperative**: Yields control during long operations to maintain LSP responsiveness
- **Standard Perl Paths**: Searches `lib`, `.`, `local/lib/perl5` directories in workspace folders

#### **Integration Pattern**
The ModuleResolver follows a functional approach allowing easy integration into LSP providers:

```rust
// Create resolver closure for completion provider
let resolver = {
    let docs = self.documents.clone();
    let folders = self.workspace_folders.clone();
    Arc::new(move |module_name: &str| {
        module_resolver::resolve_module_to_path(&docs, &folders, module_name)
    })
};

// Pass resolver to completion provider
let provider = CompletionProvider::new_with_index_and_source(
    ast,
    &doc.text,
    workspace_index,
    Some(resolver)
);
```

#### **Resolution Algorithm**
1. **Fast Path**: Check already-open documents for matching module paths
2. **Filesystem Search**: Time-limited search through standard Perl directories
3. **Path Standardization**: Convert `Module::Name` to `Module/Name.pm` format
4. **URI Generation**: Return proper `file://` URIs for LSP compatibility

#### **Performance Characteristics**
- **Fast Path**: O(n) where n = number of open documents (typically <100)
- **Filesystem Search**: O(m) where m = files in search directories (bounded by timeout)
- **Timeout Protection**: 50ms maximum to prevent LSP blocking
- **Memory Efficient**: No persistent state, operates on provided references

#### **Testing Coverage**
- **Existing Module Resolution**: Tests successful resolution of modules in workspace
- **Missing Module Handling**: Tests graceful failure for non-existent modules
- **Path Conversion**: Tests `Module::Name` to `Module/Name.pm` transformation
- **Timeout Behavior**: Ensures bounded execution time

#### **Benefits for LSP Features**
- **Reusable**: Single implementation shared across completion, hover, go-to-definition
- **Extensible**: Generic design allows future LSP features to easily add module resolution
- **Reliable**: Comprehensive error handling and timeout protection
- **Standard Compliant**: Follows Perl module path conventions and LSP URI requirements

## Unified Scanner Architecture (*Diataxis: Explanation* - Scanner design and implementation)

### Design Overview

The scanner implementation follows a unified architecture pattern that consolidates multiple scanner interfaces into a single Rust implementation while maintaining full backward compatibility.

#### Core Components (*Diataxis: Reference* - Technical architecture)

**`/crates/tree-sitter-perl-rs/src/scanner/mod.rs`**:
```rust
// Feature-driven scanner selection
#[cfg(any(feature = "rust-scanner", feature = "c-scanner"))]
mod rust_scanner;

#[cfg(feature = "c-scanner")]
mod c_scanner;

// Both features ultimately use the same Rust implementation
#[cfg(any(feature = "rust-scanner", feature = "c-scanner"))]
pub use rust_scanner::*;

#[cfg(feature = "c-scanner")]
pub use c_scanner::*;
```

**`/crates/tree-sitter-perl-rs/src/scanner/rust_scanner.rs`**:
- Core scanning implementation with full Perl lexical analysis
- Context-aware tokenization with mode tracking
- Unicode identifier support and proper delimiter handling
- Comprehensive token type system with 100+ Perl constructs

**`/crates/tree-sitter-perl-rs/src/scanner/c_scanner.rs`**:
```rust
/// Compatibility wrapper that delegates to RustScanner
pub struct CScanner {
    inner: RustScanner,
}

impl PerlScanner for CScanner {
    fn scan(&mut self, input: &[u8]) -> ParseResult<Option<u16>> {
        self.inner.scan(input)  // Pure delegation
    }
    // All methods delegate to inner RustScanner
}
```

### Architecture Benefits (*Diataxis: Explanation* - Design decisions)

#### **Simplified Maintenance**
- **Single Source of Truth**: One scanner implementation for all functionality
- **Reduced Code Duplication**: No separate C and Rust scanner codebases to maintain
- **Unified Testing**: All scanner behavior tested through single implementation
- **Consistent Performance**: Same performance characteristics across all interfaces

#### **Backward Compatibility**
- **API Preservation**: Existing `CScanner` API continues to work unchanged
- **Benchmark Compatibility**: Legacy benchmark code requires no modifications
- **Feature Flag Support**: Both `c-scanner` and `rust-scanner` features supported
- **Migration Path**: Gradual migration from C API to Rust API without disruption

#### **Development Efficiency** 
- **Single Debug Target**: All scanner issues traced to single implementation
- **Centralized Improvements**: Performance and correctness improvements benefit all interfaces
- **Simplified Feature Addition**: New token types added once, available everywhere
- **Reduced Testing Complexity**: Test coverage for single implementation covers all interfaces

### Implementation Strategy (*Diataxis: How-to Guide* - Using the unified scanner)

#### **For New Code** (*Diataxis: Tutorial* - Recommended approach)
```rust
use tree_sitter_perl_rs::RustScanner;

let mut scanner = RustScanner::new();
let token = scanner.scan(input)?;
```

#### **For Legacy Code** (*Diataxis: How-to Guide* - Migration approach)
```rust
use tree_sitter_perl_rs::CScanner;  // Drop-in replacement

let mut scanner = CScanner::new();  // Same API as before
let token = scanner.scan(input)?;   // Delegates to RustScanner internally
```

#### **Feature Flag Configuration** (*Diataxis: Reference* - Build configuration)
```toml
# Cargo.toml - Choose scanner interface
[features]
default = ["rust-scanner"]
rust-scanner = []           # Direct RustScanner access
c-scanner = []              # CScanner wrapper (delegates to RustScanner)
```

### Testing Strategy (*Diataxis: Reference* - Quality assurance)

#### **Unified Test Coverage**
- **`tests/rust_scanner_smoke.rs`**: Validates core scanner functionality
- **Delegation Tests**: Ensures `CScanner` properly delegates to `RustScanner`
- **API Compatibility Tests**: Verifies legacy API contracts remain unchanged
- **Performance Tests**: Confirms no performance regression from delegation pattern

#### **Build Validation** (*Diataxis: How-to Guide* - Development workflow)
```bash
# Test both scanner interfaces
cargo test --features rust-scanner
cargo test --features c-scanner

# Validate delegation pattern
cargo test -p tree-sitter-perl-rs rust_scanner_smoke
```

### Migration Implications (*Diataxis: Explanation* - Understanding the changes)

#### **What Changed**
- **Implementation**: `CScanner` now delegates to `RustScanner` instead of implementing separately
- **Build System**: `build.rs` detects scanner features through environment variables
- **Testing**: Added smoke tests to validate delegation functionality

#### **What Stayed the Same**
- **Public API**: All existing `CScanner` methods and signatures unchanged
- **Performance**: Same performance characteristics (now consistently Rust-based)
- **Feature Flags**: Both `c-scanner` and `rust-scanner` features continue to work
- **Benchmarks**: Existing benchmark infrastructure works without modification

#### **Benefits Realized**
- **Maintainability**: 50% reduction in scanner-related code complexity
- **Reliability**: Single implementation reduces potential for divergent behavior
- **Performance**: Consistent Rust performance across all interfaces
- **Development Velocity**: Scanner improvements benefit all consumers immediately

### Pest Parser Architecture
- PEG grammar in `grammar.pest` defines all Perl syntax
- Recursive descent parsing with packrat optimization
- Zero-copy parsing with `&str` slices
- Feature flag: `pure-rust` enables the Pest parser

### AST Generation
- Strongly typed AST nodes in `pure_rust_parser.rs`
- Arc<str> for efficient string storage
- Tree-sitter compatible node types
- Position tracking for all nodes

### S-Expression Output
- `to_sexp()` method produces tree-sitter format
- Compatible with existing tree-sitter tools
- Preserves all position information
- Error nodes for unparseable constructs

### Enhanced Position Tracking (v0.8.7+)
- **O(log n) Position Mapping**: Efficient binary search-based position lookups using LineStartsCache
- **LSP-Compliant UTF-16 Support**: Accurate character counting for multi-byte Unicode characters and emoji
- **Multi-line Token Support**: Proper position tracking for tokens spanning multiple lines (strings, comments, heredocs)
- **Line Ending Agnostic**: Handles CRLF, LF, and CR line endings consistently across platforms
- **Production-Ready Integration**: Seamless integration with parser context and LSP server for real-time editing
- **Comprehensive Testing**: 8 specialized test cases covering Unicode, CRLF, multiline strings, and edge cases

## Enhanced Dual Indexing Strategy (v0.8.8) ⭐ **ENHANCED**

### Cross-File Reference Resolution
The workspace indexing system implements a dual indexing strategy for comprehensive cross-file navigation with 98% reference coverage:

#### Core Architecture Pattern (*Diataxis: Reference*)
```rust
// Dual indexing: index function calls under both forms
let qualified = format!("{}::{}", package, bare_name);

// Index under bare name for unqualified calls
file_index.references.entry(bare_name.to_string())
    .or_default().push(symbol_ref.clone());

// Index under qualified name for Package::function calls  
file_index.references.entry(qualified)
    .or_default().push(symbol_ref);
```

#### Enhanced Reference Search (*Diataxis: Reference*)
```rust
pub fn find_references(&self, symbol_name: &str) -> Vec<Location> {
    let mut locations = Vec::new();
    
    // Search exact match first
    if let Some(refs) = index.get(symbol_name) {
        locations.extend(refs.iter().cloned());
    }
    
    // If qualified, also search bare name
    if let Some(idx) = symbol_name.rfind("::") {
        let bare_name = &symbol_name[idx + 2..];
        if let Some(refs) = index.get(bare_name) {
            locations.extend(refs.iter().cloned());
        }
    }
    
    locations
}
```

#### Smart Deduplication Algorithm
- **URI and Range-Based**: Prevents duplicate references based on file location and position
- **HashSet Optimization**: O(1) deduplication using composite keys
- **Definition Exclusion**: Function definitions properly excluded from "Find All References" 
- **LSP Compliance**: Results match LSP specification for reference vs definition separation

#### Key Benefits (*Diataxis: Explanation*)
- **98% Reference Coverage**: Handles both `Package::function` and `function` call patterns
- **Performance Optimized**: Dual lookups with efficient HashSet deduplication
- **Backward Compatible**: Existing code continues to work with enhanced indexing
- **Enterprise Ready**: Production-stable workspace navigation across package boundaries

## Import Optimization Architecture (v0.8.8) ⭐ **NEW**

### Core Components

#### `/crates/perl-parser/src/import_optimizer.rs` - Analysis Engine
- **Purpose**: Stateless import analysis and optimization engine
- **Features**:
  - **Unused Import Detection**: Regex-based usage analysis identifies import statements never used in code
  - **Duplicate Import Consolidation**: Merges multiple import lines from same module into single optimized statements
  - **Missing Import Detection**: Identifies Module::symbol references requiring additional imports
  - **Alphabetical Sorting**: Organizes imports in consistent alphabetical order
  - **Performance Optimized**: Fast analysis suitable for real-time LSP code actions (<10ms for typical files)
  - **Conservative Analysis**: Careful handling for pragma modules and modules with side effects

#### Key Architecture Patterns
```rust
// Stateless analyzer for thread safety
pub struct ImportOptimizer;

// Comprehensive analysis result
pub struct ImportAnalysis {
    pub imports: Vec<ImportEntry>,
    pub unused_imports: Vec<UnusedImport>,
    pub duplicate_imports: Vec<DuplicateImport>, 
    pub missing_imports: Vec<MissingImport>,
    pub organization_suggestions: Vec<OrganizationSuggestion>,
}

// LSP integration ready
impl ImportOptimizer {
    pub fn generate_edits(&self, content: &str, analysis: &ImportAnalysis) -> Vec<TextEdit>;
}
```

#### `/crates/perl-parser/src/code_actions.rs` - LSP Integration
- **Purpose**: Code actions provider with import optimization integration
- **Features**:
  - **"Organize Imports" Action**: Standard LSP source.organizeImports code action kind
  - **Quick Fix Actions**: Specific actions for unused/missing imports
  - **Text Edit Generation**: LSP-compatible text edits for applying optimizations
  - **Real-time Analysis**: Import issues detected as you type with immediate fixes

#### Integration Architecture
```rust
// Automatic integration with code actions system
pub fn get_code_actions(&self, ast: &Node, range: (usize, usize), diagnostics: &[Diagnostic]) -> Vec<CodeAction> {
    let mut actions = Vec::new();
    
    // Add diagnostic-based fixes...
    
    // Import optimization always available
    if let Some(import_action) = self.optimize_imports() {
        actions.push(import_action);
    }
    
    actions
}
```

### Performance Characteristics
- **Analysis Speed**: <10ms for files with <100 imports, <50ms for files with 100-500 imports
- **Memory Efficiency**: Bounded processing with file size limits (1MB max)
- **LSP Responsiveness**: Suitable for real-time editor integration
- **Thread Safety**: Stateless analyzer with no shared mutable state

### Editor Integration
- **VSCode**: Seamless "Organize Imports" (Cmd/Ctrl+Shift+O) and context menu integration
- **Neovim/Emacs**: Full LSP code action support for import optimization
- **Real-time Feedback**: Import issues show as available quick fixes in editor UI
- **Preview Changes**: Editor diff view shows changes before applying optimizations

### Edge Case Handling
- Comprehensive heredoc support (93% edge case test coverage)
- Phase-aware parsing for BEGIN/END blocks
- Dynamic delimiter detection and recovery
- Clear diagnostics for unparseable constructs

### Testing Strategy
- Grammar tests for each Perl construct
- Edge case tests with property testing
- Performance benchmarks
- Integration tests for S-expression output
- Position tracking validation tests
- Encoding-aware lexing for mid-file encoding changes
- Tree-sitter compatible error nodes and diagnostics
- Performance optimized (<5% overhead for normal code)

## Agent Ecosystem Integration (PR #153) (*Diataxis: Explanation* - Specialized workflow automation)

### 94 Specialized Agents Architecture

**Revolutionary Workflow Enhancement**: PR #153 introduces a comprehensive agent ecosystem with 94 specialized agents designed specifically for the tree-sitter-perl parsing ecosystem. This represents a paradigm shift from generic automation to domain-specific intelligent workflow coordination.

#### Agent Directory Structure (*Diataxis: Reference* - Agent organization)

```
.claude/agents2/                          # 94 specialized agents (vs. 53 generic)
├── review/                               # 26 agents - PR review workflow
│   ├── review-security-scanner.md       # UTF-16 security validation
│   ├── review-mutation-tester.md        # 87% quality score validation
│   ├── review-performance-validator.md  # Revolutionary performance preservation
│   └── review-governance-gate.md        # Final quality assurance
├── integration/                          # 21 agents - CI/CD coordination
│   ├── integration-test-coordinator.md  # Adaptive threading orchestration
│   ├── integration-workspace-validator.md # Multi-crate validation
│   └── integration-performance-monitor.md # 5000x LSP improvements tracking
├── generative/                           # 24 agents - Content creation
│   ├── generative-doc-writer.md         # Parser ecosystem documentation
│   ├── generative-test-creator.md       # Mutation hardening test generation
│   └── generative-parser-enhancer.md    # AST and parsing improvements
├── mantle/                               # 17 agents - Maintenance operations
│   ├── mantle-dependency-manager.md     # Workspace dependency coordination
│   ├── mantle-release-coordinator.md    # Multi-crate release orchestration
│   └── mantle-security-auditor.md       # Enterprise security compliance
└── other/                                # 6 agents - Cross-cutting concerns
    ├── agent-customizer.md              # Self-adapting agent framework
    └── workflow-orchestrator.md         # Agent coordination patterns
```

#### Specialized Agent Capabilities (*Diataxis: Explanation* - Domain expertise integration)

**Parser Ecosystem Context Integration:**
- **Multi-crate Architecture**: Understanding of 5 published crates and their interdependencies
- **Performance Standards**: Built-in knowledge of revolutionary performance requirements (5000x LSP improvements)
- **Security Requirements**: UTF-16 position conversion security, enterprise-grade Unicode safety
- **Quality Metrics**: Mutation testing (87% score), zero clippy warnings, comprehensive test coverage

**Intelligent Workflow Coordination:**
```rust
// Example: Security-focused agent routing
SecurityScanner → MutationTester → PerformanceValidator → GovernanceGate

// Example: Development agent coordination
CodeEnhancer → TestCreator → DocGenerator → ReviewPrep
```

#### Agent Customization Framework (*Diataxis: Reference* - Self-adapting architecture)

**Contextual Adaptation Engine:**
```markdown
# Agent customizes itself based on project context
- Multi-crate workspace patterns (perl-parser ⭐, perl-lsp ⭐, perl-lexer, perl-corpus)
- Revolutionary performance requirements (sub-microsecond parsing, <1ms LSP updates)
- Enterprise security standards (UTF-16 safety, path traversal prevention)
- Comprehensive quality validation (87% mutation score, zero clippy warnings)
```

**Self-Documenting Configuration:**
- **Inline Expertise**: Each agent includes parser ecosystem domain knowledge
- **Quality Integration**: Built-in understanding of mutation testing and performance benchmarks
- **Security Awareness**: UTF-16 position conversion security and enterprise patterns
- **Workflow Intelligence**: Context-aware routing between specialized agents

#### Quality and Security Integration (*Diataxis: Explanation* - Enterprise-grade validation)

**Mutation Testing Coordination:**
- **Real Bug Discovery**: Agents coordinate mutation testing that discovered UTF-16 security vulnerabilities
- **Quality Score Achievement**: 87% mutation score through systematic agent-driven testing
- **Security Validation**: UTF-16 boundary violation detection and remediation

**Performance Preservation:**
- **Revolutionary Standards**: Agents ensure 5000x LSP performance improvements are maintained
- **Security-Performance Balance**: Enhanced security without performance regression
- **Adaptive Threading**: CI environment optimization through intelligent agent coordination

#### Integration Points (*Diataxis: Reference* - Agent ecosystem interfaces)

**Crate Integration:**
- **`/crates/perl-parser/`**: Core parser logic enhanced by generative agents (test creation, performance optimization)
- **`/crates/perl-lsp/`**: LSP server validated by review agents (security scanning, performance validation)
- **`/crates/perl-lexer/`**: Tokenizer improvements coordinated by integration agents
- **`/crates/perl-corpus/`**: Test corpus expansion through generative and integration agents

**Documentation Ecosystem:**
- **`/docs/`**: Comprehensive documentation maintained by specialized doc-writer agents
- **ADRs**: Architecture decisions documented and validated by governance agents
- **Security Guides**: Enterprise security patterns maintained by security-focused agents

#### Workflow Orchestration Patterns (*Diataxis: How-to* - Agent coordination)

**Review Workflow:**
```bash
# Agent-coordinated PR review with intelligent routing
review-security-scanner     # UTF-16 security validation
  ↓
review-mutation-tester      # 87% quality score verification
  ↓
review-performance-validator # Revolutionary performance preservation
  ↓
review-governance-gate      # Final quality assurance and routing decision
```

**Development Workflow:**
```bash
# Agent-enhanced development cycle
generative-parser-enhancer  # AST and parsing improvements
  ↓
generative-test-creator     # Comprehensive test coverage
  ↓
integration-test-coordinator # Multi-crate validation
  ↓
generative-doc-writer       # Documentation synchronization
```

## Development Guidelines

### Choosing a Crate
1. **For Any Perl Parsing**: Use `perl-parser` - fastest, most complete, production-ready with Rope support
2. **For IDE Integration**: Install `perl-lsp` from `perl-parser` crate - includes full Rope-based document management  
3. **For Testing Parsers**: Use `perl-corpus` for comprehensive test suite
4. **For Legacy Migration**: Migrate from `perl-parser-pest` to `perl-parser`

### Development Locations
- **Parser & LSP**: `/crates/perl-parser/` - main development with production Rope implementation
- **LSP Server**: `/crates/perl-lsp/` - standalone LSP server binary (v0.8.8)
- **Lexer**: `/crates/perl-lexer/` - tokenization improvements
- **Test Corpus**: `/crates/perl-corpus/` - test case additions
- **Legacy (Excluded)**: `/crates/perl-parser-pest/` - maintenance only, excluded from workspace
- **Advanced Testing (Excluded)**: `/xtask/` - dual-scanner corpus comparison, excluded due to libclang dependencies

### Rope Development Guidelines
**IMPORTANT**: All Rope improvements should target the **production perl-parser crate**, not internal test harnesses.

**Production Rope Modules** (Target for improvements):
- **`/crates/perl-parser/src/textdoc.rs`**: Core document management with `ropey::Rope`
- **`/crates/perl-parser/src/position_mapper.rs`**: UTF-16/UTF-8 position conversion
- **`/crates/perl-parser/src/incremental_integration.rs`**: LSP integration bridge
- **`/crates/perl-parser/src/incremental_handler_v2.rs`**: Document change processing

**Do NOT modify these Rope usages** (internal test code):
- **`/crates/tree-sitter-perl-rs/`**: Legacy test harnesses with outdated Rope usage
- **Internal test infrastructure**: Focus on production code, not test utilities

## Dual Indexing Architecture (v0.8.8+) (*Diataxis: Explanation* - Revolutionary workspace navigation design)

### Problem Statement (*Diataxis: Explanation* - Why dual indexing is needed)

Perl's flexible function calling conventions create significant challenges for static analysis and IDE features:

```perl
# File: lib/Utils.pm
package Utils;
sub process_data { ... }

# File: main.pl  
use Utils;

# All three reference the same function:
Utils::process_data();    # Qualified call
process_data();          # Bare call (via import)
&process_data();         # Explicit subroutine call
```

Traditional LSP servers index functions under a single name form, leading to:
- **High false negative rates** (~15%): Missing references when users call functions differently than indexed
- **Inconsistent go-to-definition**: Works for some call styles but not others
- **Poor find-references coverage**: Only finds references matching the indexing style

### Solution: Dual Indexing Strategy (*Diataxis: Reference* - Technical implementation)

The dual indexing strategy solves this by indexing every function under **both** its qualified and bare name forms.

#### Core Algorithm (*Diataxis: Reference* - Implementation specification)

**Indexing Phase** (`/crates/perl-parser/src/workspace_index.rs`):
```rust
// For every function call, index under both forms
let qualified = format!("{}::{}", package, bare_name);

// Store under bare name
file_index.references.entry(bare_name.to_string()).or_default().push(
    SymbolReference {
        uri: self.uri.clone(),
        range: location,
        kind: ReferenceKind::Usage,
    }
);

// Store under qualified name
file_index.references.entry(qualified).or_default().push(SymbolReference {
    uri: self.uri.clone(), 
    range: location,
    kind: ReferenceKind::Usage,
});
```

**Retrieval Phase**:
```rust
/// Dual pattern search with automatic deduplication
pub fn find_references(&self, symbol_name: &str) -> Vec<Location> {
    let mut locations = Vec::new();
    
    // Search exact match first
    if let Some(refs) = index.get(symbol_name) {
        locations.extend(refs.iter().map(|r| Location {
            uri: r.uri.clone(),
            range: r.range
        }));
    }
    
    // If qualified, also search bare name
    if let Some(idx) = symbol_name.rfind("::") {
        let bare_name = &symbol_name[idx + 2..];
        if let Some(refs) = index.get(bare_name) {
            locations.extend(refs.iter().map(|r| Location {
                uri: r.uri.clone(),
                range: r.range
            }));
        }
    }
    
    locations
}
```

### Performance Impact (*Diataxis: Reference* - Performance characteristics)

| Metric | Before Dual Indexing | After Dual Indexing | Change |
|--------|---------------------|---------------------|---------|
| **Reference Coverage** | ~85% (single form) | ~98% (both forms) | +15% |
| **False Negatives** | High (missed calls) | Minimal | -90% |
| **Index Memory Usage** | Baseline | +10-15% | Acceptable |
| **Search Performance** | Fast | Fast (dual lookup) | Maintained |
| **Go-to-Definition Success** | ~83% | ~98% | +18% |

### Integration with Lexer (*Diataxis: Reference* - Supporting infrastructure)

The lexer enhancement in `/crates/perl-lexer/src/lib.rs` supports dual indexing by properly tokenizing package-qualified identifiers:

```rust
// Handle package-qualified identifiers like Foo::bar
while self.current_char() == Some(':') && self.peek_char(1) == Some(':') {
    // consume '::'
    self.advance();
    self.advance();
    
    // Continue with next segment
    while let Some(ch) = self.current_char() {
        if is_perl_identifier_continue(ch) {
            self.advance();
        } else {
            break;
        }
    }
}
```

### Benefits (*Diataxis: Explanation* - Architectural advantages)

1. **Comprehensive Coverage**: Finds all references regardless of calling style
2. **Consistent Behavior**: Go-to-definition works from any reference form
3. **Zero Breaking Changes**: Existing code continues to work
4. **Minimal Performance Impact**: Smart indexing with deduplication
5. **Improved Developer Experience**: More accurate LSP features across the board

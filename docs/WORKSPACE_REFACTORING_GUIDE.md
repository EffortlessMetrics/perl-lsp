# Workspace Refactoring Guide

## Overview

The tree-sitter-perl project provides comprehensive workspace refactoring capabilities through the `WorkspaceRefactor` system introduced in v0.8.8. This guide covers all aspects of using and extending these cross-file refactoring operations.

## Table of Contents

1. [Getting Started](#getting-started) (**Diataxis: Tutorial**)
2. [Core Operations](#core-operations) (**Diataxis: How-to**)
3. [API Reference](#api-reference) (**Diataxis: Reference**)
4. [Architecture](#architecture) (**Diataxis: Explanation**)
5. [Contributing](#contributing)
6. [Troubleshooting](#troubleshooting)

## Getting Started (**Diataxis: Tutorial**)

### Prerequisites

- Rust 1.92+ (MSRV)
- tree-sitter-perl v0.8.8 or later
- Workspace with indexed Perl files

### Basic Setup

```rust
use perl_parser::workspace_refactor::WorkspaceRefactor;
use perl_parser::workspace_index::WorkspaceIndex;
use std::path::Path;

// Create and populate workspace index
let mut index = WorkspaceIndex::new();
index.index_file_str("main.pl", "my $variable = 42;").unwrap();
index.index_file_str("lib/Utils.pm", "package Utils; sub helper { }")
    .unwrap();

// Create refactoring provider
let refactor = WorkspaceRefactor::new(index);
```

### Your First Refactoring Operation

```rust
// Rename a variable across all files in the workspace
let result = refactor.rename_symbol(
    "$variable",           // Symbol to rename
    "$renamed_variable",   // New name
    Path::new("main.pl"),  // Origin file
    (0, 0)                // Position (currently unused)
)?;

println!("Refactoring: {}", result.description);
println!("Files to modify: {}", result.file_edits.len());

// Apply the changes
for file_edit in result.file_edits {
    println!("Updating file: {:?}", file_edit.file_path);
    // Apply edits in reverse order to maintain positions
    for edit in file_edit.edits.iter().rev() {
        // Replace text at edit.start..edit.end with edit.new_text
        println!("  Replace {}..{} with '{}'", 
                 edit.start, edit.end, edit.new_text);
    }
}
```

## Core Operations (**Diataxis: How-to**)

### Symbol Renaming

Cross-file renaming with comprehensive validation and Unicode support.

```rust
// Basic variable renaming
let result = refactor.rename_symbol("$old_var", "$new_var", &path, (0, 0))?;

// Function renaming
let result = refactor.rename_symbol("old_function", "new_function", &path, (0, 0))?;

// Unicode variable names
let result = refactor.rename_symbol("$♥", "$love", &path, (0, 0))?;
```

**Features:**
- Supports all Perl symbols: scalars (`$`), arrays (`@`), hashes (`%`), functions
- Workspace-wide search with intelligent fallback to text matching
- Performance optimized with early termination (1000 match limit)
- Comprehensive input validation

### Module Extraction

Extract code sections into new Perl modules.

```rust
// Extract lines 50-100 from a large file
let result = refactor.extract_module(
    Path::new("large_script.pl"), // Source file
    50,                           // Start line (1-based)
    100,                          // End line (inclusive)
    "ExtractedModule"             // Module name (without .pm)
)?;

// Results in:
// 1. large_script.pl: lines 50-100 replaced with "use ExtractedModule;"
// 2. ExtractedModule.pm: created with the extracted content
```

**Features:**
- Line-based extraction with proper bounds checking
- Automatic use statement generation
- Preserves original indentation and formatting
- Supports Unicode content

### Import Optimization

Workspace-wide import statement optimization with comprehensive analysis and automatic fixing.

```rust
// Optimize all imports across the workspace
let result = refactor.optimize_imports()?;

println!("Optimized {} files", result.file_edits.len());
for warning in &result.warnings {
    println!("Warning: {}", warning);
}
```

#### Single-File Import Optimization

```rust
// Optimize imports for a specific file using ImportOptimizer
use perl_parser::import_optimizer::ImportOptimizer;

let optimizer = ImportOptimizer::new();
let analysis = optimizer.analyze_file(Path::new("script.pl"))?;

// Review findings
for unused in &analysis.unused_imports {
    println!("Unused import '{}' at line {}: {:?}", 
        unused.module, unused.line, unused.symbols);
}

for duplicate in &analysis.duplicate_imports {
    println!("Duplicate import '{}' on lines: {:?}", 
        duplicate.module, duplicate.lines);
}

for missing in &analysis.missing_imports {
    println!("Missing import '{}' for symbols: {:?}", 
        missing.module, missing.symbols);
}

// Generate optimized imports
let optimized = optimizer.generate_optimized_imports(&analysis);
println!("Optimized imports:\n{}", optimized);

// Generate LSP-compatible text edits
let content = std::fs::read_to_string("script.pl")?;
let edits = optimizer.generate_edits(&content, &analysis);
for edit in edits {
    println!("Edit at {}..{}: '{}'", 
        edit.location.start, edit.location.end, edit.new_text);
}
```

**Comprehensive Features:**
- **Unused Import Detection**: Identifies import statements never used in code using regex-based usage analysis
- **Duplicate Import Consolidation**: Merges multiple import lines from same module into single optimized statements
- **Missing Import Detection**: Identifies Module::symbol references requiring additional imports
- **Alphabetical Sorting**: Organizes imports in consistent alphabetical order
- **Symbol-Level Analysis**: Removes unused symbols from qw() imports while preserving used ones
- **Conservative Pragma Handling**: Special handling for pragma modules like strict, warnings, utf8
- **Performance Optimized**: Fast analysis suitable for real-time LSP code actions (<10ms typical files)
- **LSP Integration**: Seamless integration with "Organize Imports" code actions in editors

**Advanced Analysis Features:**
- **Context-Aware Usage Detection**: Handles qualified function calls (Module::function)
- **String/Comment Filtering**: Ignores module references in strings and comments
- **Nested Function Call Support**: Proper comma counting in complex nested calls
- **Regex-Based Pattern Matching**: Efficient symbol usage scanning with compiled patterns
- **Unicode Support**: Proper handling of Unicode characters in module names and symbols

**Integration with LSP Code Actions:**
```rust
// Import optimization is automatically available in LSP code actions
// Editor integration provides:
// - "Organize Imports" command (Cmd/Ctrl+Shift+O in VSCode)
// - Right-click context menu actions
// - Real-time quick fixes for import issues
// - Preview changes before applying
```

### Subroutine Movement

Move functions between modules with proper cleanup.

```rust
// Move a utility function to a dedicated module
let result = refactor.move_subroutine(
    "utility_function",        // Function name
    Path::new("main.pl"),      // Source file
    "Utils"                    // Target module
)?;

// Function is removed from main.pl and appended to Utils.pm
```

**Features:**
- Precise symbol location using WorkspaceIndex
- Complete code transfer including documentation
- Automatic file cleanup

### Variable Inlining

Replace variables with their initializer expressions.

```rust
// Inline a temporary variable
let result = refactor.inline_variable(
    "$temp_var",              // Variable to inline  
    Path::new("script.pl"),   // File containing variable
    (0, 0)                    // Position (unused)
)?;

// my $temp_var = some_expression(); 
// print $temp_var;
// 
// becomes:
//
// print some_expression();
```

**Features:**
- Scope-aware analysis and replacement
- Handles complex initializer expressions
- Removes original variable declaration

## API Reference (**Diataxis: Reference**)

### WorkspaceRefactor

Main refactoring provider with comprehensive operations.

```rust
impl WorkspaceRefactor {
    pub fn new(index: WorkspaceIndex) -> Self;
    
    pub fn rename_symbol(
        &self,
        old_name: &str,
        new_name: &str,
        file_path: &Path,
        position: (usize, usize),
    ) -> Result<RefactorResult, RefactorError>;
    
    pub fn extract_module(
        &self,
        file_path: &Path,
        start_line: usize,
        end_line: usize,
        module_name: &str,
    ) -> Result<RefactorResult, RefactorError>;
    
    pub fn optimize_imports(&self) -> Result<RefactorResult, String>;
    
    pub fn move_subroutine(
        &self,
        sub_name: &str,
        from_file: &Path,
        to_module: &str,
    ) -> Result<RefactorResult, RefactorError>;
    
    pub fn inline_variable(
        &self,
        var_name: &str,
        file_path: &Path,
        position: (usize, usize),
    ) -> Result<RefactorResult, RefactorError>;
}
```

### RefactorResult

Structured result format with comprehensive information.

```rust
pub struct RefactorResult {
    /// File edits needed to complete the refactoring
    pub file_edits: Vec<FileEdit>,
    /// Human-readable description
    pub description: String,
    /// Non-fatal warnings encountered during analysis
    pub warnings: Vec<String>,
}
```

### FileEdit & TextEdit

Precise text editing instructions.

```rust
pub struct FileEdit {
    /// Absolute path to the file to edit
    pub file_path: PathBuf,
    /// Text edits to apply (in document order)
    pub edits: Vec<TextEdit>,
}

pub struct TextEdit {
    /// Start byte offset (inclusive)
    pub start: usize,
    /// End byte offset (exclusive) 
    pub end: usize,
    /// Replacement text
    pub new_text: String,
}
```

### RefactorError

Comprehensive error handling with detailed categorization.

```rust
pub enum RefactorError {
    UriConversion(String),
    DocumentNotIndexed(String),
    InvalidPosition { file: String, details: String },
    SymbolNotFound { symbol: String, file: String },
    ParseError(String),
    InvalidInput(String),
    FileSystemError(String),
}
```

## Architecture (**Diataxis: Explanation**)

### Design Principles

The workspace refactoring system is built on several key architectural principles:

1. **Safety First**: Comprehensive validation, Unicode safety, performance limits
2. **Precision**: Byte-level positioning for accurate text edits
3. **Performance**: Optimized algorithms with early termination and smart filtering
4. **Flexibility**: Extensible architecture for new refactoring operations
5. **Error Recovery**: Graceful handling of edge cases and invalid input

### System Components

```
┌─────────────────────────────────────────────────────────────┐
│                  Workspace Refactoring System              │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐    │
│  │ Workspace   │◄──►│ Workspace   │◄──►│ Refactor    │    │
│  │ Index       │    │ Refactor    │    │ Result      │    │
│  └─────────────┘    └─────────────┘    └─────────────┘    │
│         │                   │                   │          │
│         ▼                   ▼                   ▼          │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐    │
│  │ Document    │    │ Operation   │    │ FileEdit[]  │    │
│  │ Store       │    │ Types       │    │             │    │
│  └─────────────┘    └─────────────┘    └─────────────┘    │
│         │                   │                   │          │
│         ▼                   ▼                   ▼          │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐    │
│  │ Text        │    │ Symbol      │    │ TextEdit[]  │    │
│  │ Content     │    │ Analysis    │    │             │    │
│  └─────────────┘    └─────────────┘    └─────────────┘    │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Core Algorithms

**Symbol Resolution**:
1. Consult WorkspaceIndex for precise symbol locations
2. Fallback to text-based search with performance optimizations
3. Apply safety limits (1000 matches, 500 files)
4. Early termination for large operations

**Text Processing**:
1. Byte-based searching for efficiency
2. Pre-filtering to avoid processing irrelevant files
3. BTreeMap for maintaining edit order
4. Unicode-safe boundary detection

**Error Recovery**:
1. Comprehensive input validation
2. Graceful handling of missing symbols
3. Position validation with detailed error context
4. File system error recovery

### Performance Characteristics

- **Symbol Renaming**: O(n*m) where n = files, m = average file size
- **Module Extraction**: O(1) with respect to workspace size
- **Import Optimization**: O(n*k) where n = files, k = imports per file
- **Memory Usage**: Linear with workspace size, optimized with Arc sharing
- **Safety Limits**: Built-in performance bounds prevent runaway operations

## Contributing

### Adding New Operations

1. **Design**: Plan the operation with comprehensive requirements
2. **Implement**: Add the method to `WorkspaceRefactor`
3. **Test**: Create comprehensive test cases including:
   - Basic functionality
   - Error conditions
   - Unicode support
   - Performance scenarios
   - Edge cases
4. **Document**: Update this guide and API documentation

### Testing Guidelines

```bash
# Run all workspace refactoring tests
cargo test -p perl-parser workspace_refactor

# Test specific operation
cargo test -p perl-parser workspace_refactor::tests::test_your_operation

# Test with debug output
cargo test -p perl-parser workspace_refactor::tests -- --nocapture
```

### Code Quality Standards

- **Input Validation**: Validate all parameters
- **Unicode Safety**: Support international characters
- **Error Handling**: Provide detailed error context
- **Performance**: Implement safety limits
- **Testing**: Cover all code paths and edge cases
- **Documentation**: Clear examples and API docs

## Troubleshooting

### Common Issues

**"Document not indexed in workspace"**
- Ensure files are added to the WorkspaceIndex before refactoring
- Check file paths and URI conversion

**"Symbol not found"**
- Verify symbol name and sigil ($ @ % for variables)
- Check if the symbol exists in the expected file
- Enable text-based fallback for complex cases

**"Invalid position"**
- Line numbers are 1-based (not 0-based)
- Ensure line ranges are valid for the file

**Performance Issues**
- Large workspaces may hit safety limits
- Consider batch processing for very large codebases
- Monitor memory usage with Arc sharing

### Debug Tools

```rust
// Enable detailed logging
env_logger::init();
log::debug!("Refactoring operation: {:?}", operation);

// Check workspace index contents
for symbol in index.all_symbols() {
    println!("Symbol: {:?}", symbol);
}

// Validate file contents
if let Some(doc) = index.document_store().get(&uri) {
    println!("File content: {}", &doc.text[0..100]);
}
```

### Performance Tuning

```rust
// For large codebases, consider these optimizations:

// 1. Pre-filter files
let relevant_files: Vec<_> = workspace.files()
    .filter(|f| f.contains("target_pattern"))
    .collect();

// 2. Batch operations
for batch in files.chunks(10) {
    let result = refactor.optimize_imports()?;
    // Process batch
    thread::sleep(Duration::from_millis(100)); // Rate limiting
}

// 3. Monitor performance
let start = Instant::now();
let result = refactor.rename_symbol(...)?;
println!("Operation took: {:?}", start.elapsed());
```

## Version History

### v0.8.8
- **Initial release** of comprehensive workspace refactoring system
- **19 comprehensive test cases** with 17/19 passing (filesystem-dependent failures)
- **Unicode-safe operations** with international character support
- **Performance optimization** with safety limits and early termination
- **Enterprise-grade error handling** with detailed error categorization

## Related Documentation

- [CLAUDE.md](../CLAUDE.md) - Project overview and commands
- [ARCHITECTURE.md](../ARCHITECTURE.md) - System architecture
- [CONTRIBUTING.md](../CONTRIBUTING.md) - Development guidelines
- [LSP_IMPLEMENTATION_GUIDE.md](LSP_IMPLEMENTATION_GUIDE.md) - LSP integration
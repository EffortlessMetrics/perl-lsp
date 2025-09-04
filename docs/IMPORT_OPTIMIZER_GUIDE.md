# Import Optimizer Guide

## Overview

The import optimization system provides comprehensive analysis and optimization of Perl import statements with enterprise-grade reliability and performance.

## Architecture

- **ImportOptimizer**: Core analysis engine with regex-based usage detection
- **ImportAnalysis**: Structured analysis results with unused, duplicate, and missing import tracking
- **OptimizedImportGeneration**: Alphabetical sorting and clean formatting with duplicate consolidation
- **Complete Test Coverage**: 9 comprehensive test cases covering all optimization scenarios

## Features

- **Unused Import Detection**: Regex-based usage analysis identifies import statements never used in code
- **Duplicate Import Consolidation**: Merges multiple import lines from same module into single optimized statements  
- **Missing Import Detection**: Identifies Module::symbol references requiring additional imports (planned)
- **Optimized Import Generation**: Alphabetical sorting and clean formatting of import statements
- **Performance Optimized**: Fast analysis suitable for real-time LSP code actions

## API Reference

### Core ImportOptimizer methods
```rust
impl ImportOptimizer {
    /// Create new optimizer instance
    pub fn new() -> Self;
    
    /// Analyze Perl file for import optimization opportunities
    pub fn analyze_file(&self, path: &Path) -> Result<ImportAnalysis, ImportOptimizerError>;
    
    /// Generate optimized import statements from analysis
    pub fn generate_optimized_imports(&self, analysis: &ImportAnalysis) -> String;
}

// ImportAnalysis structure
pub struct ImportAnalysis {
    pub unused_imports: Vec<UnusedImport>,
    pub duplicate_imports: Vec<DuplicateImport>,
    pub missing_imports: Vec<MissingImport>, // planned
}
```

## Tutorial: Using Import Optimization

### Step 1: Create optimizer
```rust
use perl_parser::import_optimizer::ImportOptimizer;
use std::path::Path;

let optimizer = ImportOptimizer::new();
```

### Step 2: Analyze a Perl file for import issues
```rust
let analysis = optimizer.analyze_file(Path::new("script.pl"))?;
```

### Step 3: Check for unused imports
```rust
for unused in &analysis.unused_imports {
    println!("Module {} has unused symbols: {:?}", unused.module, unused.symbols);
}
```

### Step 4: Check for duplicate imports
```rust
for duplicate in &analysis.duplicate_imports {
    println!("Module {} imported {} times", duplicate.module, duplicate.count);
}
```

### Step 5: Generate optimized imports
```rust
let optimized = optimizer.generate_optimized_imports(&analysis);
println!("Optimized imports:\n{}", optimized);
```

## Testing

### Test Commands
```bash
# Run all import optimizer tests
cargo test -p perl-parser --test import_optimizer_tests

# Test specific optimization scenarios
cargo test -p perl-parser import_optimizer_tests::test_unused_import_detection
cargo test -p perl-parser import_optimizer_tests::test_duplicate_consolidation
cargo test -p perl-parser import_optimizer_tests::test_optimized_generation

# Integration testing with LSP code actions
cargo test -p perl-parser --test lsp_code_actions_tests -- import_optimization
```

## Integration with Code Actions

### Adding New Refactorings
```rust
// In code_actions_enhanced.rs
fn your_refactoring(&self, node: &Node) -> Option<CodeAction> {
    // 1. Check if refactoring applies
    // 2. Generate new code
    // 3. Return CodeAction with TextEdits
}
```

The import optimizer integrates seamlessly with the LSP code actions system to provide real-time import optimization suggestions.
# Workspace Refactoring Tutorial (**Diataxis: Tutorial**)

This tutorial provides step-by-step guidance on using the comprehensive workspace refactoring capabilities introduced in v0.8.8. These features enable safe cross-file refactoring operations for Perl codebases.

## Prerequisites

Before starting this tutorial, ensure you have:
- tree-sitter-perl v0.8.8 or later installed
- A Perl workspace with multiple files indexed
- Basic understanding of Rust and LSP concepts

## Tutorial Overview

You'll learn to:
1. **Set up workspace refactoring** - Initialize and configure the system
2. **Rename symbols across files** - Safely rename variables and functions
3. **Extract code into modules** - Break up large files into manageable modules
4. **Optimize import statements** - Clean up and organize workspace imports
5. **Move subroutines between files** - Reorganize code architecture
6. **Inline variables** - Simplify code by removing temporary variables

## Step 1: Setting Up Workspace Refactoring

### Initialize the Refactoring System

```rust
use perl_parser::workspace_refactor::WorkspaceRefactor;
use perl_parser::workspace_index::WorkspaceIndex;
use std::path::Path;

// Create a workspace index
let mut index = WorkspaceIndex::new();

// Index your Perl files
let files = vec![
    ("src/main.pl", r#"
use strict;
use warnings;
use Utils;

my $config = load_config();
my $processor = Utils::create_processor($config);
process_files($processor);

sub load_config {
    return { debug => 1, timeout => 30 };
}
"#),
    ("src/Utils.pm", r#"
package Utils;
use strict;
use warnings;

sub create_processor {
    my ($config) = @_;
    return ProcessingEngine->new($config);
}

sub validate_input {
    my ($data) = @_;
    return defined $data && length($data) > 0;
}

1;
"#),
];

// Index all files in the workspace
for (file_path, content) in files {
    index.index_file_str(file_path, content)?;
}

// Create the refactoring provider
let refactor = WorkspaceRefactor::new(index);
```

**What you've accomplished:**
- ✅ Created a WorkspaceIndex to track symbols across files
- ✅ Indexed multiple Perl files with their content
- ✅ Initialized the WorkspaceRefactor system

## Step 1.5: Understanding Dual Function Call Indexing (v0.8.8+) (**Diataxis: Tutorial**)

Before diving into refactoring operations, let's understand the enhanced dual indexing strategy that makes comprehensive cross-file navigation possible:

### How Dual Indexing Works

When you index your workspace, the system now creates two index entries for every function call:

```rust
// Given this Perl code:
// File: src/Utils.pm
// package Utils;
// sub validate_input { ... }

// File: src/main.pl  
// use Utils;
// my $result = validate_input($data);  # Bare call
// my $result2 = Utils::validate_input($data);  # Qualified call

// The workspace index creates these entries:
// 1. "validate_input" -> [reference at main.pl:3, reference at main.pl:4]
// 2. "Utils::validate_input" -> [reference at main.pl:3, reference at main.pl:4]
```

### Testing Dual Indexing

Let's verify that dual indexing is working for our example workspace:

```rust
// Test dual indexing functionality
println!("=== Testing Dual Function Call Indexing ===");

// Find references using bare name
let bare_refs = index.find_references("create_processor");
println!("References to 'create_processor': {}", bare_refs.len());

// Find references using qualified name  
let qualified_refs = index.find_references("Utils::create_processor");
println!("References to 'Utils::create_processor': {}", qualified_refs.len());

// Both should find the same call from main.pl:
// Utils::create_processor($config)
assert_eq!(bare_refs.len(), qualified_refs.len());
```

### Benefits for Refactoring

This dual indexing strategy provides several advantages for workspace refactoring:

1. **Complete Reference Detection**: Find all function calls whether they use bare names or qualified names
2. **Safe Renaming**: When renaming functions, all call sites are updated automatically
3. **Package-Aware Operations**: The system understands package contexts and handles qualified calls correctly
4. **Deduplication**: References found via both indexing methods are automatically deduplicated

### Verifying Index Quality

```rust
// Verify that your workspace is properly indexed
let stats = index.get_statistics();
println!("Workspace Index Statistics:");
println!("  Files indexed: {}", stats.file_count);
println!("  Total symbols: {}", stats.symbol_count);  
println!("  Function calls: {}", stats.function_call_count);
println!("  Dual index entries: {}", stats.dual_index_entries);

// Check for common indexing issues
if stats.dual_index_entries < stats.function_call_count {
    println!("⚠️  Warning: Some function calls may not be dual-indexed");
    println!("💡 This could affect refactoring accuracy");
}
```

**What you've learned:**
- ✅ Function calls are indexed both as bare names and qualified names
- ✅ This enables comprehensive reference finding across packages
- ✅ Refactoring operations benefit from complete call site detection
- ✅ The system automatically handles deduplication and package contexts

## Step 2: Cross-File Symbol Renaming

### Rename a Variable Across Multiple Files

Let's rename the `$config` variable to `$settings` across all files:

```rust
// Perform cross-file symbol rename
let result = refactor.rename_symbol(
    "$config",              // Old name
    "$settings",            // New name  
    Path::new("src/main.pl"), // File where rename initiated
    (0, 0)                  // Position (currently unused)
)?;

println!("Rename operation: {}", result.description);
println!("Files affected: {}", result.file_edits.len());

// Review the changes before applying
for file_edit in &result.file_edits {
    println!("File: {:?}", file_edit.file_path);
    for edit in &file_edit.edits {
        println!("  Replace bytes {}..{} with '{}'", 
                edit.start, edit.end, edit.new_text);
    }
}

// Apply the changes (in real implementation)
apply_refactor_result(result)?;
```

**Expected Output:**
```
Rename operation: Rename '$config' to '$settings'
Files affected: 2
File: "src/main.pl"
  Replace bytes 45..52 with '$settings'
  Replace bytes 89..96 with '$settings'
File: "src/Utils.pm"
  Replace bytes 156..163 with '$settings'
```

### Rename a Subroutine with Validation

```rust
// Rename a function with comprehensive error handling
match refactor.rename_symbol("create_processor", "build_processor", Path::new("src/Utils.pm"), (0, 0)) {
    Ok(result) => {
        println!("✅ Rename successful: {}", result.description);
        
        // Check for warnings
        if !result.warnings.is_empty() {
            for warning in &result.warnings {
                println!("⚠️  Warning: {}", warning);
            }
        }
        
        apply_refactor_result(result)?;
    }
    Err(e) => {
        eprintln!("❌ Rename failed: {}", e);
        match e {
            RefactorError::InvalidInput(msg) => {
                eprintln!("Input validation error: {}", msg);
            }
            RefactorError::SymbolNotFound { symbol, file } => {
                eprintln!("Symbol '{}' not found in {}", symbol, file);
            }
            _ => eprintln!("Other error: {}", e),
        }
    }
}
```

## Step 3: Extracting Code into New Modules

### Extract Utility Functions into a Separate Module

Let's extract some functions from main.pl into a new Configuration module:

```rust
// Extract lines containing configuration logic
let result = refactor.extract_module(
    Path::new("src/main.pl"),  // Source file
    8,                         // Start line (1-based)
    12,                        // End line (1-based, inclusive)
    "Configuration"            // New module name
)?;

println!("Extraction: {}", result.description);

// This will:
// 1. Create src/Configuration.pm with the extracted code
// 2. Replace the extracted lines in main.pl with "use Configuration;"
for file_edit in &result.file_edits {
    println!("Editing: {:?}", file_edit.file_path);
    if file_edit.file_path.to_string_lossy().ends_with("Configuration.pm") {
        println!("  📦 New module created");
    } else {
        println!("  ✂️  Code extracted and replaced with use statement");
    }
}
```

**Result:** 
- **main.pl**: Configuration code replaced with `use Configuration;`
- **Configuration.pm**: New file created with extracted code

### Extract with Dependency Analysis

```rust
// More complex extraction with validation
let result = refactor.extract_module(
    Path::new("src/large_module.pl"),
    50, 100,           // Extract lines 50-100
    "ExtractedUtils"   // New module name
)?;

// Check for warnings about dependencies
if !result.warnings.is_empty() {
    println!("⚠️ Extraction completed with warnings:");
    for warning in &result.warnings {
        println!("  • {}", warning);
    }
}
```

## Step 4: Workspace Import Optimization

### Clean Up Import Statements Across All Files

```rust
// Optimize imports across the entire workspace
let result = refactor.optimize_imports()?;

println!("Import optimization: {}", result.description);
println!("Files processed: {}", result.file_edits.len());

// Review what will be optimized
for file_edit in &result.file_edits {
    println!("Optimizing imports in: {:?}", file_edit.file_path);
    for edit in &file_edit.edits {
        println!("  Replacing import block with optimized version");
    }
}

// This will:
// - Remove duplicate imports
// - Sort imports alphabetically  
// - Consolidate imports from the same module
// - Maintain clean, consistent formatting
```

**Before optimization:**
```perl
use Data::Dumper;
use strict;
use Data::Dumper qw(Dumper);
use warnings;
use List::Util;
use Data::Dumper;  # Duplicate
```

**After optimization:**
```perl
use Data::Dumper;
use List::Util;
use strict;
use warnings;
```

## Step 5: Moving Subroutines Between Files

### Move a Function to a Different Module

```rust
// Move a utility function to the Utils module
let result = refactor.move_subroutine(
    "validate_input",       // Function to move
    Path::new("src/main.pl"), // Source file
    "Utils"                 // Target module (without .pm)
)?;

println!("Subroutine movement: {}", result.description);

// This operation:
// 1. Removes the subroutine from main.pl
// 2. Appends it to Utils.pm
// 3. Preserves all formatting and comments
for file_edit in &result.file_edits {
    if file_edit.edits.iter().any(|e| e.new_text.is_empty()) {
        println!("  ✂️ Removed from: {:?}", file_edit.file_path);
    } else {
        println!("  📥 Added to: {:?}", file_edit.file_path);
    }
}
```

### Move with Error Handling

```rust
// Attempt to move a non-existent subroutine
match refactor.move_subroutine("nonexistent_function", Path::new("main.pl"), "Utils") {
    Ok(result) => {
        println!("Move successful: {}", result.description);
    }
    Err(RefactorError::SymbolNotFound { symbol, file }) => {
        println!("❌ Function '{}' not found in {}", symbol, file);
        println!("💡 Available functions:");
        // In a real implementation, you could list available functions
    }
    Err(e) => {
        println!("❌ Move failed: {}", e);
    }
}
```

## Step 6: Variable Inlining

### Inline a Temporary Variable

```rust
// Inline a temporary variable with its definition
let result = refactor.inline_variable(
    "$temp",                  // Variable to inline
    Path::new("src/processor.pl"), // File containing the variable
    (0, 0)                    // Position (currently unused)
)?;

println!("Variable inlining: {}", result.description);

// This will replace all occurrences of $temp with its initializer expression
```

**Before inlining:**
```perl
my $temp = get_user_input();
my $processed = validate_and_clean($temp);
log_processing($temp);
```

**After inlining:**
```perl
my $processed = validate_and_clean(get_user_input());
log_processing(get_user_input());
```

### Handle Inlining Errors

```rust
// Attempt to inline a variable without an initializer
match refactor.inline_variable("$var_no_init", Path::new("test.pl"), (0, 0)) {
    Ok(result) => {
        println!("Inlining successful: {}", result.description);
    }
    Err(RefactorError::ParseError(msg)) => {
        println!("❌ Cannot inline: {}", msg);
        println!("💡 Variable may not have an initializer or may be declared without assignment");
    }
    Err(e) => {
        println!("❌ Inlining failed: {}", e);
    }
}
```

## Step 7: Unicode and International Support

### Working with Unicode Variable Names

The refactoring system fully supports Unicode characters in Perl code:

```rust
// Set up a file with Unicode content
let unicode_content = r#"
use utf8;
use strict;
use warnings;

my $♥ = "love";      # Unicode variable name
my $données = 42;    # French accents
my $表达式 = "测试";  # Chinese characters

print "Value: $♥\n";
print "Data: $données\n";  
print "Expression: $表达式\n";
"#;

// Index the Unicode content
index.index_file_str("src/unicode_example.pl", unicode_content)?;

// Rename Unicode variables safely
let result = refactor.rename_symbol("$♥", "$love", Path::new("src/unicode_example.pl"), (0, 0))?;
println!("✅ Unicode rename successful: {}", result.description);

// Extract module with Unicode content  
let result = refactor.extract_module(
    Path::new("src/unicode_example.pl"),
    6, 8,
    "国际化工具"  // Unicode module name
)?;
println!("✅ Unicode module extraction successful");
```

## Step 8: Advanced Error Handling and Validation

### Comprehensive Error Handling Pattern

```rust
use perl_parser::workspace_refactor::{RefactorError, RefactorResult};

fn handle_refactor_operation<F>(operation: F) -> Result<(), Box<dyn std::error::Error>>
where
    F: FnOnce() -> Result<RefactorResult, RefactorError>,
{
    match operation() {
        Ok(result) => {
            println!("✅ Operation successful: {}", result.description);
            
            // Display warnings if any
            if !result.warnings.is_empty() {
                println!("⚠️ Warnings:");
                for warning in &result.warnings {
                    println!("  • {}", warning);
                }
            }
            
            // Apply changes
            apply_refactor_result(result)?;
            Ok(())
        }
        Err(RefactorError::InvalidInput(msg)) => {
            eprintln!("❌ Invalid input: {}", msg);
            eprintln!("💡 Please check your parameters and try again");
            Err(msg.into())
        }
        Err(RefactorError::DocumentNotIndexed(file)) => {
            eprintln!("❌ File not indexed: {}", file);
            eprintln!("💡 Make sure the file is part of your workspace");
            Err(format!("Document not indexed: {}", file).into())
        }
        Err(RefactorError::SymbolNotFound { symbol, file }) => {
            eprintln!("❌ Symbol '{}' not found in {}", symbol, file);
            eprintln!("💡 Check the symbol name and file location");
            Err(format!("Symbol not found: {}", symbol).into())
        }
        Err(RefactorError::UriConversion(msg)) => {
            eprintln!("❌ File path error: {}", msg);
            eprintln!("💡 Check file paths are valid and accessible");
            Err(msg.into())
        }
        Err(e) => {
            eprintln!("❌ Refactoring failed: {}", e);
            Err(e.into())
        }
    }
}

// Usage example
handle_refactor_operation(|| {
    refactor.rename_symbol("$old_var", "$new_var", Path::new("test.pl"), (0, 0))
})?;
```

## Step 9: Performance Optimization for Large Codebases

### Batch Processing for Large Workspaces

```rust
use std::time::{Duration, Instant};
use std::thread;

// For very large codebases, use batch processing
fn optimize_large_workspace(refactor: &WorkspaceRefactor) -> Result<(), Box<dyn std::error::Error>> {
    let start_time = Instant::now();
    
    // The system includes built-in performance safeguards:
    // - 1000 match limit per operation
    // - 500 file limit for workspace operations  
    // - Pre-filtering to avoid processing files without target strings
    
    // Optimize imports with rate limiting
    println!("🔄 Optimizing imports across workspace...");
    let result = refactor.optimize_imports()?;
    
    // Rate limiting for very large operations
    thread::sleep(Duration::from_millis(100));
    
    println!("✅ Processed {} files in {:?}", result.file_edits.len(), start_time.elapsed());
    
    // Apply changes in batches for large results
    for chunk in result.file_edits.chunks(10) {
        apply_file_edit_batch(chunk)?;
        // Small delay between batches
        thread::sleep(Duration::from_millis(50));
    }
    
    Ok(())
}
```

## Step 10: Testing Your Refactoring Operations

### Validate Refactoring Results

```rust
#[cfg(test)]
mod refactoring_tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    #[test]
    fn test_symbol_rename_workflow() -> Result<(), Box<dyn std::error::Error>> {
        // Set up temporary workspace
        let temp_dir = tempdir()?;
        let file_path = temp_dir.path().join("test.pl");
        
        let initial_content = "my $old_name = 42;\nprint $old_name;\n";
        fs::write(&file_path, initial_content)?;
        
        // Create index and refactor provider
        let mut index = WorkspaceIndex::new();
        index.index_file_str(file_path.to_str().unwrap(), initial_content)?;
        let refactor = WorkspaceRefactor::new(index);
        
        // Perform rename
        let result = refactor.rename_symbol("$old_name", "$new_name", &file_path, (0, 0))?;
        
        // Validate result
        assert!(!result.file_edits.is_empty(), "Should have file edits");
        assert!(result.description.contains("old_name"), "Description should mention old name");
        assert!(result.description.contains("new_name"), "Description should mention new name");
        
        // Apply and verify
        apply_refactor_result(result)?;
        let final_content = fs::read_to_string(&file_path)?;
        assert!(final_content.contains("$new_name"), "Should contain new variable name");
        assert!(!final_content.contains("$old_name"), "Should not contain old variable name");
        
        Ok(())
    }
    
    #[test]
    fn test_unicode_variable_rename() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        let file_path = temp_dir.path().join("unicode.pl");
        
        let unicode_content = "my $♥ = 'love';\nprint $♥;\n";
        fs::write(&file_path, unicode_content)?;
        
        let mut index = WorkspaceIndex::new();
        index.index_file_str(file_path.to_str().unwrap(), unicode_content)?;
        let refactor = WorkspaceRefactor::new(index);
        
        // Test Unicode symbol rename
        let result = refactor.rename_symbol("$♥", "$love", &file_path, (0, 0))?;
        
        assert!(!result.file_edits.is_empty(), "Unicode rename should work");
        assert!(result.description.contains("♥"), "Description should contain Unicode char");
        
        Ok(())
    }
}

// Helper function for testing
fn apply_refactor_result(result: RefactorResult) -> Result<(), Box<dyn std::error::Error>> {
    for file_edit in result.file_edits {
        // Read current content
        let mut content = if file_edit.file_path.exists() {
            std::fs::read_to_string(&file_edit.file_path)?
        } else {
            String::new()
        };
        
        // Apply edits in reverse order to maintain positions
        for edit in file_edit.edits.iter().rev() {
            if edit.start <= content.len() && edit.end <= content.len() {
                content.replace_range(edit.start..edit.end, &edit.new_text);
            }
        }
        
        // Write updated content
        std::fs::write(&file_edit.file_path, content)?;
    }
    Ok(())
}

fn apply_file_edit_batch(edits: &[FileEdit]) -> Result<(), Box<dyn std::error::Error>> {
    for edit in edits {
        // Apply individual file edit
        apply_refactor_result(RefactorResult {
            file_edits: vec![edit.clone()],
            description: "Batch edit".to_string(),
            warnings: vec![],
        })?;
    }
    Ok(())
}
```

## Conclusion

You've learned how to use the comprehensive workspace refactoring capabilities in tree-sitter-perl v0.8.8:

✅ **Symbol Renaming**: Cross-file variable and function renaming with validation  
✅ **Module Extraction**: Breaking large files into manageable modules  
✅ **Import Optimization**: Cleaning up and organizing import statements  
✅ **Subroutine Movement**: Reorganizing code architecture between files  
✅ **Variable Inlining**: Simplifying code by removing temporary variables  
✅ **Unicode Support**: Full international character support in all operations  
✅ **Error Handling**: Comprehensive validation and error recovery  
✅ **Performance**: Efficient processing for large codebases  

## Next Steps

- **Integrate with your editor**: Set up LSP integration for real-time refactoring
- **Automate workflows**: Create scripts for common refactoring patterns
- **Extend functionality**: Consider contributing additional refactoring operations
- **Performance tuning**: Optimize for your specific codebase characteristics

## Additional Resources

- [LSP_ACTUAL_STATUS.md](../LSP_ACTUAL_STATUS.md) - Complete feature status and capabilities
- [ARCHITECTURE.md](../ARCHITECTURE.md) - Technical architecture and design decisions  
- [LSP_IMPLEMENTATION_GUIDE.md](LSP_IMPLEMENTATION_GUIDE.md) - Implementation patterns and best practices
- [Workspace Refactor API Documentation](../crates/perl-parser/src/workspace_refactor.rs) - Complete API reference

---

**Note**: This tutorial provides comprehensive guidance for the workspace refactoring system. All examples include proper error handling. The 19 test cases ensure reliability across all supported operations.
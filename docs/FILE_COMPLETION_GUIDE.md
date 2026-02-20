# File Path Completion Guide

## Overview

The LSP server includes comprehensive file path completion with security and performance features.

## Core Architecture

The file completion system activates automatically when editing string literals that contain path-like content:

**Detection Logic**:
- **Context-aware activation**: Triggers inside quoted strings (`"path/to/file"` or `'path/to/file'`)
- **Path pattern recognition**: Detects `/` separators or alphanumeric file patterns
- **Smart filtering**: Only suggests files matching the current prefix

**Security Architecture**:
- **Path traversal prevention**: Blocks `../` patterns and absolute paths (except `/`)
- **Null byte protection**: Rejects strings containing `\0` characters
- **Reserved name filtering**: Prevents Windows reserved names (CON, PRN, AUX, etc.)
- **Filename validation**: UTF-8 validation, length limits (255 chars), control character filtering
- **Directory safety**: Canonicalization with safe fallbacks, hidden file filtering

## Tutorial: Using File Path Completion

### Step 1: Basic File Completion
```perl
# Type a string with path content and trigger completion
my $config_file = "config/app."; # <-- Press Ctrl+Space here
# Suggests: config/app.yaml, config/app.json, config/app.toml
```

### Step 2: Directory Navigation
```perl
# Navigate through directory structures
my $lib_file = "src/"; # <-- Completion shows src/ contents
# Shows: src/completion.rs, src/parser.rs, src/lib.rs
```

### Step 3: File Type Recognition
```perl
# Get intelligent file type information
my $script = "scripts/deploy."; # <-- Shows file types in completion details
# deploy.pl (Perl file), deploy.py (Python file), deploy.sh (file)
```

## Configuration Guide

### Enable/Disable File Completion
File completion is automatically enabled and cannot be disabled—it only activates in appropriate string contexts.

### Performance Tuning
The system includes built-in performance safeguards:
- **Max results**: 50 completions per request  
- **Max depth**: 1 level directory traversal
- **Max entries**: 200 filesystem entries examined
- **Cancellation support**: Respects LSP cancellation requests

### File Filtering
The system automatically excludes:
- Hidden files (starting with `.`)
- System directories (`node_modules`, `.git`, `target`, `build`)
- Cache directories (`__pycache__`, `.pytest_cache`, `.mypy_cache`)

## API Reference

### LSP Integration Points
```rust
// Core completion provider with file support
impl CompletionProvider {
    pub fn get_completions_with_path_cancellable(
        &self,
        source: &str,
        position: usize,
        filepath: Option<&str>,
        is_cancelled: &dyn Fn() -> bool,
    ) -> Vec<CompletionItem>;
}

// Security validation methods
fn sanitize_path(&self, path: &str) -> Option<String>;
fn is_safe_filename(&self, filename: &str) -> bool;
fn is_hidden_or_forbidden(&self, entry: &walkdir::DirEntry) -> bool;
```

### File Type Mappings
```rust
let file_type_desc = match extension.to_lowercase().as_str() {
    "pl" | "pm" | "t" => "Perl file",
    "rs" => "Rust source file", 
    "js" => "JavaScript file",
    "py" => "Python file",
    "txt" => "Text file",
    "md" => "Markdown file", 
    "json" => "JSON file",
    "yaml" | "yml" => "YAML file",
    "toml" => "TOML file",
    _ => "file",
};
```

### Performance Limits
- **Max results**: 50 completions
- **Max depth**: 1 directory level
- **Max entries examined**: 200 filesystem entries
- **Path length limit**: 1024 characters
- **Filename length limit**: 255 characters

### Security Features
- Path traversal prevention (`../` blocked)
- Null byte detection (`\0` blocked)
- Windows reserved name filtering
- Symbolic link traversal disabled  
- Hidden file exclusion
- Control character filtering

## Testing

### Test Commands
```bash
# Run file completion specific tests
cargo test -p perl-parser --test file_completion_tests

# Test individual scenarios
cargo test -p perl-parser file_completion_tests::completes_files_in_src_directory
cargo test -p perl-parser file_completion_tests::basic_security_test_rejects_path_traversal

# Test with various file patterns
cargo test -p perl-parser --test lsp_comprehensive_e2e_test -- test_completion
```

### Manual Testing Examples
```perl
# Test cases for manual validation
my $test1 = "src/comp";           # Should complete to src/completion.rs
my $test2 = "tests/";             # Should show tests/ directory contents  
my $test3 = "Cargo";              # Should complete to Cargo.toml, Cargo.lock
my $test4 = "../etc/passwd";      # Should NOT provide completions (security)
```
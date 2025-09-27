# Contributing to Perl LSP Development

## Quick Start for Contributors

This guide helps you contribute to the Perl LSP following our [Development Roadmap](LSP_DEVELOPMENT_ROADMAP.md).

## Current Status (February 2024)
- **Phase**: 1 - Foundation & Quick Wins
- **Priority**: Document Highlight, Test Conversion
- **Tests**: 34 real / 520 total (6.5% coverage)

## How to Contribute

### 1. API Documentation Requirements ⭐ **NEW: PR #160 (SPEC-149)**

**All contributions to perl-parser crate must include comprehensive API documentation** due to enforced `#![warn(missing_docs)]`:

#### Documentation Standards for Contributors
- **Public APIs**: Must include comprehensive documentation with examples
- **LSP Workflow Integration**: Document role in Parse → Index → Navigate → Complete → Analyze pipeline
- **Performance Critical Functions**: Document memory usage and large Perl file processing characteristics
- **Error Types**: Explain Perl parsing context and recovery strategies

#### Quick Documentation Checklist
```rust
/// Brief summary (1 sentence)
///
/// Detailed description with LSP workflow context
///
/// # Arguments
/// * `param` - Description with type and purpose
///
/// # Returns
/// * `Ok(Type)` - Success case description
/// * `Err(Error)` - Error conditions and recovery
///
/// # Examples
/// ```rust
/// use perl_parser::YourAPI;
/// let result = your_function(input)?;
/// assert!(result.is_valid());
/// ```
pub fn your_function(param: Type) -> Result<ReturnType, Error> {
    // Implementation
}
```

#### Validation Commands
```bash
# Validate your documentation before submitting
cargo test -p perl-parser --test missing_docs_ac_tests
cargo doc --no-deps --package perl-parser  # Should compile without warnings
```

For complete documentation standards, see [API Documentation Standards](docs/API_DOCUMENTATION_STANDARDS.md).

### 2. Pick a Task
Check our priority list and pick an unassigned task:

#### 🔴 High Priority (Great First Issues)
- [ ] Implement `textDocument/documentHighlight` 
- [ ] Convert mock tests to real tests (pick any file)
- [ ] Add document lifecycle events

#### 🟡 Good First Test Conversions
Start with these simpler test files:
- `lsp_code_lens_reference_test.rs` (2 tests)
- `lsp_execute_command_tests.rs` (4 tests)
- `lsp_golden_tests.rs` (6 tests)

### 2. Test Conversion Guide

#### Step 1: Identify Mock Tests
Look for these patterns:
```rust
// Mock patterns to replace:
- MockIO
- ExtendedTestContext
- print!/println! statements
- todo!() or unimplemented!()
- Tests that don't assert anything
```

#### Step 2: Convert to Real Test
Use this template:

```rust
use perl_parser::LspServer;
use serde_json::json;

#[test]
fn test_your_feature() {
    // 1. Create real server
    let mut server = LspServer::new();
    
    // 2. Initialize
    let init_response = server.handle_request(json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": {
            "capabilities": {}
        },
        "id": 1
    }));
    
    // 3. Open document
    server.handle_notification(json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///test.pl",
                "languageId": "perl",
                "version": 1,
                "text": "my $x = 42;\nprint $x;"
            }
        }
    }));
    
    // 4. Make request
    let response = server.handle_request(json!({
        "jsonrpc": "2.0",
        "method": "textDocument/yourMethod",
        "params": {
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 0, "character": 4 }
        },
        "id": 2
    }));
    
    // 5. Assert real behavior
    assert!(response.is_some());
    let result = response.unwrap();
    assert!(result["result"].is_array());
    // Add specific assertions based on expected behavior
}
```

#### Step 3: Verify Test Quality
Ensure your test:
- ✅ Creates a real `LspServer` instance
- ✅ Sends actual LSP protocol messages
- ✅ Asserts on real responses (not just "is_some")
- ✅ Tests both success and error cases
- ✅ Has descriptive test name and comments

### 3. Implementing New LSP Features

#### Example: Document Highlight
Here's how to implement a missing feature:

```rust
// In lsp_server.rs

fn handle_document_highlight(&self, params: Option<Value>) -> Result<Option<Value>, JsonRpcError> {
    let params = params.ok_or(JsonRpcError::invalid_params("Missing params"))?;
    
    let uri = params["textDocument"]["uri"].as_str()
        .ok_or(JsonRpcError::invalid_params("Missing uri"))?;
    
    let position = parse_position(&params["position"])?;
    
    // Get document
    let doc = self.documents.get(uri)
        .ok_or(JsonRpcError::invalid_params("Document not found"))?;
    
    // Find symbol at position
    let symbol = find_symbol_at_position(&doc.ast, position);
    
    // Find all occurrences
    let highlights = find_symbol_occurrences(&doc.ast, symbol)
        .map(|occurrence| json!({
            "range": occurrence.range,
            "kind": if occurrence.is_write { 3 } else { 2 } // Write vs Read
        }))
        .collect::<Vec<_>>();
    
    Ok(Some(json!(highlights)))
}
```

### 4. Testing Your Changes

#### Run Specific Test
```bash
cargo test -p perl-parser test_your_feature
```

#### Run All Tests
```bash
cargo test -p perl-parser
```

#### Check Test Coverage
```bash
# Count real vs mock tests
grep -r "LspServer::new()" tests/ | wc -l  # Real tests
grep -r "MockIO\|ExtendedTestContext" tests/ | wc -l  # Mock tests
```

### 5. Submission Checklist

Before submitting a PR:

- [ ] Code follows Rust idioms and style
- [ ] All tests pass (`cargo test`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] Code is formatted (`cargo fmt`)
- [ ] New features have tests
- [ ] Mock tests converted to real tests
- [ ] Documentation updated if needed
- [ ] PR description explains changes

### 6. Getting Help

- **Questions**: Open a GitHub issue with `[Question]` prefix
- **Discussion**: Use GitHub Discussions
- **Bug Reports**: Open issue with reproduction steps
- **Feature Ideas**: Check roadmap first, then open issue

## Test Conversion Progress Tracker

Track your conversions here:

| File | Tests | Status | Assignee | PR |
|------|-------|--------|----------|-----|
| lsp_code_actions_test.rs | 5 | ✅ Done | - | - |
| lsp_comprehensive_e2e_test.rs | 25 | ✅ Done | - | - |
| lsp_integration_test.rs | 3 | 🔄 In Progress | - | - |
| lsp_code_actions_tests.rs | 9 | ⏳ Pending | - | - |
| lsp_completion_tests.rs | 17 | ⏳ Pending | - | - |
| ... | ... | ... | ... | ... |

## Code Quality Standards

### For Test Conversions
1. **No Mock Objects**: Use real `LspServer` instances
2. **Real Assertions**: Test actual behavior, not just existence
3. **Error Cases**: Include tests for error conditions
4. **Performance**: Tests should complete in <100ms

### For New Features
1. **LSP Compliance**: Follow LSP specification exactly
2. **Error Handling**: Graceful degradation on errors
3. **Performance**: <100ms response time target
4. **Memory**: Efficient memory usage, no leaks

## Recognition

Contributors who complete significant work will be:
- Listed in CONTRIBUTORS.md
- Mentioned in release notes
- Given credit in commit messages

## Priority Matrix for Contributors

```
Effort →
Low                           High
┌─────────────────────────────────┐
│ Document     │ Type            │ ↑
│ Highlight    │ Definition      │ 
│              │                 │ High
│ Test         │ Implementation  │ Impact
│ Conversions  │ Finding         │
│              │                 │ ↓
│ Save Events  │ Workspace       │ Low
│ Declaration  │ Management      │
└─────────────────────────────────┘
```

Start with **High Impact + Low Effort** tasks!

## Example PRs to Learn From

Look at these PRs for good examples:
- [#xxx - Document Highlight Implementation]
- [#xxx - Test Conversion: lsp_code_actions_test.rs]
- [#xxx - Added Document Save Events]

## Questions?

Feel free to ask! We're here to help you contribute successfully.
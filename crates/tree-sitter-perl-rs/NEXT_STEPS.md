# Next Steps for Pure Rust Perl Parser

## High Priority Tasks

### 1. Fix Pest Grammar for Complete Heredoc Support
- [ ] Update grammar to handle multiple heredocs in expressions (`print <<ONE, <<TWO;`)
- [ ] Add support for escaped heredoc syntax (`<<\EOF`)  
- [ ] Fix backtick heredoc parsing (`` <<`CMD` ``)
- [ ] Ensure heredoc nodes are created in all contexts (not just assignments)

### 2. Integrate Stateful Parser into Main Flow
- [ ] Add `stateful-parsing` feature flag to Cargo.toml
- [ ] Create `EnhancedPerlParser` that automatically uses stateful parsing when needed
- [ ] Update `pure_rust_parser.rs` to optionally use stateful preprocessing
- [ ] Add configuration option to enable/disable stateful parsing

### 3. Performance Optimizations
- [ ] Implement single-pass heredoc scanning (instead of current two-pass)
- [ ] Cache regex compilations in stateful parser
- [ ] Optimize line-by-line processing for large files
- [ ] Add benchmarks for stateful vs non-stateful parsing

### 4. Complete Parser Feature Parity
- [ ] Implement format strings (`format` and `write`)
- [ ] Add POD (Plain Old Documentation) parsing
- [ ] Support DATA and END sections properly
- [ ] Handle multi-line regex with `/x` modifier

### 5. Error Handling Improvements
- [ ] Better error messages for unterminated heredocs (show line number)
- [ ] Recover from heredoc errors without losing rest of file
- [ ] Add warnings for suspicious heredoc patterns
- [ ] Implement proper source location tracking through transformations

## Medium Priority Tasks

### 6. Testing Infrastructure
- [ ] Add fuzzing for stateful parser edge cases
- [ ] Create test corpus from real Perl modules (CPAN)
- [ ] Add property-based tests for heredoc transformations
- [ ] Performance regression tests

### 7. Documentation
- [ ] Document stateful parser architecture in detail
- [ ] Add examples of all supported heredoc variants
- [ ] Create migration guide from C parser to Rust parser
- [ ] Document limitations and workarounds

### 8. Integration Tasks
- [ ] Update comparison harness to use stateful parser
- [ ] Add stateful parser support to CLI tools
- [ ] Ensure Tree-sitter compatibility mode works with heredocs
- [ ] Update benchmarks to include stateful parsing

## Low Priority / Future Enhancements

### 9. Advanced Heredoc Features
- [ ] Support for stacked heredocs (heredoc within heredoc)
- [ ] Handle heredocs in string interpolation
- [ ] Support custom quote-like operators that can contain heredocs
- [ ] Implement proper precedence for heredoc operators

### 10. Tooling
- [ ] Create heredoc extraction tool
- [ ] Add heredoc linting (e.g., warn about missing terminators)
- [ ] Implement heredoc reformatting for code formatters
- [ ] Add heredoc syntax highlighting hints

## Implementation Order

1. **Start with Grammar Fixes** - This will enable more tests to pass
2. **Then Integration** - Make stateful parsing the default
3. **Then Optimization** - Once it's working correctly
4. **Finally Documentation** - Document the final implementation

## Code Locations

- Grammar: `src/grammar.pest`
- Parser: `src/pure_rust_parser.rs`
- Stateful: `src/stateful_parser.rs`
- Tests: `src/stateful_parser.rs` (test module)
- AST: `src/pure_rust_parser.rs` (AstNode enum)
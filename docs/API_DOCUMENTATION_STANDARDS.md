# API Documentation Standards - perl-parser crate

*Diataxis: How-to Guide* - Comprehensive API documentation requirements for production-quality perl-parser crate.

## Overview

As of **PR #160 (SPEC-149)**, the perl-parser crate enforces comprehensive API documentation through `#![warn(missing_docs)]` to maintain enterprise-grade code quality. This infrastructure enables systematic tracking and resolution of 603 identified documentation gaps across the codebase.

This guide provides detailed requirements and best practices for writing effective API documentation that meets the new enforcement standards.

## Quality Enforcement

### Missing Documentation Warnings

The perl-parser crate has `#![warn(missing_docs)]` enabled in `/crates/perl-parser/src/lib.rs`, which means:

- **All public items must have documentation**
- **CI builds will warn on missing documentation**
- **Comprehensive test suite validates documentation quality** (see `/crates/perl-parser/tests/missing_docs_ac_tests.rs`)

### Validation Infrastructure

- **Automated Tests**: 12 acceptance criteria covering all documentation requirements
- **CI Integration**: Documentation quality gates prevent regression
- **Quality Metrics**: Systematic tracking of documentation coverage and quality
- **Edge Case Detection**: Enhanced validation for malformed doctests, empty documentation, and invalid cross-references

## Documentation Requirements by Item Type

### 1. Public Structs and Enums

**Required Documentation**:
- **Purpose and role** in Perl parsing workflows
- **LSP workflow integration** (Parse → Index → Navigate → Complete → Analyze)
- **Usage context** and typical scenarios
- **Field explanations** for complex structures

**Example**:
```rust
/// Represents a parsed Perl structure in the LSP Parse stage.
///
/// This struct contains the extracted components from Perl source processing,
/// providing structured access to symbols, declarations, and syntax trees.
/// Used throughout the LSP workflow for indexing and navigation.
///
/// # LSP Workflow Integration
/// - **Parse**: Primary output structure from Perl parsing
/// - **Index**: Input for symbol indexing and workspace building
/// - **Navigate**: Source data for cross-file navigation analysis
///
/// # Performance Characteristics
/// - Memory usage: O(n) where n is source code size
/// - Optimized for large Perl codebase processing
/// - Zero-copy parsing where possible
///
/// # Examples
/// ```rust
/// use perl_parser::PerlStructure;
///
/// let parsed = PerlStructure::from_source(&perl_code)?;
/// assert!(!parsed.symbols.is_empty());
/// ```
pub struct PerlStructure {
    /// Package declaration, normalized for indexing
    pub package: String,
    /// Function symbols with position information
    pub symbols: Vec<Symbol>,
    /// Parsed AST for analysis
    pub ast: AstNode,
}
```

### 2. Public Functions

**Required Sections**:
- **Brief summary** (first line)
- **Detailed description** of functionality
- **# Arguments** - All parameters with types and purposes
- **# Returns** - Return value explanation
- **# Errors** - When function returns `Result<T, E>`
- **# Examples** - Working code with assertions
- **# Performance** - For performance-critical functions
- **Cross-references** to related functions

**Example**:
```rust
/// Parses Perl source code with optimized memory usage.
///
/// Performs high-performance parsing of Perl source into structured
/// representations. Optimized for enterprise-scale processing of
/// large Perl codebases with minimal memory overhead.
///
/// # Arguments
/// * `perl_source` - Raw Perl source code to parse
/// * `options` - Parsing configuration including memory limits
///
/// # Returns
/// * `Ok(PerlStructure)` - Successfully parsed Perl structure
/// * `Err(ParseError)` - When source is malformed or corrupted
///
/// # Errors
/// Returns `ParseError` when:
/// - Perl syntax is invalid or incomplete
/// - Memory limits are exceeded during parsing
/// - Unsupported Perl constructs are encountered
///
/// Recovery strategy: Use [`validate_perl_syntax`] for pre-validation.
///
/// # Performance
/// - Time complexity: O(n) where n is source code size
/// - Memory usage: O(log n) for parse tree construction
/// - Benchmark: 1-150µs per parse depending on complexity
/// - Scales linearly with codebase size
///
/// # Examples
/// ```rust
/// use perl_parser::{parse_perl_source, ParseOptions, PerlStructure};
///
/// let options = ParseOptions::default().with_memory_limit(1024 * 1024);
/// let parsed = parse_perl_source(&perl_source, options)?;
/// assert!(!parsed.package.is_empty());
/// ```
///
/// See also [`validate_perl_syntax`] for input validation and
/// [`PerlStructure::analyze`] for post-processing.
pub fn parse_perl_source(perl_source: &str, options: ParseOptions) -> Result<PerlStructure, ParseError> {
    // Implementation
}
```

### 3. Error Types

**Required Documentation**:
- **When the error occurs** in Perl parsing workflows
- **Workflow stage context** (Parse/Index/Navigate/Complete/Analyze)
- **Recovery strategies** and error handling guidance
- **Diagnostic information** available

**Example**:
```rust
/// Error that occurs during Perl source parsing in the LSP Parse stage.
///
/// This error indicates failure to parse or analyze Perl source code
/// during the initial Parse phase of the LSP workflow. Common causes
/// include invalid syntax, unsupported constructs, or memory limits.
///
/// # Perl Parsing Workflow Context
/// - **Parse Stage**: Primary error source during Perl parsing
/// - **Recovery**: Retry with different parsing options or partial parsing
/// - **Downstream Impact**: Prevents source from reaching Analyze stage
///
/// # Error Recovery
/// 1. Use [`validate_perl_syntax`] to pre-check syntax validity
/// 2. Adjust memory limits with `ParseOptions::with_memory_limit`
/// 3. Enable partial parsing with `ParseOptions::allow_partial`
/// 4. Log syntax errors for developer feedback
///
/// # Examples
/// ```rust
/// match parse_perl_source(&source, options) {
///     Ok(structure) => process_structure(structure),
///     Err(ParseError::SyntaxError { line, .. }) => {
///         log::warn!("Syntax error at line {}", line);
///         // Continue with partial parsing
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub enum ParseError {
    /// Perl syntax is invalid or malformed at the specified location
    SyntaxError {
        /// Line number where error was detected
        line: usize,
        /// Description of the syntax error
        details: String,
    },
    // Other variants...
}
```

### 4. Module-Level Documentation

**Required Content**:
- **//! Module purpose** and scope
- **LSP workflow integration** explanation
- **Architecture relationship** to other modules
- **Usage examples** for module functionality
- **Performance characteristics** for critical modules

**Example**:
```rust
//! High-performance Perl source parsing and analysis module.
//!
//! This module provides the core functionality for the Parse stage of the
//! LSP workflow. It handles parsing of Perl source code
//! with enterprise-scale performance optimizations for large codebase processing.
//!
//! # LSP Workflow Integration
//! - **Parse**: Primary module - converts Perl source to structured representations
//! - **Index**: Consumes PerlStructure outputs for symbol indexing
//! - **Navigate**: Uses parsed metadata for cross-file navigation
//! - **Complete**: Accesses parsed content for completion suggestions
//! - **Analyze**: Indexes parsed symbols and metadata for analysis
//!
//! # Performance Characteristics
//! - Memory usage: O(log n) for most operations
//! - Time complexity: O(n) linear parsing with n = source size
//! - Scaling: Tested up to large Perl codebases
//! - Throughput: 1-150µs per parse depending on complexity
//!
//! # Architecture Integration
//! - Uses [`crate::lexer`] for low-level Perl tokenization
//! - Integrates with [`crate::ast`] for structured representation
//! - Provides input to [`crate::semantic`] for analysis
//!
//! # Examples
//! ```rust
//! use perl_parser::parser::{PerlParser, ParseOptions};
//!
//! let parser = PerlParser::new();
//! let options = ParseOptions::default().with_memory_limit(1024 * 1024 * 100); // 100MB limit
//!
//! for file in parser.parse_perl_files(&workspace_path, options)? {
//!     match file {
//!         Ok(perl_structure) => {
//!             println!("Parsed file: {}", perl_structure.package);
//!             // Continue to Analyze stage
//!         }
//!         Err(parse_error) => {
//!             eprintln!("Failed to parse file: {}", parse_error);
//!             // Log and continue processing
//!         }
//!     }
//! }
//! ```
```

### 5. Performance-Critical APIs

**Additional Requirements** for modules like `incremental_v2.rs`, `workspace_index.rs`, `parser.rs`:
- **Time and space complexity** (Big O notation)
- **Memory usage patterns** and optimization strategies
- **Large Perl codebase processing** performance implications
- **Benchmark data** and performance characteristics

### 6. Complex APIs

**Additional Requirements** for modules like `completion.rs`, `diagnostics.rs`, `workspace_index.rs`:
- **Working usage examples** with realistic scenarios
- **LSP provider configuration** examples
- **Parser configuration** examples for different use cases
- **Integration patterns** with other components

## Documentation Style Guidelines

### Rust Best Practices

1. **Brief Summary First**: Start with one-line summary
2. **Section Headers**: Use `# Arguments`, `# Returns`, `# Errors`, `# Examples`
3. **Code Blocks**: Specify language with ```rust
4. **Cross-References**: Use `[`function_name`]` for same-module, `[`module::function`]` for cross-module
5. **Consistent Formatting**: Follow rustdoc conventions

### LSP-Specific Standards

1. **Workflow Context**: Always explain role in Parse → Index → Navigate → Complete → Analyze
2. **Performance Context**: Include memory and scaling implications for critical APIs
3. **Enterprise Context**: Reference large Perl codebase processing where relevant
4. **Error Context**: Explain recovery strategies and workflow impact

## Quality Validation

### Automated Testing

The comprehensive test suite at `/crates/perl-parser/tests/missing_docs_ac_tests.rs` validates:

- **AC1**: `#![warn(missing_docs)]` enabled and compiles successfully
- **AC2**: All public structs/enums have comprehensive documentation including LSP workflow role
- **AC3**: All public functions have complete documentation with required sections
- **AC4**: Performance-critical APIs document memory usage and large Perl codebase processing
- **AC5**: Module-level documentation explains purpose and LSP architecture relationship
- **AC6**: Complex APIs include working usage examples
- **AC7**: Doctests are present for critical functionality and pass `cargo test --doc`
- **AC8**: Error types document Perl parsing workflow context and recovery strategies
- **AC9**: Related functions include cross-references using Rust documentation linking
- **AC10**: Documentation follows Rust best practices with consistent style
- **AC11**: `cargo doc` generates complete documentation without warnings
- **AC12**: CI checks enforce missing_docs warnings for new public APIs

### Edge Case Detection

Enhanced validation detects and reports:

- **Malformed Doctests**: Unbalanced braces, empty code blocks, missing assertions
- **Empty Documentation**: Trivial or placeholder documentation strings
- **Invalid Cross-References**: Malformed links, empty references, syntax errors
- **Incomplete Performance Docs**: Missing complexity analysis, scaling info, benchmarks
- **Missing Error Recovery**: Insufficient error handling documentation

### Property-Based Testing

Systematic validation using property-based tests ensures:

- **Documentation Format Consistency**: Validates formatting across arbitrary inputs
- **Cross-Reference Validation**: Tests valid and invalid reference patterns
- **Doctest Structure Validation**: Ensures proper doctest construction

## CI Integration

### Quality Gates

- **Documentation Coverage**: All public APIs must have documentation
- **Style Validation**: Automated checking of documentation formatting
- **Doctest Execution**: All doctests must compile and pass
- **Cross-Reference Validation**: Links must resolve correctly

### Development Workflow

1. **Write Code**: Implement functionality with comprehensive documentation
2. **Run Tests**: `cargo test -p perl-parser --test missing_docs_ac_tests`
3. **Validate Docs**: `cargo doc --no-deps --package perl-parser`
4. **Check Progress**: Monitor documentation warning count reduction from current 603 baseline
5. **Check Style**: Automated validation through CI pipeline
6. **Review**: Ensure documentation meets all acceptance criteria

#### Implementation Strategy Reference

For systematic documentation completion, see [DOCUMENTATION_IMPLEMENTATION_STRATEGY.md](DOCUMENTATION_IMPLEMENTATION_STRATEGY.md) which outlines:

- **Phased approach** for addressing the 603 missing documentation warnings
- **Priority-based implementation** starting with critical parser infrastructure
- **Timeline and milestones** for complete documentation coverage
- **Quality standards** and validation procedures

## Troubleshooting

### Common Issues

1. **Missing Documentation Warning**: Add comprehensive documentation following the examples above
2. **Doctest Failures**: Ensure examples compile and include proper assertions
3. **Invalid Cross-References**: Use correct `[`function_name`]` syntax
4. **Style Violations**: Follow Rust documentation conventions with proper section headers

### Getting Help

- Review existing well-documented modules for examples
- Run the test suite to identify specific documentation gaps
- Check CI pipeline output for detailed validation feedback

## Summary

Comprehensive API documentation is a critical quality requirement for the perl-parser crate. Following these standards ensures:

- **Enterprise-grade code quality** with complete API coverage
- **Developer productivity** through clear usage examples and guidance
- **Maintainability** with well-documented architecture and design decisions
- **User success** with practical examples and troubleshooting guidance

For questions or clarification, refer to the test suite validation criteria and existing well-documented modules as examples.
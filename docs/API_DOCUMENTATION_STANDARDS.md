# API Documentation Standards - perl-parser crate

*Diataxis: How-to Guide* - Comprehensive API documentation requirements for production-quality perl-parser crate.

## Overview

As of **Issue #149**, the perl-parser crate enforces comprehensive API documentation through `#![warn(missing_docs)]` to maintain enterprise-grade code quality. This guide provides detailed requirements and best practices for writing effective API documentation.

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
- **Purpose and role** in email processing workflows
- **PSTX pipeline integration** (Extract → Normalize → Thread → Render → Index)
- **Usage context** and typical scenarios
- **Field explanations** for complex structures

**Example**:
```rust
/// Represents a parsed email structure in the PSTX Extract stage.
///
/// This struct contains the extracted components from PST email processing,
/// providing structured access to email metadata, headers, and content.
/// Used throughout the PSTX pipeline for normalization and threading.
///
/// # PSTX Pipeline Integration
/// - **Extract**: Primary output structure from PST parsing
/// - **Normalize**: Input for header standardization and cleanup
/// - **Thread**: Source data for conversation threading analysis
///
/// # Performance Characteristics
/// - Memory usage: O(n) where n is email content size
/// - Optimized for 50GB PST file processing
/// - Zero-copy parsing where possible
///
/// # Examples
/// ```rust
/// use perl_parser::EmailStructure;
///
/// let email = EmailStructure::from_pst_entry(&pst_data)?;
/// assert!(!email.subject.is_empty());
/// ```
pub struct EmailStructure {
    /// Email subject line, normalized for threading
    pub subject: String,
    /// Sender information with full contact details
    pub sender: ContactInfo,
    /// Email body content with MIME type preservation
    pub body: EmailBody,
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
/// Parses PST email content with optimized memory usage.
///
/// Performs high-performance parsing of PST file entries into structured
/// email representations. Optimized for enterprise-scale processing of
/// 50GB+ PST files with minimal memory overhead.
///
/// # Arguments
/// * `pst_data` - Raw PST entry data to parse
/// * `options` - Parsing configuration including memory limits
///
/// # Returns
/// * `Ok(EmailStructure)` - Successfully parsed email structure
/// * `Err(ParseError)` - When PST data is malformed or corrupted
///
/// # Errors
/// Returns `ParseError` when:
/// - PST data is corrupted or incomplete
/// - Memory limits are exceeded during parsing
/// - Unsupported PST format versions are encountered
///
/// Recovery strategy: Use [`validate_pst_entry`] for pre-validation.
///
/// # Performance
/// - Time complexity: O(n) where n is PST entry size
/// - Memory usage: O(log n) for parse tree construction
/// - Benchmark: 1-150µs per email depending on complexity
/// - Scales linearly with file size up to 50GB
///
/// # Examples
/// ```rust
/// use perl_parser::{parse_pst_email, ParseOptions, EmailStructure};
///
/// let options = ParseOptions::default().with_memory_limit(1024 * 1024);
/// let email = parse_pst_email(&pst_data, options)?;
/// assert!(!email.subject.is_empty());
/// ```
///
/// See also [`validate_pst_entry`] for input validation and
/// [`EmailStructure::normalize`] for post-processing.
pub fn parse_pst_email(pst_data: &[u8], options: ParseOptions) -> Result<EmailStructure, ParseError> {
    // Implementation
}
```

### 3. Error Types

**Required Documentation**:
- **When the error occurs** in email processing workflows
- **Pipeline stage context** (Extract/Normalize/Thread/Render/Index)
- **Recovery strategies** and error handling guidance
- **Diagnostic information** available

**Example**:
```rust
/// Error that occurs during PST email extraction in the PSTX Extract stage.
///
/// This error indicates failure to parse or extract email content from PST
/// files during the initial Extract phase of the PSTX pipeline. Common causes
/// include corrupted PST data, unsupported format versions, or memory limits.
///
/// # Email Processing Workflow Context
/// - **Extract Stage**: Primary error source during PST parsing
/// - **Recovery**: Retry with different parsing options or skip corrupted entries
/// - **Downstream Impact**: Prevents entry from reaching Normalize stage
///
/// # Error Recovery
/// 1. Use [`validate_pst_entry`] to pre-check data integrity
/// 2. Adjust memory limits with `ParseOptions::with_memory_limit`
/// 3. Enable partial parsing with `ParseOptions::allow_partial`
/// 4. Log corruption details for PST repair tools
///
/// # Examples
/// ```rust
/// match parse_pst_email(&data, options) {
///     Ok(email) => process_email(email),
///     Err(ParseError::CorruptedData { offset, .. }) => {
///         log::warn!("Corrupted PST data at offset {}", offset);
///         // Skip this entry and continue processing
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub enum ParseError {
    /// PST data is corrupted or malformed at the specified offset
    CorruptedData {
        /// Byte offset where corruption was detected
        offset: usize,
        /// Description of the corruption type
        details: String,
    },
    // Other variants...
}
```

### 4. Module-Level Documentation

**Required Content**:
- **//! Module purpose** and scope
- **PSTX pipeline integration** explanation
- **Architecture relationship** to other modules
- **Usage examples** for module functionality
- **Performance characteristics** for critical modules

**Example**:
```rust
//! High-performance PST email parsing and extraction module.
//!
//! This module provides the core functionality for the Extract stage of the
//! PSTX email processing pipeline. It handles parsing of Microsoft PST files
//! with enterprise-scale performance optimizations for 50GB+ file processing.
//!
//! # PSTX Pipeline Integration
//! - **Extract**: Primary module - converts PST binary data to structured email objects
//! - **Normalize**: Consumes EmailStructure outputs for header standardization
//! - **Thread**: Uses extracted metadata for conversation analysis
//! - **Render**: Accesses parsed content for presentation formatting
//! - **Index**: Indexes extracted text and metadata for search
//!
//! # Performance Characteristics
//! - Memory usage: O(log n) for most operations
//! - Time complexity: O(n) linear parsing with n = file size
//! - Scaling: Tested up to 50GB PST files
//! - Throughput: 1-150µs per email depending on complexity
//!
//! # Architecture Integration
//! - Uses [`crate::lexer`] for low-level PST tokenization
//! - Integrates with [`crate::ast`] for structured representation
//! - Provides input to [`crate::semantic`] for analysis
//!
//! # Examples
//! ```rust
//! use perl_parser::pst::{PstParser, ParseOptions};
//!
//! let parser = PstParser::new();
//! let options = ParseOptions::default().with_memory_limit(1024 * 1024 * 100); // 100MB limit
//!
//! for email in parser.parse_pst_file("large_mailbox.pst", options)? {
//!     match email {
//!         Ok(email_structure) => {
//!             println!("Parsed email: {}", email_structure.subject);
//!             // Continue to Normalize stage
//!         }
//!         Err(parse_error) => {
//!             eprintln!("Failed to parse email: {}", parse_error);
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
- **50GB PST processing** performance implications
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

### PSTX-Specific Standards

1. **Pipeline Context**: Always explain role in Extract → Normalize → Thread → Render → Index
2. **Performance Context**: Include memory and scaling implications for critical APIs
3. **Enterprise Context**: Reference 50GB PST processing where relevant
4. **Error Context**: Explain recovery strategies and workflow impact

## Quality Validation

### Automated Testing

The comprehensive test suite at `/crates/perl-parser/tests/missing_docs_ac_tests.rs` validates:

- **AC1**: `#![warn(missing_docs)]` enabled and compiles successfully
- **AC2**: All public structs/enums have comprehensive documentation including PSTX pipeline role
- **AC3**: All public functions have complete documentation with required sections
- **AC4**: Performance-critical APIs document memory usage and 50GB PST processing
- **AC5**: Module-level documentation explains purpose and PSTX architecture relationship
- **AC6**: Complex APIs include working usage examples
- **AC7**: Doctests are present for critical functionality and pass `cargo test --doc`
- **AC8**: Error types document email processing workflow context and recovery strategies
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
4. **Check Style**: Automated validation through CI pipeline
5. **Review**: Ensure documentation meets all acceptance criteria

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
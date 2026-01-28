//! Token stream facade for the core parser engine.
//!
//! Re-exports the tokenizer-backed stream used during the **Parse** stage of the
//! LSP workflow (Parse → Index → Navigate → Complete → Analyze). The stream
//! provides buffered lookahead while skipping trivia to keep parsing fast and
//! deterministic for large workspaces.
//!
//! # Performance Characteristics
//!
//! - **Time complexity**: O(n) over input tokens (single pass with bounded lookahead)
//! - **Space complexity**: O(n) for token storage plus a small lookahead buffer
//! - **Optimizations**: Efficient peek caching to avoid re-lexing hot paths
//! - **Benchmarks**: Typical tokenization stays in the low ms range for large files
//! - **Large-scale notes**: Designed to scale to large file sets (50GB PST-style
//!   workloads) without unbounded memory growth
//!
//! # Examples
//!
//! ```rust
//! use perl_parser_core::tokens::token_stream::TokenStream;
//!
//! let mut stream = TokenStream::new("my $x = 1;");
//! let _ = stream.peek();
//! ```

/// Token stream types used during the Parse stage of the workflow.
pub use perl_tokenizer::token_stream::*;

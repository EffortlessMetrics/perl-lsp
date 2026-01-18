//! LSP transport layer for perl-lsp.
//!
//! This crate provides the transport layer implementation for the Perl Language Server,
//! handling message framing according to the LSP Base Protocol specification.
//!
//! # Overview
//!
//! The LSP Base Protocol uses Content-Length based message framing over stdio (or other
//! transports). This crate provides:
//!
//! - [`read_message`] - Read and parse an LSP message with Content-Length framing
//! - [`write_message`] - Write an LSP response with proper framing
//! - [`write_notification`] - Write an LSP notification with proper framing
//! - [`log_response`] - Debug logging for outgoing responses
//!
//! # Example
//!
//! ```no_run
//! use std::io::{BufReader, stdin, stdout};
//! use perl_lsp_transport::{read_message, write_message};
//! use perl_lsp_protocol::JsonRpcResponse;
//!
//! let mut reader = BufReader::new(stdin());
//! let mut writer = stdout();
//!
//! // Read an incoming message
//! if let Ok(Some(request)) = read_message(&mut reader) {
//!     // Process request and create response
//!     let response = JsonRpcResponse::null(request.id);
//!
//!     // Write the response
//!     write_message(&mut writer, &response).unwrap();
//! }
//! ```

#![deny(unsafe_code)]
#![warn(missing_docs)]

mod framing;

pub use framing::{log_response, read_message, write_message, write_notification};

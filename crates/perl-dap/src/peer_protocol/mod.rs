//! The **Perl Debugger Peer Protocol** — a small, Content-Length-framed JSON
//! protocol spoken between `perl-dap` and an external debugger engine/frontend
//! (`Devel::ptkdb` first).
//!
//! It is deliberately *not* DAP. DAP is editor-facing and large; a debugger
//! engine like ptkdb should not have to implement it. This protocol is the
//! smaller seam: the peer says hello with its capabilities, emits
//! stopped/output/source-facts events, and optionally answers
//! stack/scopes/variables/evaluate and accepts breakpoint/step/continue
//! commands. `perl-dap` owns all DAP complexity and translates.
//!
//! # Layers
//!
//! - [`message`] — the request/response/event envelope (mirrors DAP's `type`-tagged shape).
//! - [`capabilities`] — the capability sets exchanged in the handshake.
//! - [`payloads`] — typed request-argument, response-body, and event-body structs.
//! - [`framing`] — Content-Length encode/decode, reusing `perl_lsp_rs_core` framing.
//!
//! # Wire types vs. the model
//!
//! The peer protocol has its own `camelCase` wire types (in [`payloads`]) rather
//! than serializing [`crate::model`] types directly. This keeps the internal
//! model's serialization independent of the wire contract, so the model can
//! evolve without breaking peers (decision: wire/model separation).

pub mod capabilities;
pub mod framing;
pub mod message;
pub mod payloads;

pub use capabilities::{HostReportedCapabilities, PeerReportedCapabilities};
pub use framing::{PeerFrameDecoder, PeerFrameError, encode_message};
pub use message::{PeerEvent, PeerMessage, PeerRequest, PeerResponse, command, event};

/// The protocol version string exchanged in the handshake. Consumers must reject
/// an unfamiliar version rather than guess.
pub const PROTOCOL_VERSION: &str = "perl-debug-peer-v1";

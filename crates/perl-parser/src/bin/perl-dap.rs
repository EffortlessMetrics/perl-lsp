//! Perl Debug Adapter Protocol (DAP) server
//!
//! This binary provides debugging support for Perl programs through the
//! Debug Adapter Protocol, enabling debugging in VSCode and other DAP-compatible editors.

use perl_parser::debug_adapter::DebugAdapter;
use std::io;

fn main() -> io::Result<()> {
    eprintln!("Perl Debug Adapter v0.6.0");
    eprintln!("Starting DAP server...");
    
    let mut adapter = DebugAdapter::new();
    adapter.run()
}
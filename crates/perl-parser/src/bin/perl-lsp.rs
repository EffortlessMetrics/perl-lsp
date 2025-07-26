//! Perl Language Server binary
//!
//! This binary implements a Language Server Protocol server for Perl
//! that can be used with any LSP-compatible editor.
//!
//! Usage:
//!   perl-lsp [options]
//!
//! Options:
//!   --stdio      Use stdio for communication (default)
//!   --socket     Use TCP socket for communication
//!   --port       Port to listen on (default: 9257)
//!   --log        Enable logging to stderr
//!   --version    Show version information
//!   --help       Show this help message

use perl_parser::LspServer;
use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    // Parse command line arguments
    let mut use_stdio = true;
    let mut port = 9257;
    let mut enable_logging = false;
    
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--stdio" => use_stdio = true,
            "--socket" => use_stdio = false,
            "--port" => {
                if i + 1 < args.len() {
                    port = args[i + 1].parse().unwrap_or(9257);
                    i += 1;
                }
            }
            "--log" => enable_logging = true,
            "--version" => {
                println!("perl-lsp 0.1.0");
                println!("Perl Language Server using perl-parser v3");
                process::exit(0);
            }
            "--help" | "-h" => {
                print_help();
                process::exit(0);
            }
            _ => {
                eprintln!("Unknown option: {}", args[i]);
                print_help();
                process::exit(1);
            }
        }
        i += 1;
    }
    
    // Initialize logging if requested
    if enable_logging {
        eprintln!("Perl Language Server starting...");
        eprintln!("Mode: {}", if use_stdio { "stdio" } else { "socket" });
        if !use_stdio {
            eprintln!("Port: {}", port);
        }
    }
    
    // Create and run the LSP server
    let mut server = LspServer::new();
    
    if use_stdio {
        // Run in stdio mode (default)
        if let Err(e) = server.run() {
            eprintln!("LSP server error: {}", e);
            process::exit(1);
        }
    } else {
        // Socket mode not implemented yet
        eprintln!("Socket mode is not implemented yet. Please use --stdio");
        process::exit(1);
    }
}

fn print_help() {
    println!("Perl Language Server");
    println!();
    println!("Usage: perl-lsp [options]");
    println!();
    println!("Options:");
    println!("  --stdio      Use stdio for communication (default)");
    println!("  --socket     Use TCP socket for communication");
    println!("  --port       Port to listen on (default: 9257)");
    println!("  --log        Enable logging to stderr");
    println!("  --version    Show version information");
    println!("  --help       Show this help message");
    println!();
    println!("Examples:");
    println!("  # Run in stdio mode (for VSCode, Neovim, etc.)");
    println!("  perl-lsp --stdio");
    println!();
    println!("  # Run with logging enabled");
    println!("  perl-lsp --stdio --log");
}
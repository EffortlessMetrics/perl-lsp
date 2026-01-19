//! Perl Language Server binary
//!
//! This binary implements a Language Server Protocol server for Perl
//! that can be used with any LSP-compatible editor.

#![deny(clippy::option_env_unwrap)]
//!
//! Usage:
//!   perl-lsp \[options\]
//!
//! Options:
//!   --stdio      Use stdio for communication (default)
//!   --socket     Use TCP socket for communication
//!   --port       Port to listen on (default: 9257)
//!   --log        Enable logging to stderr
//!   --health     Quick health check
//!   --version    Show version information
//!   --help       Show this help message

// Import from the local perl_lsp crate (runtime moved from perl-parser)
use perl_lsp::LspServer;
use std::env;
use std::process;
use tokio::net::TcpListener;
use tokio::runtime::Runtime;
use std::sync::Arc;

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
            "--health" => {
                println!("ok {}", env!("CARGO_PKG_VERSION"));
                process::exit(0);
            }
            "--version" => {
                println!("perl-lsp {}", env!("CARGO_PKG_VERSION"));
                // build.rs always sets GIT_TAG (falls back to "unknown"), so env! is safe
                println!("Git tag: {}", env!("GIT_TAG"));
                println!("Perl Language Server using perl-parser v3");
                process::exit(0);
            }
            "--features-json" => {
                println!("{}", perl_lsp::features::to_json());
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

    if use_stdio {
        // Create and run the LSP server
        let mut server = LspServer::new();

        // Run in stdio mode (default)
        if let Err(e) = server.run() {
            eprintln!("LSP server error: {}", e);
            process::exit(1);
        }
    } else {
        // Run in socket mode
        let addr = format!("127.0.0.1:{}", port);
        let rt = Runtime::new().expect("Failed to create Tokio runtime");

        rt.block_on(async {
            let listener = TcpListener::bind(&addr).await.expect("Failed to bind port");
            let local_addr = listener.local_addr().expect("Failed to get local address");
            eprintln!("Perl LSP listening on {}", local_addr);

            loop {
                match listener.accept().await {
                    Ok((stream, peer_addr)) => {
                        eprintln!("Accepted connection from {}", peer_addr);
                        tokio::spawn(async move {
                            // Convert to std::net::TcpStream for blocking I/O
                            if let Ok(std_stream) = stream.into_std() {
                                // Set non-blocking to false because serve uses blocking I/O
                                if let Err(e) = std_stream.set_nonblocking(false) {
                                    eprintln!("Failed to set blocking mode: {}", e);
                                    return;
                                }

                                let writer = match std_stream.try_clone() {
                                    Ok(w) => w,
                                    Err(e) => {
                                        eprintln!("Failed to clone stream: {}", e);
                                        return;
                                    }
                                };
                                let reader = std_stream;

                                // We need Arc<Mutex<Box<dyn Write + Send>>> for output
                                // Note: we need to use the Mutex from parking_lot as expected by LspServer
                                // Since we can't easily import parking_lot::Mutex directly if it's not re-exported,
                                // we rely on the fact that LspServer expects a generic Mutex which is likely parking_lot.
                                // Wait, LspServer definition uses parking_lot::Mutex explicitly.
                                // We need to import parking_lot. Since we added it to dev-dependencies in Cargo.toml of perl-lsp?
                                // No, I added it to dependencies implicitly via perl-lsp dependencies?
                                // Actually, I should check if parking_lot is available.
                                // It is in [dependencies] of perl-lsp Cargo.toml.

                                let output = Arc::new(parking_lot::Mutex::new(Box::new(writer) as Box<dyn std::io::Write + Send>));

                                tokio::task::spawn_blocking(move || {
                                    let mut server = LspServer::with_output(output);
                                    let mut buf_reader = std::io::BufReader::new(reader);
                                    if let Err(e) = server.serve(&mut buf_reader) {
                                         eprintln!("Connection error: {}", e);
                                    }
                                }).await.expect("Task panic");
                            } else {
                                eprintln!("Failed to convert stream to std");
                            }
                        });
                    }
                    Err(e) => eprintln!("Failed to accept: {}", e),
                }
            }
        });
    }
}

fn print_help() {
    eprintln!("Perl Language Server");
    eprintln!();
    eprintln!("Usage: perl-lsp [options]");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --stdio          Use stdio for communication (default)");
    eprintln!("  --socket         Use TCP socket for communication");
    eprintln!("  --port           Port to listen on (default: 9257)");
    eprintln!("  --log            Enable logging to stderr");
    eprintln!("  --health         Quick health check (prints 'ok <version>')");
    eprintln!("  --version        Show version information");
    eprintln!("  --features-json  Output features catalog as JSON");
    eprintln!("  --help           Show this help message");
    eprintln!();
    eprintln!("Examples:");
    eprintln!("  # Run in stdio mode (for VSCode, Neovim, etc.)");
    eprintln!("  perl-lsp --stdio");
    eprintln!();
    eprintln!("  # Run with logging enabled");
    eprintln!("  perl-lsp --stdio --log");
    eprintln!();
    eprintln!("  # Run in TCP socket mode");
    eprintln!("  perl-lsp --socket --port 9257");
}

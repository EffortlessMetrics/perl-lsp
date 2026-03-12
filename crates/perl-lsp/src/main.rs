//! Perl Language Server binary.
//!
//! This binary wires startup options to the reusable launcher microcrate and
//! delegates command parsing, profile compatibility, and CLI interoperability to a
//! shared package boundary.

#![deny(clippy::option_env_unwrap)]
use perl_lsp::LspServer;
use perl_lsp_launcher::{LaunchAction, LaunchConfig, TransportMode, help_text, parse_args};
use std::env;
use std::process;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::runtime::Runtime;
use tokio::time::{Duration, sleep};

fn main() {
    let launch_plan = match parse_args(env::args()) {
        Ok(plan) => plan,
        Err(error) => {
            eprintln!("{error}");
            eprintln!("{}", help_text());
            process::exit(1);
        }
    };

    match launch_plan.action {
        LaunchAction::Run => run_server(launch_plan.config),
        LaunchAction::Health => {
            println!("ok {}", env!("CARGO_PKG_VERSION"));
            process::exit(0);
        }
        LaunchAction::Version => {
            print_version();
            process::exit(0);
        }
        LaunchAction::FeaturesJson => {
            println!("{}", launch_plan.config.features_json());
            process::exit(0);
        }
        LaunchAction::Help => {
            println!("{}", help_text());
            process::exit(0);
        }
    }
}

fn run_server(launch_config: LaunchConfig) {
    if launch_config.enable_logging {
        eprintln!("Perl Language Server starting...");
        eprintln!("Mode: {}", launch_config.transport.label());
        if let Some(port) = launch_config.transport.port() {
            eprintln!("Port: {port}");
        }
        eprintln!("Feature profile: {}", launch_config.feature_profile.as_str());
    }

    match launch_config.transport {
        TransportMode::Stdio => {
            let mut server = LspServer::new_with_feature_profile(launch_config.feature_profile);

            if let Err(e) = server.run() {
                eprintln!("LSP server error: {}", e);
                process::exit(1);
            }
        }
        TransportMode::Socket { port } => {
            let addr = format!("127.0.0.1:{port}");
            let feature_profile = launch_config.feature_profile;
            let rt = match Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    eprintln!("Failed to create Tokio runtime: {e}");
                    process::exit(1);
                }
            };

            rt.block_on(async {
                let listener = match TcpListener::bind(&addr).await {
                    Ok(l) => l,
                    Err(e) => {
                        eprintln!("Failed to bind to {addr}: {e}");
                        process::exit(1);
                    }
                };
                let local_addr = match listener.local_addr() {
                    Ok(a) => a,
                    Err(e) => {
                        eprintln!("Failed to get local address: {e}");
                        process::exit(1);
                    }
                };
                eprintln!("Perl LSP listening on {}", local_addr);

                loop {
                    match listener.accept().await {
                        Ok((stream, peer_addr)) => {
                            eprintln!("Accepted connection from {peer_addr}");
                            tokio::spawn(async move {
                                let std_stream = match stream.into_std() {
                                    Ok(std_stream) => std_stream,
                                    Err(error) => {
                                        eprintln!("Failed to convert stream to std: {error}");
                                        return;
                                    }
                                };

                                if let Err(e) = std_stream.set_nonblocking(false) {
                                    eprintln!("Failed to set blocking mode: {}", e);
                                    return;
                                }

                                let writer = match std_stream.try_clone() {
                                    Ok(w) => w,
                                    Err(e) => {
                                        eprintln!("Failed to clone stream: {e}");
                                        return;
                                    }
                                };
                                let reader = std_stream;
                                let profile = feature_profile;

                                let output = Arc::new(parking_lot::Mutex::new(
                                    Box::new(writer) as Box<dyn std::io::Write + Send>
                                ));

                                if let Err(e) = tokio::task::spawn_blocking(move || -> () {
                                    let mut server =
                                        LspServer::with_output_and_feature_profile(output, profile);
                                    let mut buf_reader = std::io::BufReader::new(reader);
                                    if let Err(e) = server.serve(&mut buf_reader) {
                                        eprintln!("Connection error: {e}");
                                    }
                                })
                                .await
                                {
                                    eprintln!("Task panic: {e}");
                                }
                            });
                        }
                        Err(e) => {
                            eprintln!("Failed to accept: {e}");
                            sleep(Duration::from_millis(100)).await;
                        }
                    }
                }
            });
        }
    }
}

fn print_version() {
    println!("perl-lsp {}", env!("CARGO_PKG_VERSION"));
    // build.rs always sets GIT_TAG (falls back to "unknown"), so env! is safe.
    println!("Git tag: {}", env!("GIT_TAG"));
    println!("Perl Language Server using perl-parser v3");
}

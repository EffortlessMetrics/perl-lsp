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
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use tokio::net::TcpListener;
use tokio::runtime::Runtime;

static CONNECTION_COUNTER: AtomicU64 = AtomicU64::new(1);

fn log_info(enabled: bool, message: impl AsRef<str>) {
    if enabled {
        eprintln!("[perl-lsp][info] {}", message.as_ref());
    }
}

fn log_error(message: impl AsRef<str>) {
    eprintln!("[perl-lsp][error] {}", message.as_ref());
}

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
    log_info(launch_config.enable_logging, "Perl Language Server starting");
    log_info(launch_config.enable_logging, format!("mode={}", launch_config.transport.label()));
    if let Some(port) = launch_config.transport.port() {
        log_info(launch_config.enable_logging, format!("port={port}"));
    }
    log_info(
        launch_config.enable_logging,
        format!("feature_profile={}", launch_config.feature_profile.as_str()),
    );

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
                    log_error(format!("failed to create Tokio runtime: {e}"));
                    process::exit(1);
                }
            };

            rt.block_on(async {
                let listener = match TcpListener::bind(&addr).await {
                    Ok(l) => l,
                    Err(e) => {
                        log_error(format!("failed to bind to {addr}: {e}"));
                        process::exit(1);
                    }
                };
                let local_addr = match listener.local_addr() {
                    Ok(a) => a,
                    Err(e) => {
                        log_error(format!("failed to get local address: {e}"));
                        process::exit(1);
                    }
                };
                log_info(launch_config.enable_logging, format!("listening_on={local_addr}"));

                loop {
                    match listener.accept().await {
                        Ok((stream, peer_addr)) => {
                            let connection_id = CONNECTION_COUNTER.fetch_add(1, Ordering::Relaxed);
                            log_info(
                                launch_config.enable_logging,
                                format!("connection_id={connection_id} accepted peer={peer_addr}"),
                            );
                            let connection_started = Instant::now();
                            let logging_enabled = launch_config.enable_logging;
                            tokio::spawn(async move {
                                if let Ok(std_stream) = stream.into_std() {
                                    if let Err(e) = std_stream.set_nonblocking(false) {
                                        log_error(format!(
                                            "connection_id={connection_id} failed to set blocking mode: {e}"
                                        ));
                                        return;
                                    }

                                    let writer = match std_stream.try_clone() {
                                        Ok(w) => w,
                                        Err(e) => {
                                            log_error(format!(
                                                "connection_id={connection_id} failed to clone stream: {e}"
                                            ));
                                            return;
                                        }
                                    };
                                    let reader = std_stream;
                                    let profile = feature_profile;

                                    let output = Arc::new(parking_lot::Mutex::new(
                                        Box::new(writer) as Box<dyn std::io::Write + Send>,
                                    ));

                                    if let Err(e) = tokio::task::spawn_blocking(move || -> () {
                                        let mut server = LspServer::with_output_and_feature_profile(
                                            output, profile,
                                        );
                                        let mut buf_reader = std::io::BufReader::new(reader);
                                        if let Err(e) = server.serve(&mut buf_reader) {
                                            log_error(format!(
                                                "connection_id={connection_id} connection error: {e}"
                                            ));
                                        }
                                    })
                                    .await
                                    {
                                        log_error(format!(
                                            "connection_id={connection_id} task panic: {e}"
                                        ));
                                    }

                                    log_info(
                                        logging_enabled,
                                        format!(
                                            "connection_id={connection_id} closed duration_ms={}",
                                            connection_started.elapsed().as_millis()
                                        ),
                                    );
                                } else {
                                    log_error(format!(
                                        "connection_id={connection_id} failed to convert stream to std"
                                    ));
                                }
                            });
                        }
                        Err(e) => log_error(format!("failed to accept connection: {e}")),
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

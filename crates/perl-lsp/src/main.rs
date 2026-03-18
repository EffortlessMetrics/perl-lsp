//! Perl Language Server binary.
//!
//! This binary wires startup options to the reusable launcher microcrate and
//! delegates command parsing, profile compatibility, and CLI interoperability to a
//! shared package boundary.
//!
//! Both stdio and TCP modes use the same async dispatch path (`serve_async`),
//! with a blocking reader thread feeding an `mpsc` channel.

#![deny(clippy::option_env_unwrap)]
use perl_lsp::LspServer;
use perl_lsp_launcher::{
    LaunchAction, LaunchConfig, TransportMode, format_health_output, format_info_output, help_text,
    parse_args, port_in_use_message, shell_completion,
};
use std::env;
use std::process;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::net::TcpListener;
use tokio::runtime::Runtime;
use tokio::time::{Duration, sleep};

/// Minimal stderr logger for startup/runtime diagnostics that must stay
/// separate from LSP protocol traffic on stdout.
#[derive(Clone, Copy, Debug)]
struct StderrLogger {
    enabled: bool,
}

impl StderrLogger {
    const fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    fn info(self, message: impl AsRef<str>) {
        if self.enabled {
            self.emit("INFO", message.as_ref());
        }
    }

    fn warn(self, message: impl AsRef<str>) {
        self.emit("WARN", message.as_ref());
    }

    fn error(self, message: impl AsRef<str>) {
        self.emit("ERROR", message.as_ref());
    }

    fn emit(self, level: &str, message: &str) {
        eprintln!("[{}] {level} perl-lsp: {message}", log_timestamp());
    }
}

fn log_timestamp() -> String {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => format!("{}.{:03}", duration.as_secs(), duration.subsec_millis()),
        Err(_) => "0.000".to_string(),
    }
}

/// Spawn a blocking reader thread that reads LSP messages from `reader` and
/// forwards them to `tx`. The thread exits when the channel closes or the
/// reader returns EOF or an error.
fn spawn_reader_thread<R: std::io::Read + Send + 'static>(
    reader: R,
    tx: tokio::sync::mpsc::Sender<perl_lsp::JsonRpcRequest>,
    logger: StderrLogger,
) {
    use perl_lsp::transport::ContentLengthMessageReader;
    std::thread::spawn(move || {
        let mut msg_reader = ContentLengthMessageReader::new();
        let mut buf_reader = std::io::BufReader::new(reader);
        loop {
            match msg_reader.read_next(&mut buf_reader) {
                Ok(Some(request)) => {
                    if tx.blocking_send(request).is_err() {
                        logger.info("reader thread exiting after request channel closed");
                        break;
                    }
                }
                Ok(None) => {
                    logger.info("reader thread reached EOF");
                    break;
                }
                Err(error) => {
                    logger.warn(format!("reader thread stopped after transport error: {error}"));
                    break;
                }
            }
        }
    });
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
            let use_color = is_terminal_stdout();
            println!("{}", format_health_output(env!("CARGO_PKG_VERSION"), use_color));
            process::exit(0);
        }
        LaunchAction::Info => {
            let use_color = is_terminal_stdout();
            let exe_path = env::current_exe()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| "<unknown>".to_string());
            print!(
                "{}",
                format_info_output(
                    env!("CARGO_PKG_VERSION"),
                    env!("GIT_TAG"),
                    &exe_path,
                    launch_plan.config.feature_profile,
                    use_color,
                )
            );
            process::exit(0);
        }
        LaunchAction::Check => {
            let exit_code = run_check(&launch_plan.files);
            process::exit(exit_code);
        }
        LaunchAction::Completion { ref shell } => {
            if let Some(script) = shell_completion(shell) {
                print!("{script}");
                process::exit(0);
            } else {
                eprintln!("Unknown shell: {shell}. Supported: bash, zsh, fish");
                process::exit(1);
            }
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

/// Run the `--check` batch mode: parse the given files and report errors.
fn run_check(files: &[String]) -> i32 {
    if files.is_empty() {
        eprintln!("Usage: perl-lsp --check <file.pl> [file2.pm ...]");
        eprintln!("No files specified.");
        return 1;
    }

    let mut total = 0usize;
    let mut errors = 0usize;

    for path in files {
        total += 1;
        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("{path}: error reading file: {e}");
                errors += 1;
                continue;
            }
        };

        let mut parser = perl_parser::Parser::new(&source);
        match parser.parse() {
            Ok(_) => {
                println!("{path}: ok");
            }
            Err(e) => {
                println!("{path}: FAIL - {e}");
                errors += 1;
            }
        }
    }

    if total > 1 {
        println!();
        println!("{total} files checked, {errors} with errors");
    }

    if errors > 0 { 1 } else { 0 }
}

/// Detect whether stdout is a terminal (for colored output).
///
/// Respects `NO_COLOR` (<https://no-color.org/>) and checks the actual
/// file descriptor via `std::io::IsTerminal` (stable since Rust 1.70).
fn is_terminal_stdout() -> bool {
    use std::io::IsTerminal;
    env::var("NO_COLOR").is_err() && std::io::stdout().is_terminal()
}

fn run_server(launch_config: LaunchConfig) {
    let logger = StderrLogger::new(launch_config.enable_logging);
    logger.info(format!(
        "starting server (mode={}, feature_profile={})",
        launch_config.transport.label(),
        launch_config.feature_profile.as_str()
    ));
    if let Some(port) = launch_config.transport.port() {
        logger.info(format!("configured socket port={port}"));
    }

    match launch_config.transport {
        TransportMode::Stdio => {
            let rt = match Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    logger.error(format!("failed to create Tokio runtime: {e}"));
                    process::exit(1);
                }
            };

            rt.block_on(async {
                let server =
                    Arc::new(LspServer::new_with_feature_profile(launch_config.feature_profile));

                let (tx, rx) = tokio::sync::mpsc::channel(64);
                spawn_reader_thread(std::io::stdin(), tx, logger);
                logger.info("stdio transport initialized; waiting for requests");
                server.serve_async(rx).await;
                logger.info("stdio server loop exited");
            });
        }
        TransportMode::Socket { port } => {
            let addr = format!("127.0.0.1:{port}");
            let feature_profile = launch_config.feature_profile;
            let rt = match Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    logger.error(format!("failed to create Tokio runtime: {e}"));
                    process::exit(1);
                }
            };

            rt.block_on(async {
                let listener = match TcpListener::bind(&addr).await {
                    Ok(l) => l,
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::AddrInUse {
                            logger.error(port_in_use_message(port));
                        } else {
                            logger.error(format!("failed to bind to {addr}: {e}"));
                        }
                        process::exit(1);
                    }
                };
                let local_addr = match listener.local_addr() {
                    Ok(a) => a,
                    Err(e) => {
                        logger.error(format!("failed to get local address: {e}"));
                        process::exit(1);
                    }
                };
                logger.info(format!("socket listener ready on {local_addr}"));

                loop {
                    match listener.accept().await {
                        Ok((stream, peer_addr)) => {
                            logger.info(format!("accepted connection from {peer_addr}"));
                            tokio::spawn(async move {
                                let connection_logger = logger;
                                let std_stream = match stream.into_std() {
                                    Ok(std_stream) => std_stream,
                                    Err(error) => {
                                        connection_logger.error(format!(
                                            "failed to convert socket stream for {peer_addr}: {error}"
                                        ));
                                        return;
                                    }
                                };

                                if let Err(error) = std_stream.set_nonblocking(false) {
                                    connection_logger.error(format!(
                                        "failed to set blocking mode for {peer_addr}: {error}"
                                    ));
                                    return;
                                }

                                let writer = match std_stream.try_clone() {
                                    Ok(w) => w,
                                    Err(error) => {
                                        connection_logger.error(format!(
                                            "failed to clone stream for {peer_addr}: {error}"
                                        ));
                                        return;
                                    }
                                };
                                let reader = std_stream;
                                let profile = feature_profile;

                                let output = Arc::new(parking_lot::Mutex::new(
                                    Box::new(writer) as Box<dyn std::io::Write + Send>
                                ));

                                let server = Arc::new(LspServer::with_output_and_feature_profile(
                                    output, profile,
                                ));

                                let (tx, rx) = tokio::sync::mpsc::channel(64);
                                spawn_reader_thread(reader, tx, connection_logger);
                                connection_logger.info(format!(
                                    "serving socket session for {peer_addr}"
                                ));
                                server.serve_async(rx).await;
                                connection_logger.info(format!(
                                    "socket session ended for {peer_addr}"
                                ));
                            });
                        }
                        Err(e) => {
                            logger.warn(format!("failed to accept socket connection: {e}"));
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

#[cfg(test)]
mod tests {
    use super::{StderrLogger, log_timestamp};

    #[test]
    fn timestamp_contains_millisecond_separator() {
        let timestamp = log_timestamp();
        assert!(timestamp.contains('.'));
    }

    #[test]
    fn logger_preserves_enabled_flag() {
        let logger = StderrLogger::new(true);
        assert!(logger.enabled);
    }
}

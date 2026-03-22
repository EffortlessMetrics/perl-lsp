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
    CheckFormat, LaunchAction, LaunchConfig, TransportMode, format_health_output,
    format_info_output, help_text, init_logging, log_server_startup, logging_filter, parse_args,
    port_in_use_message, shell_completion, should_enable_logging,
};
use std::env;
use std::process;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::runtime::Runtime;
use tokio::time::{Duration, sleep};

/// Spawn a blocking reader thread that reads LSP messages from `reader` and
/// forwards them to `tx`. The thread exits when the channel closes or the
/// reader returns EOF or an error.
fn spawn_reader_thread<R: std::io::Read + Send + 'static>(
    reader: R,
    tx: tokio::sync::mpsc::Sender<perl_lsp::JsonRpcRequest>,
) {
    use perl_lsp::transport::ContentLengthMessageReader;
    std::thread::spawn(move || {
        let mut msg_reader = ContentLengthMessageReader::new();
        let mut buf_reader = std::io::BufReader::new(reader);
        loop {
            match msg_reader.read_next(&mut buf_reader) {
                Ok(Some(request)) => {
                    if tx.blocking_send(request).is_err() {
                        break;
                    }
                }
                Ok(None) => break, // EOF
                Err(error) => {
                    tracing::debug!(error = %error, "Stopping reader thread after transport read failure");
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
            let exit_code = run_check(&launch_plan.files, launch_plan.check_format);
            process::exit(exit_code);
        }
        LaunchAction::CheckProject { ref dir } => {
            let exit_code = run_check_project(dir);
            process::exit(exit_code);
        }
        LaunchAction::Completion { ref shell } => {
            if let Some(script) = shell_completion(shell) {
                print!("{script}");
                process::exit(0);
            } else {
                eprintln!("Unknown shell: {shell}. Supported: bash, zsh, fish, powershell");
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

/// Status of a single file parsed in `--check` mode.
enum FileCheckStatus {
    Ok,
    Fail { messages: Vec<String> },
    Error { message: String },
}

/// Parse a single file and return its status plus collected error messages.
fn check_one_file(path: &str) -> FileCheckStatus {
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => return FileCheckStatus::Error { message: format!("error reading file: {e}") },
    };

    let mut parser = perl_parser::Parser::new(&source);
    let parse_result = parser.parse();
    let recovered = parser.errors();

    let mut messages: Vec<String> = recovered.iter().map(|e| format!("{e}")).collect();
    if let Err(ref e) = parse_result {
        messages.push(format!("{e}"));
    }

    if messages.is_empty() { FileCheckStatus::Ok } else { FileCheckStatus::Fail { messages } }
}

/// Run the `--check` batch mode: parse the given files and report errors.
fn run_check(files: &[String], format: CheckFormat) -> i32 {
    if files.is_empty() {
        eprintln!("Usage: perl-lsp --check <file.pl> [file2.pm ...]");
        eprintln!("No files specified.");
        return 1;
    }

    match format {
        CheckFormat::Text => run_check_text(files),
        CheckFormat::Json => run_check_json(files),
    }
}

/// Text-format `--check` output (original behaviour).
fn run_check_text(files: &[String]) -> i32 {
    let mut total = 0usize;
    let mut errors = 0usize;

    for path in files {
        total += 1;
        match check_one_file(path) {
            FileCheckStatus::Ok => {
                println!("{path}: ok");
            }
            FileCheckStatus::Fail { messages } => {
                println!(
                    "{path}: FAIL - {}",
                    messages.first().map(String::as_str).unwrap_or("parse error")
                );
                errors += 1;
            }
            FileCheckStatus::Error { message } => {
                eprintln!("{path}: {message}");
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

/// JSON-format `--check` output.
fn run_check_json(files: &[String]) -> i32 {
    let mut count_ok = 0usize;
    let mut count_fail = 0usize;
    let mut count_error = 0usize;

    let mut file_entries: Vec<serde_json::Value> = Vec::new();

    for path in files {
        let entry = match check_one_file(path) {
            FileCheckStatus::Ok => {
                count_ok += 1;
                serde_json::json!({
                    "path": path,
                    "status": "ok"
                })
            }
            FileCheckStatus::Fail { messages } => {
                count_fail += 1;
                let errors: Vec<serde_json::Value> =
                    messages.iter().map(|m| serde_json::json!({ "message": m })).collect();
                serde_json::json!({
                    "path": path,
                    "status": "fail",
                    "errors": errors
                })
            }
            FileCheckStatus::Error { message } => {
                count_error += 1;
                serde_json::json!({
                    "path": path,
                    "status": "error",
                    "errors": [{ "message": message }]
                })
            }
        };
        file_entries.push(entry);
    }

    let total = count_ok + count_fail + count_error;
    let output = serde_json::json!({
        "version": 1,
        "files": file_entries,
        "summary": {
            "total": total,
            "ok": count_ok,
            "fail": count_fail,
            "error": count_error
        }
    });

    println!("{output}");

    if count_fail > 0 || count_error > 0 { 1 } else { 0 }
}

/// Detect whether stdout is a terminal (for colored output).
///
/// Respects `NO_COLOR` (<https://no-color.org/>) and checks the actual
/// file descriptor via `std::io::IsTerminal` (stable since Rust 1.70).
fn is_terminal_stdout() -> bool {
    use std::io::IsTerminal;
    env::var("NO_COLOR").is_err() && std::io::stdout().is_terminal()
}

/// Error details for a single file that failed to parse cleanly.
struct FileError {
    path: String,
    errors: Vec<String>,
}

/// Run the `--check-project` mode: scan a directory and report parsability.
fn run_check_project(dir: &str) -> i32 {
    let extensions: &[&str] = &["pm", "pl", "t"];

    let walker = walkdir::WalkDir::new(dir).follow_links(true).into_iter();

    let mut total = 0usize;
    let mut clean = 0usize;
    let mut file_errors: Vec<FileError> = Vec::new();
    let mut category_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    for entry in walker.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let ext_match = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| extensions.contains(&e))
            .unwrap_or(false);
        if !ext_match {
            continue;
        }

        total += 1;
        let path_str = path.display().to_string();

        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                file_errors
                    .push(FileError { path: path_str, errors: vec![format!("read error: {e}")] });
                category_counts.entry("IO error".to_string()).and_modify(|c| *c += 1).or_insert(1);
                continue;
            }
        };

        let mut parser = perl_parser::Parser::new(&source);
        let parse_result = parser.parse();
        let recovered_errors = parser.errors();

        let mut errors_for_file: Vec<String> = Vec::new();

        // Collect recovered errors (parser returned Ok but had issues)
        for err in recovered_errors {
            let cat = categorize_error(&format!("{err}"));
            category_counts.entry(cat).and_modify(|c| *c += 1).or_insert(1);
            errors_for_file.push(format!("{err}"));
        }

        // Collect catastrophic parse failure
        if let Err(ref e) = parse_result {
            let cat = categorize_error(&format!("{e}"));
            category_counts.entry(cat).and_modify(|c| *c += 1).or_insert(1);
            errors_for_file.push(format!("{e}"));
        }

        if errors_for_file.is_empty() {
            clean += 1;
        } else {
            file_errors.push(FileError { path: path_str, errors: errors_for_file });
        }
    }

    // Print report
    println!("Perl Project Parsability Report");
    println!("===============================");
    println!();
    println!("Directory: {dir}");
    println!("Files scanned: {total}");

    if total == 0 {
        println!();
        println!("No Perl files (.pm, .pl, .t) found.");
        return 0;
    }

    let pct = if total > 0 { (clean as f64 / total as f64) * 100.0 } else { 0.0 };

    println!("Clean parses: {clean}/{total} ({pct:.1}%)");
    println!();

    if !file_errors.is_empty() {
        println!("Parse errors:");
        for fe in &file_errors {
            for err in &fe.errors {
                println!("  {}: {err}", fe.path);
            }
        }
        println!();
    }

    if !category_counts.is_empty() {
        let mut cats: Vec<_> = category_counts.into_iter().collect();
        cats.sort_by(|a, b| b.1.cmp(&a.1));
        println!("Top issue categories:");
        for (cat, count) in &cats {
            println!("  {cat}: {count}");
        }
        println!();
    }

    if pct >= 80.0 {
        println!("Assessment: PASS ({pct:.1}% parsable)");
        0
    } else {
        println!("Assessment: FAIL ({pct:.1}% parsable, threshold 80%)");
        1
    }
}

/// Categorize an error message into a broad bucket.
fn categorize_error(msg: &str) -> String {
    if msg.contains("Unexpected end of input") {
        "Unexpected EOF".to_string()
    } else if msg.contains("expected") && msg.contains("found") {
        "Unexpected token".to_string()
    } else if msg.contains("Invalid syntax") {
        "Syntax error".to_string()
    } else if msg.contains("Lexer error") {
        "Lexer error".to_string()
    } else if msg.contains("recursion") || msg.contains("Recursion") {
        "Recursion limit".to_string()
    } else if msg.contains("read error") {
        "IO error".to_string()
    } else {
        "Other".to_string()
    }
}

fn run_server(launch_config: LaunchConfig) {
    let logging_enabled = should_enable_logging(launch_config.enable_logging);
    if logging_enabled {
        init_logging(&logging_filter(
            launch_config.enable_logging,
            "perl_lsp=info,perl_lsp_launcher=info,info",
            "warn",
        ));
        log_server_startup(
            "perl-lsp",
            env!("CARGO_PKG_VERSION"),
            launch_config.transport,
            Some(launch_config.feature_profile),
        );
    }

    match launch_config.transport {
        TransportMode::Stdio => {
            // Stdio uses the same async dispatch path as TCP: a blocking reader
            // thread feeds an mpsc channel into serve_async().
            let rt = match Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    eprintln!("Failed to create Tokio runtime: {e}");
                    process::exit(1);
                }
            };

            rt.block_on(async {
                let server =
                    Arc::new(LspServer::new_with_feature_profile(launch_config.feature_profile));

                // Spawn a blocking reader thread for stdin
                let (tx, rx) = tokio::sync::mpsc::channel(64);
                spawn_reader_thread(std::io::stdin(), tx);

                // Same async dispatch path as TCP
                server.serve_async(rx).await;
            });
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
                        if e.kind() == std::io::ErrorKind::AddrInUse {
                            eprintln!("{}", port_in_use_message(port));
                        } else {
                            eprintln!("Failed to bind to {addr}: {e}");
                        }
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
                if logging_enabled {
                    tracing::info!(address = %local_addr, "perl-lsp listening");
                }

                loop {
                    match listener.accept().await {
                        Ok((stream, peer_addr)) => {
                            if logging_enabled {
                                tracing::info!(peer = %peer_addr, "perl-lsp accepted connection");
                            }
                            tokio::spawn(async move {
                                let std_stream = match stream.into_std() {
                                    Ok(std_stream) => std_stream,
                                    Err(error) => {
                                        tracing::error!(%error, "failed to convert socket stream to std stream");
                                        return;
                                    }
                                };

                                if let Err(e) = std_stream.set_nonblocking(false) {
                                    tracing::error!(error = %e, "failed to set socket stream blocking mode");
                                    return;
                                }

                                let writer = match std_stream.try_clone() {
                                    Ok(w) => w,
                                    Err(e) => {
                                        tracing::error!(error = %e, "failed to clone socket stream");
                                        return;
                                    }
                                };
                                let reader = std_stream;
                                let profile = feature_profile;

                                let output = Arc::new(parking_lot::Mutex::new(
                                    Box::new(writer) as Box<dyn std::io::Write + Send>
                                ));

                                // Create server, wrap in Arc for concurrent async dispatch
                                let server = Arc::new(LspServer::with_output_and_feature_profile(
                                    output, profile,
                                ));

                                // Spawn a blocking reader thread that feeds an async channel
                                let (tx, rx) = tokio::sync::mpsc::channel(64);
                                spawn_reader_thread(reader, tx);

                                // Run async serve loop with concurrent dispatch
                                server.serve_async(rx).await;
                            });
                        }
                        Err(e) => {
                            tracing::error!(error = %e, "perl-lsp socket accept error");
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

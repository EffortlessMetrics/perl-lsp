//! Shared CLI entrypoint for the perl-lsp binaries.

#![deny(clippy::option_env_unwrap)]
// cli.rs is user-facing CLI output — eprintln!/println! are intentional here.
#![expect(
    clippy::print_stderr,
    reason = "CLI output module — user-facing error messages, version info, and diagnostics intentionally use stderr"
)]
#![expect(
    clippy::print_stdout,
    reason = "CLI output module — health check, version, help text, and check results intentionally use stdout"
)]

use crate::LspServer;
use crate::ripr_facts_emitter::emit_tests_and_oracles;
use perl_lsp_rs_core::runtime::launcher::{
    LaunchAction, LaunchConfig, StartupTimer, TransportMode, format_health_output,
    format_info_output, format_startup_banner, help_text, init_logging, log_server_startup,
    logging_filter, parse_args, port_in_use_message, shell_completion, should_enable_logging,
    should_use_ansi_stdout,
};
use perl_lsp_rs_core::tooling::native_compat::{
    classify_perlcritic_profile, classify_perltidy_profile, render_perlcritic_compat_markdown,
    render_perltidy_compat_markdown,
};
use std::env;
use std::path::Path;
use std::process;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::runtime::Runtime;
use tokio::time::{Duration, sleep};

mod check_project;
mod doctor;

/// Run the shared perl-lsp CLI and return the process exit code.
pub fn run_cli<I>(args: I) -> i32
where
    I: IntoIterator,
    I::Item: Into<std::ffi::OsString> + Clone,
{
    let collected_args: Vec<std::ffi::OsString> = args.into_iter().map(Into::into).collect();
    let command_name = invocation_name(&collected_args);

    let launch_plan = match parse_args(collected_args) {
        Ok(plan) => plan,
        Err(error) => {
            eprintln!("{error}");
            eprintln!("{}", render_help_text(&command_name));
            return 1;
        }
    };

    match launch_plan.action {
        LaunchAction::Run => {
            run_server(&command_name, launch_plan.config);
            0
        }
        LaunchAction::Health => {
            let use_color = should_use_ansi_stdout();
            println!("{}", format_health_output(env!("CARGO_PKG_VERSION"), use_color));
            0
        }
        LaunchAction::Info => {
            let use_color = should_use_ansi_stdout();
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
            0
        }
        LaunchAction::Check => run_check(&command_name, &launch_plan.files),
        LaunchAction::CheckProject { ref dir } => check_project::run_check_project(dir),
        LaunchAction::Doctor { ref dir } => doctor::run_doctor(dir),
        LaunchAction::Completion { ref shell } => {
            if let Some(script) = shell_completion(shell) {
                print!("{}", render_shell_completion(script, &command_name));
                0
            } else {
                eprintln!("Unknown shell: {shell}. Supported: bash, zsh, fish, powershell");
                1
            }
        }
        LaunchAction::Version => {
            print_version(&command_name);
            0
        }
        LaunchAction::FeaturesJson => {
            println!("{}", launch_plan.config.features_json());
            0
        }
        LaunchAction::PerltidyCompatReport { ref profile } => run_perltidy_compat_report(profile),
        LaunchAction::PerlcriticCompatReport { ref profile } => {
            run_perlcritic_compat_report(profile)
        }
        LaunchAction::RiprFacts {
            ref schema,
            ref root,
            ref base,
            ref head,
            ref fact_classes,
            ref out,
        } => run_ripr_facts(schema, root, base.as_deref(), head.as_deref(), fact_classes, out),
        LaunchAction::Help => {
            println!("{}", render_help_text(&command_name));
            0
        }
    }
}

fn invocation_name(args: &[std::ffi::OsString]) -> String {
    args.first()
        .and_then(|arg| Path::new(arg).file_stem())
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("perllsp")
        .to_string()
}

fn render_help_text(command_name: &str) -> String {
    let placeholder = "__PERL_LSP_COMMAND_NAME__";
    help_text()
        .replace("perl-lsp", placeholder)
        .replace("perllsp", placeholder)
        .replace(placeholder, command_name)
}

fn render_shell_completion(script: &str, command_name: &str) -> String {
    let function_name = command_name.replace('-', "_");
    script.replace("_perl_lsp", &format!("_{function_name}")).replace("perl-lsp", command_name)
}

fn run_perltidy_compat_report(profile: &str) -> i32 {
    let raw = match std::fs::read_to_string(profile) {
        Ok(raw) => raw,
        Err(error) => {
            eprintln!("{profile}: error reading perltidy profile: {error}");
            return 1;
        }
    };

    let report = classify_perltidy_profile(&raw);
    print!("{}", render_perltidy_compat_markdown(profile, &report));
    0
}

fn run_perlcritic_compat_report(profile: &str) -> i32 {
    let raw = match std::fs::read_to_string(profile) {
        Ok(raw) => raw,
        Err(error) => {
            eprintln!("{profile}: error reading perlcritic profile: {error}");
            return 1;
        }
    };

    let report = classify_perlcritic_profile(&raw);
    print!("{}", render_perlcritic_compat_markdown(profile, &report));
    0
}

/// Expected schema version for `ripr-perl-facts-v1` packets.
const EXPECTED_RIPR_FACTS_SCHEMA: &str = "ripr-perl-facts-v1";

/// Run the `ripr-facts` exporter (Campaign 31, ripr-swarm#1379).
///
/// This PR (PR 4) wires the CLI surface + arg validation + the unavailable-
/// packet fallback. The emitter body (mapping FileFactShard into the packet
/// shape) lands across PRs 5-8 (perl-lsp-swarm#2592-#2595).
fn run_ripr_facts(
    schema: &str,
    root: &str,
    base: Option<&str>,
    head: Option<&str>,
    fact_classes: &str,
    out: &str,
) -> i32 {
    // Validate schema version.
    if schema != EXPECTED_RIPR_FACTS_SCHEMA {
        eprintln!(
            "ripr-facts: unsupported schema `{schema}`; expected `{EXPECTED_RIPR_FACTS_SCHEMA}`"
        );
        return 1;
    }

    // Validate root is repo-relative (forward-slash, no host/drive/temp).
    if let Err(reason) = validate_ripr_facts_path(root, "root") {
        eprintln!("ripr-facts: {reason}");
        return 1;
    }

    // Validate out path.
    if let Err(reason) = validate_ripr_facts_path(out, "out") {
        eprintln!("ripr-facts: {reason}");
        return 1;
    }

    // Validate + normalize fact classes.
    let normalized_classes = match normalize_fact_classes(fact_classes) {
        Ok(classes) => classes,
        Err(reason) => {
            eprintln!("ripr-facts: {reason}");
            return 1;
        }
    };

    // Emit the packet. PR 6 (perl-lsp-swarm#2593) adds test + oracle emission;
    // files/owners/changes (PR 5) + relations/discriminators (PR 7) + boundaries
    // (PR 8) land in subsequent PRs. When tests are found, upgrade packet_status
    // from `unavailable` to `partial` (some fact classes are populated).
    let (tests, oracles) = emit_tests_and_oracles(root);
    let has_test_facts = !tests.is_empty();

    let mut packet = build_unavailable_packet(schema, root, base, head, &normalized_classes);

    // Populate tests + oracles arrays.
    packet["tests"] = serde_json::Value::Array(tests);
    packet["oracles"] = serde_json::Value::Array(oracles);

    // Upgrade status if we found test facts.
    if has_test_facts {
        packet["packet_status"] = serde_json::json!("partial");
        // Replace the limitation with one noting partial coverage.
        packet["limitations"] = serde_json::json!([{
            "limitation_id": "emitter-partial",
            "kind": "partial_emitter",
            "message": "PR 6 (tests/oracles) landed; files/owners/changes (PR 5) + relations/discriminators (PR 7) + boundaries (PR 8) are not yet emitted.",
            "evidence_refs": []
        }]);
    }

    // Write the packet to the output path.
    if let Err(error) = write_packet(out, &packet) {
        eprintln!("ripr-facts: failed to write packet to `{out}`: {error}");
        return 1;
    }

    let status = packet["packet_status"].as_str().unwrap_or("unknown");
    eprintln!("ripr-facts: wrote {status} packet to `{out}`");
    0
}

/// Validate a path is repo-relative: forward-slash, no host/drive/temp prefix,
/// no `..` escape, no leading `/` or `./`.
fn validate_ripr_facts_path(path: &str, field: &str) -> Result<(), String> {
    if path.is_empty() {
        return Err(format!("`{field}` must not be empty"));
    }
    if path.starts_with('/') {
        return Err(format!("`{field}` must be repo-relative, not absolute: `{path}`"));
    }
    if path.starts_with("./") {
        return Err(format!("`{field}` must not start with `./`: `{path}`"));
    }
    if path.contains("..") {
        return Err(format!("`{field}` must not contain `..` (path escape): `{path}`"));
    }
    // Reject Windows drive letters (e.g. `C:\`) and UNC paths.
    if path.len() >= 2 && path.as_bytes()[1] == b':' {
        return Err(format!("`{field}` must be repo-relative, not a drive path: `{path}`"));
    }
    Ok(())
}

/// The closed vocabulary of fact classes the producer can emit.
const VALID_FACT_CLASSES: &[&str] = &[
    "files",
    "owners",
    "changes",
    "tests",
    "oracles",
    "relations",
    "dynamic_boundaries",
    "verify_commands",
    "limitations",
    "provenance",
];

/// Parse + deduplicate + deterministically order the comma-separated
/// fact-class list.
fn normalize_fact_classes(raw: &str) -> Result<Vec<String>, String> {
    let mut seen: Vec<String> = Vec::new();
    for class in raw.split(',').map(str::trim).filter(|c| !c.is_empty()) {
        if !VALID_FACT_CLASSES.contains(&class) {
            return Err(format!(
                "unknown fact class `{class}`; valid: {}",
                VALID_FACT_CLASSES.join(", ")
            ));
        }
        if !seen.iter().any(|s| s == class) {
            seen.push(class.to_string());
        }
    }
    // Deterministic order: canonical VALID_FACT_CLASSES order.
    seen.sort_by_key(|c| {
        VALID_FACT_CLASSES.iter().position(|v| *v == c.as_str()).unwrap_or(usize::MAX)
    });
    if seen.is_empty() {
        return Err("fact_classes must not be empty".to_string());
    }
    Ok(seen)
}

/// Build a schema-valid `unavailable` packet (the honest state until PRs 5-8
/// land the emitter body).
fn build_unavailable_packet(
    schema: &str,
    root: &str,
    base: Option<&str>,
    head: Option<&str>,
    fact_classes: &[String],
) -> serde_json::Value {
    serde_json::json!({
        "schema_version": schema,
        "packet_id": format!("perl-lsp-ripr-facts-unavailable-{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)),
        "packet_status": "unavailable",
        "packet_fingerprint": null,
        "producer": {
            "name": "perl-lsp",
            "version": env!("CARGO_PKG_VERSION"),
            "capabilities": fact_classes,
        },
        "root": {
            "repo_relative": root,
            "vcs_head": head,
            "path_style": "posix",
        },
        "input": {
            "base": base,
            "head": head,
            "diff_id": null,
            "requested_fact_classes": fact_classes,
        },
        "files": [],
        "owners": [],
        "changes": [],
        "tests": [],
        "oracles": [],
        "relations": [],
        "dynamic_boundaries": [],
        "verify_commands": [],
        "limitations": [{
            "limitation_id": "emitter-not-yet-implemented",
            "kind": "missing_emitter",
            "message": "The ripr-facts emitter body lands in PRs 5-8 (perl-lsp-swarm#2592-#2595). Today every call produces an unavailable packet.",
            "evidence_refs": []
        }],
        "provenance": [{
            "provenance_id": "cli-surface",
            "source": "operator_config",
            "confidence": "high"
        }]
    })
}

/// Write a JSON packet to the output path, creating parent directories.
fn write_packet(out: &str, packet: &serde_json::Value) -> std::io::Result<()> {
    let path = std::path::Path::new(out);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(packet)?;
    std::fs::write(path, json)
}

/// Spawn a blocking reader thread that reads LSP messages from `reader` and
/// forwards them to `tx`. The thread exits when the channel closes or the
/// reader returns EOF or an error.
fn spawn_reader_thread<R: std::io::Read + Send + 'static>(
    reader: R,
    tx: tokio::sync::mpsc::Sender<crate::JsonRpcRequest>,
) {
    use crate::transport::ContentLengthMessageReader;
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
                Ok(None) => break,
                Err(error) => {
                    tracing::debug!(
                        error = %error,
                        "Stopping reader thread after transport read failure"
                    );
                    break;
                }
            }
        }
    });
}

fn run_check(command_name: &str, files: &[String]) -> i32 {
    if files.is_empty() {
        eprintln!("Usage: {command_name} --check <file.pl> [file2.pm ...]");
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
                for detail in format_parse_error_context(&source, &e) {
                    println!("{detail}");
                }
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

fn format_parse_error_context(source: &str, error: &perl_parser::ParseError) -> Vec<String> {
    let contexts = perl_parser::error::get_error_contexts(std::slice::from_ref(error), source);
    let Some(context) = contexts.first() else {
        return Vec::new();
    };

    let line_number = context.line + 1;
    let column_number = context.column + 1;
    let mut lines = vec![format!("  --> line {line_number}, column {column_number}")];

    if !context.source_line.is_empty() {
        let gutter = line_number.to_string();
        lines.push(format!("  {gutter} | {}", context.source_line));
        lines.push(format!("  {} | {}^", " ".repeat(gutter.len()), " ".repeat(context.column)));
    }

    if let Some(suggestion) = &context.suggestion {
        lines.push(format!("  help: {suggestion}"));
    }

    lines
}

fn run_server(command_name: &str, launch_config: LaunchConfig) {
    let command_name = command_name.to_string();
    let mut startup_timer = StartupTimer::new();
    let logging_enabled = should_enable_logging(launch_config.enable_logging);
    if logging_enabled {
        init_logging(&logging_filter(
            launch_config.enable_logging,
            "perl_lsp=info,perl_lsp_rs_core=info,info",
            "warn",
        ));
    }
    startup_timer.checkpoint("logging_init");

    if std::env::var("PERL_LSP_QUIET").is_err() {
        eprintln!(
            "{}",
            format_startup_banner(
                env!("CARGO_PKG_VERSION"),
                launch_config.feature_profile,
                launch_config.transport.is_socket(),
            )
            .replace("perl-lsp", &command_name)
        );
    }

    match launch_config.transport {
        TransportMode::Stdio => {
            let rt = match Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    eprintln!(
                        "Perl Language Server failed to start: could not initialize the async \
                         runtime ({e}). This is usually caused by system resource limits. \
                         Try restarting VS Code or increasing your OS thread limits."
                    );
                    process::exit(1);
                }
            };

            rt.block_on(async {
                startup_timer.checkpoint("runtime_setup");
                let server = Arc::new(LspServer::new_with_feature_profile_and_tuning(
                    launch_config.feature_profile,
                    launch_config.runtime_tuning,
                ));
                startup_timer.checkpoint("server_construction");

                let (tx, rx) = tokio::sync::mpsc::channel(64);
                spawn_reader_thread(std::io::stdin(), tx);

                if logging_enabled {
                    let report = startup_timer.finish();
                    log_server_startup(
                        &command_name,
                        env!("CARGO_PKG_VERSION"),
                        launch_config.transport,
                        Some(launch_config.feature_profile),
                        Some(&report),
                    );
                }

                server.serve_async(rx).await;
            });
        }
        TransportMode::Socket { port } => {
            let addr = format!("127.0.0.1:{port}");
            let feature_profile = launch_config.feature_profile;
            let runtime_tuning = launch_config.runtime_tuning;
            let rt = match Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    eprintln!(
                        "Perl Language Server failed to start: could not initialize the async \
                         runtime ({e}). This is usually caused by system resource limits. \
                         Try restarting VS Code or increasing your OS thread limits."
                    );
                    process::exit(1);
                }
            };

            rt.block_on(async {
                let listener = match TcpListener::bind(&addr).await {
                    Ok(l) => l,
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::AddrInUse {
                            eprintln!(
                                "{}",
                                port_in_use_message(port).replace("perl-lsp", &command_name)
                            );
                        } else {
                            eprintln!(
                                "Perl Language Server could not listen on {addr}: {e}. \
                                 Try a different port with --port or check firewall settings."
                            );
                        }
                        process::exit(1);
                    }
                };
                let local_addr = match listener.local_addr() {
                    Ok(a) => a,
                    Err(e) => {
                        eprintln!(
                            "Perl Language Server started but could not determine its \
                             listening address: {e}."
                        );
                        process::exit(1);
                    }
                };
                if logging_enabled {
                    tracing::info!(server = %command_name, address = %local_addr, "server listening");
                }

                loop {
                    match listener.accept().await {
                        Ok((stream, peer_addr)) => {
                            if logging_enabled {
                                tracing::info!(server = %command_name, peer = %peer_addr, "accepted connection");
                            }
                            let command_name = command_name.clone();
                            tokio::spawn(async move {
                                let std_stream = match stream.into_std() {
                                    Ok(std_stream) => std_stream,
                                    Err(error) => {
                                        tracing::error!(
                                            %error,
                                            "failed to convert socket stream to std stream"
                                        );
                                        return;
                                    }
                                };

                                if let Err(e) = std_stream.set_nonblocking(false) {
                                    tracing::error!(
                                        error = %e,
                                        "failed to set socket stream blocking mode"
                                    );
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

                                let mut conn_timer = StartupTimer::new();
                                let server =
                                    Arc::new(LspServer::with_output_feature_profile_and_tuning(
                                        output,
                                        profile,
                                        runtime_tuning,
                                    ));
                                conn_timer.checkpoint("server_construction");

                                let (tx, rx) = tokio::sync::mpsc::channel(64);
                                spawn_reader_thread(reader, tx);

                                if logging_enabled {
                                    let report = conn_timer.finish();
                                    log_server_startup(
                                        &command_name,
                                        env!("CARGO_PKG_VERSION"),
                                        launch_config.transport,
                                        Some(profile),
                                        Some(&report),
                                    );
                                }

                                server.serve_async(rx).await;
                            });
                        }
                        Err(e) => {
                            tracing::error!(server = %command_name, error = %e, "socket accept error");
                            sleep(Duration::from_millis(100)).await;
                        }
                    }
                }
            });
        }
    }
}

fn print_version(command_name: &str) {
    println!("{command_name} {}", env!("CARGO_PKG_VERSION"));
    println!("Git tag: {}", env!("GIT_TAG"));
    println!("Perl Language Server using perl-parser v3");
}

#[cfg(test)]
mod tests {
    use super::{
        build_unavailable_packet, format_parse_error_context, invocation_name,
        normalize_fact_classes, render_help_text, render_shell_completion, run_cli, run_ripr_facts,
        validate_ripr_facts_path, write_packet,
    };
    use std::ffi::OsString;

    #[test]
    fn invocation_name_uses_file_stem_from_first_arg() {
        let args = vec![OsString::from("/usr/local/bin/perl-lsp-rs"), OsString::from("--help")];
        assert_eq!(invocation_name(&args), "perl-lsp-rs");
    }

    #[test]
    fn invocation_name_falls_back_when_first_arg_is_empty() {
        let args = vec![OsString::from("")];
        assert_eq!(invocation_name(&args), "perllsp");
    }

    #[test]
    fn render_help_text_rewrites_usage_examples_for_invocation_name() {
        let rendered = render_help_text("perl-lsp-rs");

        assert!(rendered.contains("Usage: perl-lsp-rs [options]"));
        assert!(rendered.contains("perl-lsp-rs --check lib/MyModule.pm"));
        assert!(!rendered.contains("Usage: perllsp"));
        assert!(!rendered.contains("perllsp --"));
    }

    #[test]
    fn render_help_text_does_not_rewrite_inserted_command_name() {
        let rendered = render_help_text("perllsp-dev");

        assert!(rendered.contains("Usage: perllsp-dev [options]"));
        assert!(!rendered.contains("perllsp-dev-dev"));
    }

    #[test]
    fn render_shell_completion_rewrites_function_and_command_names() {
        let script =
            "complete -F _perl_lsp perl-lsp\n# shell completion for perl-lsp and _perl_lsp";
        let rendered = render_shell_completion(script, "my-perl-lsp");

        assert!(rendered.contains("_my_perl_lsp"));
        assert!(rendered.contains("my-perl-lsp"));
        assert!(!rendered.contains("complete -F _perl_lsp perl-lsp"));
    }

    #[test]
    fn run_cli_dispatches_doctor_action() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let dir_arg = dir.path().to_str().ok_or("non-UTF-8 temp path")?;

        let exit_code = run_cli(["perl-lsp", "--doctor", dir_arg]);

        assert_eq!(exit_code, 0);
        Ok(())
    }

    #[test]
    fn run_cli_dispatches_doctor_errors() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let file = dir.path().join("not-a-workspace.pl");
        std::fs::write(&file, "use strict;\n")?;
        let file_arg = file.to_str().ok_or("non-UTF-8 temp path")?;

        let exit_code = run_cli(["perl-lsp", "--doctor", file_arg]);

        assert_eq!(exit_code, 1);
        Ok(())
    }

    #[test]
    fn format_parse_error_context_adds_line_column_and_caret()
    -> Result<(), Box<dyn std::error::Error>> {
        let source = "my $ok = 1;\nmy $broken = ;\n";
        let error = perl_parser::ParseError::unexpected("expression", ";", 25);
        let rendered = format_parse_error_context(source, &error);

        assert!(rendered.iter().any(|line| line == "  --> line 2, column 14"));
        assert!(rendered.iter().any(|line| line == "  2 | my $broken = ;"));
        assert!(rendered.iter().any(|line| line == "    |              ^"));
        Ok(())
    }

    #[test]
    fn format_parse_error_context_includes_suggestions_when_available()
    -> Result<(), Box<dyn std::error::Error>> {
        let source = "my $x = 1\n";
        let error = perl_parser::ParseError::unexpected(";", "end of input", source.len());
        let rendered = format_parse_error_context(source, &error);

        assert!(
            rendered
                .iter()
                .any(|line| line == "  help: add a semicolon ';' at the end of the statement")
        );
        Ok(())
    }

    // ── ripr-facts command tests (Campaign 31 PR 4, perl-lsp-swarm#2591) ──

    #[test]
    fn ripr_facts_validates_schema_version() {
        let rc = run_ripr_facts(
            "wrong-schema",
            ".",
            None,
            None,
            "owners,changes",
            "target/ripr/test-wrong-schema.json",
        );
        assert_eq!(rc, 1, "wrong schema must exit 1");
    }

    #[test]
    fn ripr_facts_rejects_absolute_root() {
        let rc = run_ripr_facts(
            "ripr-perl-facts-v1",
            "/absolute/path",
            None,
            None,
            "owners",
            "target/ripr/test-abs-root.json",
        );
        assert_eq!(rc, 1, "absolute root must exit 1");
    }

    #[test]
    fn ripr_facts_rejects_path_escape() {
        let rc =
            run_ripr_facts("ripr-perl-facts-v1", ".", None, None, "owners", "../../../etc/passwd");
        assert_eq!(rc, 1, "path escape must exit 1");
    }

    #[test]
    fn ripr_facts_rejects_unknown_fact_class() {
        let rc = run_ripr_facts(
            "ripr-perl-facts-v1",
            ".",
            None,
            None,
            "owners,bogus_class",
            "target/ripr/test-bad-class.json",
        );
        assert_eq!(rc, 1, "unknown fact class must exit 1");
    }

    #[test]
    fn ripr_facts_rejects_drive_path() {
        let rc = run_ripr_facts(
            "ripr-perl-facts-v1",
            "C:/repo",
            None,
            None,
            "owners",
            "target/ripr/test-drive.json",
        );
        assert_eq!(rc, 1, "Windows drive path must exit 1");
    }

    #[test]
    fn ripr_facts_rejects_dot_slash_root() {
        let rc = run_ripr_facts(
            "ripr-perl-facts-v1",
            "./repo",
            None,
            None,
            "owners",
            "target/ripr/test-dot-slash.json",
        );
        assert_eq!(rc, 1, "./ prefix must exit 1");
    }

    #[test]
    fn ripr_facts_rejects_empty_fact_classes() {
        let rc = run_ripr_facts(
            "ripr-perl-facts-v1",
            ".",
            None,
            None,
            "",
            "target/ripr/test-empty-classes.json",
        );
        assert_eq!(rc, 1, "empty fact_classes must exit 1");
    }

    #[test]
    fn ripr_facts_accepts_valid_invocation() {
        let out = "target/ripr/test-valid-invocation.json";
        let rc = run_ripr_facts(
            "ripr-perl-facts-v1",
            ".",
            Some("origin/main"),
            Some("HEAD"),
            "files,owners,changes,tests,oracles",
            out,
        );
        assert_eq!(rc, 0, "valid invocation must exit 0");
        let written = std::fs::read_to_string(out).expect("packet must be written");
        let parsed: serde_json::Value =
            serde_json::from_str(&written).expect("packet must be JSON");
        assert_eq!(parsed["packet_status"], "unavailable");
        // Clean up.
        let _ = std::fs::remove_file(out);
    }

    #[test]
    fn ripr_facts_deduplicates_and_orders_fact_classes() {
        let normalized = normalize_fact_classes("changes,owners,owners,changes,tests")
            .expect("valid classes normalize");
        // Canonical order (VALID_FACT_CLASSES order): files, owners, changes, tests, ...
        assert_eq!(normalized, vec!["owners", "changes", "tests"]);
    }

    #[test]
    fn ripr_facts_unavailable_packet_has_correct_shape() {
        let packet = build_unavailable_packet(
            "ripr-perl-facts-v1",
            ".",
            Some("origin/main"),
            Some("HEAD"),
            &["owners".to_string(), "changes".to_string()],
        );
        assert_eq!(packet["schema_version"], "ripr-perl-facts-v1");
        assert_eq!(packet["packet_status"], "unavailable");
        assert_eq!(packet["producer"]["name"], "perl-lsp");
        assert_eq!(packet["root"]["repo_relative"], ".");
        assert_eq!(packet["input"]["base"], "origin/main");
        assert_eq!(packet["input"]["head"], "HEAD");
        assert_eq!(
            packet["input"]["requested_fact_classes"],
            serde_json::json!(["owners", "changes"])
        );
        // The limitation explains why the packet is unavailable.
        assert_eq!(packet["limitations"][0]["kind"], "missing_emitter");
        // All fact arrays are empty (unavailable).
        for key in [
            "files",
            "owners",
            "changes",
            "tests",
            "oracles",
            "relations",
            "dynamic_boundaries",
            "verify_commands",
        ] {
            assert!(packet[key].as_array().unwrap().is_empty(), "array {key} should be empty");
        }
    }

    #[test]
    fn ripr_facts_writes_unavailable_packet_to_disk() -> std::io::Result<()> {
        let out = "target/ripr/test-ripr-facts-write.json";
        let packet = build_unavailable_packet(
            "ripr-perl-facts-v1",
            ".",
            None,
            None,
            &["owners".to_string()],
        );
        write_packet(out, &packet)?;
        let written = std::fs::read_to_string(out)?;
        let parsed: serde_json::Value = serde_json::from_str(&written)?;
        assert_eq!(parsed["schema_version"], "ripr-perl-facts-v1");
        assert_eq!(parsed["packet_status"], "unavailable");
        Ok(())
    }

    #[test]
    fn ripr_facts_rejects_empty_root() {
        let rc = run_ripr_facts(
            "ripr-perl-facts-v1",
            "",
            None,
            None,
            "owners",
            "target/ripr/test-empty-root.json",
        );
        assert_eq!(rc, 1, "empty root must exit 1");
    }

    #[test]
    fn ripr_facts_rejects_empty_out() {
        let rc = run_ripr_facts("ripr-perl-facts-v1", ".", None, None, "owners", "");
        assert_eq!(rc, 1, "empty out path must exit 1");
    }

    #[test]
    fn ripr_facts_validates_path_helper_directly() {
        // Directly test the path validator for all branches.
        assert!(validate_ripr_facts_path(".", "test").is_ok());
        assert!(validate_ripr_facts_path("target/ripr/x.json", "test").is_ok());
        assert!(validate_ripr_facts_path("", "test").is_err());
        assert!(validate_ripr_facts_path("/abs", "test").is_err());
        assert!(validate_ripr_facts_path("./rel", "test").is_err());
        assert!(validate_ripr_facts_path("../escape", "test").is_err());
        assert!(validate_ripr_facts_path("C:/drive", "test").is_err());
    }
}

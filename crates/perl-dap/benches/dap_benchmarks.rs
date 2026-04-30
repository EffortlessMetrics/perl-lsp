//! DAP Performance Benchmarks (AC14, AC15)
//!
//! Benchmarks cover both:
//! - Phase 1 config/platform setup paths
//! - Live session paths (launch/attach/stack/variables/evaluate/stepping)
//!
//! Specification: docs/reference/DAP_IMPLEMENTATION_SPECIFICATION.md#performance-specifications
//!
//! # Running Benchmarks
//!
//! ```bash
//! # Run all benchmarks
//! cargo bench -p perl-dap --bench dap_benchmarks
//!
//! # Run specific benchmark groups
//! cargo bench -p perl-dap --bench dap_benchmarks -- configuration
//! cargo bench -p perl-dap --bench dap_benchmarks -- platform
//! cargo bench -p perl-dap --bench dap_benchmarks -- dap_live
//!
//! # Run with shorter measurement time (for CI)
//! cargo bench -p perl-dap --bench dap_benchmarks -- --measurement-time 5
//! ```

use criterion::{criterion_group, criterion_main, Criterion};
use perl_dap::configuration::LaunchConfiguration;
use perl_dap::debug_adapter::{DapMessage, DebugAdapter};
use perl_dap::platform::{
    format_command_args, normalize_path, resolve_perl_path, setup_environment,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::hint::black_box;
use std::io::Write;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;
use tempfile::tempdir;

// ========== Configuration Benchmarks (AC14) ==========

fn benchmark_launch_config_validation(c: &mut Criterion) {
    use std::fs;

    let mut group = c.benchmark_group("configuration_validation");
    group.measurement_time(Duration::from_secs(10));

    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("benchmark_test.pl");
    if let Err(e) = fs::write(&temp_file, "#!/usr/bin/env perl\nprint 'test';\n") {
        eprintln!("Warning: Failed to create temp file for benchmark: {e}");
        group.finish();
        return;
    }

    group.bench_function("launch_config_validation", |b| {
        let config = LaunchConfiguration {
            program: temp_file.clone(),
            args: vec![],
            cwd: Some(temp_dir.clone()),
            env: HashMap::new(),
            perl_path: None,
            include_paths: vec![],
        };

        b.iter(|| {
            let _ = black_box(config.validate());
        })
    });

    group.bench_function("launch_config_path_resolution", |b| {
        let mut config = LaunchConfiguration {
            program: PathBuf::from("script.pl"),
            args: vec![],
            cwd: Some(PathBuf::from("build")),
            env: HashMap::new(),
            perl_path: None,
            include_paths: vec![
                PathBuf::from("lib"),
                PathBuf::from("local/lib"),
                PathBuf::from("vendor/lib"),
            ],
        };

        let workspace_root = black_box(PathBuf::from("/workspace"));

        b.iter(|| {
            let _ = config.resolve_paths(&workspace_root);
        })
    });

    let _ = fs::remove_file(&temp_file);
    group.finish();
}

// ========== Platform Utilities Benchmarks (AC14) ==========

fn benchmark_perl_path_resolution(c: &mut Criterion) {
    let mut group = c.benchmark_group("platform_perl");
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("perl_path_resolution", |b| {
        b.iter(|| {
            let _ = black_box(resolve_perl_path());
        })
    });

    group.finish();
}

fn benchmark_path_normalization(c: &mut Criterion) {
    let mut group = c.benchmark_group("platform_path");
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("path_normalization_simple", |b| {
        let path = PathBuf::from("/tmp/test/script.pl");
        b.iter(|| {
            black_box(normalize_path(black_box(&path)));
        })
    });

    group.bench_function("path_normalization_relative", |b| {
        let path = PathBuf::from("relative/path/script.pl");
        b.iter(|| {
            black_box(normalize_path(black_box(&path)));
        })
    });

    #[cfg(windows)]
    group.bench_function("path_normalization_windows_drive", |b| {
        let path = PathBuf::from(r"C:\Users\test\script.pl");
        b.iter(|| {
            black_box(normalize_path(black_box(&path)));
        })
    });

    #[cfg(target_os = "linux")]
    group.bench_function("path_normalization_wsl", |b| {
        let path = PathBuf::from("/mnt/c/Users/test/script.pl");
        b.iter(|| {
            black_box(normalize_path(black_box(&path)));
        })
    });

    group.bench_function("path_normalization_batch", |b| {
        let paths = vec![
            PathBuf::from("/usr/local/lib/perl5"),
            PathBuf::from("/home/user/lib"),
            PathBuf::from("./local/lib/perl5"),
            PathBuf::from("../vendor/lib"),
            PathBuf::from("/tmp/test.pl"),
        ];

        b.iter(|| {
            for path in &paths {
                black_box(normalize_path(black_box(path)));
            }
        })
    });

    group.finish();
}

fn benchmark_environment_setup(c: &mut Criterion) {
    let mut group = c.benchmark_group("platform_environment");
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("environment_setup_empty", |b| {
        b.iter(|| {
            black_box(setup_environment(&[]));
        })
    });

    group.bench_function("environment_setup_single_path", |b| {
        let include_paths = vec![PathBuf::from("/usr/local/lib/perl5")];
        b.iter(|| {
            black_box(setup_environment(black_box(&include_paths)));
        })
    });

    group.bench_function("environment_setup_multiple_paths", |b| {
        let include_paths = vec![
            PathBuf::from("/usr/local/lib/perl5"),
            PathBuf::from("/home/user/lib"),
            PathBuf::from("./local/lib/perl5"),
        ];
        b.iter(|| {
            black_box(setup_environment(black_box(&include_paths)));
        })
    });

    group.bench_function("environment_setup_large_paths", |b| {
        let include_paths = vec![
            PathBuf::from("/usr/local/lib/perl5"),
            PathBuf::from("/usr/local/lib/perl5/site_perl"),
            PathBuf::from("/usr/local/lib/perl5/vendor_perl"),
            PathBuf::from("/home/user/perl5/lib/perl5"),
            PathBuf::from("/home/user/lib"),
            PathBuf::from("./local/lib/perl5"),
            PathBuf::from("./local/lib/perl5/site_perl"),
            PathBuf::from("../vendor/lib"),
            PathBuf::from("../vendor/lib/perl5"),
            PathBuf::from("/opt/perl/lib"),
        ];
        b.iter(|| {
            black_box(setup_environment(black_box(&include_paths)));
        })
    });

    group.finish();
}

fn benchmark_arg_formatting(c: &mut Criterion) {
    let mut group = c.benchmark_group("platform_args");
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("arg_formatting_simple", |b| {
        let args = vec!["--verbose".to_string(), "--debug".to_string()];
        b.iter(|| {
            black_box(format_command_args(black_box(&args)));
        })
    });

    group.bench_function("arg_formatting_with_spaces", |b| {
        let args = vec!["simple".to_string(), "with space".to_string(), "another arg".to_string()];
        b.iter(|| {
            black_box(format_command_args(black_box(&args)));
        })
    });

    group.bench_function("arg_formatting_with_special_chars", |b| {
        let args = vec![
            "simple".to_string(),
            "with space".to_string(),
            "with\"quote".to_string(),
            "special!@#$chars".to_string(),
        ];
        b.iter(|| {
            black_box(format_command_args(black_box(&args)));
        })
    });

    group.bench_function("arg_formatting_complex", |b| {
        let args = vec![
            "--input".to_string(),
            "file with spaces.txt".to_string(),
            "--output".to_string(),
            "result file.txt".to_string(),
            "--verbose".to_string(),
            "--config".to_string(),
            "path to config.json".to_string(),
            "--flag1".to_string(),
            "--flag2".to_string(),
            "--data".to_string(),
            "some data with spaces".to_string(),
        ];
        b.iter(|| {
            black_box(format_command_args(black_box(&args)));
        })
    });

    group.finish();
}

// ========== Existing DAP Session/Dispatch Benchmarks ==========

fn benchmark_dap_initialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("dap_session");
    group.measurement_time(Duration::from_secs(10));

    let mut adapter = DebugAdapter::new();
    let init_args = json!({
        "clientId": "vscode",
        "clientName": "Visual Studio Code",
        "adapterId": "perl-rs",
        "linesStartAt1": true,
        "columnsStartAt1": true,
        "pathFormat": "path"
    });

    group.bench_function("dap_initialize_request", |b| {
        b.iter(|| {
            black_box(adapter.handle_request(1, "initialize", Some(init_args.clone())));
        })
    });

    group.finish();
}

fn benchmark_dap_dispatch(c: &mut Criterion) {
    let mut group = c.benchmark_group("dap_dispatch");
    group.measurement_time(Duration::from_secs(10));

    let mut adapter = DebugAdapter::new();

    group.bench_function("dap_threads_request", |b| {
        b.iter(|| {
            black_box(adapter.handle_request(1, "threads", None));
        })
    });

    group.bench_function("dap_stacktrace_request", |b| {
        let args = json!({ "threadId": 1 });
        b.iter(|| {
            black_box(adapter.handle_request(1, "stackTrace", Some(args.clone())));
        })
    });

    group.finish();
}

// ========== Phase 3: Live Session Benchmarks ==========

fn perl_available() -> bool {
    std::process::Command::new("perl").arg("--version").output().is_ok()
}

fn write_script(path: &Path, line_count: usize) -> std::io::Result<()> {
    let mut script = std::fs::File::create(path)?;
    writeln!(script, "use strict;")?;
    writeln!(script, "use warnings;")?;
    writeln!(script, "my @nums = (1..200);")?;
    writeln!(script, "my %map = (a => 1, b => 2, c => 3);")?;
    writeln!(script, "my $x = 0;")?;
    for i in 0..line_count {
        writeln!(script, "$x += {i};")?;
    }
    writeln!(script, "print $x;")?;
    script.flush()
}

fn initialize_adapter(adapter: &mut DebugAdapter) {
    let _ = adapter.handle_request(
        1,
        "initialize",
        Some(json!({
            "clientId": "bench",
            "clientName": "criterion",
            "adapterId": "perl-rs",
            "linesStartAt1": true,
            "columnsStartAt1": true,
            "pathFormat": "path"
        })),
    );
}

fn launch_adapter(adapter: &mut DebugAdapter, program_path: &Path) {
    let path = program_path.to_string_lossy().to_string();
    let _ = adapter.handle_request(
        2,
        "launch",
        Some(json!({
            "program": path,
            "args": [],
            "stopOnEntry": true,
            "env": {
                "PERL_PERTURB_KEYS": "0",
                "PERL_HASH_SEED": "0",
                "LC_ALL": "C",
                "TZ": "UTC"
            }
        })),
    );
}

fn disconnect_adapter(adapter: &mut DebugAdapter, request_seq: i64) {
    let _ = adapter.handle_request(request_seq, "disconnect", Some(json!({})));
}

fn response_body(response: DapMessage) -> Option<Value> {
    match response {
        DapMessage::Response { body, .. } => body,
        _ => None,
    }
}

fn first_scope_reference(scopes_body: &Value) -> i64 {
    scopes_body
        .get("scopes")
        .and_then(Value::as_array)
        .and_then(|scopes| scopes.first())
        .and_then(|scope| scope.get("variablesReference"))
        .and_then(Value::as_i64)
        .unwrap_or(11)
}

fn first_child_reference(vars_body: &Value) -> i64 {
    vars_body
        .get("variables")
        .and_then(Value::as_array)
        .and_then(|vars| {
            vars.iter().find_map(|var| {
                var.get("variablesReference")
                    .and_then(Value::as_i64)
                    .filter(|reference| *reference > 0)
            })
        })
        .unwrap_or(1102)
}

fn benchmark_live_launch(c: &mut Criterion) {
    let mut group = c.benchmark_group("dap_live_launch");
    group.measurement_time(Duration::from_secs(10));

    if !perl_available() {
        group.bench_function("launch_cold", |b| b.iter(|| black_box(())));
        group.bench_function("launch_warm", |b| b.iter(|| black_box(())));
        group.finish();
        return;
    }

    group.bench_function("launch_cold", |b| {
        b.iter(|| {
            if let Ok(dir) = tempdir() {
                let script = dir.path().join("cold_launch_bench.pl");
                if write_script(&script, 120).is_ok() {
                    let mut adapter = DebugAdapter::new();
                    initialize_adapter(&mut adapter);
                    launch_adapter(&mut adapter, &script);
                    disconnect_adapter(&mut adapter, 3);
                }
            }
        })
    });

    let warm_dir = tempdir();
    if let Ok(dir) = warm_dir {
        let warm_script = dir.path().join("warm_launch_bench.pl");
        if write_script(&warm_script, 120).is_ok() {
            group.bench_function("launch_warm", |b| {
                b.iter(|| {
                    let mut adapter = DebugAdapter::new();
                    initialize_adapter(&mut adapter);
                    launch_adapter(&mut adapter, &warm_script);
                    disconnect_adapter(&mut adapter, 3);
                })
            });
        } else {
            group.bench_function("launch_warm", |b| b.iter(|| black_box(())));
        }
    } else {
        group.bench_function("launch_warm", |b| b.iter(|| black_box(())));
    }

    group.finish();
}

fn benchmark_live_attach_loopback(c: &mut Criterion) {
    let mut group = c.benchmark_group("dap_live_attach");
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("attach_loopback", |b| {
        b.iter(|| {
            if let Ok(listener) = TcpListener::bind("127.0.0.1:0") {
                let port = listener.local_addr().map(|addr| addr.port()).unwrap_or(0);
                let server = thread::spawn(move || {
                    if let Ok((stream, _)) = listener.accept() {
                        thread::sleep(Duration::from_millis(15));
                        drop(stream);
                    }
                });

                if port > 0 {
                    let mut adapter = DebugAdapter::new();
                    initialize_adapter(&mut adapter);
                    let _ = adapter.handle_request(
                        2,
                        "attach",
                        Some(json!({
                            "host": "127.0.0.1",
                            "port": port,
                            "timeout": 500
                        })),
                    );
                    disconnect_adapter(&mut adapter, 3);
                }

                let _ = server.join();
            }
        })
    });

    group.finish();
}

fn benchmark_live_session_paths(c: &mut Criterion) {
    let mut group = c.benchmark_group("dap_live_session");
    group.measurement_time(Duration::from_secs(10));

    let fixture = tempdir().ok().and_then(|dir| {
        let path = dir.path().join("live_session_bench.pl");
        if write_script(&path, 300).is_ok() {
            Some((dir, path))
        } else {
            None
        }
    });

    let Some((fixture_dir, program_path)) = fixture else {
        group.bench_function("set_breakpoints_100", |b| b.iter(|| black_box(())));
        group.bench_function("step_continue_p95", |b| b.iter(|| black_box(())));
        group.bench_function("stack_trace_live", |b| b.iter(|| black_box(())));
        group.bench_function("variables_root", |b| b.iter(|| black_box(())));
        group.bench_function("variables_child_page", |b| b.iter(|| black_box(())));
        group.bench_function("evaluate_safe_blocked", |b| b.iter(|| black_box(())));
        group.bench_function("evaluate_live_simple", |b| b.iter(|| black_box(())));
        group.finish();
        return;
    };

    if !perl_available() {
        group.bench_function("set_breakpoints_100", |b| b.iter(|| black_box(())));
        group.bench_function("step_continue_p95", |b| b.iter(|| black_box(())));
        group.bench_function("stack_trace_live", |b| b.iter(|| black_box(())));
        group.bench_function("variables_root", |b| b.iter(|| black_box(())));
        group.bench_function("variables_child_page", |b| b.iter(|| black_box(())));
        group.bench_function("evaluate_safe_blocked", |b| b.iter(|| black_box(())));
        group.bench_function("evaluate_live_simple", |b| b.iter(|| black_box(())));
        group.finish();
        return;
    }

    let _fixture_guard = fixture_dir;
    let source_path = program_path.to_string_lossy().to_string();

    group.bench_function("set_breakpoints_100", |b| {
        b.iter(|| {
            let mut adapter = DebugAdapter::new();
            initialize_adapter(&mut adapter);
            launch_adapter(&mut adapter, &program_path);
            let _ = adapter.handle_request(
                3,
                "setBreakpoints",
                Some(json!({
                    "source": { "path": source_path },
                    "breakpoints": (1..=100).map(|line| json!({ "line": line })).collect::<Vec<_>>()
                })),
            );
            disconnect_adapter(&mut adapter, 4);
        })
    });

    group.bench_function("step_continue_p95", |b| {
        b.iter(|| {
            let mut adapter = DebugAdapter::new();
            initialize_adapter(&mut adapter);
            launch_adapter(&mut adapter, &program_path);
            for i in 0..40 {
                if i % 2 == 0 {
                    let _ =
                        adapter.handle_request(10 + i, "continue", Some(json!({ "threadId": 1 })));
                } else {
                    let _ = adapter.handle_request(10 + i, "next", Some(json!({ "threadId": 1 })));
                }
            }
            disconnect_adapter(&mut adapter, 100);
        })
    });

    group.bench_function("stack_trace_live", |b| {
        b.iter(|| {
            let mut adapter = DebugAdapter::new();
            initialize_adapter(&mut adapter);
            launch_adapter(&mut adapter, &program_path);
            let _ = adapter.handle_request(3, "stackTrace", Some(json!({ "threadId": 1 })));
            disconnect_adapter(&mut adapter, 4);
        })
    });

    group.bench_function("variables_root", |b| {
        b.iter(|| {
            let mut adapter = DebugAdapter::new();
            initialize_adapter(&mut adapter);
            launch_adapter(&mut adapter, &program_path);
            let scopes = adapter.handle_request(3, "scopes", Some(json!({ "frameId": 1 })));
            let root_ref = response_body(scopes).as_ref().map(first_scope_reference).unwrap_or(11);
            let _ = adapter.handle_request(
                4,
                "variables",
                Some(json!({
                    "variablesReference": root_ref,
                    "start": 0,
                    "count": 100
                })),
            );
            disconnect_adapter(&mut adapter, 5);
        })
    });

    group.bench_function("variables_child_page", |b| {
        b.iter(|| {
            let mut adapter = DebugAdapter::new();
            initialize_adapter(&mut adapter);
            launch_adapter(&mut adapter, &program_path);
            let scopes = adapter.handle_request(3, "scopes", Some(json!({ "frameId": 1 })));
            let root_ref = response_body(scopes).as_ref().map(first_scope_reference).unwrap_or(11);
            let roots = adapter.handle_request(
                4,
                "variables",
                Some(json!({
                    "variablesReference": root_ref,
                    "start": 0,
                    "count": 200
                })),
            );
            let child_ref =
                response_body(roots).as_ref().map(first_child_reference).unwrap_or(1102);
            let _ = adapter.handle_request(
                5,
                "variables",
                Some(json!({
                    "variablesReference": child_ref,
                    "start": 20,
                    "count": 50
                })),
            );
            disconnect_adapter(&mut adapter, 6);
        })
    });

    group.bench_function("evaluate_safe_blocked", |b| {
        b.iter(|| {
            let mut adapter = DebugAdapter::new();
            initialize_adapter(&mut adapter);
            launch_adapter(&mut adapter, &program_path);
            let _ = adapter.handle_request(
                3,
                "evaluate",
                Some(json!({
                    "expression": "system('echo blocked')",
                    "allowSideEffects": false
                })),
            );
            disconnect_adapter(&mut adapter, 4);
        })
    });

    group.bench_function("evaluate_live_simple", |b| {
        b.iter(|| {
            let mut adapter = DebugAdapter::new();
            initialize_adapter(&mut adapter);
            launch_adapter(&mut adapter, &program_path);
            let _ = adapter.handle_request(
                3,
                "evaluate",
                Some(json!({
                    "expression": "$x",
                    "allowSideEffects": false
                })),
            );
            disconnect_adapter(&mut adapter, 4);
        })
    });

    group.finish();
}

criterion_group!(configuration_benches, benchmark_launch_config_validation);

criterion_group!(
    platform_benches,
    benchmark_perl_path_resolution,
    benchmark_path_normalization,
    benchmark_environment_setup,
    benchmark_arg_formatting
);

criterion_group!(session_benches, benchmark_dap_initialization, benchmark_dap_dispatch);

criterion_group!(
    live_session_benches,
    benchmark_live_launch,
    benchmark_live_attach_loopback,
    benchmark_live_session_paths
);

criterion_main!(configuration_benches, platform_benches, session_benches, live_session_benches);

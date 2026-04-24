//! DAP Performance Benchmarks (AC14, AC15) - Phase 1
//!
//! Performance benchmarks for Phase 1 DAP adapter operations:
//! - Configuration creation and validation
//! - Path resolution and normalization
//! - Perl binary resolution
//! - Environment setup
//! - Argument formatting
//!
//! Specification: docs/reference/DAP_IMPLEMENTATION_SPECIFICATION.md#performance-specifications
//!
//! # Performance Targets (Phase 1)
//!
//! - Configuration creation: <50ms
//! - Configuration validation: <50ms
//! - Path normalization: <10ms per path
//! - Environment setup: <20ms
//! - Perl path resolution: <100ms
//!
//! # Running Benchmarks
//!
//! ```bash
//! # Run all benchmarks
//! cargo bench -p perl-dap --bench dap_benchmarks
//!
//! # Run specific benchmark group
//! cargo bench -p perl-dap --bench dap_benchmarks -- configuration
//! cargo bench -p perl-dap --bench dap_benchmarks -- platform
//! cargo bench -p perl-dap --bench dap_benchmarks -- dap_live_session
//!
//! # Stable benchmark names for machine diffing
//! cargo bench -p perl-dap --bench dap_benchmarks -- dap_live_session/launch_warm
//!
//! # Run with shorter measurement time (for CI)
//! cargo bench -p perl-dap -- --measurement-time 5
//! ```

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use perl_dap::configuration::LaunchConfiguration;
use perl_dap::debug_adapter::{DapMessage, DebugAdapter};
use perl_dap::platform::{
    format_command_args, normalize_path, resolve_perl_path, setup_environment,
};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::fs;
use std::hint::black_box;
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;

// ========== Configuration Benchmarks (AC14) ==========

/// Benchmark LaunchConfiguration validation
/// Target: <50ms
fn benchmark_launch_config_validation(c: &mut Criterion) {
    use std::fs;

    let mut group = c.benchmark_group("configuration_validation");
    group.measurement_time(Duration::from_secs(10));

    // Create temp file for validation
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("benchmark_test.pl");
    if let Err(e) = fs::write(&temp_file, "#!/usr/bin/env perl\nprint 'test';\n") {
        eprintln!("Warning: Failed to create temp file for benchmark: {}", e);
        // Skip benchmarks that require the temp file
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
            // Validation may fail in some environments; benchmark the call anyway
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
            // Path resolution may fail; benchmark the call anyway
            let _ = config.resolve_paths(&workspace_root);
        })
    });

    // Clean up temp file
    let _ = fs::remove_file(&temp_file);

    group.finish();
}

// ========== Platform Utilities Benchmarks (AC14) ==========

/// Benchmark Perl path resolution
/// Target: <100ms
fn benchmark_perl_path_resolution(c: &mut Criterion) {
    let mut group = c.benchmark_group("platform_perl");
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("perl_path_resolution", |b| {
        b.iter(|| {
            // This will fail if perl not found, which is OK for benchmarking
            let _ = black_box(resolve_perl_path());
        })
    });

    group.finish();
}

/// Benchmark path normalization (cross-platform)
/// Target: <10ms per path
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

/// Benchmark environment setup (PERL5LIB construction)
/// Target: <20ms
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

/// Benchmark command argument formatting
/// Target: <20ms (part of environment setup)
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

// ========== Phase 3: Live Session Benchmarks ==========

fn create_live_session_fixture() -> Option<(TempDir, PathBuf)> {
    let temp_dir = tempfile::tempdir().ok()?;
    let script_path = temp_dir.path().join("live_session_benchmark.pl");
    let script = r#"#!/usr/bin/env perl
use strict;
use warnings;

my $counter = 0;
my @items = qw(alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu);
my %lookup = map { $_ => length($_) } @items;

sub do_work {
    my ($n) = @_;
    my $sum = 0;
    for my $i (0 .. $n) {
        $sum += $i;
    }
    return $sum;
}

for my $round (1 .. 200) {
    $counter += do_work($round);
    my $token = $items[$round % scalar @items];
    $counter += $lookup{$token};
}

print "counter=$counter\n";
"#;

    fs::write(&script_path, script).ok()?;
    Some((temp_dir, script_path))
}

fn launch_args_for_script(script_path: &PathBuf) -> Value {
    json!({
        "program": script_path,
        "args": [],
        "stopOnEntry": true
    })
}

fn response_succeeded(response: DapMessage) -> bool {
    matches!(response, DapMessage::Response { success: true, .. })
}

fn benchmark_live_session_paths(c: &mut Criterion) {
    let mut group = c.benchmark_group("dap_live_session");
    group.measurement_time(Duration::from_secs(10));

    let Some((_temp_dir, script_path)) = create_live_session_fixture() else {
        group.finish();
        return;
    };

    if resolve_perl_path().is_err() {
        group.finish();
        return;
    }

    let launch_args = launch_args_for_script(&script_path);

    group.bench_function("launch_cold", |b| {
        b.iter_batched(
            DebugAdapter::new,
            |mut adapter| {
                let launched = adapter.handle_request(1, "launch", Some(launch_args.clone()));
                black_box(response_succeeded(launched));
                let _ = black_box(adapter.handle_request(2, "disconnect", None));
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("launch_warm", |b| {
        let mut adapter = DebugAdapter::new();
        b.iter(|| {
            let launched = adapter.handle_request(10, "launch", Some(launch_args.clone()));
            black_box(response_succeeded(launched));
            let _ = black_box(adapter.handle_request(11, "disconnect", None));
        })
    });

    group.bench_function("attach_loopback", |b| {
        let mut adapter = DebugAdapter::new();
        let pid = std::process::id();
        b.iter(|| {
            let attached = adapter.handle_request(20, "attach", Some(json!({ "processId": pid })));
            black_box(response_succeeded(attached));
            let _ = black_box(adapter.handle_request(21, "disconnect", None));
        })
    });

    group.bench_function("set_breakpoints_100", |b| {
        let mut adapter = DebugAdapter::new();
        let breakpoints = (1..=100).map(|line| json!({ "line": line })).collect::<Vec<_>>();
        let args = json!({
            "source": { "path": script_path },
            "breakpoints": breakpoints
        });
        b.iter(|| {
            black_box(adapter.handle_request(30, "setBreakpoints", Some(args.clone())));
        })
    });

    group.bench_function("step_continue_p95", |b| {
        let mut adapter = DebugAdapter::new();
        b.iter(|| {
            let mut samples = Vec::with_capacity(40);
            for i in 0..40 {
                let start = std::time::Instant::now();
                let response = if i % 2 == 0 {
                    adapter.handle_request(
                        40 + i,
                        "continue",
                        Some(json!({ "threadId": 1, "singleThread": false })),
                    )
                } else {
                    adapter.handle_request(40 + i, "next", Some(json!({ "threadId": 1 })))
                };
                black_box(response_succeeded(response));
                samples.push(start.elapsed());
            }
            samples.sort_unstable();
            let idx = ((samples.len() as f64) * 0.95).ceil() as usize;
            let p95 = samples[idx.saturating_sub(1).min(samples.len().saturating_sub(1))];
            black_box(p95);
        })
    });

    group.bench_function("stack_trace_live", |b| {
        let mut adapter = DebugAdapter::new();
        let pid = std::process::id();
        let _ = adapter.handle_request(50, "attach", Some(json!({ "processId": pid })));
        b.iter(|| {
            black_box(adapter.handle_request(51, "stackTrace", Some(json!({ "threadId": pid }))));
        });
        let _ = adapter.handle_request(52, "disconnect", None);
    });

    group.bench_function("variables_root", |b| {
        let mut adapter = DebugAdapter::new();
        b.iter(|| {
            black_box(adapter.handle_request(
                60,
                "variables",
                Some(json!({
                    "variablesReference": 11
                })),
            ));
        })
    });

    group.bench_function("variables_child_page", |b| {
        let mut adapter = DebugAdapter::new();
        b.iter(|| {
            let root_response = adapter.handle_request(
                70,
                "variables",
                Some(json!({
                    "variablesReference": 11
                })),
            );

            let child_ref = match root_response {
                DapMessage::Response { body, .. } => body
                    .and_then(|value| value.get("variables").and_then(|v| v.as_array()).cloned())
                    .and_then(|vars| {
                        vars.into_iter()
                            .find_map(|var| var.get("variablesReference").and_then(|n| n.as_i64()))
                    })
                    .unwrap_or(0),
                _ => 0,
            };

            if child_ref > 0 {
                black_box(adapter.handle_request(
                    71,
                    "variables",
                    Some(json!({
                        "variablesReference": child_ref,
                        "start": 5,
                        "count": 10
                    })),
                ));
            } else {
                black_box(child_ref);
            }
        })
    });

    group.bench_function("evaluate_safe_blocked", |b| {
        let mut adapter = DebugAdapter::new();
        b.iter(|| {
            black_box(adapter.handle_request(
                80,
                "evaluate",
                Some(json!({
                    "expression": "system('echo blocked')",
                    "context": "watch",
                    "allowSideEffects": false
                })),
            ));
        })
    });

    group.bench_function("evaluate_live_simple", |b| {
        let mut adapter = DebugAdapter::new();
        b.iter(|| {
            let launched = adapter.handle_request(90, "launch", Some(launch_args.clone()));
            if response_succeeded(launched) {
                black_box(adapter.handle_request(
                    91,
                    "evaluate",
                    Some(json!({
                        "expression": "$counter",
                        "context": "watch",
                        "frameId": 1
                    })),
                ));
            } else {
                black_box(adapter.handle_request(
                    91,
                    "evaluate",
                    Some(json!({
                        "expression": "$counter",
                        "context": "watch"
                    })),
                ));
            }
            let _ = black_box(adapter.handle_request(92, "disconnect", None));
        })
    });

    group.finish();
}

// ========== Benchmark Groups ==========

criterion_group!(configuration_benches, benchmark_launch_config_validation);

criterion_group!(
    platform_benches,
    benchmark_perl_path_resolution,
    benchmark_path_normalization,
    benchmark_environment_setup,
    benchmark_arg_formatting
);

// ========== Phase 2: Session Management Benchmarks (AC5.2) ==========

/// Benchmark DAP initialization
/// Target: <50ms
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

/// Benchmark DAP request dispatching (without process spawning)
/// Target: <100ms
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

criterion_group!(session_benches, benchmark_dap_initialization, benchmark_dap_dispatch);

criterion_group!(live_session_benches, benchmark_live_session_paths);

criterion_main!(configuration_benches, platform_benches, session_benches, live_session_benches);

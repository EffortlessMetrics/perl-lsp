//! DAP Performance Benchmarks (AC14, AC15)
//!
//! Performance benchmarks for DAP adapter operations:
//! - Configuration creation and validation
//! - Path resolution and normalization
//! - Perl binary resolution
//! - Environment setup
//! - Argument formatting
//! - Request dispatch
//! - Live-session launch/attach/stack/variables/evaluate paths
//!
//! # Running Benchmarks
//!
//! ```bash
//! # Run all benchmarks in this file
//! cargo bench -p perl-dap --bench dap_benchmarks
//!
//! # Run specific phase-1 groups
//! cargo bench -p perl-dap --bench dap_benchmarks -- configuration
//! cargo bench -p perl-dap --bench dap_benchmarks -- platform
//!
//! # Run only live-session groups
//! cargo bench -p perl-dap --bench dap_benchmarks -- live_session
//!
//! # Run with shorter measurement time (for CI)
//! cargo bench -p perl-dap --bench dap_benchmarks -- --measurement-time 5
//! ```

use criterion::{Criterion, criterion_group, criterion_main};
use perl_dap::configuration::LaunchConfiguration;
use perl_dap::debug_adapter::DebugAdapter;
use perl_dap::platform::{
    format_command_args, normalize_path, resolve_perl_path, setup_environment,
};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::fs;
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

fn benchmark_fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/performance/small_file.pl")
}

fn live_launch_args(script_path: &Path) -> Value {
    json!({
        "program": script_path,
        "args": [],
        "stopOnEntry": true,
        "env": {
            "PERL_PERTURB_KEYS": "0",
            "PERL_HASH_SEED": "0",
            "LC_ALL": "C",
            "TZ": "UTC"
        }
    })
}

fn setup_live_session(adapter: &mut DebugAdapter, script_path: &Path) {
    let _ = adapter.handle_request(1, "initialize", Some(json!({ "adapterId": "perl-rs" })));
    let _ = adapter.handle_request(2, "launch", Some(live_launch_args(script_path)));
    let _ = adapter.handle_request(3, "configurationDone", None);
}

fn teardown_live_session(adapter: &mut DebugAdapter) {
    let _ = adapter.handle_request(99, "disconnect", None);
}

fn response_is_success(response: &perl_dap::debug_adapter::DapMessage) -> bool {
    matches!(response, perl_dap::debug_adapter::DapMessage::Response { success: true, .. })
}

fn p95_duration(mut samples: Vec<Duration>) -> Duration {
    if samples.is_empty() {
        return Duration::from_nanos(0);
    }
    samples.sort_unstable();
    let idx = ((samples.len() as f64) * 0.95).ceil() as usize;
    samples[idx.saturating_sub(1).min(samples.len().saturating_sub(1))]
}

// ========== Configuration Benchmarks (AC14) ==========

fn benchmark_launch_config_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("configuration_validation");
    group.measurement_time(Duration::from_secs(10));

    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("benchmark_test.pl");
    if fs::write(&temp_file, "#!/usr/bin/env perl\nprint 'test';\n").is_err() {
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

// ========== Session Management Benchmarks (AC5.2) ==========

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

// ========== Live Session Hot Path Benchmarks (AC15 Layer 2) ==========

fn benchmark_live_session_paths(c: &mut Criterion) {
    let mut group = c.benchmark_group("live_session");
    group.measurement_time(Duration::from_secs(10));

    let script_path = benchmark_fixture_path();
    let script = script_path.to_string_lossy().to_string();

    // Stable, diff-friendly benchmark names for long-term tracking.
    group.bench_function("launch_cold", |b| {
        b.iter(|| {
            let mut adapter = DebugAdapter::new();
            let _ = black_box(adapter.handle_request(
                1,
                "initialize",
                Some(json!({ "adapterId": "perl-rs" })),
            ));
            let _ = black_box(adapter.handle_request(
                2,
                "launch",
                Some(live_launch_args(&script_path)),
            ));
            let _ = black_box(adapter.handle_request(3, "configurationDone", None));
            teardown_live_session(&mut adapter);
        })
    });

    group.bench_function("launch_warm", |b| {
        let mut adapter = DebugAdapter::new();
        let _ = adapter.handle_request(1, "initialize", Some(json!({ "adapterId": "perl-rs" })));

        b.iter(|| {
            let _ = black_box(adapter.handle_request(
                2,
                "launch",
                Some(live_launch_args(&script_path)),
            ));
            let _ = black_box(adapter.handle_request(3, "configurationDone", None));
            let _ = black_box(adapter.handle_request(4, "disconnect", None));
        })
    });

    group.bench_function("attach_loopback", |b| {
        let mut adapter = DebugAdapter::new();
        let _ = adapter.handle_request(1, "initialize", Some(json!({ "adapterId": "perl-rs" })));
        let pid = std::process::id();

        b.iter(|| {
            let response = adapter.handle_request(
                2,
                "attach",
                Some(json!({ "processId": pid, "stopOnEntry": true })),
            );
            black_box(response_is_success(&response));
            let _ = black_box(adapter.handle_request(3, "disconnect", None));
        })
    });

    group.bench_function("set_breakpoints_100", |b| {
        let mut adapter = DebugAdapter::new();
        let _ = adapter.handle_request(1, "initialize", Some(json!({ "adapterId": "perl-rs" })));
        let breakpoints: Vec<Value> = (1..=100).map(|line| json!({ "line": line })).collect();

        b.iter(|| {
            black_box(adapter.handle_request(
                2,
                "setBreakpoints",
                Some(json!({
                    "source": { "path": script },
                    "breakpoints": breakpoints
                })),
            ));
        })
    });

    group.bench_function("step_continue_p95", |b| {
        let mut adapter = DebugAdapter::new();
        let _ = adapter.handle_request(1, "initialize", Some(json!({ "adapterId": "perl-rs" })));
        let iterations = 50;

        b.iter(|| {
            let mut samples = Vec::with_capacity(iterations);
            for i in 0..iterations {
                let start = Instant::now();
                let response = if i % 2 == 0 {
                    adapter.handle_request(
                        10 + i as i64,
                        "continue",
                        Some(json!({ "threadId": 1 })),
                    )
                } else {
                    adapter.handle_request(10 + i as i64, "next", Some(json!({ "threadId": 1 })))
                };
                black_box(response);
                samples.push(start.elapsed());
            }
            black_box(p95_duration(samples));
        })
    });

    group.bench_function("stack_trace_live", |b| {
        b.iter_batched(
            || {
                let mut adapter = DebugAdapter::new();
                setup_live_session(&mut adapter, &script_path);
                adapter
            },
            |mut adapter| {
                black_box(adapter.handle_request(
                    4,
                    "stackTrace",
                    Some(json!({ "threadId": 1, "startFrame": 0, "levels": 20 })),
                ));
                teardown_live_session(&mut adapter);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.bench_function("variables_root", |b| {
        let mut adapter = DebugAdapter::new();
        let _ = adapter.handle_request(1, "initialize", Some(json!({ "adapterId": "perl-rs" })));

        b.iter(|| {
            black_box(adapter.handle_request(
                2,
                "variables",
                Some(json!({ "variablesReference": 11 })),
            ));
        })
    });

    group.bench_function("variables_child_page", |b| {
        let mut adapter = DebugAdapter::new();
        let _ = adapter.handle_request(1, "initialize", Some(json!({ "adapterId": "perl-rs" })));

        b.iter(|| {
            black_box(adapter.handle_request(
                2,
                "variables",
                Some(json!({
                    "variablesReference": 1101,
                    "start": 10,
                    "count": 25
                })),
            ));
        })
    });

    group.bench_function("evaluate_safe_blocked", |b| {
        let mut adapter = DebugAdapter::new();
        let _ = adapter.handle_request(1, "initialize", Some(json!({ "adapterId": "perl-rs" })));

        b.iter(|| {
            black_box(adapter.handle_request(
                2,
                "evaluate",
                Some(json!({
                    "expression": "system('rm -rf /tmp/nope')",
                    "context": "repl"
                })),
            ));
        })
    });

    group.bench_function("evaluate_live_simple", |b| {
        b.iter_batched(
            || {
                let mut adapter = DebugAdapter::new();
                setup_live_session(&mut adapter, &script_path);
                adapter
            },
            |mut adapter| {
                black_box(adapter.handle_request(
                    5,
                    "evaluate",
                    Some(json!({
                        "expression": "$ARGV[0]",
                        "context": "watch"
                    })),
                ));
                teardown_live_session(&mut adapter);
            },
            criterion::BatchSize::SmallInput,
        );
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
criterion_group!(live_session_benches, benchmark_live_session_paths);

criterion_main!(configuration_benches, platform_benches, session_benches, live_session_benches);

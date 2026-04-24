//! DAP Performance Benchmarks
//!
//! Covers:
//! - Phase 1 configuration/platform setup
//! - Baseline request dispatch
//! - Live-session adapter hot paths (launch/attach/breakpoints/stack/variables/evaluate)
//!
//! # Running Benchmarks
//!
//! ```bash
//! # Run all benchmark groups
//! cargo bench -p perl-dap --bench dap_benchmarks
//!
//! # Run only live-session benchmarks
//! cargo bench -p perl-dap --bench dap_benchmarks -- dap_live
//!
//! # Run only launch variants
//! cargo bench -p perl-dap --bench dap_benchmarks -- launch_
//!
//! # Output JSON from Criterion for machine-readable trend diffing
//! cargo bench -p perl-dap --bench dap_benchmarks -- --message-format=json
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
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

const BENCH_PREFIX: &str = "dap_live";

struct BenchScript {
    path: PathBuf,
}

impl BenchScript {
    fn new() -> Option<Self> {
        let mut path = std::env::temp_dir();
        path.push(format!("perl_dap_bench_{}.pl", std::process::id()));
        let content = r#"
use strict;
use warnings;

my @values = (1..240);
my %meta = (
    alpha => 1,
    beta  => 2,
    gamma => 3,
    delta => 4,
);

sub compute {
    my ($limit) = @_;
    my $sum = 0;
    for my $i (0..$limit) {
        $sum += $i;
    }
    return $sum;
}

my $result = compute(240);
my $cursor = 0;

while (1) {
    $cursor++;
    my $peek = $values[$cursor % scalar(@values)];
    $meta{last} = $peek;
    last if $cursor > 1_000_000;
}

print "$result\n";
"#;

        if fs::write(&path, content).is_ok() { Some(Self { path }) } else { None }
    }
}

impl Drop for BenchScript {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn initialize_adapter(adapter: &mut DebugAdapter) {
    let init_args = json!({
        "clientId": "criterion",
        "clientName": "criterion",
        "adapterId": "perl-rs",
        "linesStartAt1": true,
        "columnsStartAt1": true,
        "pathFormat": "path"
    });
    let _ = adapter.handle_request(1, "initialize", Some(init_args));
}

fn launch_adapter(script: &BenchScript, stop_on_entry: bool) -> Option<DebugAdapter> {
    let mut adapter = DebugAdapter::new();
    initialize_adapter(&mut adapter);

    let launch = adapter.handle_request(
        2,
        "launch",
        Some(json!({
            "program": script.path.to_string_lossy().to_string(),
            "cwd": std::env::temp_dir().to_string_lossy().to_string(),
            "stopOnEntry": stop_on_entry,
            "args": []
        })),
    );

    if matches!(launch, DapMessage::Response { success: true, .. }) { Some(adapter) } else { None }
}

fn shutdown_adapter(adapter: &mut DebugAdapter) {
    let _ = adapter.handle_request(9_001, "disconnect", Some(json!({ "terminateDebuggee": true })));
}

fn find_child_reference(body: &Value) -> Option<i64> {
    let vars = body.get("variables")?.as_array()?;
    vars.iter().find_map(|variable| {
        variable
            .get("variables_reference")
            .or_else(|| variable.get("variablesReference"))
            .and_then(Value::as_i64)
            .filter(|reference| *reference > 0)
    })
}

fn create_loopback_listener() -> Option<(TcpListener, u16)> {
    let listener = TcpListener::bind("127.0.0.1:0").ok()?;
    let port = listener.local_addr().ok()?.port();
    Some((listener, port))
}

fn keep_socket_open(mut stream: TcpStream) {
    let _ = stream.write_all(b" ");
    thread::sleep(Duration::from_millis(25));
}

// ========== Configuration Benchmarks (existing) ==========

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

// ========== Existing Session Baselines ==========

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

// ========== Live Session Benchmarks ==========

fn benchmark_live_session_paths(c: &mut Criterion) {
    let mut group = c.benchmark_group(BENCH_PREFIX);
    group.measurement_time(Duration::from_secs(10));

    let Some(script) = BenchScript::new() else {
        group.finish();
        return;
    };

    group.bench_function("launch_cold", |b| {
        b.iter_batched(
            || launch_adapter(&script, true),
            |maybe_adapter| {
                if let Some(mut adapter) = maybe_adapter {
                    black_box(adapter.handle_request(3, "configurationDone", Some(json!({}))));
                    shutdown_adapter(&mut adapter);
                }
            },
            BatchSize::SmallInput,
        )
    });

    if let Some(mut warm_adapter) = launch_adapter(&script, true) {
        group.bench_function("launch_warm", |b| {
            b.iter(|| {
                let response = warm_adapter.handle_request(
                    100,
                    "restart",
                    Some(json!({
                        "arguments": {
                            "program": script.path.to_string_lossy().to_string(),
                            "cwd": std::env::temp_dir().to_string_lossy().to_string(),
                            "stopOnEntry": true,
                            "args": []
                        }
                    })),
                );
                black_box(response);
            })
        });
        shutdown_adapter(&mut warm_adapter);
    }

    group.bench_function("attach_loopback", |b| {
        b.iter_batched(
            create_loopback_listener,
            |listener| {
                if let Some((listener, port)) = listener {
                    let join = thread::spawn(move || {
                        if let Ok((stream, _)) = listener.accept() {
                            keep_socket_open(stream);
                        }
                    });

                    let mut adapter = DebugAdapter::new();
                    initialize_adapter(&mut adapter);
                    let response = adapter.handle_request(
                        20,
                        "attach",
                        Some(json!({ "host": "127.0.0.1", "port": port, "timeout": 50 })),
                    );
                    black_box(response);
                    let _ = join.join();
                }
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("set_breakpoints_100", |b| {
        let mut adapter = DebugAdapter::new();
        initialize_adapter(&mut adapter);
        let breakpoints = (1..=100).map(|line| json!({ "line": line })).collect::<Vec<_>>();
        let request = json!({
            "source": { "path": script.path.to_string_lossy().to_string() },
            "breakpoints": breakpoints
        });
        b.iter(|| {
            black_box(adapter.handle_request(30, "setBreakpoints", Some(request.clone())));
        });
    });

    group.bench_function("step_continue_p95", |b| {
        b.iter_batched(
            || launch_adapter(&script, true),
            |maybe_adapter| {
                if let Some(mut adapter) = maybe_adapter {
                    let _ = adapter.handle_request(41, "next", Some(json!({ "threadId": 1 })));
                    black_box(adapter.handle_request(
                        42,
                        "continue",
                        Some(json!({ "threadId": 1 })),
                    ));
                    shutdown_adapter(&mut adapter);
                }
            },
            BatchSize::SmallInput,
        );
    });

    if let Some(mut live_adapter) = launch_adapter(&script, true) {
        group.bench_function("stack_trace_live", |b| {
            b.iter(|| {
                black_box(live_adapter.handle_request(
                    50,
                    "stackTrace",
                    Some(json!({ "threadId": 1 })),
                ));
            })
        });

        group.bench_function("variables_root", |b| {
            b.iter(|| {
                black_box(live_adapter.handle_request(
                    60,
                    "variables",
                    Some(json!({ "variablesReference": 11 })),
                ));
            })
        });

        group.bench_function("variables_child_page", |b| {
            b.iter(|| {
                let root = live_adapter.handle_request(
                    70,
                    "variables",
                    Some(json!({ "variablesReference": 11 })),
                );
                let child_ref = match root {
                    DapMessage::Response { body: Some(body), .. } => find_child_reference(&body),
                    _ => None,
                }
                .unwrap_or(1101);

                black_box(live_adapter.handle_request(
                    71,
                    "variables",
                    Some(json!({ "variablesReference": child_ref, "start": 0, "count": 25 })),
                ));
            })
        });

        group.bench_function("evaluate_safe_blocked", |b| {
            b.iter(|| {
                black_box(live_adapter.handle_request(
                    80,
                    "evaluate",
                    Some(json!({
                        "expression": "system('touch /tmp/not_allowed')",
                        "allowSideEffects": false
                    })),
                ));
            })
        });

        group.bench_function("evaluate_live_simple", |b| {
            b.iter(|| {
                black_box(live_adapter.handle_request(
                    81,
                    "evaluate",
                    Some(json!({ "expression": "$result", "allowSideEffects": true })),
                ));
            })
        });

        shutdown_adapter(&mut live_adapter);
    }

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

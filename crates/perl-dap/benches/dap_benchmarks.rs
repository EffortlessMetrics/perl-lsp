//! DAP Performance Benchmarks (AC14, AC15) - Phase 1
//!
//! Performance benchmarks for Phase 1 DAP adapter operations:
//! - Configuration creation and validation
//! - Path resolution and normalization
//! - Perl binary resolution
//! - Environment setup
//! - Argument formatting
//!
//! Specification: docs/DAP_IMPLEMENTATION_SPECIFICATION.md#performance-specifications
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
//! cargo bench -p perl-dap
//!
//! # Run specific benchmark group
//! cargo bench -p perl-dap --bench dap_benchmarks -- configuration
//! cargo bench -p perl-dap --bench dap_benchmarks -- platform
//!
//! # Run with shorter measurement time (for CI)
//! cargo bench -p perl-dap -- --measurement-time 5
//! ```

#![allow(clippy::unwrap_used, clippy::expect_used)]

use criterion::{Criterion, criterion_group, criterion_main};
use perl_dap::configuration::{AttachConfiguration, LaunchConfiguration};
use perl_dap::platform::{
    format_command_args, normalize_path, resolve_perl_path, setup_environment,
};
use std::collections::HashMap;
use std::hint::black_box;
use std::path::PathBuf;
use std::time::Duration;

// ========== Configuration Benchmarks (AC14) ==========

/// Benchmark LaunchConfiguration creation
/// Target: <50ms
fn benchmark_launch_config_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("configuration");
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("launch_config_creation", |b| {
        b.iter(|| {
            let _config = LaunchConfiguration {
                program: black_box(PathBuf::from("/tmp/test.pl")),
                args: black_box(vec!["arg1".to_string(), "arg2".to_string()]),
                cwd: Some(PathBuf::from("/tmp")),
                env: HashMap::new(),
                perl_path: None,
                include_paths: vec![],
            };
        })
    });

    group.bench_function("launch_config_creation_with_include_paths", |b| {
        b.iter(|| {
            let _config = LaunchConfiguration {
                program: black_box(PathBuf::from("/tmp/test.pl")),
                args: black_box(vec![]),
                cwd: None,
                env: HashMap::new(),
                perl_path: None,
                include_paths: black_box(vec![
                    PathBuf::from("/usr/local/lib/perl5"),
                    PathBuf::from("/home/user/lib"),
                    PathBuf::from("./local/lib/perl5"),
                ]),
            };
        })
    });

    group.bench_function("launch_config_creation_with_env", |b| {
        let mut env = HashMap::new();
        env.insert("PERL5LIB".to_string(), "/custom/lib".to_string());
        env.insert("DEBUG".to_string(), "1".to_string());

        b.iter(|| {
            let _config = LaunchConfiguration {
                program: black_box(PathBuf::from("/tmp/test.pl")),
                args: black_box(vec![]),
                cwd: None,
                env: black_box(env.clone()),
                perl_path: Some(PathBuf::from("/usr/bin/perl")),
                include_paths: vec![],
            };
        })
    });

    group.finish();
}

/// Benchmark LaunchConfiguration validation
/// Target: <50ms
fn benchmark_launch_config_validation(c: &mut Criterion) {
    use std::fs;

    let mut group = c.benchmark_group("configuration_validation");
    group.measurement_time(Duration::from_secs(10));

    // Create temp file for validation
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("benchmark_test.pl");
    fs::write(&temp_file, "#!/usr/bin/env perl\nprint 'test';\n")
        .expect("Failed to create temp file");

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
            black_box(config.validate()).expect("Validation should succeed");
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
            config.resolve_paths(&workspace_root).expect("Path resolution should succeed");
        })
    });

    // Clean up temp file
    let _ = fs::remove_file(&temp_file);

    group.finish();
}

/// Benchmark AttachConfiguration creation
/// Target: <50ms (trivial, but measure for baseline)
fn benchmark_attach_config_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("attach_configuration");
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("attach_config_creation", |b| {
        b.iter(|| {
            let _config = AttachConfiguration {
                host: black_box("localhost".to_string()),
                port: black_box(13603),
                timeout_ms: Some(5000),
            };
        })
    });

    group.bench_function("attach_config_creation_remote", |b| {
        b.iter(|| {
            let _config = AttachConfiguration {
                host: black_box("192.168.1.100".to_string()),
                port: black_box(9000),
                timeout_ms: Some(5000),
            };
        })
    });

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

// ========== Benchmark Groups ==========

criterion_group!(
    configuration_benches,
    benchmark_launch_config_creation,
    benchmark_launch_config_validation,
    benchmark_attach_config_creation
);

criterion_group!(
    platform_benches,
    benchmark_perl_path_resolution,
    benchmark_path_normalization,
    benchmark_environment_setup,
    benchmark_arg_formatting
);

criterion_main!(configuration_benches, platform_benches);

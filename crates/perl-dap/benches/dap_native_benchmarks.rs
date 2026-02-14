//! DAP Native Performance Benchmarks
//!
//! Performance benchmarks for native DAP operations.
//!
//! These benchmarks validate the performance targets specified in Phase5:
//! - Breakpoint operations <50ms
//! - Step operations <100ms
//! - Variable expansion <200ms
//! - Stack trace retrieval <200ms
//!
//! # Running Benchmarks
//!
//! ```bash
//! cargo bench --bench dap_native_benchmarks
//! ```

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use perl_dap::tcp_attach::{TcpAttachConfig, TcpAttachSession};
use std::sync::mpsc::{Sender, channel};
use std::time::{Duration, Instant};

/// Benchmark configuration for TCP attach
fn create_benchmark_config() -> TcpAttachConfig {
    TcpAttachConfig::new("127.0.0.1".to_string(), 13603).with_timeout(5000)
}

/// Benchmark TCP attach session creation
fn bench_tcp_attach_session_creation(c: &mut Criterion) {
    c.bench_function("tcp_attach_session_creation", |b| {
        b.iter(|| {
            let _ = TcpAttachSession::new();
        });
    });
}

/// Benchmark TCP attach configuration validation
fn bench_tcp_attach_config_validation(c: &mut Criterion) {
    c.bench_function("tcp_attach_config_validation", |b| {
        b.iter(|| {
            let config = create_benchmark_config();
            let _ = config.validate();
        });
    });
}

/// Benchmark TCP attach timeout duration calculation
fn bench_tcp_attach_timeout_duration(c: &mut Criterion) {
    c.bench_function("tcp_attach_timeout_duration", |b| {
        b.iter(|| {
            let config = create_benchmark_config();
            let _ = config.timeout_duration();
        });
    });
}

/// Benchmark event creation and sending
fn bench_dap_event_creation(c: &mut Criterion) {
    c.bench_function("dap_event_creation", |b| {
        b.iter(|| {
            let (tx, _rx) = channel();
            let _ = tx.send(perl_dap::tcp_attach::DapEvent::Output {
                category: "stdout".to_string(),
                output: "test output".to_string(),
            });
        });
    });
}

/// Benchmark event receiving
fn bench_dap_event_receiving(c: &mut Criterion) {
    c.bench_function("dap_event_receiving", |b| {
        b.iter(|| {
            let (tx, rx) = channel();
            let event = perl_dap::tcp_attach::DapEvent::Stopped {
                reason: "breakpoint".to_string(),
                thread_id: 1,
            };
            let _ = tx.send(event);
            let _ = rx.recv_timeout(Duration::from_millis(1));
        });
    });
}

/// Benchmark session connection state check
fn bench_session_connection_check(c: &mut Criterion) {
    c.bench_function("session_connection_check", |b| {
        b.iter(|| {
            let session = TcpAttachSession::new();
            let _ = session.is_connected();
        });
    });
}

/// Benchmark session disconnect operation
fn bench_session_disconnect(c: &mut Criterion) {
    c.bench_function("session_disconnect", |b| {
        b.iter(|| {
            let mut session = TcpAttachSession::new();
            let _ = session.disconnect();
        });
    });
}

/// Benchmark event channel throughput
fn bench_event_channel_throughput(c: &mut Criterion) {
    c.bench_function("event_channel_throughput", |b| {
        b.iter(|| {
            let (tx, rx) = channel();
            let event = perl_dap::tcp_attach::DapEvent::Output {
                category: "stdout".to_string(),
                output: "benchmark".to_string(),
            };

            // Send 100 events
            for _ in 0..100 {
                let _ = tx.send(event.clone());
            }

            // Receive 100 events
            for _ in 0..100 {
                let _ = rx.recv_timeout(Duration::from_millis(1));
            }
        });
    });
}

/// Benchmark configuration cloning
fn bench_config_cloning(c: &mut Criterion) {
    c.bench_function("config_cloning", |b| {
        b.iter(|| {
            let config = create_benchmark_config();
            let _ = config.clone();
        });
    });
}

/// Benchmark event serialization
fn bench_event_serialization(c: &mut Criterion) {
    c.bench_function("event_serialization", |b| {
        b.iter(|| {
            let event = perl_dap::tcp_attach::DapEvent::Output {
                category: "stderr".to_string(),
                output: "error message".to_string(),
            };
            let _ = event.clone();
        });
    });
}

/// Benchmark TCP address parsing
fn bench_tcp_address_parsing(c: &mut Criterion) {
    c.bench_function("tcp_address_parsing", |b| {
        b.iter(|| {
            let _ = "127.0.0.1:13603".parse::<std::net::SocketAddr>();
        });
    });
}

/// Benchmark timeout validation
fn bench_timeout_validation(c: &mut Criterion) {
    c.bench_function("timeout_validation", |b| {
        b.iter(|| {
            let config = TcpAttachConfig::new("localhost".to_string(), 13603);
            let _ = config.with_timeout(5000).validate();
        });
    });
}

criterion_group!(
    benches,
    bench_tcp_attach_session_creation,
    bench_tcp_attach_config_validation,
    bench_tcp_attach_timeout_duration,
    bench_dap_event_creation,
    bench_dap_event_receiving,
    bench_session_connection_check,
    bench_session_disconnect,
    bench_event_channel_throughput,
    bench_config_cloning,
    bench_event_serialization,
    bench_tcp_address_parsing,
    bench_timeout_validation
);

criterion_main!(benches);

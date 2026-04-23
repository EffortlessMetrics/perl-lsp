# Known Flaky Tests

This document catalogs tests that exhibit non-deterministic behavior (flakiness) in the perl-lsp and perl-parser test suites. Each entry includes root cause analysis, current mitigations, and guidance for reliable local execution.

## Quick Reference

| Test File/Group | Failure Type | Requires | Tracking Issue |
|-----------------|--------------|----------|----------------|
| `lsp_document_symbols_test` | BrokenPipe | `RUST_TEST_THREADS=2` | - |
| `lsp_document_links_test` | BrokenPipe | `RUST_TEST_THREADS=2` | - |
| `lsp_encoding_edge_cases` | BrokenPipe, Timeout | `RUST_TEST_THREADS=2` | Issue #200 |
| `lsp_cancellation_infrastructure_tests` | Timeout, Race | `RUST_TEST_THREADS=1` | Issue #48 |
| `lsp_cancellation_parser_integration_tests` | Timeout, Race | `RUST_TEST_THREADS=1` | Issue #48 |

---

## Root Cause Analysis

Understanding why tests fail is critical for both immediate mitigation and long-term resolution. This section provides detailed technical analysis of each failure mode.

### BrokenPipe Failures

#### What is BrokenPipe?

A `BrokenPipe` error (errno EPIPE) occurs when a process attempts to write to a pipe whose reading end has been closed. In the context of LSP tests, this typically happens when:

```
┌─────────────────┐     stdin      ┌─────────────────┐
│   Test Process  │ ──────────────▶│   LSP Server    │
│   (Writer)      │                │   (Reader)      │
└─────────────────┘                └─────────────────┘
         │                                │
         │           stdout               │
         ◀────────────────────────────────┘
         
When LSP Server terminates unexpectedly:
         │                                
         ╳  BrokenPipe!                   
         │                                
```

#### Why BrokenPipe Occurs in LSP Tests

1. **Process Termination During Communication**
   - The LSP server process may crash or exit while the test is still sending requests
   - Common causes: unhandled panics, memory exhaustion, signal termination

2. **Resource Contention in CI**
   - Multiple concurrent LSP server instances compete for system resources
   - Memory pressure causes the OS to terminate processes (OOM killer)
   - File descriptor exhaustion prevents new pipe creation

3. **Race Conditions in Shutdown**
   - Test completes and closes stdin before server finishes writing response
   - Server attempts to write to stdout after test has closed the read end

#### Technical Deep Dive

The LSP test infrastructure uses stdio for JSON-RPC communication:

```rust
// From crates/perl-lsp-rs/tests/common/mod.rs
pub struct LspServer {
    pub process: Child,
    writer: BufWriter<ChildStdin>,  // Writes to server's stdin
    rx: Receiver<Value>,             // Receives from server's stdout
    // ...
}
```

When the server process terminates:
1. The OS closes all file descriptors associated with the process
2. The pipe's write end (server's stdout) is closed
3. If the test tries to read, it gets EOF
4. If the test tries to write to server's stdin, it gets `BrokenPipe`

#### Detection Pattern

```rust
// Error handling in common/mod.rs
const ERR_CONNECTION_CLOSED: i64 = -32050;

fn map_send_error(e: io::Error) -> TestError {
    if e.kind() == io::ErrorKind::BrokenPipe {
        TestError::ConnectionClosed("LSP server terminated unexpectedly".into())
    } else {
        TestError::Io(e)
    }
}
```

---

### Timeout Failures

#### What Causes Timeouts?

Timeouts occur when operations take longer than the configured threshold. In LSP tests, this happens due to:

```
Timeline of a Timeout Failure:

T=0ms     Test sends request to LSP server
T=0-5ms   Server receives and queues request
T=5-50ms  Server initializes parser/analyzers
T=50-???  Server processes request (variable time)
          │
          ├─▶ Normal: <500ms for simple operations
          ├─▶ Slow:   <2000ms for workspace operations
          └─▶ Problem: >5000ms (default timeout)
          
T=5000ms  Test times out waiting for response
```

#### Why Timeouts Occur in LSP Tests

1. **Unicode Processing Overhead**
   - Complex grapheme clusters require expensive boundary detection
   - UTF-8 to UTF-16 position conversion is O(n) for each position
   - Emoji and surrogate pairs add additional processing

2. **Resource Contention**
   - Multiple LSP servers compete for CPU time
   - Parser initialization is CPU-intensive
   - Memory pressure causes swapping

3. **Large File Processing**
   - Test fixtures include 2000+ line files
   - Full AST parsing is O(file_size)
   - Workspace indexing requires reading multiple files

4. **CI Environment Constraints**
   - Limited CPU cores (often 2)
   - Shared resources with other jobs
   - Network filesystem latency

#### Adaptive Timeout Strategy

The test infrastructure implements adaptive timeouts based on thread constraints:

```rust
// From crates/perl-lsp-rs/tests/lsp_encoding_edge_cases.rs
fn compute_adaptive_timeout() -> std::time::Duration {
    let rust_test_threads = std::env::var("RUST_TEST_THREADS")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(usize::MAX);

    match rust_test_threads {
        0..=2  => Duration::from_secs(60),  // High contention
        3..=4  => Duration::from_secs(45),  // Medium contention
        _      => Duration::from_secs(30),  // Low/no contention
    }
}
```

#### Timeout Profiles

| Profile | Base Timeout | Scaled (RUST_TEST_THREADS=2) | Use Case |
|---------|-------------|------------------------------|----------|
| Quick | 100ms | 400ms | Fast validation |
| Standard | 500ms | 2000ms | Normal operations |
| Initialization | 2000ms | 8000ms | Server startup |
| Workspace | 5000ms | 20000ms | Full workspace ops |
| Stress | 10000ms | 40000ms | Heavy load scenarios |

---

### Race Conditions

#### What are Race Conditions?

Race conditions occur when the outcome of a program depends on the relative timing of events. In concurrent tests, this means:

```
Thread A                    Thread B
─────────                   ─────────
lock.acquire()
                            lock.acquire() [BLOCKED]
modify shared_state
                            ⬇ (waiting)
lock.release()
                            lock.acquire() [SUCCESS]
                            modify shared_state
                            
If timing differs:
                            
lock.acquire()
modify shared_state
                            lock.acquire() [SUCCESS - before release!]
                            read partial_state [RACE!]
```

#### Why Race Conditions Occur in LSP Tests

1. **Shared Cancellation State**
   - Cancellation tokens are accessed by multiple threads
   - Atomic operations provide ordering guarantees but tests may assume specific timing

2. **LSP Server Initialization**
   - Server startup is asynchronous
   - Tests may send requests before server is ready
   - Global mutex (`LSP_SERVER_MUTEX`) serializes but doesn't eliminate timing issues

3. **Concurrent Test Execution**
   - Rust's test harness runs tests in parallel by default
   - Tests share system resources (ports, memory, file descriptors)
   - Order of test execution is non-deterministic

4. **CI Environment Variability**
   - Resource availability varies between runs
   - CPU scheduling differs from local execution
   - Network/disk latency affects timing

#### Detection and Mitigation

```rust
// Global mutex to serialize LSP server creation
static LSP_SERVER_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

// Thread requirement validation in cancellation tests
fn validate_thread_requirements() {
    let threads = max_concurrent_threads();
    if threads > 1 {
        panic!(
            "This test requires RUST_TEST_THREADS=1. \
             Current: {}. Set environment variable and retry.",
            threads
        );
    }
}
```

---

## Flaky Test Details

### 1. lsp_document_symbols_test

**File**: `crates/perl-lsp-rs/tests/lsp_document_symbols_test.rs`

**Test Count**: 10+ tests

**Symptoms**:
- `BrokenPipe` errors during LSP server communication
- Intermittent timeouts during response reading
- Server shutdown race conditions

**Root Cause Analysis**:

The test file spawns LSP server instances using the in-process `LspServer` struct rather than the common test harness. This bypasses the global mutex serialization:

```rust
// From lsp_document_symbols_test.rs - Uses in-process server
fn setup_server() -> LspServer {
    let mut server = LspServer::new();
    // ... initialization
    server
}
```

When multiple tests run in parallel:
1. Each creates its own parser instance
2. Memory usage multiplies by test count
3. CPU contention slows all parsing operations
4. Some tests timeout waiting for responses

**Current Mitigations**:
- Graceful error handling in `common/mod.rs` with `map_send_error()`
- Global mutex `LSP_SERVER_MUTEX` to serialize server creation (not used by this test)
- Adaptive timeout scaling based on `RUST_TEST_THREADS`

**Reliable Local Execution**:
```bash
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs --test lsp_document_symbols_test -- --test-threads=2
```

**Proposed Long-term Fix**:

1. **Migrate to Common Test Harness**
   ```rust
   // Instead of:
   fn setup_server() -> LspServer { LspServer::new() }
   
   // Use:
   fn setup_server() -> LspServer { 
       start_lsp_server()  // Uses common harness with mutex
   }
   ```

2. **Add Test-specific Timeouts**
   ```rust
   // Per-test timeout configuration
   #[test]
   fn test_document_symbols_basic() -> TestResult {
       let _guard = TEST_MUTEX.lock().unwrap();
       let timeout = get_adaptive_timeout(TimeoutProfile::Standard);
       // ... test implementation
   }
   ```

3. **Implement Test Isolation**
   - Use unique port/workspace directories per test
   - Add cleanup guarantees via `Drop` implementations

**Tracking Issue**: None (mitigated)

---

### 2. lsp_document_links_test

**File**: `crates/perl-lsp-rs/tests/lsp_document_links_test.rs`

**Test Count**: 2 tests

**Symptoms**:
- `BrokenPipe` errors when sending notifications
- Server process exits unexpectedly

**Root Cause Analysis**:

The test file contains minimal tests that don't actually spawn an LSP server process:

```rust
// From lsp_document_links_test.rs
#[test]
fn test_document_links_basic() -> Result<(), Box<dyn std::error::Error>> {
    use url::Url;
    let uri: Url = "file:///workspace/test.pl".parse()?;
    let _text = r#"
    use Data::Dumper;
    require JSON::XS;
    "#;
    // This would call the internal function, but we can't access it directly
    assert!(uri.scheme() == "file");
    Ok(())
}
```

The flakiness is inherited from running alongside other LSP tests that do spawn servers. When the test suite runs in parallel, resource contention from other tests affects the overall test environment.

**Current Mitigations**:
- Error tolerant notification sending (ignores `BrokenPipe` during teardown)
- Shared LSP server creation mutex in common harness

**Reliable Local Execution**:
```bash
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs --test lsp_document_links_test -- --test-threads=2
```

**Proposed Long-term Fix**:

1. **Complete Test Implementation**
   ```rust
   #[test]
   fn test_document_links_basic() -> TestResult {
       let mut server = start_lsp_server();
       initialize_lsp(&mut server);
       
       let content = r#"
       use Data::Dumper;
       require JSON::XS;
       use Foo::Bar::Baz;
       "#;
       
       send_notification(&mut server, json!({
           "jsonrpc": "2.0",
           "method": "textDocument/didOpen",
           "params": {
               "textDocument": {
                   "uri": "file:///test.pl",
                   "languageId": "perl",
                   "version": 1,
                   "text": content
               }
           }
       }));
       
       let response = send_request(&mut server, json!({
           "jsonrpc": "2.0",
           "id": 1,
           "method": "textDocument/documentLink",
           "params": {
               "textDocument": {"uri": "file:///test.pl"}
           }
       }));
       
       // Validate links are returned
       let links = response["result"].as_array().expect("links array");
       assert!(!links.is_empty());
       Ok(())
   }
   ```

2. **Isolate from Other Tests**
   - Move to dedicated test file with `#[serial]` attribute
   - Or implement proper resource cleanup

**Tracking Issue**: None (mitigated)

---

### 3. lsp_encoding_edge_cases

**File**: `crates/perl-lsp-rs/tests/lsp_encoding_edge_cases.rs`

**Test Count**: 15+ tests

**Symptoms**:
- Timeouts during Unicode content processing
- `BrokenPipe` errors with complex grapheme clusters
- Performance regression on constrained hardware

**Root Cause Analysis**:

Unicode processing requires significantly more computational resources:

```rust
// Unicode complexity analysis from the test file
fn analyze_unicode_complexity(text: &str) -> (usize, usize, usize) {
    let mut char_count = 0;
    let mut emoji_count = 0;
    let mut complex_char_count = 0;

    for ch in text.chars() {
        char_count += 1;
        let ch_u32 = ch as u32;
        
        // Emojis: U+1F300 to U+1F9FF, Miscellaneous Symbols: U+2600 to U+27BF
        if matches!(ch_u32, 0x1F300..=0x1F9FF | 0x2600..=0x27BF) {
            emoji_count += 1;
        }
        
        // Surrogate pairs, combining marks, etc.
        if ch_u32 > 0xFFFF || ch.len_utf8() > 2 {
            complex_char_count += 1;
        }
    }
    (char_count, emoji_count, complex_char_count)
}
```

**Why This Causes Flakiness**:

1. **UTF-16 Position Conversion**
   - LSP uses UTF-16 positions (legacy requirement)
   - Each position requires scanning from string start
   - O(n) per position lookup, O(n²) for multiple lookups

2. **Grapheme Cluster Handling**
   - Combining characters (e.g., é = e + ◌́) count as one grapheme
   - But may be multiple UTF-8 bytes and UTF-16 code units
   - Position calculations must account for all encodings

3. **Memory Pressure**
   - Large Unicode strings consume more memory
   - Multiple concurrent tests multiply memory usage
   - GC/swap introduces non-deterministic delays

**Current Mitigations**:
- Adaptive timeout computation via `compute_adaptive_timeout()`:
  ```rust
  if rust_test_threads <= 2 {
      Duration::from_secs(60)  // High contention
  } else if rust_test_threads <= 4 {
      Duration::from_secs(45)  // Medium contention
  } else {
      Duration::from_secs(30)  // Low/no contention
  }
  ```
- Simplified Unicode test cases focused on critical symbols
- Graceful fallback when document symbols request times out

**Reliable Local Execution**:
```bash
# Standard execution with thread constraints
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs --test lsp_encoding_edge_cases -- --test-threads=2

# For specific problematic tests
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs --test lsp_encoding_edge_cases -- test_emoji_and_special_unicode --nocapture

# Debug Unicode processing
LSP_TEST_ECHO_STDERR=1 RUST_TEST_THREADS=1 cargo test -p perl-lsp-rs --test lsp_encoding_edge_cases -- --nocapture
```

**Proposed Long-term Fix**:

1. **Implement UTF-16 Position Cache**
   ```rust
   struct Utf16PositionCache {
       text: String,
       // Maps byte offset to UTF-16 code unit count
       byte_to_utf16: Vec<usize>,
       // Maps UTF-16 code unit offset to byte position
       utf16_to_byte: Vec<usize>,
   }
   
   impl Utf16PositionCache {
       fn new(text: &str) -> Self {
           let mut byte_to_utf16 = Vec::with_capacity(text.len());
           let mut utf16_to_byte = Vec::new();
           let mut utf16_count = 0;
           
           for (byte_idx, ch) in text.char_indices() {
               byte_to_utf16.push(utf16_count);
               let utf16_len = ch.len_utf16();
               for _ in 0..utf16_len {
                   utf16_to_byte.push(byte_idx);
               }
               utf16_count += utf16_len;
           }
           
           Self { text: text.to_string(), byte_to_utf16, utf16_to_byte }
       }
       
       fn utf16_to_byte(&self, utf16_pos: usize) -> Option<usize> {
           self.utf16_to_byte.get(utf16_pos).copied()
       }
   }
   ```

2. **Parallelize Unicode Processing**
   - Use `rayon` for parallel grapheme analysis
   - Split large documents into chunks for concurrent processing

3. **Add Performance Regression Tests**
   ```rust
   #[test]
   fn test_unicode_processing_performance() {
       let large_unicode_text = generate_complex_unicode(10000);
       let start = Instant::now();
       
       let cache = Utf16PositionCache::new(&large_unicode_text);
       
       // Should complete in <100ms for 10K characters
       assert!(start.elapsed() < Duration::from_millis(100));
   }
   ```

**Tracking Issue**: Issue #200

---

### 4. lsp_cancellation_infrastructure_tests (4 tests)

**File**: `crates/perl-lsp-rs/tests/lsp_cancellation_infrastructure_tests.rs`

**Affected Tests**:
- `test_infrastructure_cleanup_and_resource_management_ac9`
- `test_deadlock_detection_and_prevention_ac10`
- `test_lsp_infrastructure_integration_ac11`
- `test_lsp_regression_prevention_ac11`

**Symptoms**:
- Race conditions during cancellation token operations
- Deadlock detection false positives
- LSP initialization failures in CI environments

**Root Cause Analysis**:

These tests validate thread-safety and cancellation infrastructure. The test fixture creates extensive monitoring:

```rust
// From lsp_cancellation_infrastructure_tests.rs
struct InfrastructureTestFixture {
    server: LspServer,
    resource_monitor: ResourceMonitor,
    thread_safety_monitor: ThreadSafetyMonitor,
    integration_validator: IntegrationValidator,
}

struct ResourceMonitor {
    memory_snapshots: Arc<Mutex<Vec<MemorySnapshot>>>,
    file_handle_count: Arc<AtomicUsize>,
    thread_count: Arc<AtomicUsize>,
    network_connections: Arc<AtomicUsize>,
    cleanup_operations: Arc<AtomicU64>,
}
```

**Why This Causes Flakiness**:

1. **Concurrent State Access**
   - Multiple threads read/write shared atomic state
   - Memory ordering (Relaxed vs SeqCst) affects visibility
   - Timing between threads is non-deterministic

2. **Deadlock Detection Complexity**
   - Tests intentionally create deadlock scenarios
   - Detection logic must distinguish real deadlocks from slow operations
   - Timeout-based detection is inherently racy

3. **CI Resource Constraints**
   - Limited CPU cores amplify timing issues
   - Memory pressure causes unpredictable scheduling
   - Shared runners have variable performance

**Current Mitigations**:
- Explicit `RUST_TEST_THREADS=1` requirement check at test start:
  ```rust
  fn validate_single_threaded() {
      let threads = max_concurrent_threads();
      if threads > 1 {
          eprintln!("ERROR: This test requires RUST_TEST_THREADS=1");
          eprintln!("Current value: {}", threads);
          panic!("Thread constraint violation");
      }
  }
  ```
- CI environment detection with graceful skip:
  ```rust
  if std::env::var("CI").is_ok()
      || std::env::var("GITHUB_ACTIONS").is_ok()
      || std::env::var("CONTINUOUS_INTEGRATION").is_ok()
  {
      eprintln!("Skipping in CI environment for stability");
      return;
  }
  ```
- Enhanced retry logic for LSP initialization (up to 2 retries)

**Reliable Local Execution**:
```bash
# Required: Single-threaded execution
RUST_TEST_THREADS=1 cargo test -p perl-lsp-rs --test lsp_cancellation_infrastructure_tests -- --test-threads=1

# Individual test execution
RUST_TEST_THREADS=1 cargo test test_infrastructure_cleanup_and_resource_management_ac9 -- --nocapture

# With debug output
LSP_TEST_DEBUG_READER=1 LSP_TEST_ECHO_STDERR=1 RUST_TEST_THREADS=1 cargo test -p perl-lsp-rs --test lsp_cancellation_infrastructure_tests -- --nocapture
```

**Proposed Long-term Fix**:

1. **Implement Deterministic Test Mode**
   ```rust
   #[cfg(test)]
   mod deterministic {
       use std::sync::Barrier;
       
       pub struct DeterministicTestFixture {
           barrier: Arc<Barrier>,
       }
       
       impl DeterministicTestFixture {
           pub fn new(thread_count: usize) -> Self {
               Self {
                   barrier: Arc::new(Barrier::new(thread_count)),
               }
           }
           
           pub fn sync_point(&self) {
               self.barrier.wait();
           }
       }
   }
   ```

2. **Use Mock Time for Timeout Tests**
   ```rust
   struct MockClock {
       current_time: Arc<AtomicU64>,
   }
   
   impl MockClock {
       fn advance(&self, duration: Duration) {
           self.current_time.fetch_add(
               duration.as_millis() as u64,
               Ordering::SeqCst
           );
       }
   }
   ```

3. **Separate CI and Local Test Modes**
   ```rust
   #[cfg_attr(not(feature = "ci-tests"), ignore)]
   #[test]
   fn test_deadlock_detection() {
       // Only runs with --features ci-tests
   }
   ```

**Tracking Issue**: Issue #48 (LSP Cancellation Enhancement)

---

### 5. lsp_cancellation_parser_integration_tests (5 tests)

**File**: `crates/perl-lsp-rs/tests/lsp_cancellation_parser_integration_tests.rs`

**Affected Tests**:
- `test_incremental_parsing_checkpoint_cancellation_ac6`
- `test_workspace_indexing_cancellation_integrity_ac7`
- `test_dual_pattern_indexing_cancellation_ac7`
- `test_cross_file_reference_cancellation_ac8`
- `test_multi_tier_resolver_cancellation_ac8`

**Symptoms**:
- Parser fixture initialization timeouts
- Workspace indexing race conditions
- Cross-file reference resolution failures

**Root Cause Analysis**:

Parser integration tests create substantial test workspaces:

```rust
// From lsp_cancellation_parser_integration_tests.rs
fn create_parser_integration_test_files() -> HashMap<String, String> {
    let mut files = HashMap::new();
    
    // Base module for incremental parsing tests
    files.insert("file:///lib/BaseModule.pm".to_string(), r#"
        package BaseModule;
        use strict;
        use warnings;
        
        sub base_function { ... }
        sub complex_base_function { ... }
        sub cross_reference_target { ... }
        1;
    "#.to_string());
    
    // Extended module for workspace indexing tests
    files.insert("file:///lib/ExtendedModule.pm".to_string(), ...);
    
    // Additional files for cross-file testing
    // ...
}
```

**Why This Causes Flakiness**:

1. **Large Fixture Initialization**
   - Multiple files must be parsed before tests can run
   - Each file requires full AST generation
   - Total initialization time can exceed default timeouts

2. **Incremental Parsing Complexity**
   - Tests modify files and verify incremental updates
   - Incremental parsing must complete within <1ms requirement
   - Concurrent modifications cause race conditions

3. **Cross-File State Dependencies**
   - Tests assume specific indexing state
   - Concurrent tests may modify shared workspace
   - State isolation is incomplete

**Current Mitigations**:
- Feature-gated stress tests (`#[cfg_attr(not(feature = "stress-tests"), ignore)]`)
- Adaptive fixture initialization timeout:
  ```rust
  let adaptive_timeout = match max_concurrent_threads() {
      0..=2 => Duration::from_secs(30),  // Heavily constrained
      3..=4 => Duration::from_secs(20),  // Moderately constrained
      5..=8 => Duration::from_secs(15),  // Lightly constrained
      _ => Duration::from_secs(10),      // Unconstrained
  };
  ```
- CI environment skip for stability

**Reliable Local Execution**:
```bash
# Required: Single-threaded execution
RUST_TEST_THREADS=1 cargo test -p perl-lsp-rs --test lsp_cancellation_parser_integration_tests -- --test-threads=1

# Run stress tests (normally ignored)
RUST_TEST_THREADS=1 cargo test -p perl-lsp-rs --test lsp_cancellation_parser_integration_tests --features stress-tests -- --test-threads=1

# Individual test
RUST_TEST_THREADS=1 cargo test test_incremental_parsing_checkpoint_cancellation_ac6 -- --nocapture
```

**Proposed Long-term Fix**:

1. **Implement Test Workspace Isolation**
   ```rust
   struct IsolatedTestWorkspace {
       temp_dir: tempfile::TempDir,
       workspace_root: PathBuf,
   }
   
   impl IsolatedTestWorkspace {
       fn new() -> Self {
           let temp_dir = tempfile::tempdir().expect("temp dir");
           let workspace_root = temp_dir.path().to_path_buf();
           
           // Create isolated workspace with unique path
           Self { temp_dir, workspace_root }
       }
   }
   
   impl Drop for IsolatedTestWorkspace {
       fn drop(&mut self) {
           // Guaranteed cleanup
       }
   }
   ```

2. **Pre-computed Test Fixtures**
   ```rust
   // Build fixtures at compile time
   static PARSER_FIXTURES: LazyLock<HashMap<&'static str, &'static str>> = 
       LazyLock::new(|| {
           HashMap::from([
               ("base_module", include_str!("../fixtures/base_module.pm")),
               ("extended_module", include_str!("../fixtures/extended_module.pm")),
           ])
       });
   ```

3. **Incremental Parsing Benchmarks**
   ```rust
   #[bench]
   fn bench_incremental_parse(b: &mut test::Bencher) {
       let mut parser = Parser::new(FIXTURE_CODE);
       parser.parse();
       
       b.iter(|| {
           let change = TextChange {
               range: Range::new(Position::new(10, 0), Position::new(10, 5)),
               text: "modified".to_string(),
           };
           parser.apply_change(&change);
           parser.parse_incremental()
       });
       
       // Assert <1ms requirement
       assert!(b.avg_time() < Duration::from_millis(1));
   }
   ```

**Tracking Issue**: Issue #48 (LSP Cancellation Enhancement)

---

## Mitigation Guide

### Configuring RUST_TEST_THREADS

The `RUST_TEST_THREADS` environment variable controls how many tests run concurrently. Setting this appropriately is critical for flaky test mitigation.

#### What is RUST_TEST_THREADS?

```
Default behavior (RUST_TEST_THREADS not set):
┌─────────────────────────────────────────────────────────┐
│  Test Thread 1  │  Test Thread 2  │  Test Thread 3  │ ... │
│  ─────────────  │  ─────────────  │  ─────────────  │     │
│  LSP Server A   │  LSP Server B   │  LSP Server C   │     │
│  Parser Inst A  │  Parser Inst B  │  Parser Inst C  │     │
└─────────────────────────────────────────────────────────┘
         ↓                ↓                ↓
    Resource contention → Timeouts → Flaky failures

With RUST_TEST_THREADS=2:
┌─────────────────────────────────────┐
│  Test Thread 1  │  Test Thread 2  │
│  ─────────────  │  ─────────────  │
│  LSP Server A   │  LSP Server B   │
└─────────────────────────────────────┘
         ↓                ↓
    Reduced contention → Stable tests
```

#### Setting RUST_TEST_THREADS

**Temporary (single command):**
```bash
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs
```

**Shell session:**
```bash
export RUST_TEST_THREADS=2
cargo test -p perl-lsp-rs
```

**Permanent (shell config):**
```bash
# Add to ~/.bashrc or ~/.zshrc
export RUST_TEST_THREADS=2
```

**Cargo configuration:**
```toml
# .cargo/config.toml
[env]
RUST_TEST_THREADS = "2"
```

#### Recommended Values by Test Type

| Test Type | RUST_TEST_THREADS | --test-threads | Rationale |
|-----------|-------------------|----------------|-----------|
| Standard LSP tests | 2 | 2 | Balance parallelism and stability |
| Cancellation tests | 1 | 1 | Eliminate race conditions |
| Encoding tests | 2 | 2 | Reduce timeout risk |
| Unit tests | default | default | No I/O, safe to parallelize |
| CI pipeline | 2 | 2 | Proven stable configuration |

### Running Tests Locally

#### Standard Test Run
```bash
# Run all LSP tests with stable configuration
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs -- --test-threads=2
```

#### Specific Test File
```bash
# Run single test file
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs --test lsp_document_symbols_test -- --test-threads=2
```

#### Specific Test Function
```bash
# Run single test with pattern match
RUST_TEST_THREADS=1 cargo test -p perl-lsp-rs test_deadlock_detection -- --nocapture
```

#### Debug Mode
```bash
# Enable verbose output
LSP_TEST_ECHO_STDERR=1 LSP_TEST_DEBUG_READER=1 RUST_TEST_THREADS=1 \
    cargo test -p perl-lsp-rs --test lsp_cancellation_infrastructure_tests -- --nocapture
```

#### Extended Timeout
```bash
# For slow machines or debugging
LSP_TEST_TIMEOUT_MS=30000 RUST_TEST_THREADS=1 cargo test -p perl-lsp-rs -- --test-threads=1
```

### CI Configuration

#### GitHub Actions

```yaml
name: LSP Tests

on: [push, pull_request]

env:
  RUST_TEST_THREADS: "2"

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
      
      - name: Run standard LSP tests
        run: cargo test -p perl-lsp-rs -- --test-threads=2
      
      - name: Run cancellation tests (single-threaded)
        env:
          RUST_TEST_THREADS: "1"
        run: |
          cargo test -p perl-lsp-rs --test lsp_cancellation_infrastructure_tests -- --test-threads=1
          cargo test -p perl-lsp-rs --test lsp_cancellation_parser_integration_tests -- --test-threads=1
      
      - name: Run encoding tests
        run: cargo test -p perl-lsp-rs --test lsp_encoding_edge_cases -- --test-threads=2
```

#### Justfile Integration

```just
# Run LSP tests with proper threading
test-lsp:
    RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs -- --test-threads=2

# Run cancellation tests (requires single thread)
test-lsp-cancellation:
    RUST_TEST_THREADS=1 cargo test -p perl-lsp-rs --test lsp_cancellation_* -- --test-threads=1

# Run all tests with CI configuration
ci-test: test-lsp test-lsp-cancellation
```

---

## General Guidance

### How to Identify a Flaky Test

A test is considered flaky if it exhibits any of these behaviors:

1. **Non-deterministic failures**: Passes sometimes, fails others with identical code
2. **Environment sensitivity**: Fails only in CI or only locally
3. **Thread sensitivity**: Behavior changes with different `RUST_TEST_THREADS` values
4. **Timeout-related failures**: Different timeout thresholds change pass/fail rate
5. **Resource contention symptoms**: `BrokenPipe`, connection refused, or deadlock errors

**Debugging Commands**:
```bash
# Run with maximum verbosity
RUST_TEST_THREADS=1 cargo test <test_name> -- --nocapture 2>&1 | tee test_output.log

# Enable LSP debug output
LSP_TEST_ECHO_STDERR=1 RUST_TEST_THREADS=1 cargo test <test_name> -- --nocapture

# Enable reader thread debugging
LSP_TEST_DEBUG_READER=1 RUST_TEST_THREADS=1 cargo test <test_name> -- --nocapture

# Run with extended timeout
LSP_TEST_TIMEOUT_MS=30000 cargo test <test_name>

# Run test multiple times to detect flakiness
for i in {1..10}; do
    echo "Run $i"
    RUST_TEST_THREADS=2 cargo test <test_name> -- --test-threads=2 || break
done
```

### How to Report a New Flaky Test

When you encounter a new flaky test, please create a GitHub issue with:

1. **Test Name**: Full test path (e.g., `lsp_cancellation_infrastructure_tests::test_infrastructure_cleanup_and_resource_management_ac9`)

2. **Environment Details**:
   - OS and version
   - Rust version (`rustc --version`)
   - `RUST_TEST_THREADS` value (if set)
   - CI vs local execution

3. **Failure Mode**:
   - Error message (full stack trace if available)
   - Frequency (e.g., "fails 1 in 5 runs")
   - Any patterns (e.g., "only fails when run with other tests")

4. **Reproduction Steps**:
   ```bash
   # Command that reproduces the failure
   cargo test <test_name> -- --nocapture
   ```

5. **Label the Issue**: Use `flaky-test` and `ci-reliability` labels

### Process for Fixing Flaky Tests

1. **Immediate Mitigation**: Add to this document with known workarounds

2. **Short-term Fix Options**:
   - Add thread constraint requirements (`RUST_TEST_THREADS=1`)
   - Implement adaptive timeouts
   - Add retry logic for transient failures
   - Feature-gate stress tests

3. **Long-term Resolution**:
   - Identify and fix root cause (often race conditions)
   - Add proper synchronization primitives
   - Refactor test to be deterministic
   - Consider mocking external dependencies

4. **Validation**:
   - Run test 100+ times locally to verify fix
   - Monitor CI for 1-2 weeks after fix
   - Remove from this document once stable

---

## Environment Variables Reference

| Variable | Purpose | Default |
|----------|---------|---------|
| `RUST_TEST_THREADS` | Limits concurrent test execution | System core count |
| `LSP_TEST_TIMEOUT_MS` | Default per-request timeout (ms) | 5000 |
| `LSP_TEST_SHORT_MS` | Short timeout for optional responses (ms) | 500 |
| `LSP_TEST_ECHO_STDERR` | Echo perl-lsp stderr in tests | Disabled |
| `LSP_TEST_DEBUG_READER` | Debug LSP reader thread | Disabled |
| `PERL_LSP_BIN` | Explicit path to perl-lsp binary | Auto-detected |

---

## Related Documentation

- [ADR-0018: Adaptive Threading for LSP Tests](../adr/0018-adaptive-threading-tests.md) - Design rationale for adaptive timeouts
- [ADR-006: LSP Cancellation Infrastructure](../adr/ADR_006_LSP_CANCELLATION_INFRASTRUCTURE.md) - Cancellation system architecture
- [AGENTS.md](../../AGENTS.md) - Project development guidelines

---

## Changelog

| Date | Change | Author |
|------|--------|--------|
| 2025-12-31 | Initial documentation | Perl LSP Team |
| 2026-03-13 | Added comprehensive root cause analysis, mitigation guide, and proposed fixes | Perl LSP Team |

---

*Last updated: 2026-03-13*

use perl_lsp_perltidy::{PerlTidyConfig, PerlTidyFormatter};
use perl_subprocess_runtime::{SubprocessError, SubprocessOutput, SubprocessRuntime};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

// --- Missing perltidy binary handling ---

/// A mock runtime that always returns an error, simulating a missing binary.
struct MissingBinaryRuntime;

impl SubprocessRuntime for MissingBinaryRuntime {
    fn run_command(
        &self,
        program: &str,
        _args: &[&str],
        _stdin: Option<&[u8]>,
    ) -> Result<SubprocessOutput, SubprocessError> {
        Err(SubprocessError::new(format!("Failed to start {program}: No such file or directory")))
    }
}

#[test]
fn format_returns_error_when_binary_missing() {
    let runtime = Arc::new(MissingBinaryRuntime);
    let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

    let result = formatter.format("my $x = 1;");

    assert!(result.is_err());
    let err = perl_tdd_support::must_err(result);
    assert!(err.contains("Failed to start perltidy"));
}

#[test]
fn format_file_returns_error_when_binary_missing() {
    let runtime = Arc::new(MissingBinaryRuntime);
    let formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

    let result = formatter.format_file(Path::new("test.pl"));

    assert!(result.is_err());
    let err = perl_tdd_support::must_err(result);
    assert!(err.contains("Failed to start perltidy"));
}

#[test]
fn format_range_returns_error_when_binary_missing() {
    let runtime = Arc::new(MissingBinaryRuntime);
    let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

    let code = "line0\nline1\nline2";
    let result = formatter.format_range(code, 1, 1);

    assert!(result.is_err());
    let err = perl_tdd_support::must_err(result);
    assert!(err.contains("Failed to start perltidy"));
}

#[test]
fn get_suggestions_returns_error_when_binary_missing() {
    let runtime = Arc::new(MissingBinaryRuntime);
    let mut formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime);

    let result = formatter.get_suggestions("my $x = 1;");

    assert!(result.is_err());
    let err = perl_tdd_support::must_err(result);
    assert!(err.contains("Failed to start perltidy"));
}

// ──────────────────────────── PerlTidyConfig: timeout field ──────────────────

#[test]
fn perltidy_config_default_has_timeout() {
    let config = PerlTidyConfig::default();
    assert_eq!(config.timeout_secs, 10);
}

struct TrackingRuntime {
    active: AtomicUsize,
    max_active: AtomicUsize,
    invocations: AtomicUsize,
    emitted_stdout_bytes: AtomicUsize,
}

impl TrackingRuntime {
    fn new() -> Self {
        Self {
            active: AtomicUsize::new(0),
            max_active: AtomicUsize::new(0),
            invocations: AtomicUsize::new(0),
            emitted_stdout_bytes: AtomicUsize::new(0),
        }
    }

    fn active(&self) -> usize {
        self.active.load(Ordering::SeqCst)
    }

    fn max_active(&self) -> usize {
        self.max_active.load(Ordering::SeqCst)
    }

    fn invocations(&self) -> usize {
        self.invocations.load(Ordering::SeqCst)
    }

    fn emitted_stdout_bytes(&self) -> usize {
        self.emitted_stdout_bytes.load(Ordering::SeqCst)
    }
}

struct ActiveInvocation<'a> {
    runtime: &'a TrackingRuntime,
}

impl<'a> ActiveInvocation<'a> {
    fn new(runtime: &'a TrackingRuntime) -> Self {
        let active = runtime.active.fetch_add(1, Ordering::SeqCst) + 1;
        runtime.max_active.fetch_max(active, Ordering::SeqCst);
        runtime.invocations.fetch_add(1, Ordering::SeqCst);
        Self { runtime }
    }
}

impl Drop for ActiveInvocation<'_> {
    fn drop(&mut self) {
        self.runtime.active.fetch_sub(1, Ordering::SeqCst);
    }
}

impl SubprocessRuntime for TrackingRuntime {
    fn run_command(
        &self,
        program: &str,
        args: &[&str],
        stdin: Option<&[u8]>,
    ) -> Result<SubprocessOutput, SubprocessError> {
        let _active = ActiveInvocation::new(self);
        assert_eq!(program, "perltidy");
        assert!(stdin.is_none(), "format_file should not pass stdin");
        let file_arg = args.last().ok_or_else(|| SubprocessError::new("missing file path"))?;
        assert!(Path::new(file_arg).exists(), "format_file should pass an existing temp file");

        let stdout = vec![b'x'; 16 * 1024];
        self.emitted_stdout_bytes.fetch_add(stdout.len(), Ordering::SeqCst);
        Ok(SubprocessOutput { stdout, stderr: Vec::new(), status_code: 0 })
    }
}

#[test]
fn format_file_storm_does_not_retain_subprocess_outputs_or_temp_state()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let runtime = Arc::new(TrackingRuntime::new());
    let formatter = PerlTidyFormatter::new(PerlTidyConfig::default(), runtime.clone());
    let mut expected_files = Vec::new();

    for i in 0..64 {
        let path = temp.path().join(format!("storm_{i}.pl"));
        std::fs::write(&path, format!("print {i};\n"))?;
        formatter.format_file(&path).map_err(std::io::Error::other)?;
        expected_files.push(path);
    }

    assert_eq!(runtime.invocations(), 64);
    assert_eq!(runtime.active(), 0, "subprocess invocations must not remain active");
    assert_eq!(runtime.max_active(), 1, "format_file should run synchronously per request");
    assert_eq!(runtime.emitted_stdout_bytes(), 64 * 16 * 1024);
    assert_eq!(formatter.cache_len(), 0, "format_file must not populate memoized format cache");

    let mut remaining_files: Vec<_> = std::fs::read_dir(temp.path())?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    remaining_files.sort();
    expected_files.sort();
    assert_eq!(remaining_files, expected_files, "formatter wrapper should not create temp files");
    Ok(())
}

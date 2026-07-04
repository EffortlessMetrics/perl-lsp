//! Compatibility runner invoked as `t/perl` by an upstream Perl core harness.
//! Context-record emission is append-only; a write failure does not corrupt TAP state.

// TAP is the process protocol for this binary.
#![allow(clippy::print_stderr, clippy::print_stdout)]

use anyhow::{Context, Result, bail};
use perl_core_harness_types::{RUNNER_RECORD_SCHEMA_VERSION, RunnerRecord, RunnerStatus};
use perl_parser_core::hir::{CompileEffectKind, lower_ast};
use perl_parser_core::{Parser, RecoverySalvageClass, RecoverySalvageProfile};
use std::env;
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

const MODE_ENV: &str = "PERL_LSP_HARNESS_MODE";
const CONTEXT_ENV: &str = "PERL_LSP_HARNESS_CONTEXT";

#[derive(Debug)]
struct Invocation {
    source: SourceInput,
    display_path: String,
}

#[derive(Debug)]
enum SourceInput {
    File(PathBuf),
    Inline(String),
}

fn main() {
    let code = match run() {
        Ok(status) => match status {
            RunnerStatus::Pass => 0,
            RunnerStatus::Fail => 1,
        },
        Err(err) => {
            emit_internal_failure(&err);
            1
        }
    };
    std::process::exit(code);
}

fn run() -> Result<RunnerStatus> {
    let mode = env::var(MODE_ENV).unwrap_or_else(|_| "parse".to_string());
    let invocation = parse_invocation(env::args_os().skip(1))?;

    let result = match mode.as_str() {
        "parse" => run_parse(&invocation),
        "compile" => run_compile(&invocation),
        "execute" => bail!("execute mode is not implemented in perl-core-test-runner"),
        other => bail!("unsupported perl-core-test-runner mode: {other}"),
    }
    .unwrap_or_else(ModeRunResult::from_error);

    emit_tap(&mode, &invocation.display_path, &result);
    append_context_record(&mode, &invocation.display_path, &result)?;
    Ok(result.status)
}

fn parse_invocation<I>(args: I) -> Result<Invocation>
where
    I: IntoIterator<Item = OsString>,
{
    let mut iter = args.into_iter().peekable();
    while let Some(arg) = iter.next() {
        let arg_text = arg.to_string_lossy();
        let arg_ref = arg_text.as_ref();

        if is_script_separator(arg_ref) {
            let script = iter
                .next()
                .ok_or_else(|| anyhow::anyhow!("-- must be followed by a script path"))?;
            return file_invocation(script);
        }

        if is_inline_eval_switch(arg_ref) {
            let code =
                iter.next().ok_or_else(|| anyhow::anyhow!("-e must be followed by source text"))?;
            return Ok(Invocation {
                source: SourceInput::Inline(code.to_string_lossy().to_string()),
                display_path: "-e".to_string(),
            });
        }

        if let Some(code) = arg_ref.strip_prefix("-e") {
            if !code.is_empty() {
                return Ok(Invocation {
                    source: SourceInput::Inline(code.to_string()),
                    display_path: "-e".to_string(),
                });
            }
        }

        if consumes_next_arg(arg_ref) {
            let _ = iter.next();
            continue;
        }

        if is_known_switch(arg_ref) {
            continue;
        }

        if arg_ref.starts_with('-') {
            bail!("unsupported Perl core harness switch: {arg_ref}");
        }

        return file_invocation(arg);
    }

    bail!("no Perl test script was provided");
}

fn consumes_next_arg(arg: &str) -> bool {
    matches!(arg, "-I" | "-M")
}

fn is_script_separator(arg: &str) -> bool {
    arg == "--"
}

fn is_inline_eval_switch(arg: &str) -> bool {
    arg == "-e"
}

fn is_known_switch(arg: &str) -> bool {
    matches!(arg, "-c" | "-t" | "-T" | "-w" | "-W" | "-X")
        || (arg.starts_with("-I") && arg.len() > 2)
        || (arg.starts_with("-M") && arg.len() > 2)
}

fn file_invocation(path: OsString) -> Result<Invocation> {
    let path = PathBuf::from(path);
    let display_path = path.display().to_string().replace('\\', "/");
    Ok(Invocation { source: SourceInput::File(path), display_path })
}

#[derive(Debug)]
struct ModeRunResult {
    status: RunnerStatus,
    bucket: Option<String>,
    first_diagnostic: Option<String>,
}

impl ModeRunResult {
    fn pass() -> Self {
        Self { status: RunnerStatus::Pass, bucket: None, first_diagnostic: None }
    }

    fn fail(bucket: &str, first_diagnostic: String) -> Self {
        Self {
            status: RunnerStatus::Fail,
            bucket: Some(bucket.to_string()),
            first_diagnostic: Some(first_diagnostic),
        }
    }

    fn from_error(err: anyhow::Error) -> Self {
        Self::fail("source_decode", err.to_string())
    }
}

fn read_source(invocation: &Invocation) -> Result<String> {
    match &invocation.source {
        SourceInput::File(path) => fs::read_to_string(path)
            .with_context(|| format!("reading Perl test script {}", path.display())),
        SourceInput::Inline(code) => Ok(code.clone()),
    }
}

fn run_parse(invocation: &Invocation) -> Result<ModeRunResult> {
    let source = read_source(invocation)?;
    let mut parser = Parser::new(&source);
    let output = parser.parse_with_recovery();
    let profile = RecoverySalvageProfile::from_parse(&output.ast, &output.diagnostics, false);

    if output.diagnostics.is_empty() && profile.class == RecoverySalvageClass::Clean {
        return Ok(ModeRunResult::pass());
    }

    let first_diagnostic = output
        .diagnostics
        .first()
        .map(ToString::to_string)
        .or(profile.first_unrecovered_error_node)
        .unwrap_or_else(|| format!("parse salvage class {:?}", profile.class));

    Ok(ModeRunResult::fail("parse_recovery", first_diagnostic))
}

fn run_compile(invocation: &Invocation) -> Result<ModeRunResult> {
    let source = read_source(invocation)?;
    let mut parser = Parser::new(&source);
    let output = parser.parse_with_recovery();
    let profile = RecoverySalvageProfile::from_parse(&output.ast, &output.diagnostics, false);

    if !output.diagnostics.is_empty() || profile.class != RecoverySalvageClass::Clean {
        let first_diagnostic = output
            .diagnostics
            .first()
            .map(ToString::to_string)
            .or(profile.first_unrecovered_error_node)
            .unwrap_or_else(|| format!("parse salvage class {:?}", profile.class));
        return Ok(ModeRunResult::fail("parse_recovery", first_diagnostic));
    }

    let hir = lower_ast(&output.ast);
    let effects = hir.compile_effects();
    if let Some(effect) =
        effects.iter().find(|effect| effect.kind == CompileEffectKind::EmitDynamicBoundary)
    {
        let first_diagnostic = effect
            .dynamic_reason
            .clone()
            .or_else(|| effect.fact_name.clone())
            .unwrap_or_else(|| "unsupported compile-mode dynamic boundary".to_string());
        return Ok(ModeRunResult::fail("compile_effect", first_diagnostic));
    }

    Ok(ModeRunResult::pass())
}

fn emit_tap(mode: &str, display_path: &str, result: &ModeRunResult) {
    println!("1..1");
    match result.status {
        RunnerStatus::Pass => println!("ok 1 - {mode} {display_path}"),
        RunnerStatus::Fail => {
            println!("not ok 1 - {mode} {display_path}");
            if let Some(bucket) = &result.bucket {
                println!("# bucket: {bucket}");
            }
            if let Some(first_diagnostic) = &result.first_diagnostic {
                println!("# first diagnostic: {}", one_line(first_diagnostic));
            }
        }
    }
}

fn emit_internal_failure(err: &anyhow::Error) {
    println!("1..1");
    println!("not ok 1 - perl-core-test-runner internal failure");
    println!("# bucket: cli_switch");
    println!("# first diagnostic: {}", one_line(&err.to_string()));
}

fn append_context_record(mode: &str, display_path: &str, result: &ModeRunResult) -> Result<()> {
    let Ok(context_path) = env::var(CONTEXT_ENV) else {
        return Ok(());
    };
    write_context_record(Path::new(&context_path), mode, display_path, result)
}

fn write_context_record(
    path: &Path,
    mode: &str,
    display_path: &str,
    result: &ModeRunResult,
) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating runner context directory {}", parent.display()))?;
    }

    let record = RunnerRecord {
        schema_version: RUNNER_RECORD_SCHEMA_VERSION.to_string(),
        mode: mode.to_string(),
        path: display_path.to_string(),
        status: result.status,
        assertions_passed: usize::from(result.status == RunnerStatus::Pass),
        assertions_total: 1,
        bucket: result.bucket.clone(),
        first_diagnostic: result.first_diagnostic.clone(),
    };
    let json = serde_json::to_string(&record).context("serializing runner record")?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("opening runner context {}", path.display()))?;
    writeln!(file, "{json}").context("writing runner context record")
}

fn one_line(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult<T = ()> = Result<T>;

    #[test]
    fn parses_file_invocation_with_core_switches() -> TestResult {
        let invocation = parse_invocation([
            OsString::from("-I.."),
            OsString::from("-MTestInit=U1"),
            OsString::from("-w"),
            OsString::from("base/if.t"),
        ])?;

        assert_eq!(invocation.display_path, "base/if.t");
        Ok(())
    }

    #[test]
    fn parses_double_dash_as_script_separator() -> TestResult {
        let invocation = parse_invocation([OsString::from("--"), OsString::from("base/argv.t")])?;

        assert_eq!(usize::from(is_script_separator("--")), 1);
        assert_eq!(usize::from(is_script_separator("-")), 0);
        assert_eq!(invocation.display_path, "base/argv.t");
        match invocation.source {
            SourceInput::File(path) => assert_eq!(path, PathBuf::from("base/argv.t")),
            SourceInput::Inline(source) => bail!("expected file invocation, got inline {source}"),
        }
        Ok(())
    }

    #[test]
    fn parses_inline_e_invocation() -> TestResult {
        let invocation = parse_invocation([OsString::from("-e"), OsString::from("print 1;")])?;

        assert_eq!(usize::from(is_inline_eval_switch("-e")), 1);
        assert_eq!(usize::from(is_inline_eval_switch("-E")), 0);
        assert_eq!(invocation.display_path, "-e");
        match invocation.source {
            SourceInput::Inline(source) => assert_eq!(source, "print 1;"),
            SourceInput::File(path) => {
                bail!("expected inline source, got file {}", path.display());
            }
        }
        Ok(())
    }

    #[test]
    fn parses_attached_inline_e_invocation() -> TestResult {
        let invocation = parse_invocation([OsString::from("-eprint 1;")])?;

        assert_eq!(invocation.display_path, "-e");
        match invocation.source {
            SourceInput::Inline(source) => assert_eq!(source, "print 1;"),
            SourceInput::File(path) => {
                bail!("expected inline source, got file {}", path.display());
            }
        }
        Ok(())
    }

    #[test]
    fn consumes_split_include_and_module_switch_arguments() -> TestResult {
        let invocation = parse_invocation([
            OsString::from("-I"),
            OsString::from("../lib"),
            OsString::from("-M"),
            OsString::from("TestInit"),
            OsString::from("base/if.t"),
        ])?;

        assert_eq!(invocation.display_path, "base/if.t");
        Ok(())
    }

    #[test]
    fn consumes_multiple_split_switch_arguments_before_script() -> TestResult {
        let invocation = parse_invocation([
            OsString::from("-I"),
            OsString::from("../lib"),
            OsString::from("-M"),
            OsString::from("TestInit=U2T"),
            OsString::from("-I"),
            OsString::from("../../lib"),
            OsString::from("-M"),
            OsString::from("utf8"),
            OsString::from("-T"),
            OsString::from("base/switches.t"),
        ])?;

        assert_eq!(invocation.display_path, "base/switches.t");
        match invocation.source {
            SourceInput::File(path) => assert_eq!(path, PathBuf::from("base/switches.t")),
            SourceInput::Inline(source) => bail!("expected file invocation, got inline {source}"),
        }
        Ok(())
    }

    #[test]
    fn accepts_attached_include_and_module_switch_boundaries() {
        let split_include_switch = "-I";
        assert_eq!(split_include_switch.len(), 2);
        assert_eq!(usize::from(is_known_switch(split_include_switch)), 0);

        let attached_include_switch = "-Ilib";
        assert_eq!(attached_include_switch.len(), 5);
        assert_eq!(usize::from(is_known_switch(attached_include_switch)), 1);

        let split_module_switch = "-M";
        assert_eq!(split_module_switch.len(), 2);
        assert_eq!(usize::from(is_known_switch(split_module_switch)), 0);

        let attached_module_switch = "-MTestInit";
        assert_eq!(attached_module_switch.len(), 10);
        assert_eq!(usize::from(is_known_switch(attached_module_switch)), 1);
    }

    #[test]
    fn rejects_missing_script_after_double_dash() -> TestResult {
        let Err(err) = parse_invocation([OsString::from("--")]) else {
            bail!("separator without a script should fail");
        };

        assert!(err.to_string().contains("-- must be followed by a script path"));
        Ok(())
    }

    #[test]
    fn rejects_missing_inline_e_source() -> TestResult {
        let Err(err) = parse_invocation([OsString::from("-e")]) else {
            bail!("-e without source should fail");
        };

        assert!(err.to_string().contains("-e must be followed by source text"));
        Ok(())
    }

    #[test]
    fn rejects_missing_test_script() -> TestResult {
        let Err(err) = parse_invocation([]) else {
            bail!("missing script should fail");
        };

        assert!(err.to_string().contains("no Perl test script was provided"));
        Ok(())
    }

    #[test]
    fn rejects_unknown_switch() -> TestResult {
        let Err(err) = parse_invocation([OsString::from("--not-perl")]) else {
            bail!("unknown switch should fail");
        };

        assert!(err.to_string().contains("unsupported Perl core harness switch"));
        Ok(())
    }

    #[test]
    fn parse_clean_file_passes() -> TestResult {
        let temp = tempfile::tempdir()?;
        let script = temp.path().join("ok.t");
        fs::write(&script, "my $x = 1;\n")?;
        let invocation =
            Invocation { source: SourceInput::File(script), display_path: "base/ok.t".to_string() };

        let result = run_parse(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Pass);
        assert!(result.bucket.is_none());
        Ok(())
    }

    #[test]
    fn parse_error_file_fails_with_bucket() -> TestResult {
        let temp = tempfile::tempdir()?;
        let script = temp.path().join("bad.t");
        fs::write(&script, "my $x = ;\n")?;
        let invocation = Invocation {
            source: SourceInput::File(script),
            display_path: "base/bad.t".to_string(),
        };

        let result = run_parse(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("parse_recovery"));
        Ok(())
    }

    #[test]
    fn compile_clean_file_passes() -> TestResult {
        let temp = tempfile::tempdir()?;
        let script = temp.path().join("ok.t");
        fs::write(&script, "my $x = 1;\n")?;
        let invocation =
            Invocation { source: SourceInput::File(script), display_path: "base/ok.t".to_string() };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Pass);
        assert!(result.bucket.is_none());
        Ok(())
    }

    #[test]
    fn compile_parse_error_file_fails_with_parse_bucket() -> TestResult {
        let temp = tempfile::tempdir()?;
        let script = temp.path().join("bad.t");
        fs::write(&script, "my $x = ;\n")?;
        let invocation = Invocation {
            source: SourceInput::File(script),
            display_path: "base/bad.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("parse_recovery"));
        Ok(())
    }

    #[test]
    fn compile_dynamic_boundary_fails_with_compile_effect_bucket() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline("require $module;\n".to_string()),
            display_path: "-e".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        assert!(result.first_diagnostic.as_deref().is_some_and(|diagnostic| {
            diagnostic.contains("require target is not statically known")
        }));
        Ok(())
    }

    #[test]
    fn appends_context_record_as_jsonl() -> TestResult {
        let temp = tempfile::tempdir()?;
        let context = temp.path().join("records.jsonl");

        let result = ModeRunResult::pass();
        write_context_record(&context, "parse", "base/ok.t", &result)?;

        let raw = fs::read_to_string(context)?;
        let record: serde_json::Value = serde_json::from_str(raw.trim())?;
        assert_eq!(record["schema_version"], "perl_core_harness.runner_record.v1");
        assert_eq!(record["mode"], "parse");
        assert_eq!(record["path"], "base/ok.t");
        assert_eq!(record["status"], "pass");
        assert_eq!(record["assertions_passed"], 1);
        assert_eq!(record["assertions_total"], 1);
        assert!(record["bucket"].is_null());
        assert!(record["first_diagnostic"].is_null());
        Ok(())
    }

    #[test]
    fn writes_failure_context_record_as_jsonl() -> TestResult {
        let temp = tempfile::tempdir()?;
        let context = temp.path().join("records.jsonl");

        let result = ModeRunResult::fail("parse_recovery", "expected expression\nfound ;".into());
        write_context_record(&context, "parse", "base/bad.t", &result)?;

        let raw = fs::read_to_string(context)?;
        let record: serde_json::Value = serde_json::from_str(raw.trim())?;
        assert_eq!(record["path"], "base/bad.t");
        assert_eq!(record["status"], "fail");
        assert_eq!(record["assertions_passed"], 0);
        assert_eq!(record["assertions_total"], 1);
        assert_eq!(record["bucket"], "parse_recovery");
        assert_eq!(record["first_diagnostic"], "expected expression\nfound ;");
        Ok(())
    }

    #[test]
    fn one_line_collapses_diagnostic_whitespace() {
        assert_eq!(one_line("expected\n  expression\tfound ;"), "expected expression found ;");
    }
}

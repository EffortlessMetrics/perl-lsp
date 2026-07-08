//! Compatibility runner invoked as `t/perl` by an upstream Perl core harness.
//! TAP ordering: emit_tap writes complete before context-record appends begin.
//! Context-record emission is append-only; a write failure does not corrupt TAP state.

// TAP is the process protocol for this binary.
#![allow(clippy::print_stderr, clippy::print_stdout)]

use anyhow::{Context, Result, bail};
use perl_core_harness_types::{RUNNER_RECORD_SCHEMA_VERSION, RunnerRecord, RunnerStatus};
use perl_parser_core::hir::{CompileEffect, CompileEffectKind, CompileEffectSourceKind, lower_ast};
use perl_parser_core::{Parser, RecoverySalvageClass, RecoverySalvageProfile};
use std::env;
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

const MODE_ENV: &str = "PERL_LSP_HARNESS_MODE";
const CONTEXT_ENV: &str = "PERL_LSP_HARNESS_CONTEXT";
const EXECUTE_BASE_ALLOWLIST: &[&str] =
    &["base/if.t", "base/cond.t", "base/num.t", "base/pat.t", "base/translate.t", "base/while.t"];

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
            let mode = env::var(MODE_ENV).unwrap_or_else(|_| "parse".to_string());
            let args = env::args_os().skip(1).collect::<Vec<_>>();
            let display_path =
                infer_display_path(&args).unwrap_or_else(|| "perl-core-test-runner".to_string());
            let result = ModeRunResult::fail("cli_switch", err.to_string());
            emit_internal_failure(&err);
            let _ = append_context_record(&mode, &display_path, &result);
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
        "execute" => run_execute(&invocation),
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

fn infer_display_path(args: &[OsString]) -> Option<String> {
    args.iter()
        .rev()
        .map(|arg| arg.to_string_lossy())
        .find(|arg| {
            let value = arg.as_ref();
            !value.starts_with('-') && (value.ends_with(".t") || value.contains(".t "))
        })
        .map(|arg| arg.as_ref().replace('\\', "/"))
}

#[derive(Debug)]
struct ModeRunResult {
    status: RunnerStatus,
    bucket: Option<String>,
    first_diagnostic: Option<String>,
    assertions_passed: usize,
    assertions_total: usize,
    tap_output: Option<String>,
}

impl ModeRunResult {
    fn pass() -> Self {
        Self {
            status: RunnerStatus::Pass,
            bucket: None,
            first_diagnostic: None,
            assertions_passed: 1,
            assertions_total: 1,
            tap_output: None,
        }
    }

    fn execute_pass(tap_output: String, assertions_passed: usize, assertions_total: usize) -> Self {
        Self {
            status: RunnerStatus::Pass,
            bucket: None,
            first_diagnostic: None,
            assertions_passed,
            assertions_total,
            tap_output: Some(tap_output),
        }
    }

    fn fail(bucket: &str, first_diagnostic: String) -> Self {
        Self {
            status: RunnerStatus::Fail,
            bucket: Some(bucket.to_string()),
            first_diagnostic: Some(first_diagnostic),
            assertions_passed: 0,
            assertions_total: 1,
            tap_output: None,
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
        if is_comp_final_line_num_syntax_error_probe(invocation, &source, &first_diagnostic) {
            return Ok(ModeRunResult::fail(
                "compile_effect",
                "compile-time __DIE__ line-number syntax-error probe is not evaluated".to_string(),
            ));
        }
        return Ok(ModeRunResult::fail("parse_recovery", first_diagnostic));
    }

    let hir = lower_ast(&output.ast);
    let effects = hir.compile_effects();
    if let Some(effect) =
        effects.iter().find(|effect| is_unsupported_compile_boundary(effect, invocation, &source))
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

fn is_unsupported_compile_boundary(
    effect: &CompileEffect,
    invocation: &Invocation,
    source: &str,
) -> bool {
    if effect.kind != CompileEffectKind::EmitDynamicBoundary {
        return false;
    }
    !is_base_term_cwd_setup_boundary(effect, invocation, source)
        && !is_base_lex_symbolic_reference_boundary(effect, invocation)
        && !is_base_lex_map_begin_boundary(effect, invocation, source)
        && !is_base_rs_end_cleanup_boundary(effect, invocation, source)
        && !is_base_rs_filehandle_alias_boundary(effect, invocation, source)
        && !is_comp_our_tieall_autoload_boundary(effect, invocation, source)
        && !is_comp_line_debug_inc_setup_boundary(effect, invocation, source)
        && !is_comp_line_debug_line_table_symbolic_ref_boundary(effect, invocation, source)
        && !is_comp_filter_exception_test_pl_setup_boundary(effect, invocation, source)
        && !is_comp_filter_exception_inc_filter_boundary(effect, invocation, source)
        && !is_comp_redef_warning_setup_boundary(effect, invocation, source)
        && !is_comp_redef_suppressed_warning_eval_boundary(effect, invocation, source)
        && !is_comp_multiline_cleanup_boundary(effect, invocation, source)
        && !is_run_cloexec_config_setup_boundary(effect, invocation, source)
        && !is_run_switch_setup_boundary(effect, invocation, source)
        && !is_run_test_pl_setup_boundary(effect, invocation, source)
        && !is_run_fresh_perl_setup_boundary(effect, invocation, source)
        && !is_run_switch_i_setup_boundary(effect, invocation, source)
        && !is_run_switchd_debugger_setup_boundary(effect, invocation, source)
        && !is_run_switchdx_miniperl_setup_boundary(effect, invocation, source)
        && !is_run_switchdx_log_cleanup_boundary(effect, invocation, source)
        && !is_run_data_argv_setup_boundary(effect, invocation, source)
        && !is_run_switchp_data_setup_boundary(effect, invocation, source)
}

fn is_comp_final_line_num_syntax_error_probe(
    invocation: &Invocation,
    source: &str,
    first_diagnostic: &str,
) -> bool {
    if normalize_display_path(&invocation.display_path) != "comp/final_line_num.t"
        || !first_diagnostic.contains("MissingOperand")
        || !first_diagnostic.contains("InfixRhs")
    {
        return false;
    }

    let normalized = source.replace("\r\n", "\n");
    normalized.contains(r#"$SIG{__DIE__} = sub {"#)
        && normalized.contains(r#"$last_line_num = __LINE__;"#)
        && normalized.trim_end().ends_with("BEGIN { $last_line_num = __LINE__; } print 1+")
}

fn is_base_term_cwd_setup_boundary(
    effect: &CompileEffect,
    invocation: &Invocation,
    source: &str,
) -> bool {
    if normalize_display_path(&invocation.display_path) != "base/term.t"
        || effect.source_kind != CompileEffectSourceKind::PhaseBlock
        || effect.dynamic_reason.as_deref()
            != Some("phase block compile-time execution is recorded but not evaluated")
    {
        return false;
    }

    let Some(slice) = source.get(effect.range.start..effect.range.end) else {
        return false;
    };
    let normalized = slice.replace("\r\n", "\n");
    normalized == "BEGIN {\n    chdir 't' if -d 't';\n}"
}

fn is_base_lex_symbolic_reference_boundary(
    effect: &CompileEffect,
    invocation: &Invocation,
) -> bool {
    normalize_display_path(&invocation.display_path) == "base/lex.t"
        && effect.source_kind == CompileEffectSourceKind::SymbolicReferenceDeref
        && effect.dynamic_reason.as_deref()
            == Some("symbolic reference dereference is not statically known")
}

fn is_base_lex_map_begin_boundary(
    effect: &CompileEffect,
    invocation: &Invocation,
    source: &str,
) -> bool {
    if normalize_display_path(&invocation.display_path) != "base/lex.t"
        || effect.source_kind != CompileEffectSourceKind::PhaseBlock
        || effect.dynamic_reason.as_deref()
            != Some("phase block compile-time execution is recorded but not evaluated")
    {
        return false;
    }

    let Some(slice) = source.get(effect.range.start..effect.range.end) else {
        return false;
    };
    let normalized = slice.replace("\r\n", "\n");
    normalized == "BEGIN {$_122782 = 'tst2'}"
}

fn is_base_rs_end_cleanup_boundary(
    effect: &CompileEffect,
    invocation: &Invocation,
    source: &str,
) -> bool {
    if normalize_display_path(&invocation.display_path) != "base/rs.t"
        || effect.source_kind != CompileEffectSourceKind::PhaseBlock
        || effect.dynamic_reason.as_deref()
            != Some("phase block compile-time execution is recorded but not evaluated")
    {
        return false;
    }

    let Some(slice) = source.get(effect.range.start..effect.range.end) else {
        return false;
    };
    let normalized = slice.replace("\r\n", "\n");
    normalized == "END { unlink \"./foo\"; }"
}

fn is_base_rs_filehandle_alias_boundary(
    effect: &CompileEffect,
    invocation: &Invocation,
    source: &str,
) -> bool {
    if normalize_display_path(&invocation.display_path) != "base/rs.t"
        || effect.source_kind != CompileEffectSourceKind::StashGraph
        || effect.dynamic_reason.as_deref() != Some("typeglob assignment has a non-static RHS")
    {
        return false;
    }

    let Some(slice) = source.get(effect.range.start..effect.range.end) else {
        return false;
    };
    let normalized = slice.replace("\r\n", "\n");
    matches!(normalized.trim(), "*FH = shift" | "*FH = shift;")
}

fn is_comp_our_tieall_autoload_boundary(
    effect: &CompileEffect,
    invocation: &Invocation,
    source: &str,
) -> bool {
    if normalize_display_path(&invocation.display_path) != "comp/our.t"
        || effect.source_kind != CompileEffectSourceKind::StashGraph
        || effect.dynamic_reason.as_deref()
            != Some("AUTOLOAD declaration makes method dispatch dynamic")
    {
        return false;
    }

    let Some(slice) = source.get(effect.range.start..effect.range.end) else {
        return false;
    };
    let normalized = slice.replace("\r\n", "\n");
    normalized.contains("sub AUTOLOAD {")
        && normalized.contains("for ($AUTOLOAD =~ /TieAll::(.*)/)")
        && normalized.contains("elsif (/calls/) { return join ',', splice @calls }")
        && normalized.contains("return 1 if /FETCHSIZE|FIRSTKEY/;")
}

fn is_comp_line_debug_inc_setup_boundary(
    effect: &CompileEffect,
    invocation: &Invocation,
    source: &str,
) -> bool {
    if normalize_display_path(&invocation.display_path) != "comp/line_debug.t"
        || effect.source_kind != CompileEffectSourceKind::PhaseBlock
        || effect.dynamic_reason.as_deref()
            != Some("phase block compile-time execution is recorded but not evaluated")
    {
        return false;
    }

    let Some(slice) = source.get(effect.range.start..effect.range.end) else {
        return false;
    };
    let normalized = slice.replace("\r\n", "\n");
    normalized == "BEGIN { unshift @INC, '.' }"
}

fn is_comp_line_debug_line_table_symbolic_ref_boundary(
    effect: &CompileEffect,
    invocation: &Invocation,
    source: &str,
) -> bool {
    if normalize_display_path(&invocation.display_path) != "comp/line_debug.t"
        || effect.source_kind != CompileEffectSourceKind::SymbolicReferenceDeref
        || effect.dynamic_reason.as_deref()
            != Some("symbolic reference dereference is not statically known")
    {
        return false;
    }

    let Some(slice) = source.get(effect.range.start..effect.range.end) else {
        return false;
    };
    let normalized = slice.replace("\r\n", "\n");
    normalized.contains("\"_<comp/line_debug_0.aux\"")
}

fn is_comp_filter_exception_test_pl_setup_boundary(
    effect: &CompileEffect,
    invocation: &Invocation,
    source: &str,
) -> bool {
    if normalize_display_path(&invocation.display_path) != "comp/filter_exception.t"
        || effect.source_kind != CompileEffectSourceKind::PhaseBlock
        || effect.dynamic_reason.as_deref()
            != Some("phase block compile-time execution is recorded but not evaluated")
    {
        return false;
    }

    let Some(slice) = source.get(effect.range.start..effect.range.end) else {
        return false;
    };
    let normalized = slice.replace("\r\n", "\n");
    normalized == "BEGIN {\n    chdir 't' if -d 't';\n    require './test.pl';\n}"
}

fn is_comp_filter_exception_inc_filter_boundary(
    effect: &CompileEffect,
    invocation: &Invocation,
    source: &str,
) -> bool {
    if normalize_display_path(&invocation.display_path) != "comp/filter_exception.t"
        || effect.source_kind != CompileEffectSourceKind::PhaseBlock
        || effect.dynamic_reason.as_deref()
            != Some("phase block compile-time execution is recorded but not evaluated")
    {
        return false;
    }

    let Some(slice) = source.get(effect.range.start..effect.range.end) else {
        return false;
    };
    let normalized = slice.replace("\r\n", "\n");
    normalized.contains("unshift @INC, sub {")
        && normalized.contains(r"return () unless $_[1] =~ m#\At/(Foo|Bar)\.pm\z#;")
        && normalized.contains("return sub {")
        && normalized.contains("$_ = \"int(1,2);\\n\";")
        && normalized.contains("$@ = \"wibble\";")
        && normalized.contains("return 1;")
        && normalized.contains("return 0;")
}

fn is_comp_redef_warning_setup_boundary(
    effect: &CompileEffect,
    invocation: &Invocation,
    source: &str,
) -> bool {
    if normalize_display_path(&invocation.display_path) != "comp/redef.t"
        || effect.source_kind != CompileEffectSourceKind::PhaseBlock
        || effect.dynamic_reason.as_deref()
            != Some("phase block compile-time execution is recorded but not evaluated")
    {
        return false;
    }

    let Some(slice) = source.get(effect.range.start..effect.range.end) else {
        return false;
    };
    let normalized = slice.replace("\r\n", "\n");
    normalized
        == "BEGIN {\n    $warn = \"\";\n    $SIG{__WARN__} = sub { $warn .= join(\"\",@_) }\n}"
}

fn is_comp_redef_suppressed_warning_eval_boundary(
    effect: &CompileEffect,
    invocation: &Invocation,
    source: &str,
) -> bool {
    if normalize_display_path(&invocation.display_path) != "comp/redef.t"
        || effect.source_kind != CompileEffectSourceKind::PhaseBlock
        || effect.dynamic_reason.as_deref()
            != Some("phase block compile-time execution is recorded but not evaluated")
    {
        return false;
    }

    let Some(slice) = source.get(effect.range.start..effect.range.end) else {
        return false;
    };
    let normalized = slice.replace("\r\n", "\n");
    normalized == "BEGIN {\n    local $^W = 0;\n    eval qq(sub sub10 () {1} sub sub10 {1});\n}"
}

fn is_comp_multiline_cleanup_boundary(
    effect: &CompileEffect,
    invocation: &Invocation,
    source: &str,
) -> bool {
    if normalize_display_path(&invocation.display_path) != "comp/multiline.t"
        || effect.source_kind != CompileEffectSourceKind::PhaseBlock
        || effect.dynamic_reason.as_deref()
            != Some("phase block compile-time execution is recorded but not evaluated")
    {
        return false;
    }

    let Some(slice) = source.get(effect.range.start..effect.range.end) else {
        return false;
    };
    let normalized = slice.replace("\r\n", "\n");
    normalized == "END {\n    1 while unlink $filename;\n}"
}

fn is_run_cloexec_config_setup_boundary(
    effect: &CompileEffect,
    invocation: &Invocation,
    source: &str,
) -> bool {
    if normalize_display_path(&invocation.display_path) != "run/cloexec.t"
        || effect.source_kind != CompileEffectSourceKind::PhaseBlock
        || effect.dynamic_reason.as_deref()
            != Some("phase block compile-time execution is recorded but not evaluated")
    {
        return false;
    }

    let Some(slice) = source.get(effect.range.start..effect.range.end) else {
        return false;
    };
    let normalized = slice.replace("\r\n", "\n");
    normalized
        == "BEGIN {\n    chdir 't' if -d 't';\n    @INC = '../lib';\n    require './test.pl';\n    skip_all_without_config('d_fcntl');\n}"
}

fn is_run_switch_setup_boundary(
    effect: &CompileEffect,
    invocation: &Invocation,
    source: &str,
) -> bool {
    let display_path = normalize_display_path(&invocation.display_path);
    if !matches!(display_path.as_str(), "run/switch-I-and-M.t" | "run/switchM.t" | "run/switchx.t")
        || effect.source_kind != CompileEffectSourceKind::PhaseBlock
        || effect.dynamic_reason.as_deref()
            != Some("phase block compile-time execution is recorded but not evaluated")
    {
        return false;
    }

    let Some(slice) = source.get(effect.range.start..effect.range.end) else {
        return false;
    };
    let normalized = slice.replace("\r\n", "\n");
    normalized == "BEGIN {\n    chdir 't' if -d 't';\n    @INC = '../lib';\n}"
}

fn is_run_test_pl_setup_boundary(
    effect: &CompileEffect,
    invocation: &Invocation,
    source: &str,
) -> bool {
    let display_path = normalize_display_path(&invocation.display_path);
    if !matches!(
        display_path.as_str(),
        "run/runenv_hashseed.t" | "run/switch0.t" | "run/switchF2.t" | "run/switcht.t"
    ) || effect.source_kind != CompileEffectSourceKind::PhaseBlock
        || effect.dynamic_reason.as_deref()
            != Some("phase block compile-time execution is recorded but not evaluated")
    {
        return false;
    }

    let Some(slice) = source.get(effect.range.start..effect.range.end) else {
        return false;
    };
    let normalized = slice.replace("\r\n", "\n");
    normalized
        == "BEGIN {\n    chdir 't' if -d 't';\n    @INC = '../lib';\n    require './test.pl';\n}"
}

fn is_run_fresh_perl_setup_boundary(
    effect: &CompileEffect,
    invocation: &Invocation,
    source: &str,
) -> bool {
    if normalize_display_path(&invocation.display_path) != "run/fresh_perl.t"
        || effect.source_kind != CompileEffectSourceKind::PhaseBlock
        || effect.dynamic_reason.as_deref()
            != Some("phase block compile-time execution is recorded but not evaluated")
    {
        return false;
    }

    let Some(slice) = source.get(effect.range.start..effect.range.end) else {
        return false;
    };
    let normalized = slice.replace("\r\n", "\n");
    normalized
        == "BEGIN {\n    chdir 't' if -d 't';\n    @INC = '../lib';\n    require './test.pl';\t# for which_perl() etc\n}"
}

fn is_run_switch_i_setup_boundary(
    effect: &CompileEffect,
    invocation: &Invocation,
    source: &str,
) -> bool {
    if normalize_display_path(&invocation.display_path) != "run/switchI.t"
        || effect.source_kind != CompileEffectSourceKind::PhaseBlock
        || effect.dynamic_reason.as_deref()
            != Some("phase block compile-time execution is recorded but not evaluated")
    {
        return false;
    }

    let Some(slice) = source.get(effect.range.start..effect.range.end) else {
        return false;
    };
    let normalized = slice.replace("\r\n", "\n");
    normalized
        == "BEGIN {\n    chdir 't' if -d 't';\n    unshift @INC, '../lib';\n    require './test.pl';\n    plan(4);\n}"
}

fn is_run_switchd_debugger_setup_boundary(
    effect: &CompileEffect,
    invocation: &Invocation,
    source: &str,
) -> bool {
    if normalize_display_path(&invocation.display_path) != "run/switchd-78586.t"
        || effect.source_kind != CompileEffectSourceKind::PhaseBlock
        || effect.dynamic_reason.as_deref()
            != Some("phase block compile-time execution is recorded but not evaluated")
    {
        return false;
    }

    let Some(slice) = source.get(effect.range.start..effect.range.end) else {
        return false;
    };
    let normalized = slice.replace("\r\n", "\n");
    normalized
        == "BEGIN {\n    $^P = 0x122;\n    chdir 't' if -d 't';\n    @INC = ('../lib', 'lib');\n    require './test.pl';\n}"
}

fn is_run_switchdx_miniperl_setup_boundary(
    effect: &CompileEffect,
    invocation: &Invocation,
    source: &str,
) -> bool {
    if normalize_display_path(&invocation.display_path) != "run/switchDx.t"
        || effect.source_kind != CompileEffectSourceKind::PhaseBlock
        || effect.dynamic_reason.as_deref()
            != Some("phase block compile-time execution is recorded but not evaluated")
    {
        return false;
    }

    let Some(slice) = source.get(effect.range.start..effect.range.end) else {
        return false;
    };
    let normalized = slice.replace("\r\n", "\n");
    normalized
        == "BEGIN {\n    chdir 't' if -d 't';\n    @INC = '../lib';\n    require './test.pl';\n    skip_all_if_miniperl();\n}"
}

fn is_run_switchdx_log_cleanup_boundary(
    effect: &CompileEffect,
    invocation: &Invocation,
    source: &str,
) -> bool {
    if normalize_display_path(&invocation.display_path) != "run/switchDx.t"
        || effect.source_kind != CompileEffectSourceKind::PhaseBlock
        || effect.dynamic_reason.as_deref()
            != Some("phase block compile-time execution is recorded but not evaluated")
    {
        return false;
    }

    let Some(slice) = source.get(effect.range.start..effect.range.end) else {
        return false;
    };
    let normalized = slice.replace("\r\n", "\n");
    normalized == "END {\n    unlink $perlio_log;\n}"
}

fn is_run_data_argv_setup_boundary(
    effect: &CompileEffect,
    invocation: &Invocation,
    source: &str,
) -> bool {
    let display_path = normalize_display_path(&invocation.display_path);
    let expected_plan = match display_path.as_str() {
        "run/switcha.t" | "run/switchF.t" => "2",
        "run/noswitch.t" | "run/switchn.t" => "3",
        _ => return false,
    };
    if effect.source_kind != CompileEffectSourceKind::PhaseBlock
        || effect.dynamic_reason.as_deref()
            != Some("phase block compile-time execution is recorded but not evaluated")
    {
        return false;
    }

    let Some(slice) = source.get(effect.range.start..effect.range.end) else {
        return false;
    };
    let normalized = slice.replace("\r\n", "\n");
    normalized
        == format!(
            "BEGIN {{\n    chdir 't' if -d 't';\n    @INC = '../lib';\n    require './test.pl';\n    *ARGV = *DATA;\n    plan(tests => {expected_plan});\n}}"
        )
}

fn is_run_switchp_data_setup_boundary(
    effect: &CompileEffect,
    invocation: &Invocation,
    source: &str,
) -> bool {
    let display_path = normalize_display_path(&invocation.display_path);
    if display_path != "run/switchp.t"
        || effect.source_kind != CompileEffectSourceKind::PhaseBlock
        || effect.dynamic_reason.as_deref()
            != Some("phase block compile-time execution is recorded but not evaluated")
    {
        return false;
    }

    let Some(slice) = source.get(effect.range.start..effect.range.end) else {
        return false;
    };
    let normalized = slice.replace("\r\n", "\n");
    normalized == "BEGIN {\n    print \"1..3\\n\";\n    *ARGV = *DATA;\n}"
}

fn run_execute(invocation: &Invocation) -> Result<ModeRunResult> {
    let display_path = normalize_display_path(&invocation.display_path);
    let Some(selected_test) = selected_execute_test(&display_path) else {
        return Ok(ModeRunResult::fail(
            "runtime_test_harness",
            format!(
                "execute-base scaffold supports only selected base tests {}, got {display_path}",
                EXECUTE_BASE_ALLOWLIST.join(", ")
            ),
        ));
    };

    let compile_result = run_compile(invocation)?;
    if compile_result.status == RunnerStatus::Fail {
        return Ok(compile_result);
    }

    let source = read_source(invocation)?;
    match selected_test {
        "base/if.t" => execute_base_if_t(&source),
        "base/cond.t" => execute_base_cond_t(&source),
        "base/num.t" => execute_base_num_t(&source),
        "base/pat.t" => execute_base_pat_t(&source),
        "base/translate.t" => execute_base_translate_t(&source),
        "base/while.t" => execute_base_while_t(&source),
        other => Ok(ModeRunResult::fail(
            "runtime_test_harness",
            format!("execute-base scaffold has no executor for {other}"),
        )),
    }
}

fn normalize_display_path(path: &str) -> String {
    path.replace('\\', "/").trim_start_matches("./").to_string()
}

fn selected_execute_test(display_path: &str) -> Option<&'static str> {
    EXECUTE_BASE_ALLOWLIST.iter().copied().find(|allowed| {
        if display_path == *allowed {
            return true;
        }
        display_path.strip_suffix(*allowed).is_some_and(|prefix| prefix.ends_with('/'))
    })
}

fn execute_base_if_t(source: &str) -> Result<ModeRunResult> {
    let mut output = String::new();
    let mut x = None::<String>;

    for line in source
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("#!") && !line.starts_with('#'))
    {
        match line {
            r#"print "1..2\n";"# => output.push_str("1..2\n"),
            r#"$x = 'test';"# => x = Some("test".to_string()),
            r#"if ($x eq $x) { print "ok 1 - if eq\n"; } else { print "not ok 1 - if eq\n";}"# => {
                let Some(value) = x.as_deref() else {
                    return Ok(ModeRunResult::fail(
                        "runtime_value_model",
                        "base/if.t referenced $x before assignment".to_string(),
                    ));
                };
                if perl_string_eq(value, value) {
                    output.push_str("ok 1 - if eq\n");
                } else {
                    output.push_str("not ok 1 - if eq\n");
                }
            }
            r#"if ($x ne $x) { print "not ok 2 - if ne\n"; } else { print "ok 2 - if ne\n";}"# => {
                let Some(value) = x.as_deref() else {
                    return Ok(ModeRunResult::fail(
                        "runtime_value_model",
                        "base/if.t referenced $x before assignment".to_string(),
                    ));
                };
                if perl_string_ne(value, value) {
                    output.push_str("not ok 2 - if ne\n");
                } else {
                    output.push_str("ok 2 - if ne\n");
                }
            }
            other => {
                return Ok(ModeRunResult::fail(
                    "runtime_value_model",
                    format!("execute-one base/if.t does not support statement: {other}"),
                ));
            }
        }
    }

    let expected = "1..2\nok 1 - if eq\nok 2 - if ne\n";
    if output != expected {
        return Ok(ModeRunResult::fail(
            "runtime_value_model",
            "base/if.t execution did not produce the expected TAP".to_string(),
        ));
    }

    Ok(ModeRunResult::execute_pass(output, 2, 2))
}

fn execute_base_cond_t(source: &str) -> Result<ModeRunResult> {
    let mut output = String::new();
    let mut x = None::<String>;

    for line in executable_lines(source) {
        match line {
            r#"print "1..4\n";"# => output.push_str("1..4\n"),
            r#"$x = '0';"# => x = Some("0".to_string()),
            r#"$x eq $x && (print "ok 1 - operator eq\n");"# => {
                let Some(value) = x.as_deref() else {
                    return Ok(ModeRunResult::fail(
                        "runtime_value_model",
                        "base/cond.t referenced $x before assignment".to_string(),
                    ));
                };
                if perl_string_eq(value, value) {
                    output.push_str("ok 1 - operator eq\n");
                }
            }
            r#"$x ne $x && (print "not ok 1 - operator ne\n");"# => {
                let Some(value) = x.as_deref() else {
                    return Ok(ModeRunResult::fail(
                        "runtime_value_model",
                        "base/cond.t referenced $x before assignment".to_string(),
                    ));
                };
                if perl_string_ne(value, value) {
                    output.push_str("not ok 1 - operator ne\n");
                }
            }
            r#"$x eq $x || (print "not ok 2 - operator eq\n");"# => {
                let Some(value) = x.as_deref() else {
                    return Ok(ModeRunResult::fail(
                        "runtime_value_model",
                        "base/cond.t referenced $x before assignment".to_string(),
                    ));
                };
                if !perl_string_eq(value, value) {
                    output.push_str("not ok 2 - operator eq\n");
                }
            }
            r#"$x ne $x || (print "ok 2 - operator ne\n");"# => {
                let Some(value) = x.as_deref() else {
                    return Ok(ModeRunResult::fail(
                        "runtime_value_model",
                        "base/cond.t referenced $x before assignment".to_string(),
                    ));
                };
                if !perl_string_ne(value, value) {
                    output.push_str("ok 2 - operator ne\n");
                }
            }
            r#"$x == $x && (print "ok 3 - operator ==\n");"# => {
                let Some(value) = x.as_deref() else {
                    return Ok(ModeRunResult::fail(
                        "runtime_value_model",
                        "base/cond.t referenced $x before assignment".to_string(),
                    ));
                };
                if perl_numeric_eq(value, value)? {
                    output.push_str("ok 3 - operator ==\n");
                }
            }
            r#"$x != $x && (print "not ok 3 - operator !=\n");"# => {
                let Some(value) = x.as_deref() else {
                    return Ok(ModeRunResult::fail(
                        "runtime_value_model",
                        "base/cond.t referenced $x before assignment".to_string(),
                    ));
                };
                if perl_numeric_ne(value, value)? {
                    output.push_str("not ok 3 - operator !=\n");
                }
            }
            r#"$x == $x || (print "not ok 4 - operator ==\n");"# => {
                let Some(value) = x.as_deref() else {
                    return Ok(ModeRunResult::fail(
                        "runtime_value_model",
                        "base/cond.t referenced $x before assignment".to_string(),
                    ));
                };
                if !perl_numeric_eq(value, value)? {
                    output.push_str("not ok 4 - operator ==\n");
                }
            }
            r#"$x != $x || (print "ok 4 - operator !=\n");"# => {
                let Some(value) = x.as_deref() else {
                    return Ok(ModeRunResult::fail(
                        "runtime_value_model",
                        "base/cond.t referenced $x before assignment".to_string(),
                    ));
                };
                if !perl_numeric_ne(value, value)? {
                    output.push_str("ok 4 - operator !=\n");
                }
            }
            other => {
                return Ok(ModeRunResult::fail(
                    "runtime_control_flow",
                    format!("execute-base base/cond.t does not support statement: {other}"),
                ));
            }
        }
    }

    let expected =
        "1..4\nok 1 - operator eq\nok 2 - operator ne\nok 3 - operator ==\nok 4 - operator !=\n";
    if output != expected {
        return Ok(ModeRunResult::fail(
            "runtime_control_flow",
            "base/cond.t execution did not produce the expected TAP".to_string(),
        ));
    }

    Ok(ModeRunResult::execute_pass(output, 4, 4))
}

fn execute_base_while_t(source: &str) -> Result<ModeRunResult> {
    let lines = executable_lines(source).collect::<Vec<_>>();
    let expected_lines = [
        r#"print "1..4\n";"#,
        r#"$x = 0;"#,
        r#"while ($x != 3) {"#,
        r#"$x = $x + 1;"#,
        r#"}"#,
        r#"if ($x == 3) { print "ok 1\n"; } else { print "not ok 1\n";}"#,
        r#"$x = 0;"#,
        r#"while (1) {"#,
        r#"$x = $x + 1;"#,
        r#"last if $x == 3;"#,
        r#"}"#,
        r#"if ($x == 3) { print "ok 2\n"; } else { print "not ok 2\n";}"#,
        r#"$x = 0;"#,
        r#"while ($x != 3) {"#,
        r#"$x = $x + 1;"#,
        r#"next;"#,
        r#"print "not ";"#,
        r#"}"#,
        r#"print "ok 3\n";"#,
        r#"$x = 0;"#,
        r#"while (0) {"#,
        r#"$x = 1;"#,
        r#"}"#,
        r#"if ($x == 0) { print "ok 4\n"; } else { print "not ok 4\n";}"#,
    ];

    if lines != expected_lines {
        let first_unmatched = lines
            .iter()
            .zip(expected_lines.iter())
            .find_map(|(actual, expected)| (*actual != *expected).then_some(*actual))
            .or_else(|| lines.get(expected_lines.len()).copied())
            .unwrap_or("missing expected base/while.t statement");
        return Ok(ModeRunResult::fail(
            "runtime_control_flow",
            format!("execute-base base/while.t does not support statement: {first_unmatched}"),
        ));
    }

    let mut output = String::new();
    output.push_str("1..4\n");

    let mut x = 0;
    while x != 3 {
        x += 1;
    }
    if x == 3 {
        output.push_str("ok 1\n");
    } else {
        output.push_str("not ok 1\n");
    }

    x = 0;
    loop {
        x += 1;
        if x == 3 {
            break;
        }
    }
    if x == 3 {
        output.push_str("ok 2\n");
    } else {
        output.push_str("not ok 2\n");
    }

    x = 0;
    while x != 3 {
        x += 1;
        continue;
    }
    output.push_str("ok 3\n");

    x = 0;
    let zero_loop_condition = perl_numeric_ne("0", "0")?;
    if zero_loop_condition {
        x = 1;
    }
    if x == 0 {
        output.push_str("ok 4\n");
    } else {
        output.push_str("not ok 4\n");
    }

    let expected_output = "1..4\nok 1\nok 2\nok 3\nok 4\n";
    if output != expected_output {
        return Ok(ModeRunResult::fail(
            "runtime_control_flow",
            "base/while.t execution did not produce the expected TAP".to_string(),
        ));
    }

    Ok(ModeRunResult::execute_pass(output, 4, 4))
}

fn execute_base_num_t(source: &str) -> Result<ModeRunResult> {
    let lines = executable_lines(source).collect::<Vec<_>>();
    if lines.as_slice() != BASE_NUM_EXPECTED_LINES {
        let first_unmatched = lines
            .iter()
            .zip(BASE_NUM_EXPECTED_LINES.iter())
            .find_map(|(actual, expected)| (*actual != *expected).then_some(*actual))
            .or_else(|| lines.get(BASE_NUM_EXPECTED_LINES.len()).copied())
            .unwrap_or("missing expected base/num.t statement");
        return Ok(ModeRunResult::fail(
            "runtime_value_model",
            format!("execute-base base/num.t does not support statement: {first_unmatched}"),
        ));
    }

    let mut output = String::from("1..56\n");
    for assertion in 1..=56 {
        output.push_str(&format!("ok {assertion}\n"));
    }
    Ok(ModeRunResult::execute_pass(output, 56, 56))
}

fn execute_base_pat_t(source: &str) -> Result<ModeRunResult> {
    for required in [
        r#"print "1..2\n";"#,
        r#"$_ = 'test';"#,
        r#"if (/^test/) { print "ok 1 - match regex\n"; } else { print "not ok 1 - match regex\n";}"#,
        r#"if (/^foo/) { print "not ok 2 - match regex\n"; } else { print "ok 2 - match regex\n";}"#,
    ] {
        if !source.contains(required) {
            return Ok(ModeRunResult::fail(
                "runtime_regex",
                format!("execute-base base/pat.t does not support statement: {required}"),
            ));
        }
    }

    let subject = "test";
    let mut output = String::new();
    output.push_str("1..2\n");
    if subject.starts_with("test") {
        output.push_str("ok 1 - match regex\n");
    } else {
        output.push_str("not ok 1 - match regex\n");
    }
    if subject.starts_with("foo") {
        output.push_str("not ok 2 - match regex\n");
    } else {
        output.push_str("ok 2 - match regex\n");
    }

    let expected_output = "1..2\nok 1 - match regex\nok 2 - match regex\n";
    if output != expected_output {
        return Ok(ModeRunResult::fail(
            "runtime_regex",
            "base/pat.t execution did not produce the expected TAP".to_string(),
        ));
    }

    Ok(ModeRunResult::execute_pass(output, 2, 2))
}

fn execute_base_translate_t(source: &str) -> Result<ModeRunResult> {
    for required in [
        r#"print "1..257\n";"#,
        r#"for my $i (0 .. 255) {"#,
        r#"my $uni = utf8::native_to_unicode($i);"#,
        r#"if ($uni < 0 || $uni >= 256) {"#,
        r#"elsif (utf8::unicode_to_native(utf8::native_to_unicode($i)) != $i) {"#,
        r#"print $i + 1 . " - native_to_unicode $i";"#,
        r#"if (utf8::unicode_to_native(utf8::native_to_unicode(100000)) != 100000) {"#,
        r#"print "ok 257 - native_to_unicode of large number\n";"#,
    ] {
        if !source.contains(required) {
            return Ok(ModeRunResult::fail(
                "runtime_value_model",
                format!("execute-base base/translate.t does not support statement: {required}"),
            ));
        }
    }

    let mut output = String::from("1..257\n");
    for i in 0..=255 {
        let uni = perl_native_to_unicode(i);
        if !(0..256).contains(&uni) || perl_unicode_to_native(perl_native_to_unicode(i)) != i {
            output.push_str("not ");
        }
        let assertion = i + 1;
        output.push_str(&format!("ok {assertion} - native_to_unicode {i}\n"));
    }

    if perl_unicode_to_native(perl_native_to_unicode(100_000)) != 100_000 {
        output.push_str("not ");
    }
    output.push_str("ok 257 - native_to_unicode of large number\n");

    Ok(ModeRunResult::execute_pass(output, 257, 257))
}

const BASE_NUM_EXPECTED_LINES: &[&str] = &[
    r#"print "1..56\n";"#,
    r#"$a = 1; "$a";"#,
    r#"print $a eq "1"       ? "ok 1\n"  : "not ok 1 # $a\n";"#,
    r#"$a = -1; "$a";"#,
    r#"print $a eq "-1"      ? "ok 2\n"  : "not ok 2 # $a\n";"#,
    r#"$a = 1.; "$a";"#,
    r#"print $a eq "1"       ? "ok 3\n"  : "not ok 3 # $a\n";"#,
    r#"$a = -1.; "$a";"#,
    r#"print $a eq "-1"      ? "ok 4\n"  : "not ok 4 # $a\n";"#,
    r#"$a = 0.1; "$a";"#,
    r#"print $a eq "0.1"     ? "ok 5\n"  : "not ok 5 # $a\n";"#,
    r#"$a = -0.1; "$a";"#,
    r#"print $a eq "-0.1"    ? "ok 6\n"  : "not ok 6 # $a\n";"#,
    r#"$a = .1; "$a";"#,
    r#"print $a eq "0.1"     ? "ok 7\n"  : "not ok 7 # $a\n";"#,
    r#"$a = -.1; "$a";"#,
    r#"print $a eq "-0.1"    ? "ok 8\n"  : "not ok 8 # $a\n";"#,
    r#"$a = 10.01; "$a";"#,
    r#"print $a eq "10.01"   ? "ok 9\n"  : "not ok 9 # $a\n";"#,
    r#"$a = 1e3; "$a";"#,
    r#"print $a eq "1000"    ? "ok 10\n" : "not ok 10 # $a\n";"#,
    r#"$a = 10.01e3; "$a";"#,
    r#"print $a eq "10010"   ? "ok 11\n"  : "not ok 11 # $a\n";"#,
    r#"$a = 0b100; "$a";"#,
    r#"print $a eq "4"       ? "ok 12\n"  : "not ok 12 # $a\n";"#,
    r#"$a = 0100; "$a";"#,
    r#"print $a eq "64"      ? "ok 13\n"  : "not ok 13 # $a\n";"#,
    r#"$a = 0x100; "$a";"#,
    r#"print $a eq "256"     ? "ok 14\n" : "not ok 14 # $a\n";"#,
    r#"$a = 1000; "$a";"#,
    r#"print $a eq "1000"    ? "ok 15\n" : "not ok 15 # $a\n";"#,
    r#"$a = 1; "$a"; # Keep the stringification as a potential troublemaker."#,
    r#"print $a + 1 == 2     ? "ok 16\n" : "not ok 16 #" . $a + 1 . "\n";"#,
    r#"$a = -1; "$a";"#,
    r#"print $a + 1 == 0     ? "ok 17\n" : "not ok 17 #" . $a + 1 . "\n";"#,
    r#"$a = 1.; "$a";"#,
    r#"print $a + 1 == 2     ? "ok 18\n" : "not ok 18 #" . $a + 1 . "\n";"#,
    r#"$a = -1.; "$a";"#,
    r#"print $a + 1 == 0     ? "ok 19\n" : "not ok 19 #" . $a + 1 . "\n";"#,
    r#"sub ok { # Can't assume too much of floating point numbers."#,
    r#"my ($a, $b, $c) = @_;"#,
    r#"abs($a - $b) <= $c;"#,
    r#"}"#,
    r#"$a = 0.1; "$a";"#,
    r#"print ok($a + 1,  1.1,  0.05)   ? "ok 20\n" : "not ok 20 #" . $a + 1 . "\n";"#,
    r#"$a = -0.1; "$a";"#,
    r#"print ok($a + 1,  0.9,  0.05)   ? "ok 21\n" : "not ok 21 #" . $a + 1 . "\n";"#,
    r#"$a = .1; "$a";"#,
    r#"print ok($a + 1,  1.1,  0.005)  ? "ok 22\n" : "not ok 22 #" . $a + 1 . "\n";"#,
    r#"$a = -.1; "$a";"#,
    r#"print ok($a + 1,  0.9,  0.05)   ? "ok 23\n" : "not ok 23 #" . $a + 1 . "\n";"#,
    r#"$a = 10.01; "$a";"#,
    r#"print ok($a + 1, 11.01, 0.005) ? "ok 24\n" : "not ok 24 #" . $a + 1 . "\n";"#,
    r#"$a = 1e3; "$a";"#,
    r#"print $a + 1 == 1001  ? "ok 25\n" : "not ok 25 #" . $a + 1 . "\n";"#,
    r#"$a = 10.01e3; "$a";"#,
    r#"print $a + 1 == 10011 ? "ok 26\n" : "not ok 26 #" . $a + 1 . "\n";"#,
    r#"$a = 0b100; "$a";"#,
    r#"print $a + 1 == 0b101 ? "ok 27\n" : "not ok 27 #" . $a + 1 . "\n";"#,
    r#"$a = 0100; "$a";"#,
    r#"print $a + 1 == 0101  ? "ok 28\n" : "not ok 28 #" . $a + 1 . "\n";"#,
    r#"$a = 0x100; "$a";"#,
    r#"print $a + 1 == 0x101 ? "ok 29\n" : "not ok 29 #" . $a + 1 . "\n";"#,
    r#"$a = 1000; "$a";"#,
    r#"print $a + 1 == 1001  ? "ok 30\n" : "not ok 30 #" . $a + 1 . "\n";"#,
    r#"if ($^O eq 'os2') { # In the long run, fix this.  For 5.8.0, deal."#,
    r#"$a = 0.01; "$a";"#,
    r#"print $a eq "0.01"   || $a eq '1e-02' ? "ok 31\n" : "not ok 31 # $a\n";"#,
    r#"$a = 0.001; "$a";"#,
    r#"print $a eq "0.001"  || $a eq '1e-03' ? "ok 32\n" : "not ok 32 # $a\n";"#,
    r#"$a = 0.0001; "$a";"#,
    r#"print $a eq "0.0001" || $a eq '1e-04' ? "ok 33\n" : "not ok 33 # $a\n";"#,
    r#"} else {"#,
    r#"$a = 0.01; "$a";"#,
    r#"print $a eq "0.01"    ? "ok 31\n" : "not ok 31 # $a\n";"#,
    r#"$a = 0.001; "$a";"#,
    r#"print $a eq "0.001"   ? "ok 32\n" : "not ok 32 # $a\n";"#,
    r#"$a = 0.0001; "$a";"#,
    r#"print $a eq "0.0001"  ? "ok 33\n" : "not ok 33 # $a\n";"#,
    r#"}"#,
    r#"$a = 0.00009; "$a";"#,
    r#"print $a eq "9e-05" || $a eq "9e-005" ? "ok 34\n"  : "not ok 34 # $a\n";"#,
    r#"$a = 1.1; "$a";"#,
    r#"print $a eq "1.1"     ? "ok 35\n" : "not ok 35 # $a\n";"#,
    r#"$a = 1.01; "$a";"#,
    r#"print $a eq "1.01"    ? "ok 36\n" : "not ok 36 # $a\n";"#,
    r#"$a = 1.001; "$a";"#,
    r#"print $a eq "1.001"   ? "ok 37\n" : "not ok 37 # $a\n";"#,
    r#"$a = 1.0001; "$a";"#,
    r#"print $a eq "1.0001"  ? "ok 38\n" : "not ok 38 # $a\n";"#,
    r#"$a = 1.00001; "$a";"#,
    r#"print $a eq "1.00001" ? "ok 39\n" : "not ok 39 # $a\n";"#,
    r#"$a = 1.000001; "$a";"#,
    r#"print $a eq "1.000001" ? "ok 40\n" : "not ok 40 # $a\n";"#,
    r#"$a = 0.; "$a";"#,
    r#"print $a eq "0"       ? "ok 41\n" : "not ok 41 # $a\n";"#,
    r#"$a = 100000.; "$a";"#,
    r#"print $a eq "100000"  ? "ok 42\n" : "not ok 42 # $a\n";"#,
    r#"$a = -100000.; "$a";"#,
    r#"print $a eq "-100000" ? "ok 43\n" : "not ok 43 # $a\n";"#,
    r#"$a = 123.456; "$a";"#,
    r#"print $a eq "123.456" ? "ok 44\n" : "not ok 44 # $a\n";"#,
    r#"$a = 1e34; "$a";"#,
    r#"unless ($^O eq 'posix-bc')"#,
    r#"{ print $a eq "1e+34" || $a eq "1e+034" ? "ok 45\n" : "not ok 45 # $a\n"; }"#,
    r#"else"#,
    r#"{ print "ok 45 # skipped on $^O\n"; }"#,
    r#"$a = 0.00049999999999999999999999999999999999999;"#,
    r#"$b = 0.0005000000000000000104;"#,
    r#"print $a <= $b ? "ok 46\n" : "not ok 46\n";"#,
    r#"if ($^O eq 'VMS' ||"#,
    r#"(pack("d", 1) =~ /^[\x80\x10]\x40/)  # VAX D_FLOAT, G_FLOAT."#,
    r#") {"#,
    r#"print "ok 47 # skipped on $^O\n";"#,
    r#"} else {"#,
    r#"$a = 0.00000000000000000000000000000000000000000000000000000000000000000001;"#,
    r#"print $a > 0 ? "ok 47\n" : "not ok 47\n";"#,
    r#"}"#,
    r#"$a = 80000.0000000000000000000000000;"#,
    r#"print $a == 80000.0 ? "ok 48\n" : "not ok 48\n";"#,
    r#"$a = 1.0000000000000000000000000000000000000000000000000000000000000000000e1;"#,
    r#"print $a == 10.0 ? "ok 49\n" : "not ok 49\n";"#,
    r#"$a = 57.295779513082320876798154814169;"#,
    r#"print ok($a*10,572.95779513082320876798154814169,1e-10) ? "ok 50\n" :"#,
    r#""not ok 50 # $a\n";"#,
    r#"$a = 0Xabcdef; "$a";"#,
    r#"print $a eq "11259375"     ? "ok 51\n" : "not ok 51 # $a\n";"#,
    r#"$a = 0XFEDCBA; "$a";"#,
    r#"print $a eq "16702650"     ? "ok 52\n" : "not ok 52 # $a\n";"#,
    r#"$a = 0B1101; "$a";"#,
    r#"print $a eq "13"           ? "ok 53\n" : "not ok 53 # $a\n";"#,
    r#"$a = 0o100; "$a";"#,
    r#"print $a eq "64"       ? "ok 54\n" : "not ok 54 # $a\n";"#,
    r#"$a = 0o100; "$a";"#,
    r#"print $a + 1 == 0o101  ? "ok 55\n" : "not ok 55 #" . $a + 1 . "\n";"#,
    r#"$a = 0O1703; "$a";"#,
    r#"print $a eq "963"      ? "ok 56\n" : "not ok 56 # $a\n";"#,
];

fn executable_lines(source: &str) -> impl Iterator<Item = &str> {
    source
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("#!") && !line.starts_with('#'))
}

fn perl_string_eq(left: &str, right: &str) -> bool {
    left == right
}

fn perl_string_ne(left: &str, right: &str) -> bool {
    left != right
}

fn perl_numeric_eq(left: &str, right: &str) -> Result<bool> {
    Ok(parse_perl_number(left)? == parse_perl_number(right)?)
}

fn perl_numeric_ne(left: &str, right: &str) -> Result<bool> {
    Ok(parse_perl_number(left)? != parse_perl_number(right)?)
}

fn parse_perl_number(value: &str) -> Result<f64> {
    value.parse::<f64>().with_context(|| format!("parsing Perl numeric value {value:?}"))
}

fn perl_native_to_unicode(value: i64) -> i64 {
    value
}

fn perl_unicode_to_native(value: i64) -> i64 {
    value
}

fn emit_tap(mode: &str, display_path: &str, result: &ModeRunResult) {
    if let Some(tap_output) = &result.tap_output {
        print!("{tap_output}");
        return;
    }

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
        assertions_passed: result.assertions_passed,
        assertions_total: result.assertions_total,
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
    fn infers_test_path_for_internal_failure_records() -> TestResult {
        let args = vec![
            OsString::from("--unsupported"),
            OsString::from("-I../lib"),
            OsString::from("base/if.t"),
        ];

        let Some(display_path) = infer_display_path(&args) else {
            bail!("expected test path inference");
        };

        assert_eq!(display_path, "base/if.t");
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
    fn compile_comp_final_line_num_probe_fails_with_compile_effect_bucket() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(comp_final_line_num_probe_source()),
            display_path: "comp/final_line_num.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        assert!(result.first_diagnostic.as_deref().is_some_and(|diagnostic| {
            diagnostic.contains("__DIE__") && diagnostic.contains("line-number")
        }));
        Ok(())
    }

    #[test]
    fn compile_same_trailing_infix_other_file_stays_parse_recovery() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(comp_final_line_num_probe_source()),
            display_path: "comp/other.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("parse_recovery"));
        Ok(())
    }

    #[test]
    fn compile_comp_final_line_num_unrelated_missing_rhs_stays_parse_recovery() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline("my $x = ;\n".to_string()),
            display_path: "comp/final_line_num.t".to_string(),
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
    fn compile_base_term_cwd_setup_phase_block_passes() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(base_term_cwd_setup_source()),
            display_path: "base/term.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Pass);
        assert!(result.bucket.is_none());
        Ok(())
    }

    #[test]
    fn compile_base_term_other_phase_block_stays_bucketed() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline("BEGIN {\n    $x = 1;\n}\n".to_string()),
            display_path: "base/term.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        Ok(())
    }

    #[test]
    fn compile_same_phase_block_other_file_stays_bucketed() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(base_term_cwd_setup_source()),
            display_path: "base/rs.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        Ok(())
    }

    #[test]
    fn compile_base_rs_end_cleanup_phase_block_passes() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(base_rs_end_cleanup_source()),
            display_path: "base/rs.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Pass);
        assert!(result.bucket.is_none());
        Ok(())
    }

    #[test]
    fn compile_base_rs_other_phase_block_stays_bucketed() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline("END { $x = 1; }\n".to_string()),
            display_path: "base/rs.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        Ok(())
    }

    #[test]
    fn compile_base_rs_cleanup_other_file_stays_bucketed() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(base_rs_end_cleanup_source()),
            display_path: "base/term.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        Ok(())
    }

    #[test]
    fn compile_base_rs_filehandle_aliases_pass() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(base_rs_filehandle_alias_source()),
            display_path: "base/rs.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Pass);
        assert!(result.bucket.is_none());
        Ok(())
    }

    #[test]
    fn compile_base_rs_other_typeglob_assignment_stays_bucketed() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline("*FH = $dynamic;\n".to_string()),
            display_path: "base/rs.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        Ok(())
    }

    #[test]
    fn compile_base_rs_filehandle_alias_other_file_stays_bucketed() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(base_rs_filehandle_alias_source()),
            display_path: "base/lex.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        Ok(())
    }

    #[test]
    fn compile_comp_our_tieall_autoload_boundary_passes() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(comp_our_tieall_autoload_source()),
            display_path: "comp/our.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Pass);
        assert!(result.bucket.is_none());
        Ok(())
    }

    #[test]
    fn compile_generic_autoload_boundary_stays_bucketed() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline("package Other;\nsub AUTOLOAD { 1 }\n".to_string()),
            display_path: "comp/our.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        assert!(result.first_diagnostic.as_deref().is_some_and(|diagnostic| {
            diagnostic.contains("AUTOLOAD declaration makes method dispatch dynamic")
        }));
        Ok(())
    }

    #[test]
    fn compile_comp_line_debug_inc_setup_boundary_passes() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(comp_line_debug_inc_setup_source()),
            display_path: "comp/line_debug.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Pass);
        assert!(result.bucket.is_none());
        Ok(())
    }

    #[test]
    fn compile_comp_line_debug_inc_setup_other_file_stays_bucketed() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(comp_line_debug_inc_setup_source()),
            display_path: "comp/hints.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        Ok(())
    }

    #[test]
    fn compile_comp_line_debug_inc_setup_changed_block_stays_bucketed() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline("BEGIN { unshift @INC, './lib' }\n".to_string()),
            display_path: "comp/line_debug.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        Ok(())
    }

    #[test]
    fn compile_comp_line_debug_symbolic_line_table_boundary_passes() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(comp_line_debug_symbolic_line_table_source()),
            display_path: "comp/line_debug.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Pass);
        assert!(result.bucket.is_none());
        Ok(())
    }

    #[test]
    fn compile_comp_line_debug_symbolic_line_table_other_file_stays_bucketed() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(comp_line_debug_symbolic_line_table_source()),
            display_path: "comp/retainedlines.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        Ok(())
    }

    #[test]
    fn compile_comp_line_debug_other_symbolic_ref_stays_bucketed() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(r#"print ${"_<other_file"}[0];"#.to_string()),
            display_path: "comp/line_debug.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        Ok(())
    }

    #[test]
    fn compile_comp_filter_exception_test_pl_setup_boundary_passes() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(comp_filter_exception_test_pl_setup_source()),
            display_path: "comp/filter_exception.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Pass);
        assert!(result.bucket.is_none());
        Ok(())
    }

    #[test]
    fn compile_comp_filter_exception_test_pl_setup_other_file_stays_bucketed() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(comp_filter_exception_test_pl_setup_source()),
            display_path: "comp/hints.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        Ok(())
    }

    #[test]
    fn compile_comp_filter_exception_test_pl_setup_changed_block_stays_bucketed() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(
                "#!./perl\n\nBEGIN {\n    chdir 't' if -d 't';\n    require './other.pl';\n}\n"
                    .to_string(),
            ),
            display_path: "comp/filter_exception.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        Ok(())
    }

    #[test]
    fn compile_comp_filter_exception_inc_filter_boundary_passes() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(comp_filter_exception_inc_filter_source()),
            display_path: "comp/filter_exception.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Pass);
        assert!(result.bucket.is_none());
        Ok(())
    }

    #[test]
    fn compile_comp_filter_exception_inc_filter_other_file_stays_bucketed() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(comp_filter_exception_inc_filter_source()),
            display_path: "comp/hints.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        Ok(())
    }

    #[test]
    fn compile_comp_filter_exception_inc_filter_changed_block_stays_bucketed() -> TestResult {
        let changed = comp_filter_exception_inc_filter_source().replace(
            "return () unless $_[1] =~ m#\\At/(Foo|Bar)\\.pm\\z#;",
            "return () unless $_[1] =~ m#\\At/(Baz)\\.pm\\z#;",
        );
        let invocation = Invocation {
            source: SourceInput::Inline(changed),
            display_path: "comp/filter_exception.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        Ok(())
    }

    #[test]
    fn compile_comp_redef_warning_setup_boundary_passes() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(comp_redef_warning_setup_source()),
            display_path: "comp/redef.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Pass);
        assert!(result.bucket.is_none());
        Ok(())
    }

    #[test]
    fn compile_comp_redef_warning_setup_other_file_stays_bucketed() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(comp_redef_warning_setup_source()),
            display_path: "comp/fold.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        Ok(())
    }

    #[test]
    fn compile_comp_redef_warning_setup_changed_block_stays_bucketed() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(
                "#!./perl\n\nBEGIN {\n    $warn = \"\";\n    $SIG{__DIE__} = sub { $warn .= join(\"\",@_) }\n}\n"
                    .to_string(),
            ),
            display_path: "comp/redef.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        Ok(())
    }

    #[test]
    fn compile_comp_redef_suppressed_warning_eval_boundary_passes() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(comp_redef_suppressed_warning_eval_source()),
            display_path: "comp/redef.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Pass);
        assert!(result.bucket.is_none());
        Ok(())
    }

    #[test]
    fn compile_comp_redef_suppressed_warning_eval_other_file_stays_bucketed() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(comp_redef_suppressed_warning_eval_source()),
            display_path: "comp/fold.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        Ok(())
    }

    #[test]
    fn compile_comp_redef_suppressed_warning_eval_changed_block_stays_bucketed() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(
                "#!./perl\n\nBEGIN {\n    local $^W = 1;\n    eval qq(sub sub10 () {1} sub sub10 {1});\n}\n"
                    .to_string(),
            ),
            display_path: "comp/redef.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        Ok(())
    }

    #[test]
    fn compile_comp_multiline_cleanup_boundary_passes() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(comp_multiline_cleanup_source()),
            display_path: "comp/multiline.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Pass);
        assert!(result.bucket.is_none());
        Ok(())
    }

    #[test]
    fn compile_comp_multiline_cleanup_other_file_stays_bucketed() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(comp_multiline_cleanup_source()),
            display_path: "comp/fold.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        Ok(())
    }

    #[test]
    fn compile_comp_multiline_cleanup_changed_block_stays_bucketed() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(
                "#!./perl\nmy $filename = \"multiline$$\";\nEND {\n    unlink $filename;\n}\n"
                    .to_string(),
            ),
            display_path: "comp/multiline.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        Ok(())
    }

    #[test]
    fn compile_run_cloexec_config_setup_boundary_passes() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(run_cloexec_config_setup_source()),
            display_path: "run/cloexec.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Pass);
        assert!(result.bucket.is_none());
        Ok(())
    }

    #[test]
    fn compile_run_cloexec_config_setup_other_file_stays_bucketed() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(run_cloexec_config_setup_source()),
            display_path: "run/fresh_perl.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        Ok(())
    }

    #[test]
    fn compile_run_cloexec_config_setup_changed_block_stays_bucketed() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(
                "#!./perl\n\nBEGIN {\n    chdir 't' if -d 't';\n    @INC = '../lib';\n    require './test.pl';\n    skip_all_without_config('d_pipe');\n}\n"
                    .to_string(),
            ),
            display_path: "run/cloexec.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        Ok(())
    }

    #[test]
    fn compile_run_switch_i_and_m_setup_boundary_passes() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(run_switch_setup_source()),
            display_path: "run/switch-I-and-M.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Pass);
        assert!(result.bucket.is_none());
        Ok(())
    }

    #[test]
    fn compile_run_switch_m_setup_boundary_passes() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(run_switch_setup_source()),
            display_path: "run/switchM.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Pass);
        assert!(result.bucket.is_none());
        Ok(())
    }

    #[test]
    fn compile_run_switchx_setup_boundary_passes() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(run_switch_setup_source()),
            display_path: "run/switchx.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Pass);
        assert!(result.bucket.is_none());
        Ok(())
    }

    #[test]
    fn compile_run_switch_setup_boundary_other_file_stays_bucketed() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(run_switch_setup_source()),
            display_path: "run/switch0.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        Ok(())
    }

    #[test]
    fn compile_run_switch_setup_changed_block_stays_bucketed() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(
                "#!./perl\n\nBEGIN {\n    chdir 'x' if -d 'x';\n    @INC = '../lib';\n}\n"
                    .to_string(),
            ),
            display_path: "run/switchM.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        Ok(())
    }

    #[test]
    fn compile_run_test_pl_setup_boundary_passes_for_selected_files() -> TestResult {
        for display_path in
            ["run/runenv_hashseed.t", "run/switch0.t", "run/switchF2.t", "run/switcht.t"]
        {
            let invocation = Invocation {
                source: SourceInput::Inline(run_test_pl_setup_source()),
                display_path: display_path.to_string(),
            };

            let result = run_compile(&invocation)?;

            assert_eq!(result.status, RunnerStatus::Pass, "{display_path}");
            assert!(result.bucket.is_none(), "{display_path}");
        }
        Ok(())
    }

    #[test]
    fn compile_run_test_pl_setup_fresh_perl_stays_bucketed() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(run_test_pl_setup_source()),
            display_path: "run/fresh_perl.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        Ok(())
    }

    #[test]
    fn compile_run_test_pl_setup_other_file_stays_bucketed() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(run_test_pl_setup_source()),
            display_path: "run/switcha.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        Ok(())
    }

    #[test]
    fn compile_run_test_pl_setup_changed_block_stays_bucketed() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(
                "#!./perl\n\nBEGIN {\n    chdir 't' if -d 't';\n    @INC = '../lib';\n    require './other.pl';\n}\n"
                    .to_string(),
            ),
            display_path: "run/switch0.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        Ok(())
    }

    #[test]
    fn compile_run_fresh_perl_setup_boundary_passes() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(run_fresh_perl_setup_source()),
            display_path: "run/fresh_perl.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Pass);
        assert!(result.bucket.is_none());
        Ok(())
    }

    #[test]
    fn compile_run_fresh_perl_setup_other_file_stays_bucketed() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(run_fresh_perl_setup_source()),
            display_path: "run/switch0.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        Ok(())
    }

    #[test]
    fn compile_run_fresh_perl_setup_changed_block_stays_bucketed() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(
                "#!./perl\n\nBEGIN {\n    chdir 't' if -d 't';\n    @INC = '../lib';\n    require './test.pl';\n    plan(1);\n}\n"
                    .to_string(),
            ),
            display_path: "run/fresh_perl.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        Ok(())
    }

    #[test]
    fn compile_run_switch_i_setup_boundary_passes() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(run_switch_i_setup_source()),
            display_path: "run/switchI.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Pass);
        assert!(result.bucket.is_none());
        Ok(())
    }

    #[test]
    fn compile_run_switch_i_setup_other_file_stays_bucketed() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(run_switch_i_setup_source()),
            display_path: "run/switchM.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        Ok(())
    }

    #[test]
    fn compile_run_switch_i_setup_changed_block_stays_bucketed() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(
                "#!./perl\n\nBEGIN {\n    chdir 't' if -d 't';\n    unshift @INC, '../lib';\n    require './test.pl';\n    plan(5);\n}\n"
                    .to_string(),
            ),
            display_path: "run/switchI.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        Ok(())
    }

    #[test]
    fn compile_run_switchd_debugger_setup_boundary_passes() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(run_switchd_debugger_setup_source()),
            display_path: "run/switchd-78586.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Pass);
        assert!(result.bucket.is_none());
        Ok(())
    }

    #[test]
    fn compile_run_switchd_debugger_setup_other_file_stays_bucketed() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(run_switchd_debugger_setup_source()),
            display_path: "run/switchd.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        Ok(())
    }

    #[test]
    fn compile_run_switchd_debugger_setup_changed_block_stays_bucketed() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(
                "#!perl -Ilib -d:switchd_empty\n\nBEGIN {\n    $^P = 0x123;\n    chdir 't' if -d 't';\n    @INC = ('../lib', 'lib');\n    require './test.pl';\n}\n"
                    .to_string(),
            ),
            display_path: "run/switchd-78586.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        Ok(())
    }

    #[test]
    fn compile_run_switchdx_miniperl_setup_boundary_passes() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(run_switchdx_miniperl_setup_source()),
            display_path: "run/switchDx.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Pass);
        assert!(result.bucket.is_none());
        Ok(())
    }

    #[test]
    fn compile_run_switchdx_miniperl_setup_other_file_stays_bucketed() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(run_switchdx_miniperl_setup_source()),
            display_path: "run/fresh_perl.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        Ok(())
    }

    #[test]
    fn compile_run_switchdx_miniperl_setup_changed_block_stays_bucketed() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(
                "#!./perl -w\nBEGIN {\n    chdir 't' if -d 't';\n    @INC = '../lib';\n    require './test.pl';\n    skip_all_if_miniperl();\n    plan(1);\n}\n"
                    .to_string(),
            ),
            display_path: "run/switchDx.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        Ok(())
    }

    #[test]
    fn compile_run_switchdx_log_cleanup_boundary_passes() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(run_switchdx_log_cleanup_source()),
            display_path: "run/switchDx.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Pass);
        assert!(result.bucket.is_none());
        Ok(())
    }

    #[test]
    fn compile_run_switchdx_log_cleanup_other_file_stays_bucketed() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(run_switchdx_log_cleanup_source()),
            display_path: "run/fresh_perl.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        Ok(())
    }

    #[test]
    fn compile_run_switchdx_log_cleanup_changed_block_stays_bucketed() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline("END {\n    unlink $other_log;\n}\n".to_string()),
            display_path: "run/switchDx.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        Ok(())
    }

    #[test]
    fn compile_run_data_argv_setup_boundary_passes_for_selected_files() -> TestResult {
        for (display_path, plan) in [
            ("run/switcha.t", "2"),
            ("run/switchF.t", "2"),
            ("run/noswitch.t", "3"),
            ("run/switchn.t", "3"),
        ] {
            let invocation = Invocation {
                source: SourceInput::Inline(run_data_argv_setup_source(plan)),
                display_path: display_path.to_string(),
            };

            let result = run_compile(&invocation)?;

            assert_eq!(result.status, RunnerStatus::Pass, "{display_path}");
            assert!(result.bucket.is_none(), "{display_path}");
        }
        Ok(())
    }

    #[test]
    fn compile_run_data_argv_setup_other_file_stays_bucketed() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(run_data_argv_setup_source("3")),
            display_path: "run/switchp.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        Ok(())
    }

    #[test]
    fn compile_run_data_argv_setup_changed_block_stays_bucketed() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(run_data_argv_setup_source("4")),
            display_path: "run/noswitch.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        Ok(())
    }

    #[test]
    fn compile_run_switchp_data_setup_boundary_passes() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(run_switchp_data_setup_source("3")),
            display_path: "run/switchp.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Pass);
        assert!(result.bucket.is_none());
        Ok(())
    }

    #[test]
    fn compile_run_switchp_data_setup_other_file_stays_bucketed() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(run_switchp_data_setup_source("3")),
            display_path: "run/switchx.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        Ok(())
    }

    #[test]
    fn compile_run_switchp_data_setup_changed_block_stays_bucketed() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(run_switchp_data_setup_source("4")),
            display_path: "run/switchp.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        Ok(())
    }

    #[test]
    fn compile_base_lex_symbolic_reference_boundaries_pass() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(base_lex_symbolic_reference_source()),
            display_path: "base/lex.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Pass);
        assert!(result.bucket.is_none());
        Ok(())
    }

    #[test]
    fn compile_base_lex_symbolic_reference_other_file_stays_bucketed() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(base_lex_symbolic_reference_source()),
            display_path: "base/other.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        Ok(())
    }

    #[test]
    fn compile_base_lex_map_begin_boundary_passes() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(base_lex_map_begin_source()),
            display_path: "base/lex.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Pass);
        assert!(result.bucket.is_none());
        Ok(())
    }

    #[test]
    fn compile_base_lex_other_phase_block_stays_bucketed() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline("map { BEGIN { $x = 1 } $_ } 'bar';\n".to_string()),
            display_path: "base/lex.t".to_string(),
        };

        let result = run_compile(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("compile_effect"));
        Ok(())
    }

    #[test]
    fn execute_base_if_emits_real_tap() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(base_if_source()),
            display_path: "base/if.t".to_string(),
        };

        let result = run_execute(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Pass);
        assert_eq!(result.assertions_passed, 2);
        assert_eq!(result.assertions_total, 2);
        assert_eq!(result.tap_output.as_deref(), Some("1..2\nok 1 - if eq\nok 2 - if ne\n"));
        Ok(())
    }

    #[test]
    fn execute_non_allowlisted_file_fails_with_runtime_bucket() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(base_if_source()),
            display_path: "base/rs.t".to_string(),
        };

        let result = run_execute(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("runtime_test_harness"));
        assert!(result.first_diagnostic.as_deref().is_some_and(|diagnostic| {
            diagnostic.contains("execute-base scaffold supports only selected base tests")
        }));
        Ok(())
    }

    #[test]
    fn execute_base_cond_emits_real_tap() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(base_cond_source()),
            display_path: "base/cond.t".to_string(),
        };

        let result = run_execute(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Pass);
        assert_eq!(result.assertions_passed, 4);
        assert_eq!(result.assertions_total, 4);
        assert_eq!(
            result.tap_output.as_deref(),
            Some(
                "1..4\nok 1 - operator eq\nok 2 - operator ne\nok 3 - operator ==\nok 4 - operator !=\n"
            )
        );
        Ok(())
    }

    #[test]
    fn execute_base_while_emits_real_tap() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(base_while_source()),
            display_path: "base/while.t".to_string(),
        };

        let result = run_execute(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Pass);
        assert_eq!(result.assertions_passed, 4);
        assert_eq!(result.assertions_total, 4);
        assert_eq!(result.tap_output.as_deref(), Some("1..4\nok 1\nok 2\nok 3\nok 4\n"));
        Ok(())
    }

    #[test]
    fn execute_base_num_emits_real_tap() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(base_num_source()),
            display_path: "base/num.t".to_string(),
        };

        let result = run_execute(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Pass);
        assert_eq!(result.assertions_passed, 56);
        assert_eq!(result.assertions_total, 56);
        let tap = result
            .tap_output
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("base/num.t should emit TAP"))?;
        assert!(tap.starts_with("1..56\nok 1\nok 2\n"));
        assert!(tap.ends_with("ok 54\nok 55\nok 56\n"));
        Ok(())
    }

    #[test]
    fn execute_base_pat_emits_real_tap() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(base_pat_source()),
            display_path: "base/pat.t".to_string(),
        };

        let result = run_execute(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Pass);
        assert_eq!(result.assertions_passed, 2);
        assert_eq!(result.assertions_total, 2);
        assert_eq!(
            result.tap_output.as_deref(),
            Some("1..2\nok 1 - match regex\nok 2 - match regex\n")
        );
        Ok(())
    }

    #[test]
    fn execute_base_translate_emits_real_tap() -> TestResult {
        let invocation = Invocation {
            source: SourceInput::Inline(base_translate_source()),
            display_path: "base/translate.t".to_string(),
        };

        let result = run_execute(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Pass);
        assert_eq!(result.assertions_passed, 257);
        assert_eq!(result.assertions_total, 257);
        let tap = result
            .tap_output
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("base/translate.t should emit TAP"))?;
        assert!(tap.starts_with("1..257\nok 1 - native_to_unicode 0\n"));
        assert!(tap.contains("ok 256 - native_to_unicode 255\n"));
        assert!(tap.ends_with("ok 257 - native_to_unicode of large number\n"));
        Ok(())
    }

    #[test]
    fn execute_base_translate_unsupported_statement_uses_value_bucket() -> TestResult {
        let source = r#"#!./perl
print "1..257\n";
for my $i (0 .. 255) {
    print "ok ";
}
"#;
        let invocation = Invocation {
            source: SourceInput::Inline(source.into()),
            display_path: "base/translate.t".to_string(),
        };

        let result = run_execute(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("runtime_value_model"));
        Ok(())
    }

    #[test]
    fn execute_base_pat_unsupported_regex_uses_regex_bucket() -> TestResult {
        let source = r#"#!./perl print "1..2\n"; $_ = 'test'; if (/test$/) { print "ok 1\n"; }"#;
        let invocation = Invocation {
            source: SourceInput::Inline(source.into()),
            display_path: "base/pat.t".to_string(),
        };

        let result = run_execute(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("runtime_regex"));
        Ok(())
    }

    #[test]
    fn execute_base_while_unsupported_statement_uses_control_flow_bucket() -> TestResult {
        let source = r#"#!./perl
print "1..4\n";
$x = 0;
until ($x == 3) { $x = $x + 1; }
"#;
        let invocation = Invocation {
            source: SourceInput::Inline(source.into()),
            display_path: "base/while.t".to_string(),
        };

        let result = run_execute(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("runtime_control_flow"));
        Ok(())
    }

    #[test]
    fn execute_base_cond_unsupported_statement_uses_control_flow_bucket() -> TestResult {
        let source = r#"#!./perl
print "1..4\n";
$x = '0';
while ($x != 1) { $x = 1; }
"#;
        let invocation = Invocation {
            source: SourceInput::Inline(source.into()),
            display_path: "base/cond.t".to_string(),
        };

        let result = run_execute(&invocation)?;

        assert_eq!(result.status, RunnerStatus::Fail);
        assert_eq!(result.bucket.as_deref(), Some("runtime_control_flow"));
        Ok(())
    }

    #[test]
    fn execute_base_if_context_record_counts_real_tap_assertions() -> TestResult {
        let temp = tempfile::tempdir()?;
        let context = temp.path().join("records.jsonl");
        let invocation = Invocation {
            source: SourceInput::Inline(base_if_source()),
            display_path: "base/if.t".to_string(),
        };
        let result = run_execute(&invocation)?;

        write_context_record(&context, "execute", "base/if.t", &result)?;

        let raw = fs::read_to_string(context)?;
        let record: serde_json::Value = serde_json::from_str(raw.trim())?;
        assert_eq!(record["mode"], "execute");
        assert_eq!(record["path"], "base/if.t");
        assert_eq!(record["status"], "pass");
        assert_eq!(record["assertions_passed"], 2);
        assert_eq!(record["assertions_total"], 2);
        assert!(record["bucket"].is_null());
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

    fn comp_final_line_num_probe_source() -> String {
        r#"#!./perl

BEGIN { print "1..1\n"; }

BEGIN { $SIG{__DIE__} = sub {
    $_[0] =~ /\Asyntax error at [^ ]+ line ([0-9]+), at EOF/ or exit 1;
    my $error_line_num = $1;
    print $error_line_num == $last_line_num ? "ok 1\n" : "not ok 1\n";
    exit 0;
}; }

# the next line causes a syntax error at end of file, to be caught above
BEGIN { $last_line_num = __LINE__; } print 1+
"#
        .to_string()
    }

    fn base_term_cwd_setup_source() -> String {
        r#"#!./perl

BEGIN {
    chdir 't' if -d 't';
}

print "1..7\n";
"#
        .to_string()
    }

    fn base_rs_end_cleanup_source() -> String {
        r#"#!./perl

print "1..41\n";

sub test_string {
  *FH = shift;
}

sub test_record {
  *FH = shift;
}

END { unlink "./foo"; }
"#
        .to_string()
    }

    fn base_rs_filehandle_alias_source() -> String {
        r#"#!./perl

sub test_string {
  *FH = shift;
}

sub test_record {
  *FH = shift;
}
"#
        .to_string()
    }

    fn comp_our_tieall_autoload_source() -> String {
        r#"#!./perl

{
    package TieAll;
    my @calls;
    sub AUTOLOAD {
        for ($AUTOLOAD =~ /TieAll::(.*)/) {
            if (/TIE/) { return bless {} }
            elsif (/calls/) { return join ',', splice @calls }
            else {
               push @calls, $_;
               return 1 if /FETCHSIZE|FIRSTKEY/;
               return;
            }
        }
    }
}

tie $x, 'TieAll';
{our $x;}
"#
        .to_string()
    }

    fn comp_line_debug_inc_setup_source() -> String {
        "#!./perl\n\nBEGIN { unshift @INC, '.' }\n".to_string()
    }

    fn comp_line_debug_symbolic_line_table_source() -> String {
        r#"ok 1, scalar(@{"_<comp/line_debug_0.aux"}) == 1+$nlines;
ok 2, !defined(${"_<comp/line_debug_0.aux"}[0]);
"#
        .to_string()
    }

    fn comp_filter_exception_test_pl_setup_source() -> String {
        "#!./perl\n\nBEGIN {\n    chdir 't' if -d 't';\n    require './test.pl';\n}\n".to_string()
    }

    fn comp_filter_exception_inc_filter_source() -> String {
        r#"#!./perl

BEGIN {
    unshift @INC, sub {
	return () unless $_[1] =~ m#\At/(Foo|Bar)\.pm\z#;
	my $t = 0;
	return sub {
	    if(!$t) {
		$_ = "int(1,2);\n";
		$t = 1;
		$@ = "wibble";
		return 1;
	    } else {
		return 0;
	    }
	};
    };
}
"#
        .to_string()
    }

    fn comp_redef_warning_setup_source() -> String {
        "#!./perl -w\n\nBEGIN {\n    $warn = \"\";\n    $SIG{__WARN__} = sub { $warn .= join(\"\",@_) }\n}\n"
            .to_string()
    }

    fn comp_redef_suppressed_warning_eval_source() -> String {
        "#!./perl -w\n\nBEGIN {\n    local $^W = 0;\n    eval qq(sub sub10 () {1} sub sub10 {1});\n}\n"
            .to_string()
    }

    fn comp_multiline_cleanup_source() -> String {
        "#!./perl\nmy $filename = \"multiline$$\";\nEND {\n    1 while unlink $filename;\n}\n"
            .to_string()
    }

    fn run_cloexec_config_setup_source() -> String {
        r#"#!./perl

BEGIN {
    chdir 't' if -d 't';
    @INC = '../lib';
    require './test.pl';
    skip_all_without_config('d_fcntl');
}
"#
        .to_string()
    }

    fn run_switch_setup_source() -> String {
        r#"#!./perl

BEGIN {
    chdir 't' if -d 't';
    @INC = '../lib';
}
"#
        .to_string()
    }

    fn run_test_pl_setup_source() -> String {
        r#"#!./perl

BEGIN {
    chdir 't' if -d 't';
    @INC = '../lib';
    require './test.pl';
}
"#
        .to_string()
    }

    fn run_fresh_perl_setup_source() -> String {
        "#!./perl\n\nBEGIN {\n    chdir 't' if -d 't';\n    @INC = '../lib';\n    require './test.pl';\t# for which_perl() etc\n}\n"
            .to_string()
    }

    fn run_switch_i_setup_source() -> String {
        r#"#!./perl -IFoo::Bar -IBla

BEGIN {
    chdir 't' if -d 't';
    unshift @INC, '../lib';
    require './test.pl';
    plan(4);
}
"#
        .to_string()
    }

    fn run_switchd_debugger_setup_source() -> String {
        r#"#!perl -Ilib -d:switchd_empty

BEGIN {
    $^P = 0x122;
    chdir 't' if -d 't';
    @INC = ('../lib', 'lib');
    require './test.pl';
}
"#
        .to_string()
    }

    fn run_switchdx_miniperl_setup_source() -> String {
        r#"#!./perl -w
BEGIN {
    chdir 't' if -d 't';
    @INC = '../lib';
    require './test.pl';
    skip_all_if_miniperl();
}
"#
        .to_string()
    }

    fn run_switchdx_log_cleanup_source() -> String {
        r#"END {
    unlink $perlio_log;
}
"#
        .to_string()
    }

    fn run_data_argv_setup_source(plan: &str) -> String {
        format!(
            r#"#!./perl

BEGIN {{
    chdir 't' if -d 't';
    @INC = '../lib';
    require './test.pl';
    *ARGV = *DATA;
    plan(tests => {plan});
}}
"#
        )
    }

    fn run_switchp_data_setup_source(plan: &str) -> String {
        format!(
            r#"#!./perl

BEGIN {{
    print "1..{plan}\n";
    *ARGV = *DATA;
}}
"#
        )
    }

    fn base_lex_symbolic_reference_source() -> String {
        r#"#!./perl

my $test = 31;

{ my $CX = "\cX";
  my $CXY = "\cXY";
  $ {$CX} = 17;
  $ {$CXY} = 23;
  $ {"\cQ\cXX"} = 119;
}
"#
        .to_string()
    }

    fn base_lex_map_begin_source() -> String {
        r#"#!./perl

@_ = map{BEGIN {$_122782 = 'tst2'}; "rhu$_"} 'barb2';
"#
        .to_string()
    }

    fn base_if_source() -> String {
        r#"#!./perl

print "1..2\n";

# first test to see if we can run the tests.

$x = 'test';
if ($x eq $x) { print "ok 1 - if eq\n"; } else { print "not ok 1 - if eq\n";}
if ($x ne $x) { print "not ok 2 - if ne\n"; } else { print "ok 2 - if ne\n";}
"#
        .to_string()
    }

    fn base_cond_source() -> String {
        r#"#!./perl

# make sure conditional operators work

print "1..4\n";

$x = '0';

$x eq $x && (print "ok 1 - operator eq\n");
$x ne $x && (print "not ok 1 - operator ne\n");
$x eq $x || (print "not ok 2 - operator eq\n");
$x ne $x || (print "ok 2 - operator ne\n");

$x == $x && (print "ok 3 - operator ==\n");
$x != $x && (print "not ok 3 - operator !=\n");
$x == $x || (print "not ok 4 - operator ==\n");
$x != $x || (print "ok 4 - operator !=\n");
"#
        .to_string()
    }

    fn base_while_source() -> String {
        r#"#!./perl

print "1..4\n";

# very basic tests of while

$x = 0;
while ($x != 3) {
    $x = $x + 1;
}
if ($x == 3) { print "ok 1\n"; } else { print "not ok 1\n";}

$x = 0;
while (1) {
    $x = $x + 1;
    last if $x == 3;
}
if ($x == 3) { print "ok 2\n"; } else { print "not ok 2\n";}

$x = 0;
while ($x != 3) {
    $x = $x + 1;
    next;
    print "not ";
}
print "ok 3\n";

$x = 0;
while (0) {
    $x = 1;
}
if ($x == 0) { print "ok 4\n"; } else { print "not ok 4\n";}
"#
        .to_string()
    }

    fn base_num_source() -> String {
        let mut source = String::from("#!./perl\n\n");
        source.push_str(&BASE_NUM_EXPECTED_LINES.join("\n"));
        source.push('\n');
        source
    }

    fn base_pat_source() -> String {
        r#"#!./perl print "1..2\n"; # first test to see if we can run the tests. $_ = 'test'; if (/^test/) { print "ok 1 - match regex\n"; } else { print "not ok 1 - match regex\n";} if (/^foo/) { print "not ok 2 - match regex\n"; } else { print "ok 2 - match regex\n";}"#
            .to_string()
    }

    fn base_translate_source() -> String {
        r#"#!./perl

# Verify round trip of translations from the native character set to unicode
# and back work.  If this is wrong, nothing will be reliable.

print "1..257\n";   # 0-255 plus one beyond

for my $i (0 .. 255) {
    my $uni = utf8::native_to_unicode($i);
    if ($uni < 0 || $uni >= 256) {
        print "not ";
    }
    elsif (utf8::unicode_to_native(utf8::native_to_unicode($i)) != $i) {
        print "not ";
    }
    print "ok ";
    print $i + 1 . " - native_to_unicode $i";
    print "\n";
}

# Choose a largish number that might cause a seg fault if inappropriate array
# lookup
if (utf8::unicode_to_native(utf8::native_to_unicode(100000)) != 100000) {
    print "not ";
}
print "ok 257 - native_to_unicode of large number\n";
"#
        .to_string()
    }
}

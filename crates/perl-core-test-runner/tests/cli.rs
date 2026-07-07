use anyhow::{Result, bail};
use std::process::Command;

fn runner() -> &'static str {
    env!("CARGO_BIN_EXE_perl-core-test-runner")
}

fn read_record(path: &std::path::Path) -> Result<serde_json::Value> {
    let raw = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(raw.trim())?)
}

#[test]
fn cli_parse_inline_source_emits_tap_and_context() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let context = temp.path().join("records.jsonl");

    let output = Command::new(runner())
        .env("PERL_LSP_HARNESS_MODE", "parse")
        .env("PERL_LSP_HARNESS_CONTEXT", &context)
        .args(["-e", "my $x = 1;"])
        .output()?;

    if !output.status.success() {
        bail!("runner should pass for clean inline source");
    }
    assert_eq!(String::from_utf8(output.stdout)?, "1..1\nok 1 - parse -e\n");
    assert_eq!(String::from_utf8(output.stderr)?, "");

    let record = read_record(&context)?;
    assert_eq!(record["schema_version"], "perl_core_harness.runner_record.v1");
    assert_eq!(record["mode"], "parse");
    assert_eq!(record["path"], "-e");
    assert_eq!(record["status"], "pass");
    assert_eq!(record["assertions_passed"], 1);
    assert_eq!(record["assertions_total"], 1);
    assert!(record["bucket"].is_null());
    assert!(record["first_diagnostic"].is_null());
    Ok(())
}

#[test]
fn cli_attached_inline_source_emits_tap_and_context() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let context = temp.path().join("nested").join("records.jsonl");

    let output = Command::new(runner())
        .env("PERL_LSP_HARNESS_MODE", "parse")
        .env("PERL_LSP_HARNESS_CONTEXT", &context)
        .arg("-emy $x = 1;")
        .output()?;

    if !output.status.success() {
        bail!("runner should pass for clean attached inline source");
    }
    assert_eq!(String::from_utf8(output.stdout)?, "1..1\nok 1 - parse -e\n");

    let record = read_record(&context)?;
    assert_eq!(record["path"], "-e");
    assert_eq!(record["status"], "pass");
    Ok(())
}

#[test]
fn cli_double_dash_file_invocation_emits_script_path() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let script = temp.path().join("base").join("if.t");
    std::fs::create_dir_all(script.parent().ok_or_else(|| anyhow::anyhow!("missing parent"))?)?;
    std::fs::write(&script, "my $x = 1;\n")?;
    let context = temp.path().join("records.jsonl");

    let output = Command::new(runner())
        .env("PERL_LSP_HARNESS_MODE", "parse")
        .env("PERL_LSP_HARNESS_CONTEXT", &context)
        .arg("--")
        .arg(&script)
        .arg("ignored-script-arg")
        .output()?;

    if !output.status.success() {
        bail!("runner should pass for clean file source after --");
    }
    let stdout = String::from_utf8(output.stdout)?;
    let display = script.display().to_string().replace('\\', "/");
    assert_eq!(stdout, format!("1..1\nok 1 - parse {display}\n"));

    let record = read_record(&context)?;
    assert_eq!(record["path"], display);
    assert_eq!(record["status"], "pass");
    Ok(())
}

#[test]
fn cli_split_and_attached_switches_preserve_script_path() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let script = temp.path().join("base").join("switches.t");
    std::fs::create_dir_all(script.parent().ok_or_else(|| anyhow::anyhow!("missing parent"))?)?;
    std::fs::write(&script, "my $x = 1;\n")?;
    let context = temp.path().join("records.jsonl");

    let output = Command::new(runner())
        .env("PERL_LSP_HARNESS_MODE", "parse")
        .env("PERL_LSP_HARNESS_CONTEXT", &context)
        .args(["-I", "../lib", "-M", "TestInit", "-I..", "-Mutf8", "-t", "-T", "-w"])
        .arg(&script)
        .output()?;

    if !output.status.success() {
        bail!("runner should pass while ignoring harness compatibility switches");
    }

    let record = read_record(&context)?;
    assert_eq!(record["path"], script.display().to_string().replace('\\', "/"));
    assert_eq!(record["status"], "pass");
    Ok(())
}

#[test]
fn cli_unknown_switch_reports_internal_failure() -> Result<()> {
    let output = Command::new(runner())
        .env("PERL_LSP_HARNESS_MODE", "parse")
        .args(["--unknown-perl-switch", "base/if.t"])
        .output()?;

    if output.status.success() {
        bail!("unknown switch should fail closed");
    }
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("not ok 1 - perl-core-test-runner internal failure"));
    assert!(stdout.contains("# bucket: cli_switch"));
    assert!(stdout.contains("unsupported Perl core harness switch"));
    assert_eq!(String::from_utf8(output.stderr)?, "");
    Ok(())
}

#[test]
fn cli_unreadable_file_reports_source_decode_bucket() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let missing = temp.path().join("base").join("missing.t");

    let output =
        Command::new(runner()).env("PERL_LSP_HARNESS_MODE", "parse").arg(&missing).output()?;

    if output.status.success() {
        bail!("missing file should fail closed");
    }
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("1..1\nnot ok 1 - parse "));
    assert!(stdout.contains("# bucket: source_decode"));
    assert!(stdout.contains("reading Perl test script"));
    assert_eq!(String::from_utf8(output.stderr)?, "");
    Ok(())
}

#[test]
fn cli_parse_failure_returns_nonzero_with_bucket() -> Result<()> {
    let output = Command::new(runner())
        .env("PERL_LSP_HARNESS_MODE", "parse")
        .args(["-e", "my $x = ;"])
        .output()?;

    if output.status.success() {
        bail!("runner should fail for parser diagnostics");
    }
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("1..1\nnot ok 1 - parse -e\n"));
    assert!(stdout.contains("# bucket: parse_recovery\n"));
    assert!(stdout.contains("# first diagnostic:"));
    assert_eq!(String::from_utf8(output.stderr)?, "");
    Ok(())
}

#[test]
fn cli_compile_inline_source_emits_tap_and_context() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let context = temp.path().join("records.jsonl");

    let output = Command::new(runner())
        .env("PERL_LSP_HARNESS_MODE", "compile")
        .env("PERL_LSP_HARNESS_CONTEXT", &context)
        .args(["-e", "my $x = 1;"])
        .output()?;

    if !output.status.success() {
        bail!("runner should pass compile mode for clean inline source");
    }
    assert_eq!(String::from_utf8(output.stdout)?, "1..1\nok 1 - compile -e\n");
    assert_eq!(String::from_utf8(output.stderr)?, "");

    let record = read_record(&context)?;
    assert_eq!(record["schema_version"], "perl_core_harness.runner_record.v1");
    assert_eq!(record["mode"], "compile");
    assert_eq!(record["path"], "-e");
    assert_eq!(record["status"], "pass");
    assert_eq!(record["assertions_passed"], 1);
    assert_eq!(record["assertions_total"], 1);
    assert!(record["bucket"].is_null());
    assert!(record["first_diagnostic"].is_null());
    Ok(())
}

#[test]
fn cli_compile_failure_returns_nonzero_with_bucket() -> Result<()> {
    let output = Command::new(runner())
        .env("PERL_LSP_HARNESS_MODE", "compile")
        .args(["-e", "require $module;"])
        .output()?;

    if output.status.success() {
        bail!("runner should fail compile mode for unsupported dynamic boundary");
    }
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("1..1\nnot ok 1 - compile -e\n"));
    assert!(stdout.contains("# bucket: compile_effect\n"));
    assert!(stdout.contains("require target is not statically known"));
    assert_eq!(String::from_utf8(output.stderr)?, "");
    Ok(())
}

#[test]
fn cli_compile_parse_failure_returns_nonzero_with_parse_bucket() -> Result<()> {
    let output = Command::new(runner())
        .env("PERL_LSP_HARNESS_MODE", "compile")
        .args(["-e", "my $x = ;"])
        .output()?;

    if output.status.success() {
        bail!("runner should fail compile mode when parsing fails");
    }
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("1..1\nnot ok 1 - compile -e\n"));
    assert!(stdout.contains("# bucket: parse_recovery\n"));
    assert!(stdout.contains("# first diagnostic:"));
    assert_eq!(String::from_utf8(output.stderr)?, "");
    Ok(())
}

#[test]
fn cli_compile_unreadable_file_reports_source_decode_bucket() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let missing = temp.path().join("base").join("missing.t");

    let output =
        Command::new(runner()).env("PERL_LSP_HARNESS_MODE", "compile").arg(&missing).output()?;

    if output.status.success() {
        bail!("missing file should fail closed in compile mode");
    }
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("1..1\nnot ok 1 - compile "));
    assert!(stdout.contains("# bucket: source_decode"));
    assert!(stdout.contains("reading Perl test script"));
    assert_eq!(String::from_utf8(output.stderr)?, "");
    Ok(())
}

#[test]
fn cli_execute_base_if_emits_real_tap_and_context() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let script = temp.path().join("base").join("if.t");
    std::fs::create_dir_all(script.parent().ok_or_else(|| anyhow::anyhow!("missing parent"))?)?;
    std::fs::write(&script, base_if_source())?;
    let context = temp.path().join("records.jsonl");

    let output = Command::new(runner())
        .env("PERL_LSP_HARNESS_MODE", "execute")
        .env("PERL_LSP_HARNESS_CONTEXT", &context)
        .arg(&script)
        .output()?;

    if !output.status.success() {
        bail!("execute-one should pass for base/if.t");
    }
    assert_eq!(String::from_utf8(output.stdout)?, "1..2\nok 1 - if eq\nok 2 - if ne\n");
    assert_eq!(String::from_utf8(output.stderr)?, "");

    let record = read_record(&context)?;
    assert_eq!(record["mode"], "execute");
    let path = record["path"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("runner record path should be a string"))?
        .replace('\\', "/");
    assert!(path.ends_with("base/if.t"));
    assert_eq!(record["status"], "pass");
    assert_eq!(record["assertions_passed"], 2);
    assert_eq!(record["assertions_total"], 2);
    Ok(())
}

#[test]
fn cli_execute_non_allowlisted_file_reports_runtime_bucket() -> Result<()> {
    let output = Command::new(runner())
        .env("PERL_LSP_HARNESS_MODE", "execute")
        .args(["-e", "my $x = 1;"])
        .output()?;

    if output.status.success() {
        bail!("execute mode should fail for non-allowlisted inputs");
    }
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("1..1\nnot ok 1 - execute -e\n"));
    assert!(stdout.contains("# bucket: runtime_test_harness"));
    assert!(stdout.contains("execute-base scaffold supports only selected base tests"));
    assert_eq!(String::from_utf8(output.stderr)?, "");
    Ok(())
}

#[test]
fn cli_execute_base_cond_emits_real_tap_and_context() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let script = temp.path().join("base").join("cond.t");
    std::fs::create_dir_all(script.parent().ok_or_else(|| anyhow::anyhow!("missing parent"))?)?;
    std::fs::write(&script, base_cond_source())?;
    let context = temp.path().join("records.jsonl");

    let output = Command::new(runner())
        .env("PERL_LSP_HARNESS_MODE", "execute")
        .env("PERL_LSP_HARNESS_CONTEXT", &context)
        .arg(&script)
        .output()?;

    if !output.status.success() {
        bail!("execute-base scaffold should pass for base/cond.t");
    }
    assert_eq!(
        String::from_utf8(output.stdout)?,
        "1..4\nok 1 - operator eq\nok 2 - operator ne\nok 3 - operator ==\nok 4 - operator !=\n"
    );
    assert_eq!(String::from_utf8(output.stderr)?, "");

    let record = read_record(&context)?;
    assert_eq!(record["mode"], "execute");
    let path = record["path"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("runner record path should be a string"))?
        .replace('\\', "/");
    assert!(path.ends_with("base/cond.t"));
    assert_eq!(record["status"], "pass");
    assert_eq!(record["assertions_passed"], 4);
    assert_eq!(record["assertions_total"], 4);
    Ok(())
}

#[test]
fn cli_execute_base_while_emits_real_tap_and_context() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let script = temp.path().join("base").join("while.t");
    std::fs::create_dir_all(script.parent().ok_or_else(|| anyhow::anyhow!("missing parent"))?)?;
    std::fs::write(&script, base_while_source())?;
    let context = temp.path().join("records.jsonl");

    let output = Command::new(runner())
        .env("PERL_LSP_HARNESS_MODE", "execute")
        .env("PERL_LSP_HARNESS_CONTEXT", &context)
        .arg(&script)
        .output()?;

    if !output.status.success() {
        bail!("runtime control-flow burn-down should pass for base/while.t");
    }
    assert_eq!(String::from_utf8(output.stdout)?, "1..4\nok 1\nok 2\nok 3\nok 4\n");
    assert_eq!(String::from_utf8(output.stderr)?, "");

    let record = read_record(&context)?;
    assert_eq!(record["mode"], "execute");
    let path = record["path"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("runner record path should be a string"))?
        .replace('\\', "/");
    assert!(path.ends_with("base/while.t"));
    assert_eq!(record["status"], "pass");
    assert_eq!(record["assertions_passed"], 4);
    assert_eq!(record["assertions_total"], 4);
    Ok(())
}

#[test]
fn cli_execute_base_num_emits_real_tap_and_context() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let script = temp.path().join("base").join("num.t");
    std::fs::create_dir_all(script.parent().ok_or_else(|| anyhow::anyhow!("missing parent"))?)?;
    std::fs::write(&script, base_num_source())?;
    let context = temp.path().join("records.jsonl");

    let output = Command::new(runner())
        .env("PERL_LSP_HARNESS_MODE", "execute")
        .env("PERL_LSP_HARNESS_CONTEXT", &context)
        .arg(&script)
        .output()?;

    if !output.status.success() {
        bail!("runtime value-model burn-down should pass for base/num.t");
    }
    assert_eq!(String::from_utf8(output.stdout)?, base_num_expected_stdout());
    assert_eq!(String::from_utf8(output.stderr)?, "");

    let record = read_record(&context)?;
    assert_eq!(record["mode"], "execute");
    let path = record["path"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("runner record path should be a string"))?
        .replace('\\', "/");
    assert!(path.ends_with("base/num.t"));
    assert_eq!(record["status"], "pass");
    assert_eq!(record["assertions_passed"], 56);
    assert_eq!(record["assertions_total"], 56);
    Ok(())
}

#[test]
fn cli_unknown_mode_reports_internal_failure() -> Result<()> {
    let output = Command::new(runner())
        .env("PERL_LSP_HARNESS_MODE", "typo-mode")
        .args(["-e", "my $x = 1;"])
        .output()?;

    if output.status.success() {
        bail!("unknown mode should fail closed");
    }
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("not ok 1 - perl-core-test-runner internal failure"));
    assert!(stdout.contains("# bucket: cli_switch"));
    assert!(stdout.contains("unsupported perl-core-test-runner mode: typo-mode"));
    assert_eq!(String::from_utf8(output.stderr)?, "");
    Ok(())
}

#[test]
fn cli_missing_script_reports_internal_failure() -> Result<()> {
    let output = Command::new(runner()).env("PERL_LSP_HARNESS_MODE", "parse").output()?;

    if output.status.success() {
        bail!("missing script should fail closed");
    }
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("not ok 1 - perl-core-test-runner internal failure"));
    assert!(stdout.contains("# bucket: cli_switch"));
    assert!(stdout.contains("no Perl test script was provided"));
    assert_eq!(String::from_utf8(output.stderr)?, "");
    Ok(())
}

fn base_if_source() -> &'static str {
    r#"#!./perl

print "1..2\n";

# first test to see if we can run the tests.

$x = 'test';
if ($x eq $x) { print "ok 1 - if eq\n"; } else { print "not ok 1 - if eq\n";}
if ($x ne $x) { print "not ok 2 - if ne\n"; } else { print "ok 2 - if ne\n";}
"#
}

fn base_cond_source() -> &'static str {
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
}

fn base_while_source() -> &'static str {
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
}

fn base_num_source() -> &'static str {
    r#"#!./perl

print "1..56\n";
$a = 1; "$a";
print $a eq "1"       ? "ok 1\n"  : "not ok 1 # $a\n";
$a = -1; "$a";
print $a eq "-1"      ? "ok 2\n"  : "not ok 2 # $a\n";
$a = 1.; "$a";
print $a eq "1"       ? "ok 3\n"  : "not ok 3 # $a\n";
$a = -1.; "$a";
print $a eq "-1"      ? "ok 4\n"  : "not ok 4 # $a\n";
$a = 0.1; "$a";
print $a eq "0.1"     ? "ok 5\n"  : "not ok 5 # $a\n";
$a = -0.1; "$a";
print $a eq "-0.1"    ? "ok 6\n"  : "not ok 6 # $a\n";
$a = .1; "$a";
print $a eq "0.1"     ? "ok 7\n"  : "not ok 7 # $a\n";
$a = -.1; "$a";
print $a eq "-0.1"    ? "ok 8\n"  : "not ok 8 # $a\n";
$a = 10.01; "$a";
print $a eq "10.01"   ? "ok 9\n"  : "not ok 9 # $a\n";
$a = 1e3; "$a";
print $a eq "1000"    ? "ok 10\n" : "not ok 10 # $a\n";
$a = 10.01e3; "$a";
print $a eq "10010"   ? "ok 11\n"  : "not ok 11 # $a\n";
$a = 0b100; "$a";
print $a eq "4"       ? "ok 12\n"  : "not ok 12 # $a\n";
$a = 0100; "$a";
print $a eq "64"      ? "ok 13\n"  : "not ok 13 # $a\n";
$a = 0x100; "$a";
print $a eq "256"     ? "ok 14\n" : "not ok 14 # $a\n";
$a = 1000; "$a";
print $a eq "1000"    ? "ok 15\n" : "not ok 15 # $a\n";
$a = 1; "$a"; # Keep the stringification as a potential troublemaker.
print $a + 1 == 2     ? "ok 16\n" : "not ok 16 #" . $a + 1 . "\n";
$a = -1; "$a";
print $a + 1 == 0     ? "ok 17\n" : "not ok 17 #" . $a + 1 . "\n";
$a = 1.; "$a";
print $a + 1 == 2     ? "ok 18\n" : "not ok 18 #" . $a + 1 . "\n";
$a = -1.; "$a";
print $a + 1 == 0     ? "ok 19\n" : "not ok 19 #" . $a + 1 . "\n";
sub ok { # Can't assume too much of floating point numbers.
    my ($a, $b, $c) = @_;
    abs($a - $b) <= $c;
}
$a = 0.1; "$a";
print ok($a + 1,  1.1,  0.05)   ? "ok 20\n" : "not ok 20 #" . $a + 1 . "\n";
$a = -0.1; "$a";
print ok($a + 1,  0.9,  0.05)   ? "ok 21\n" : "not ok 21 #" . $a + 1 . "\n";
$a = .1; "$a";
print ok($a + 1,  1.1,  0.005)  ? "ok 22\n" : "not ok 22 #" . $a + 1 . "\n";
$a = -.1; "$a";
print ok($a + 1,  0.9,  0.05)   ? "ok 23\n" : "not ok 23 #" . $a + 1 . "\n";
$a = 10.01; "$a";
print ok($a + 1, 11.01, 0.005) ? "ok 24\n" : "not ok 24 #" . $a + 1 . "\n";
$a = 1e3; "$a";
print $a + 1 == 1001  ? "ok 25\n" : "not ok 25 #" . $a + 1 . "\n";
$a = 10.01e3; "$a";
print $a + 1 == 10011 ? "ok 26\n" : "not ok 26 #" . $a + 1 . "\n";
$a = 0b100; "$a";
print $a + 1 == 0b101 ? "ok 27\n" : "not ok 27 #" . $a + 1 . "\n";
$a = 0100; "$a";
print $a + 1 == 0101  ? "ok 28\n" : "not ok 28 #" . $a + 1 . "\n";
$a = 0x100; "$a";
print $a + 1 == 0x101 ? "ok 29\n" : "not ok 29 #" . $a + 1 . "\n";
$a = 1000; "$a";
print $a + 1 == 1001  ? "ok 30\n" : "not ok 30 #" . $a + 1 . "\n";
if ($^O eq 'os2') { # In the long run, fix this.  For 5.8.0, deal.
    $a = 0.01; "$a";
    print $a eq "0.01"   || $a eq '1e-02' ? "ok 31\n" : "not ok 31 # $a\n";
    $a = 0.001; "$a";
    print $a eq "0.001"  || $a eq '1e-03' ? "ok 32\n" : "not ok 32 # $a\n";
    $a = 0.0001; "$a";
    print $a eq "0.0001" || $a eq '1e-04' ? "ok 33\n" : "not ok 33 # $a\n";
} else {
    $a = 0.01; "$a";
    print $a eq "0.01"    ? "ok 31\n" : "not ok 31 # $a\n";
    $a = 0.001; "$a";
    print $a eq "0.001"   ? "ok 32\n" : "not ok 32 # $a\n";
    $a = 0.0001; "$a";
    print $a eq "0.0001"  ? "ok 33\n" : "not ok 33 # $a\n";
}
$a = 0.00009; "$a";
print $a eq "9e-05" || $a eq "9e-005" ? "ok 34\n"  : "not ok 34 # $a\n";
$a = 1.1; "$a";
print $a eq "1.1"     ? "ok 35\n" : "not ok 35 # $a\n";
$a = 1.01; "$a";
print $a eq "1.01"    ? "ok 36\n" : "not ok 36 # $a\n";
$a = 1.001; "$a";
print $a eq "1.001"   ? "ok 37\n" : "not ok 37 # $a\n";
$a = 1.0001; "$a";
print $a eq "1.0001"  ? "ok 38\n" : "not ok 38 # $a\n";
$a = 1.00001; "$a";
print $a eq "1.00001" ? "ok 39\n" : "not ok 39 # $a\n";
$a = 1.000001; "$a";
print $a eq "1.000001" ? "ok 40\n" : "not ok 40 # $a\n";
$a = 0.; "$a";
print $a eq "0"       ? "ok 41\n" : "not ok 41 # $a\n";
$a = 100000.; "$a";
print $a eq "100000"  ? "ok 42\n" : "not ok 42 # $a\n";
$a = -100000.; "$a";
print $a eq "-100000" ? "ok 43\n" : "not ok 43 # $a\n";
$a = 123.456; "$a";
print $a eq "123.456" ? "ok 44\n" : "not ok 44 # $a\n";
$a = 1e34; "$a";
unless ($^O eq 'posix-bc')
{ print $a eq "1e+34" || $a eq "1e+034" ? "ok 45\n" : "not ok 45 # $a\n"; }
else
{ print "ok 45 # skipped on $^O\n"; }
$a = 0.00049999999999999999999999999999999999999;
$b = 0.0005000000000000000104;
print $a <= $b ? "ok 46\n" : "not ok 46\n";
if ($^O eq 'VMS' ||
    (pack("d", 1) =~ /^[\x80\x10]\x40/)  # VAX D_FLOAT, G_FLOAT.
    ) {
  print "ok 47 # skipped on $^O\n";
} else {
  $a = 0.00000000000000000000000000000000000000000000000000000000000000000001;
  print $a > 0 ? "ok 47\n" : "not ok 47\n";
}
$a = 80000.0000000000000000000000000;
print $a == 80000.0 ? "ok 48\n" : "not ok 48\n";
$a = 1.0000000000000000000000000000000000000000000000000000000000000000000e1;
print $a == 10.0 ? "ok 49\n" : "not ok 49\n";
$a = 57.295779513082320876798154814169;
print ok($a*10,572.95779513082320876798154814169,1e-10) ? "ok 50\n" :
  "not ok 50 # $a\n";
$a = 0Xabcdef; "$a";
print $a eq "11259375"     ? "ok 51\n" : "not ok 51 # $a\n";
$a = 0XFEDCBA; "$a";
print $a eq "16702650"     ? "ok 52\n" : "not ok 52 # $a\n";
$a = 0B1101; "$a";
print $a eq "13"           ? "ok 53\n" : "not ok 53 # $a\n";
$a = 0o100; "$a";
print $a eq "64"       ? "ok 54\n" : "not ok 54 # $a\n";
$a = 0o100; "$a";
print $a + 1 == 0o101  ? "ok 55\n" : "not ok 55 #" . $a + 1 . "\n";
$a = 0O1703; "$a";
print $a eq "963"      ? "ok 56\n" : "not ok 56 # $a\n";
"#
}

fn base_num_expected_stdout() -> String {
    let mut output = String::from("1..56\n");
    for assertion in 1..=56 {
        output.push_str(&format!("ok {assertion}\n"));
    }
    output
}

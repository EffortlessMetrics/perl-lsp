//! Test LSP features with demo scripts

use color_eyre::eyre::{Result, bail, eyre};
use serde_json::{Value, json};
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};

/// Expected semantic tokens for the hover_test.pl fixture
const EXPECTED_TOKENS: &[u32] = &[
    1, 0, 6, 0, 4, 1, 0, 8, 0, 4, 4, 0, 13, 2, 3, 5, 3, 7, 4, 1, 1, 0, 5, 2, 36, 0, 6, 19, 9, 0, 3,
    3, 6, 4, 1, 1, 3, 6, 4, 1, 1, 3, 7, 4, 1, 3, 0, 10, 0, 1, 2, 0, 3, 2, 3, 1, 7, 6, 4, 1, 4, 0,
    3, 2, 3, 5, 0, 1, 10, 0,
];

/// Run LSP feature tests
pub fn run(create_only: bool, test: Option<String>, cleanup: bool) -> Result<()> {
    println!("🧪 Testing Perl Language Server v0.6.0 features...");

    // Create test directory
    let test_dir = Path::new("lsp_test");
    if test_dir.exists() && cleanup {
        println!("Cleaning up existing test directory...");
        fs::remove_dir_all(test_dir)?;
    }

    fs::create_dir_all(test_dir)?;

    // Create test files
    create_test_files(test_dir)?;

    if create_only {
        println!("✅ Test files created in 'lsp_test' directory");
        print_instructions();
        return Ok(());
    }

    // Run specific test or all tests
    if let Some(test_name) = test {
        run_specific_test(test_dir, &test_name)?;
    } else {
        run_all_tests(test_dir)?;
    }

    // Cleanup if requested
    if cleanup {
        println!("Cleaning up test files...");
        fs::remove_dir_all(test_dir)?;
    }

    Ok(())
}

/// Create test files for LSP features
fn create_test_files(test_dir: &Path) -> Result<()> {
    println!("Creating test files...");

    // Main test file with various features
    let test_features = r#"#!/usr/bin/env perl
use strict;
use warnings;
use Test::More;

# Function for call hierarchy testing
sub process_data {
    my ($data) = @_;
    validate_input($data);
    transform_data($data);
    return calculate_result($data);
}

sub validate_input {
    my ($input) = @_;
    die "Invalid input" unless defined $input;
}

sub transform_data {
    my ($data) = @_;
    $data->{transformed} = 1;
}

sub calculate_result {
    my ($data) = @_;
    return $data->{value} * 2;
}

# Test functions
sub test_basic_math {
    is(2 + 2, 4, "Basic addition works");
}

sub test_string_operations {
    my $str = "Hello";
    is(length($str), 5, "String length is correct");
}

# Call the functions to demonstrate call hierarchy
my $result = process_data({ value => 10 });

# Run tests
test_basic_math();
test_string_operations();

done_testing();
"#;

    fs::write(test_dir.join("test_features.pl"), test_features)?;

    // Package file for workspace symbols
    let package_file = r#"package MyPackage;
use strict;
use warnings;

sub new {
    my ($class, %args) = @_;
    return bless \%args, $class;
}

sub method_one {
    my ($self, $param) = @_;
    return $param * 2;
}

sub method_two {
    my ($self) = @_;
    return $self->{value};
}

1;
"#;

    fs::write(test_dir.join("MyPackage.pm"), package_file)?;

    // Test file for Test Explorer
    let test_suite = r#"#!/usr/bin/env perl
use strict;
use warnings;
use Test::More;

# Test basic functionality
ok(1, "True is true");
is(2 + 2, 4, "Math works");

# Test string operations
my $str = "test";
like($str, qr/test/, "Regex matching works");

# Test array operations
my @arr = (1, 2, 3);
is(scalar(@arr), 3, "Array has correct size");

done_testing();
"#;

    fs::write(test_dir.join("test_suite.t"), test_suite)?;

    // Advanced features test
    let advanced_test = r#"#!/usr/bin/env perl
use strict;
use warnings;

# Inlay hints test
sub greet {
    my ($name, $greeting, $punctuation) = @_;
    return "$greeting, $name$punctuation";
}

# Call with positional arguments (should show parameter hints)
my $message = greet("World", "Hello", "!");

# Type hints test
my $scalar = "test";       # Should show: string
my @array = (1, 2, 3);     # Should show: array
my %hash = (a => 1);       # Should show: hash
my $ref = \@array;         # Should show: arrayref

# Complex call hierarchy
sub main {
    setup();
    process();
    cleanup();
}

sub setup {
    initialize_config();
    load_data();
}

sub process {
    validate_data();
    transform_data();
    save_results();
}

sub cleanup {
    close_handles();
    free_resources();
}

# Stub implementations
sub initialize_config { }
sub load_data { }
sub validate_data { }
sub transform_data { }
sub save_results { }
sub close_handles { }
sub free_resources { }

main();
"#;

    fs::write(test_dir.join("advanced_features.pl"), advanced_test)?;

    println!("✅ Created test files:");
    println!("   - test_features.pl (call hierarchy, basic tests)");
    println!("   - MyPackage.pm (workspace symbols)");
    println!("   - test_suite.t (Test Explorer)");
    println!("   - advanced_features.pl (inlay hints, complex hierarchy)");

    Ok(())
}

/// Run specific test
fn run_specific_test(test_dir: &Path, test_name: &str) -> Result<()> {
    match test_name {
        "syntax" => test_syntax_highlighting(test_dir),
        "hover" => test_hover_functionality(test_dir),
        "hierarchy" => test_call_hierarchy(test_dir),
        "inlay" => test_inlay_hints(test_dir),
        "test" => test_test_runner(test_dir),
        "symbols" => test_workspace_symbols(test_dir),
        "completion" => test_completion(test_dir),
        _ => bail!(
            "Unknown test: {}. Available tests: syntax, hover, hierarchy, inlay, test, symbols, completion",
            test_name
        ),
    }
}

/// Run all tests
fn run_all_tests(test_dir: &Path) -> Result<()> {
    println!("\n📋 Running all LSP feature tests...\n");

    test_syntax_highlighting(test_dir)?;
    test_hover_functionality(test_dir)?;
    test_call_hierarchy(test_dir)?;
    test_inlay_hints(test_dir)?;
    test_test_runner(test_dir)?;
    test_workspace_symbols(test_dir)?;
    test_completion(test_dir)?;

    println!("\n✅ All tests completed!");
    Ok(())
}

/// Test syntax highlighting
fn test_syntax_highlighting(_test_dir: &Path) -> Result<()> {
    println!("🎨 Testing syntax highlighting...");

    // Start the LSP server in stdio mode
    let mut child = Command::new("cargo")
        .args(["run", "-p", "perl-parser", "--bin", "perl-lsp", "--", "--stdio"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| eyre!("failed to start LSP server: {e}"))?;

    let mut writer = child.stdin.take().ok_or_else(|| eyre!("no stdin"))?;
    let mut reader = BufReader::new(child.stdout.take().ok_or_else(|| eyre!("no stdout"))?);

    // Helper to write a JSON-RPC message
    fn write_message(writer: &mut std::process::ChildStdin, value: &Value) -> Result<()> {
        let content = serde_json::to_string(value)?;
        write!(writer, "Content-Length: {}\r\n\r\n{}", content.len(), content)?;
        writer.flush()?;
        Ok(())
    }

    // Helper to read a JSON-RPC message
    fn read_message(reader: &mut BufReader<std::process::ChildStdout>) -> Result<Value> {
        let mut header = String::new();
        let mut content_length = None;
        loop {
            header.clear();
            reader.read_line(&mut header)?;
            if header == "\r\n" {
                break;
            }
            if header.to_ascii_lowercase().starts_with("content-length:") {
                let len = header.split(':').nth(1).unwrap().trim().parse()?;
                content_length = Some(len);
            }
        }
        let len = content_length.ok_or_else(|| eyre!("missing content length"))?;
        let mut buf = vec![0; len];
        reader.read_exact(&mut buf)?;
        Ok(serde_json::from_slice(&buf)?)
    }

    // Initialize server
    write_message(
        &mut writer,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {"capabilities": {}, "rootUri": null, "processId": std::process::id()}
        }),
    )?;
    read_message(&mut reader)?;
    write_message(&mut writer, &json!({"jsonrpc": "2.0", "method": "initialized", "params": {}}))?;

    // Open the fixture file
    let fixture = Path::new("crates/perl-parser/tests/fixtures/hover_test.pl");
    let text = fs::read_to_string(fixture)?;
    let uri = format!("file://{}", fixture.canonicalize()?.display());
    write_message(
        &mut writer,
        &json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": text
                }
            }
        }),
    )?;

    // Request semantic tokens
    write_message(
        &mut writer,
        &json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/semanticTokens/full",
            "params": {"textDocument": {"uri": uri}}
        }),
    )?;
    let response = read_message(&mut reader)?;
    let data = response["result"]["data"]
        .as_array()
        .ok_or_else(|| eyre!("no semantic token data"))?
        .iter()
        .map(|v| v.as_u64().unwrap() as u32)
        .collect::<Vec<_>>();

    if data != EXPECTED_TOKENS {
        bail!("semantic tokens differ: {:?} != {:?}", data, EXPECTED_TOKENS);
    }

    // Shut down server
    let _ = child.kill();
    println!("   ✓ Semantic tokens match expected values");
    Ok(())
}

/// Test hover functionality
fn test_hover_functionality(_test_dir: &Path) -> Result<()> {
    println!("💭 Testing hover functionality...");
    println!("   ℹ Hover over function names to see signatures");
    println!("   ℹ Hover over variables to see types");
    Ok(())
}

/// Test call hierarchy
fn test_call_hierarchy(_test_dir: &Path) -> Result<()> {
    println!("📞 Testing call hierarchy...");
    println!("   ℹ Right-click on 'process_data' and select 'Show Call Hierarchy'");
    println!("   ℹ Check advanced_features.pl for complex hierarchy");
    Ok(())
}

/// Test inlay hints
fn test_inlay_hints(_test_dir: &Path) -> Result<()> {
    println!("💡 Testing inlay hints...");
    println!("   ℹ Check advanced_features.pl for parameter and type hints");
    println!("   ℹ Configure via settings: perl.inlayHints.*");
    Ok(())
}

/// Test test runner
fn test_test_runner(test_dir: &Path) -> Result<()> {
    println!("🧪 Testing test runner...");

    // Run the actual test file
    let output =
        Command::new("perl").arg(test_dir.join("test_suite.t").to_str().unwrap()).output()?;

    if output.status.success() {
        println!("   ✓ Test file executes successfully");
        println!("   ℹ Open Testing panel in VSCode to see discovered tests");
    } else {
        println!("   ✗ Test file failed to execute");
    }

    Ok(())
}

/// Test workspace symbols
fn test_workspace_symbols(_test_dir: &Path) -> Result<()> {
    println!("🔍 Testing workspace symbols...");
    println!("   ℹ Press Ctrl+T (Cmd+T on Mac) and search for 'method'");
    println!("   ℹ Should find method_one and method_two from MyPackage.pm");
    Ok(())
}

/// Test completion
fn test_completion(_test_dir: &Path) -> Result<()> {
    println!("📝 Testing completion...");
    println!("   ℹ Type 'process' and see autocomplete suggestions");
    println!("   ℹ Should suggest process_data function");
    Ok(())
}

/// Print instructions for manual testing
fn print_instructions() {
    println!("\n📖 Manual Testing Instructions:");
    println!("\n1. Open VSCode in the test directory:");
    println!("   code lsp_test");
    println!("\n2. Install the Perl Language Server extension");
    println!("\n3. Features to test:");
    println!("   - Syntax highlighting (semantic tokens)");
    println!("   - Hover over functions to see signatures");
    println!("   - Right-click functions for 'Show Call Hierarchy'");
    println!("   - Check for inlay hints (parameter names)");
    println!("   - Open Testing panel for discovered tests");
    println!("   - Press Ctrl+T for workspace symbols");
    println!("   - Try code completion");
    println!("\n4. Configuration:");
    println!("   - Open settings (Ctrl+,) and search for 'perl'");
    println!("   - Adjust inlay hints, test runner settings, etc.");
}

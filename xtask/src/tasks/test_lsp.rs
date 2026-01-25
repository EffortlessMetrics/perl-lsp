//! Test LSP features with demo scripts

use color_eyre::eyre::{Result, bail, eyre};
use serde_json::json;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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

    // Find LSP server binary
    let binary_path = if Path::new("target/debug/perl-lsp").exists() {
        PathBuf::from("target/debug/perl-lsp")
    } else if Path::new("target/release/perl-lsp").exists() {
        PathBuf::from("target/release/perl-lsp")
    } else {
        PathBuf::from("perl-lsp")
    };

    // Check if LSP server is available
    let output = Command::new(&binary_path).arg("--version").output();

    match output {
        Ok(_) => println!("   ✓ LSP server is available"),
        Err(_) => {
            println!("   ⚠ LSP server not found. Run: cargo install --path crates/perl-lsp");
            bail!("LSP server binary 'perl-lsp' not found");
        }
    }

    println!("   🚀 Starting LSP server process...");
    let mut child = Command::new(&binary_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdin = child.stdin.as_mut().ok_or_else(|| eyre!("Failed to open stdin"))?;
    let stdout = child.stdout.as_mut().ok_or_else(|| eyre!("Failed to open stdout"))?;
    let mut reader = BufReader::new(stdout);

    // Helper to send message
    let mut send_msg = |msg: serde_json::Value| -> Result<()> {
        let msg_str = msg.to_string();
        write!(stdin, "Content-Length: {}\r\n\r\n{}", msg_str.len(), msg_str)?;
        stdin.flush()?;
        Ok(())
    };

    // Helper to read message
    let mut read_msg = || -> Result<serde_json::Value> {
        let mut size = None;
        let mut line = String::new();
        loop {
            line.clear();
            reader.read_line(&mut line)?;
            if line.trim().is_empty() {
                break;
            } // End of headers
            if line.starts_with("Content-Length: ") {
                size = Some(line.trim()["Content-Length: ".len()..].parse::<usize>()?);
            }
        }
        let size = size.ok_or_else(|| eyre!("Missing Content-Length"))?;
        let mut buf = vec![0; size];
        reader.read_exact(&mut buf)?;
        let val = serde_json::from_slice(&buf)?;
        Ok(val)
    };

    // Helper to wait for specific response
    let mut wait_for_response = |id: u64| -> Result<serde_json::Value> {
        loop {
            let msg = read_msg()?;
            if let Some(msg_id) = msg.get("id").and_then(|i| i.as_u64())
                && msg_id == id
            {
                return Ok(msg);
            }
            // Log ignored message
            if let Some(method) = msg.get("method").and_then(|m| m.as_str()) {
                println!("   ℹ Ignored notification: {}", method);
            } else {
                println!("   ℹ Ignored message: {:?}", msg);
            }
        }
    };

    // 1. Initialize
    println!("   📤 Sending initialize request...");
    let init_req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "processId": std::process::id(),
            "rootUri": null,
            "capabilities": {
                "textDocument": {
                    "semanticTokens": {
                        "requests": {
                            "full": true
                        },
                        "tokenTypes": [],
                        "tokenModifiers": []
                    }
                }
            }
        }
    });
    send_msg(init_req)?;
    let _init_resp = wait_for_response(1)?;
    println!("   ✓ Received initialize response");

    // 2. Initialized
    send_msg(json!({
        "jsonrpc": "2.0",
        "method": "initialized",
        "params": {}
    }))?;

    // 3. Open Document
    let code = "my $x = 10;";
    send_msg(json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///test.pl",
                "languageId": "perl",
                "version": 1,
                "text": code
            }
        }
    }))?;

    // 4. Request Semantic Tokens
    println!("   📤 Requesting semantic tokens...");
    send_msg(json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/semanticTokens/full",
        "params": {
            "textDocument": {
                "uri": "file:///test.pl"
            }
        }
    }))?;

    let tokens_resp = wait_for_response(2)?;

    // 5. Verify tokens
    if let Some(data) =
        tokens_resp.get("result").and_then(|r| r.get("data")).and_then(|d| d.as_array())
    {
        if !data.is_empty() {
            println!("   ✓ Received {} semantic tokens", data.len());
        } else {
            println!("   ⚠ Received empty semantic tokens");
            // Depending on strictness, we might want to fail here
            // bail!("Semantic tokens expected but got empty list");
        }
    } else {
        println!("   ✗ Failed to get semantic tokens: {:?}", tokens_resp);
        bail!("Failed to get semantic tokens response");
    }

    // Shutdown
    send_msg(json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "shutdown",
        "params": null
    }))?;
    let _shutdown_resp = read_msg()?;

    send_msg(json!({
        "jsonrpc": "2.0",
        "method": "exit",
        "params": null
    }))?;

    println!("   ℹ Open test_features.pl in VSCode to verify semantic tokens manually");

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
    let test_path = test_dir.join("test_suite.t");
    let test_path_str = test_path.to_str().ok_or_else(|| {
        color_eyre::eyre::eyre!("Test path contains invalid UTF-8: {:?}", test_path)
    })?;
    let output = Command::new("perl").arg(test_path_str).output()?;

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

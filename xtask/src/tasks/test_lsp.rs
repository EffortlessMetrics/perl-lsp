//! Test LSP features with demo scripts

use color_eyre::eyre::{bail, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

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

    // Check if LSP server is available
    let output = Command::new("perl-lsp").arg("--version").output();

    match output {
        Ok(_) => println!("   ✓ LSP server is available"),
        Err(_) => println!("   ⚠ LSP server not found. Run: cargo install --path crates/perl-lsp"),
    }

    // TODO: Add actual syntax highlighting test
    println!("   ℹ Open test_features.pl in VSCode to verify semantic tokens");

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

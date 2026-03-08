# ExecuteCommand Configuration and Troubleshooting Guide (*Diataxis: How-to Guide* - Problem-oriented executeCommand solutions)

*Comprehensive guide for configuring, troubleshooting, and optimizing executeCommand functionality in Perl LSP server. This guide provides solutions for common problems and advanced configuration scenarios.*

## Overview

This guide addresses specific problems you may encounter when using executeCommand functionality (Issue #145) in production environments. Each section provides step-by-step solutions for real-world challenges.

## Configuration Problems and Solutions

### Problem: Setting Up External Perlcritic

**Scenario**: You want to use external perlcritic for enhanced policy coverage but encounter installation or path issues.

**Solution Steps**:

1. **Install perlcritic using your system package manager**:
```bash
# Ubuntu/Debian
sudo apt-get update
sudo apt-get install perlcritic

# CentOS/RHEL/Fedora
sudo dnf install perl-Perl-Critic  # or sudo yum install perl-Perl-Critic

# macOS with Homebrew
brew install perl-critic

# Arch Linux
sudo pacman -S perl-critic
```

2. **Alternative: Install via CPAN**:
```bash
# Install via CPAN (works on all platforms)
cpan Perl::Critic

# Or using cpanminus
cpanm Perl::Critic

# For system-wide installation
sudo cpan Perl::Critic
```

3. **Verify installation and PATH**:
```bash
# Check installation
which perlcritic
perlcritic --version

# Test on sample file
echo 'print "hello"' > /tmp/test.pl
perlcritic /tmp/test.pl
rm /tmp/test.pl
```

4. **Configure PATH if needed**:
```bash
# Find perlcritic location
find /usr -name perlcritic 2>/dev/null
find /opt -name perlcritic 2>/dev/null
find $HOME -name perlcritic 2>/dev/null

# Add to PATH in your shell profile
echo 'export PATH="/usr/local/bin:$PATH"' >> ~/.bashrc  # or ~/.zshrc
source ~/.bashrc
```

5. **Test LSP integration**:
```bash
# Verify LSP server can find perlcritic
cargo test -p perl-parser --test execute_command_tests -- test_command_exists_behavior
```

### Problem: Configuring Perlcritic Policies

**Scenario**: You want to customize which policies perlcritic checks and their severity levels.

**Solution Steps**:

1. **Create a .perlcriticrc configuration file**:
```bash
# Create in your project root or home directory
cat > ~/.perlcriticrc << 'EOF'
# Perlcritic configuration for LSP integration
severity = 3
theme = core
verbose = %f:%l:%c: %m. %e. (%p)\n

# Enable/disable specific policies
[BuiltinFunctions::ProhibitVoidGrep]
severity = 5

[InputOutput::RequireCheckedSyscalls]
functions = :builtins

[Variables::ProhibitPunctuationVars]
severity = 4
EOF
```

2. **Test configuration**:
```bash
# Test perlcritic with custom config
perlcritic --profile ~/.perlcriticrc /path/to/test.pl

# Verify different severity levels
perlcritic --severity 1 /path/to/test.pl  # Show all
perlcritic --severity 5 /path/to/test.pl  # Show only brutal
```

3. **Project-specific configuration**:
```bash
# Create project-specific .perlcriticrc
cat > .perlcriticrc << 'EOF'
# Project-specific perlcritic settings
severity = 2
theme = bugs + maintenance + complexity

# Disable specific policies for this project
[-Subroutines::ProhibitExplicitReturnUndef]
[-InputOutput::RequireBriefOpen]
EOF
```

4. **Validate LSP integration with custom config**:
```bash
# Test that LSP server respects configuration
cargo test -p perl-lsp --test lsp_execute_command_tests -- test_perlcritic_dual_analyzer
```

### Problem: Performance Optimization

**Scenario**: perl.runCritic is too slow for large files or causes LSP timeouts.

**Solution Steps**:

1. **Identify performance bottlenecks**:
```bash
# Test file size impact
ls -lh your_large_file.pl
wc -l your_large_file.pl

# Time external perlcritic directly
time perlcritic your_large_file.pl

# Compare with built-in analyzer
cargo test -p perl-parser --test execute_command_tests -- test_run_builtin_critic_with_valid_file
```

2. **Optimize perlcritic configuration for speed**:
```bash
cat > .perlcriticrc << 'EOF'
# Performance-optimized configuration
severity = 4          # Check only important and brutal issues
theme = core          # Focus on core policies
verbose = %f:%l:%c: %m\n  # Minimal output format

# Disable slow policies
[-Documentation::RequirePodSections]
[-Miscellanea::RequireRcsKeywords]
[-BuiltinFunctions::ProhibitComplexMappings]
EOF
```

3. **Use built-in analyzer for large files**:
```bash
# Built-in analyzer is optimized for speed
# Test performance characteristics
cargo test -p perl-parser --test execute_command_tests -- test_run_builtin_critic_arithmetic_mutations

# Validate <300ms performance target for files <100KB
cargo test -p perl-lsp --test lsp_performance_tests -- test_execute_command_latency
```

4. **Configure LSP timeouts**:
```bash
# Test with extended timeouts for large files
LSP_TEST_TIMEOUT_MS=10000 cargo test -p perl-lsp --test lsp_execute_command_tests

# Use adaptive threading for better performance
RUST_TEST_THREADS=2 cargo test -p perl-lsp -- --test-threads=2
```

## Troubleshooting Common Issues

### Problem: "perlcritic not found" Error

**Scenario**: LSP server always uses built-in analyzer despite perlcritic being installed.

**Diagnostic Steps**:

1. **Verify perlcritic in PATH**:
```bash
# Check current PATH
echo $PATH

# Test perlcritic accessibility
which perlcritic || echo "perlcritic not found in PATH"

# Test from same environment as LSP server
env PATH="$PATH" which perlcritic
```

2. **Check LSP server environment**:
```bash
# Start LSP server with environment logging
env PATH="$PATH" perl-lsp --stdio --log

# Test command detection logic
cargo test -p perl-parser --test execute_command_tests -- test_command_exists_behavior
```

3. **Debug command existence check**:
```bash
# Test the exact command existence logic
echo '#!/bin/bash
command -v perlcritic && echo "Found via command -v"
which perlcritic && echo "Found via which"
type perlcritic && echo "Found via type"' > /tmp/debug_perlcritic.sh
chmod +x /tmp/debug_perlcritic.sh
/tmp/debug_perlcritic.sh
```

**Solutions**:

- **Fix PATH**: Ensure perlcritic location is in PATH before starting LSP server
- **Symlink**: Create symlink in standard location: `sudo ln -s /full/path/to/perlcritic /usr/local/bin/`
- **Use built-in**: Built-in analyzer provides reliable alternative with core policies

### Problem: Analysis Results Missing or Incomplete

**Scenario**: perl.runCritic returns no violations for file with obvious issues.

**Diagnostic Steps**:

1. **Test file syntax**:
```bash
# Check if file has syntax errors
perl -c your_file.pl

# Test with minimal example
echo '#!/usr/bin/perl
my $var = 42;
print $var;' > /tmp/test_minimal.pl

# Run analysis on minimal file
cargo test -p perl-parser --test execute_command_tests -- test_execute_command_run_critic_builtin
```

2. **Verify policy configuration**:
```bash
# Test with default configuration
perlcritic --noprofile /tmp/test_minimal.pl

# Check current configuration
perlcritic --profile-strictness /tmp/test_minimal.pl
```

3. **Test built-in analyzer specifically**:
```bash
# Test built-in analyzer detection
cargo test -p perl-parser --test execute_command_tests -- test_execute_command_run_critic_builtin

# Verify specific policy detection
cargo test -p perl-parser --test execute_command_tests -- test_format_violation_structure
```

**Solutions**:

- **Fix syntax errors**: Address Perl syntax issues before running analysis
- **Adjust severity**: Lower perlcritic severity threshold to show more issues
- **Check configuration**: Ensure .perlcriticrc doesn't disable relevant policies

### Problem: LSP Client Integration Issues

**Scenario**: executeCommand doesn't appear in your editor or returns errors.

**Diagnostic Steps**:

1. **Verify server capabilities**:
```bash
# Test capability advertisement
cargo test -p perl-lsp --test lsp_behavioral_tests -- test_execute_command_capabilities

# Manual LSP protocol test
echo -e 'Content-Length: 58\r\n\r\n{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | perl-lsp --stdio
```

2. **Test JSON-RPC protocol directly**:
```bash
# Create test request
cat > /tmp/test_request.json << 'EOF'
Content-Length: 140

{"jsonrpc":"2.0","id":1,"method":"workspace/executeCommand","params":{"command":"perl.runCritic","arguments":["/tmp/test.pl"]}}
EOF

# Test with LSP server
perl-lsp --stdio < /tmp/test_request.json
```

3. **Check editor-specific integration**:

**VSCode**:
- Verify Perl extension is installed and enabled
- Check Output panel for LSP errors
- Restart LSP server via Command Palette

**Neovim**:
```lua
-- Debug LSP client status
:lua print(vim.inspect(vim.lsp.get_active_clients()))

-- Test command execution directly
:lua vim.lsp.buf.execute_command({command="perl.runCritic", arguments={vim.fn.expand("%:p")}})
```

**Emacs**:
```elisp
;; Check LSP server status
(lsp-describe-session)

;; Test command execution
(lsp-execute-command "perl.runCritic" (list (buffer-file-name)))
```

### Problem: Permission and Security Issues

**Scenario**: executeCommand fails due to file permissions or security restrictions.

**Diagnostic Steps**:

1. **Check file permissions**:
```bash
# Verify file is readable
ls -la your_file.pl

# Test file access
cat your_file.pl > /dev/null && echo "File readable" || echo "File not readable"
```

2. **Test with known-good file**:
```bash
# Create test file with proper permissions
echo '#!/usr/bin/perl
use strict;
use warnings;
print "hello world\n";' > /tmp/test_permissions.pl
chmod 644 /tmp/test_permissions.pl

# Test analysis
cargo test -p perl-parser --test execute_command_tests -- test_execute_command_run_critic_builtin
```

3. **Validate path normalization**:
```bash
# Test URI path handling
cargo test -p perl-parser --test execute_command_tests -- test_normalize_file_path_uri_handling

# Test path traversal prevention
cargo test -p perl-parser --test file_completion_tests -- basic_security_test_rejects_path_traversal
```

**Solutions**:

- **Fix permissions**: `chmod 644 your_file.pl`
- **Use absolute paths**: Avoid relative paths that might resolve incorrectly
- **Verify workspace security**: Ensure LSP server has access to project directories

## Advanced Configuration Scenarios

### Scenario: CI/CD Integration

**Problem**: Need to run perl.runCritic in automated CI/CD pipelines.

**Solution**:

1. **Docker container setup**:
```dockerfile
# Dockerfile for Perl LSP with perlcritic
FROM rust:latest
RUN apt-get update && apt-get install -y perlcritic perl
COPY . /workspace
WORKDIR /workspace
RUN cargo build --release -p perl-lsp
CMD ["./target/release/perl-lsp", "--stdio"]
```

2. **GitHub Actions integration**:
```yaml
name: Perl Quality Analysis
on: [push, pull_request]

jobs:
  analyze:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v3
    - uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
    - name: Install perlcritic
      run: sudo apt-get install -y perlcritic
    - name: Test executeCommand
      run: |
        RUST_TEST_THREADS=2 cargo test -p perl-lsp --test lsp_execute_command_tests -- --test-threads=2
```

3. **Jenkins pipeline**:
```groovy
pipeline {
    agent any
    stages {
        stage('Setup') {
            steps {
                sh 'apt-get update && apt-get install -y perlcritic'
            }
        }
        stage('Analyze') {
            steps {
                sh 'cargo test -p perl-lsp --test lsp_execute_command_tests'
            }
        }
    }
}
```

### Scenario: Multiple Project Configuration

**Problem**: Different projects need different perlcritic configurations.

**Solution**:

1. **Project-specific .perlcriticrc files**:
```bash
# Legacy project - lenient settings
cat > legacy_project/.perlcriticrc << 'EOF'
severity = 4
theme = core
[-Subroutines::RequireExplicitReturnUndef]
[-Variables::ProhibitPunctuationVars]
EOF

# New project - strict settings
cat > new_project/.perlcriticrc << 'EOF'
severity = 2
theme = core + bugs + maintenance
verbose = %f:%l:%c: [%p] %m\n
EOF
```

2. **Environment-based configuration**:
```bash
# Development environment
export PERLCRITIC_PROFILE="$HOME/.perlcriticrc.dev"

# Production environment
export PERLCRITIC_PROFILE="$HOME/.perlcriticrc.prod"

# Test configuration switching
perlcritic --profile "$PERLCRITIC_PROFILE" test_file.pl
```

### Scenario: Custom Policy Integration

**Problem**: Need to integrate custom Perl::Critic policies with executeCommand.

**Solution**:

1. **Install custom policies**:
```bash
# Install custom policy modules
cpanm Perl::Critic::Policy::Custom::MyPolicy

# Verify installation
perl -MPerl::Critic::Policy::Custom::MyPolicy -e 'print "Custom policy loaded\n"'
```

2. **Configure custom policies**:
```bash
cat > .perlcriticrc << 'EOF'
# Custom policy configuration
[Custom::MyPolicy]
severity = 3
enabled = 1
custom_option = value

# Built-in policies
severity = 3
theme = core + custom
EOF
```

3. **Test integration**:
```bash
# Test custom policies
perlcritic --profile .perlcriticrc test_file.pl

# Validate LSP integration
cargo test -p perl-lsp --test lsp_execute_command_tests -- test_perlcritic_dual_analyzer
```

## Monitoring and Maintenance

### Performance Monitoring

**Set up performance monitoring**:

1. **Enable performance metrics**:
```bash
# Test performance benchmarks
cargo test -p perl-lsp --test lsp_performance_tests -- test_execute_command_latency

# Monitor memory usage
cargo test -p perl-parser --test execute_command_tests -- test_run_builtin_critic_arithmetic_mutations
```

2. **Log analysis timing**:
```bash
# Enable detailed logging
RUST_LOG=debug cargo test -p perl-lsp --test lsp_execute_command_tests -- --nocapture

# Monitor specific timing thresholds
LSP_TEST_TIMEOUT_MS=20000 cargo test -p perl-lsp
```

### Health Checks

**Regular validation procedures**:

1. **Automated health check script**:
```bash
#!/bin/bash
# health_check_executecommand.sh

echo "Testing executeCommand health..."

# Test 1: Basic functionality
if cargo test -p perl-parser --test execute_command_tests --quiet; then
    echo "✅ Basic executeCommand: PASS"
else
    echo "❌ Basic executeCommand: FAIL"
    exit 1
fi

# Test 2: Performance validation
if cargo test -p perl-lsp --test lsp_performance_tests --quiet; then
    echo "✅ Performance validation: PASS"
else
    echo "❌ Performance validation: FAIL"
    exit 1
fi

# Test 3: Integration tests
if RUST_TEST_THREADS=2 cargo test -p perl-lsp --test lsp_execute_command_tests --quiet; then
    echo "✅ Integration tests: PASS"
else
    echo "❌ Integration tests: FAIL"
    exit 1
fi

echo "All executeCommand health checks passed!"
```

2. **Cron job for regular validation**:
```bash
# Add to crontab for daily validation
0 2 * * * /path/to/health_check_executecommand.sh >> /var/log/perl_lsp_health.log 2>&1
```

## Summary

This guide provides comprehensive solutions for:

✅ **Installation and Configuration**: Setting up external perlcritic with custom policies
✅ **Performance Optimization**: Tuning for large files and production environments
✅ **Troubleshooting**: Diagnosing and fixing common issues
✅ **Security**: Handling permissions and path validation
✅ **CI/CD Integration**: Automated quality analysis in build pipelines
✅ **Advanced Scenarios**: Multi-project and custom policy configurations
✅ **Monitoring**: Health checks and performance validation

The executeCommand functionality provides robust code quality analysis with comprehensive configuration options and reliability patterns.
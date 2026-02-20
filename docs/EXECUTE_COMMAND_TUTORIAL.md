# ExecuteCommand Tutorial (*Diataxis: Tutorial* - Learning-oriented executeCommand guide)

*Get started with the new executeCommand functionality in Perl LSP server. This tutorial walks you through using perl.runCritic and other commands to enhance your Perl development workflow.*

## Overview

The `workspace/executeCommand` LSP method (Issue #145) adds powerful command execution capabilities to the Perl LSP server. This tutorial teaches you how to use these commands effectively in your development environment.

### What You'll Learn

- How to set up and use `perl.runCritic` for code quality analysis
- Understanding the dual analyzer strategy (external + built-in)
- Integrating executeCommand with your LSP editor workflow
- Performance optimization and troubleshooting techniques
- Testing and validating executeCommand functionality

### Prerequisites

- Perl LSP server v0.8.8+ installed and running
- Basic understanding of LSP protocol and your LSP-compatible editor
- Perl development environment with test files

### Time to Complete

Approximately 30-45 minutes for complete walkthrough with all examples.

## Getting Started with perl.runCritic

### Step 1: Verify LSP Server Capabilities

First, ensure your LSP server advertises executeCommand capabilities:

```bash
# Test that executeCommand is supported
cargo test -p perl-lsp --test lsp_behavioral_tests -- test_execute_command_capabilities

# Verify perl.runCritic is in supported commands list
cargo test -p perl-parser --test execute_command_tests -- test_supported_commands_includes_run_critic
```

**Expected Output**: Tests should pass, confirming executeCommand support.

### Step 2: Create a Sample Perl File

Create a test file to analyze:

```perl
#!/usr/bin/perl
# File: /tmp/sample_analysis.pl

my $name = "Alice";
my $age = 30;

print "Hello $name, you are $age years old\n";

sub greet {
    my ($person) = @_;
    print "Greetings, $person!\n";
}

greet($name);
```

**Learning Goal**: This file intentionally lacks `use strict` and `use warnings` pragmas that perl.runCritic will detect.

### Step 3: Test Built-in Analyzer

Run the built-in analyzer (always available):

```bash
# Test built-in analyzer functionality
cargo test -p perl-parser --test execute_command_tests -- test_execute_command_run_critic_builtin

# The test creates a similar file and validates policy detection
```

**What Happens**:
- Built-in analyzer detects missing `use strict` and `use warnings`
- Analysis completes in ~100ms for typical files
- Returns structured JSON with violations, line numbers, and explanations

**Example Response Structure**:
```json
{
  "status": "success",
  "violations": [
    {
      "policy": "RequireUseStrict",
      "description": "Missing 'use strict' pragma",
      "explanation": "Always use 'use strict' to catch common errors",
      "severity": 3,
      "line": 1,
      "column": 1,
      "file": "/tmp/sample_analysis.pl"
    },
    {
      "policy": "RequireUseWarnings",
      "description": "Missing 'use warnings' pragma",
      "explanation": "Always use 'use warnings' to catch potential issues",
      "severity": 3,
      "line": 1,
      "column": 1,
      "file": "/tmp/sample_analysis.pl"
    }
  ],
  "violationCount": 2,
  "analyzerUsed": "builtin"
}
```

### Step 4: Understanding the Dual Analyzer Strategy

The perl.runCritic command implements a sophisticated dual strategy:

1. **Primary Path**: External perlcritic (if installed)
2. **Fallback Path**: Built-in analyzer (always available)
3. **Seamless Transition**: Automatic fallback with no user intervention

**Test the Strategy**:
```bash
# Test dual analyzer behavior
cargo test -p perl-lsp --test lsp_execute_command_tests -- test_perlcritic_dual_analyzer
```

**Learning Goal**: Understand that you always get analysis results, regardless of external tool availability.

### Step 5: Install External Perlcritic (Optional Enhancement)

For more comprehensive analysis, install external perlcritic:

```bash
# Ubuntu/Debian
sudo apt-get install perlcritic

# macOS
brew install perl-critic

# Or via CPAN
cpan Perl::Critic

# Verify installation
which perlcritic
perlcritic --version
```

**Benefits of External Perlcritic**:
- 150+ policy checks (vs basic policies in built-in)
- Configurable severity levels
- Custom policy configuration support
- Industry-standard Perl best practices

### Step 6: LSP Protocol Integration

#### Manual LSP Request (Advanced)

You can manually test the LSP protocol integration:

```json
// Send this via your LSP client or testing tool
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "workspace/executeCommand",
  "params": {
    "command": "perl.runCritic",
    "arguments": ["/tmp/sample_analysis.pl"]
  }
}
```

#### Editor Integration Examples

**VSCode**: Commands appear in Command Palette (`Ctrl+Shift+P`):
- "Perl: Run Critic Analysis"
- "Perl: Run Tests"
- "Perl: Run File"

**Neovim with nvim-lspconfig**:
```lua
-- Add to your Neovim configuration
vim.api.nvim_create_user_command('PerlCritic', function()
  local file_path = vim.fn.expand('%:p')
  vim.lsp.buf.execute_command({
    command = 'perl.runCritic',
    arguments = { file_path }
  })
end, {})
```

**Emacs with lsp-mode**:
```elisp
;; Add to your Emacs configuration
(defun perl-run-critic ()
  "Run perl.runCritic on current file"
  (interactive)
  (lsp-execute-command "perl.runCritic" (list (buffer-file-name))))
```

## Working with Analysis Results

### Step 7: Understanding Violations

Each violation includes key information:

- **Policy**: The rule that was violated (e.g., "RequireUseStrict")
- **Severity**: Numerical severity (1-5, with 5 being most severe)
- **Description**: Human-readable description of the issue
- **Explanation**: Detailed explanation and fix guidance
- **Location**: Precise line and column numbers
- **File**: Full file path for multi-file analysis

### Step 8: Fix Common Issues

Based on the sample file, make these improvements:

```perl
#!/usr/bin/perl
use strict;      # Added: addresses RequireUseStrict
use warnings;    # Added: addresses RequireUseWarnings

my $name = "Alice";
my $age = 30;

print "Hello $name, you are $age years old\n";

sub greet {
    my ($person) = @_;
    print "Greetings, $person!\n";
    return;      # Added: good practice for explicit return
}

greet($name);
```

### Step 9: Re-analyze the Fixed File

Save your fixes and re-run analysis:

```bash
# Test with a clean file (should have fewer violations)
echo '#!/usr/bin/perl
use strict;
use warnings;

my $name = "Alice";
print "Hello $name!\n";' > /tmp/clean_sample.pl

# The built-in analyzer should show fewer violations
cargo test -p perl-parser --test execute_command_tests -- test_execute_command_run_critic_builtin
```

## Performance and Reliability

### Step 10: Performance Characteristics

Understanding timing expectations:

| File Size | Built-in Analyzer | External Perlcritic | Notes |
|-----------|-------------------|---------------------|-------|
| <1KB      | ~50ms            | ~200ms              | Typical small scripts |
| 1-10KB    | ~100ms           | ~500ms              | Standard modules |
| 10-100KB  | ~300ms           | ~1.5s               | Large applications |

### Step 11: Testing Performance

```bash
# Validate performance targets
cargo test -p perl-lsp --test lsp_performance_tests -- test_execute_command_latency

# Test with adaptive threading (recommended for CI)
RUST_TEST_THREADS=2 cargo test -p perl-lsp --test lsp_execute_command_tests -- --test-threads=2
```

### Step 12: Error Handling

Test error scenarios:

```bash
# Test with non-existent file
cargo test -p perl-parser --test execute_command_tests -- test_execute_command_run_critic_missing_file

# Test parameter validation
cargo test -p perl-parser --test execute_command_tests -- test_parameter_validation_missing_file_path
```

**Learning Goal**: The system handles errors gracefully with informative messages.

## Integration with Code Actions

### Step 13: Combining with Code Actions

The executeCommand workflow integrates with textDocument/codeAction for complete development support:

```bash
# Test integrated workflow
cargo test -p perl-lsp --test lsp_comprehensive_e2e_test -- test_execute_command_and_code_actions

# Test specific code action integration
cargo test -p perl-lsp --test lsp_code_actions_tests -- test_modernize_code_actions
```

**Workflow**:
1. **Execute** perl.runCritic to find issues
2. **Analyze** results in diagnostics
3. **Apply** code actions to fix issues
4. **Re-execute** to verify fixes

## Advanced Usage

### Step 14: Other executeCommand Operations

Explore the full command set:

```bash
# perl.runTests - Execute Perl test files
cargo test -p perl-lsp --test lsp_behavioral_tests -- test_execute_command_run_tests

# perl.runFile - Execute single Perl file
cargo test -p perl-lsp --test lsp_behavioral_tests -- test_execute_command_run_file

# perl.debugTests - Debug preparation
cargo test -p perl-lsp --test lsp_behavioral_tests -- test_execute_command_debug_tests
```

### Step 15: Quality Assurance Validation

Validate your setup meets all acceptance criteria:

```bash
# AC1: Complete executeCommand LSP method implementation
cargo test -p perl-lsp --test lsp_execute_command_tests -- test_ac1_execute_command_implementation

# AC2: perl.runCritic command integration
cargo test -p perl-lsp --test lsp_execute_command_tests -- test_ac2_perlcritic_integration

# AC3: Advanced refactoring operations
cargo test -p perl-lsp --test lsp_code_actions_tests -- test_ac3_advanced_refactoring_operations
```

## Troubleshooting Common Issues

### Issue: executeCommand Not Available

**Problem**: Editor doesn't show perl.runCritic command
**Solution**:
```bash
# Verify server capabilities
cargo test -p perl-lsp --test lsp_behavioral_tests -- test_execute_command_capabilities

# Check LSP server logs for capability advertisement
perl-lsp --stdio --log
```

### Issue: Analysis Takes Too Long

**Problem**: perl.runCritic timeout or slow response
**Solutions**:
- Check file size: `wc -l your_file.pl`
- Test external tool directly: `time perlcritic your_file.pl`
- Use built-in analyzer for faster results

### Issue: No Violations Found

**Problem**: Clean code shows no policy violations
**Expected**: This is correct behavior! Clean code should have minimal violations.
**Verify**: Test with a file missing `use strict` to confirm detection works.

## Next Steps

### Recommended Learning Path

1. **Explore Code Actions**: Learn about RefactorExtract and SourceOrganizeImports
2. **Cross-file Analysis**: Try executeCommand with multi-file projects
3. **Custom Workflows**: Integrate with your build and CI systems
4. **Performance Tuning**: Optimize for large codebases

### Further Reading

- [LSP Implementation Guide](/docs/LSP_IMPLEMENTATION_GUIDE.md) - Complete LSP feature reference
- [Commands Reference](/docs/COMMANDS_REFERENCE.md) - All command specifications
- [LSP Development Guide](/docs/LSP_DEVELOPMENT_GUIDE.md) - Advanced workflow integration

### Community and Support

- Report issues with executeCommand functionality
- Contribute policy improvements to built-in analyzer
- Share integration examples for other editors

## Summary

You've successfully learned how to:

✅ Set up and use perl.runCritic for code quality analysis
✅ Understand dual analyzer strategy (external + built-in fallback)
✅ Integrate executeCommand with your LSP development workflow
✅ Handle errors and troubleshoot common issues
✅ Validate performance and reliability characteristics
✅ Combine with code actions for complete development support

The executeCommand functionality elevates Perl LSP server capabilities from ~89% to ~91% functional coverage, providing development tools while maintaining the performance improvements of the Perl LSP ecosystem.

**Total Tutorial Time**: ~30-45 minutes for complete walkthrough
**Key Achievement**: Comprehensive understanding of executeCommand integration and practical usage patterns
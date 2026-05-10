# ExecuteCommand Configuration and Troubleshooting Guide (*Diataxis: How-to Guide* - Problem-oriented executeCommand solutions)

*Practical guidance for configuring, troubleshooting, and validating `workspace/executeCommand` operations in perl-lsp.*

## Overview

`workspace/executeCommand` exposes server-side commands such as `perl.runTests`,
`perl.runFile`, `perl.runTestSub`, `perl.debugTests`, and `perl.runCritic`.
`perl.runCritic` now uses the native critic path by default. The native path
shares perl-lsp parser, semantic, diagnostic, suppression, severity, code-action,
and receipt surfaces with normal editor diagnostics.

External `perlcritic` remains available only as an explicit compatibility
adapter for migration, comparison, or custom policy stacks that are not native
yet. You do not need to install `perlcritic` for the default native critic
workflow.

## Configuration Problems and Solutions

### Problem: Selecting the Critic Engine

**Scenario**: You want to confirm whether `perl.runCritic` uses the native critic
or the explicit legacy adapter.

**Solution Steps**:

1. **Use the native engine for normal development**:

```toml
[critic]
engine = "native"
profile = "recommended"
```

2. **Select explicit external compatibility only when needed**:

```toml
[critic]
engine = "legacy"
```

3. **Verify the native default policy guard**:

```bash
cargo xtask native-tooling check-defaults
```

This check confirms native critic defaults do not silently shell out to
`perlcritic`.

### Problem: Configuring Native Critic Rules

**Scenario**: You want to tune which native critic rules run and how findings are
reported.

**Solution Steps**:

1. **Set a profile and severity floor**:

```toml
[critic]
engine = "native"
profile = "recommended"

[diagnostics]
perlcritic = true
perlcritic_severity = 3
```

2. **Use profiles for normal rule selection**:

```toml
[critic]
engine = "native"
profile = "recommended" # or "strict"
```

3. **Use compatibility reports for existing include/exclude lists**:

```bash
cargo xtask native-tooling perlcritic-compat \
  --profile .perlcriticrc \
  --receipt target/receipts/native-tooling/perlcritic-compat.json \
  --summary target/receipts/native-tooling/perlcritic-compat.md
```

4. **Suppress intentional findings inline**:

```perl
## no perl-lsp-critic native.variables.unused_lexical -- kept for API parity
my $unused;
```

Native suppressions are structured: they preserve the rule ID, scope, line, and
optional reason so diagnostics, code actions, and receipts agree.

### Problem: Understanding `.perlcriticrc` Compatibility

**Scenario**: You already have a `.perlcriticrc` and need to know what the native
critic can cover.

**Solution Steps**:

1. **Generate a compatibility receipt**:

```bash
cargo xtask native-tooling perlcritic-compat \
  --profile .perlcriticrc \
  --receipt target/receipts/native-tooling/perlcritic-compat.json \
  --summary target/receipts/native-tooling/perlcritic-compat.md
```

2. **Review classifications**:

- `native-equivalent` - native rule covers the Perl::Critic policy directly
- `native-superset` - native rule covers the policy and adds parser/LSP context
- `approximated` - native behavior is close but not identical
- `unsupported-safe` - unsupported policy has no required runtime effect
- `external-only` - keep explicit legacy compatibility for this policy

3. **Check dashboard status**:

```bash
cargo xtask native-tooling status --markdown docs/project/status/native_tooling.md
```

The dashboard rolls formatter, critic, and compatibility receipt counts into a
single status surface.

### Problem: Performance Optimization

**Scenario**: `perl.runCritic` is slow for large files or causes editor timeouts.

**Solution Steps**:

1. **Prefer native critic for editor workflows**:

```bash
cargo xtask native-tooling check-defaults
```

The native path avoids process startup and parser-output translation overhead
from the legacy adapter.

2. **Use focused runtime tests for diagnostics behavior**:

```bash
cargo test -p perl-lsp-rs native_critic_engine --profile agent --locked --lib -- --nocapture
cargo test -p perl-lsp-rs diagnostics --profile agent --locked --lib -- --nocapture
```

3. **Use fewer rules for large or noisy projects**:

```toml
[critic]
engine = "native"
profile = "recommended"

[diagnostics]
perlcritic_severity = 4
```

4. **Keep legacy compatibility out of the editor hot path**:

Use `engine = "legacy"` only for migration checks, custom Perl::Critic
policy stacks, or parity investigations.

## Troubleshooting Common Issues

### Problem: `perlcritic` Is Missing

**Scenario**: You selected external legacy compatibility and the command fails
because `perlcritic` is not available.

**Diagnostic Steps**:

```bash
which perlcritic || echo "perlcritic not found in PATH"
perlcritic --version
env PATH="$PATH" which perlcritic
```

**Solutions**:

- Prefer native critic by removing the explicit external legacy setting.
- Install `perlcritic` only if you intentionally need compatibility mode.
- Ensure the editor process inherits the same `PATH` that can find `perlcritic`.

Example compatibility install commands:

```bash
# Ubuntu/Debian
sudo apt-get update
sudo apt-get install perlcritic

# macOS with Homebrew
brew install perl-critic

# CPAN
cpan Perl::Critic
```

### Problem: Analysis Results Are Missing or Incomplete

**Scenario**: `perl.runCritic` returns no findings for code where you expected a
diagnostic.

**Diagnostic Steps**:

1. **Check the source parses cleanly enough for native analysis**:

```bash
perl -c your_file.pl
```

2. **Confirm the active critic profile and filters**:

```toml
[critic]
engine = "native"
profile = "recommended"
```

3. **Check whether the rule is below the severity floor or outside the selected profile**:

```toml
[critic]
profile = "strict"

[diagnostics]
perlcritic_severity = 1
```

4. **Run focused diagnostics tests when changing the implementation**:

```bash
cargo test -p perl-lsp-rs-core native_critic --profile agent --locked -- --nocapture
cargo test -p perl-lsp-rs native_critic_engine --profile agent --locked --lib -- --nocapture
```

**Solutions**:

- Fix syntax errors before analysis.
- Lower the severity floor if you intentionally want lower-severity findings.
- Remove accidental excludes.
- Generate a compatibility report if you expected a Perl::Critic policy that has
  not been mapped to a native rule yet.

### Problem: Suppressions Do Not Apply

**Scenario**: A native critic diagnostic remains visible after adding a
suppression comment.

**Diagnostic Steps**:

1. **Use the native rule ID exactly**:

```perl
## no critic native.security.two_arg_open -- legacy open form is deliberate
open(FH, $path);
```

or:

```perl
## no perl-lsp-critic native.security.two_arg_open -- migration shim
open(FH, $path);
```

2. **Verify the suppression is in the intended scope**:

Line-local suppressions apply to nearby findings; file- or block-wide behavior
depends on the suppression form supported by the rule path.

3. **Run focused suppression tests when changing rule behavior**:

```bash
cargo test -p perl-lsp-rs-core native_critic --profile agent --locked -- --nocapture
```

**Solutions**:

- Use the stable native rule ID from the diagnostic.
- Keep suppression comments close to the relevant code.
- Include a reason so future triage can distinguish intentional debt from a
  stale suppression.

### Problem: LSP Client Integration Issues

**Scenario**: `workspace/executeCommand` does not appear in your editor or
returns errors.

**Diagnostic Steps**:

1. **Verify server capabilities**:

```bash
cargo test -p perl-lsp-rs --test lsp_behavioral_tests -- test_execute_command_capabilities
```

2. **Test JSON-RPC protocol directly**:

```bash
cat > /tmp/test_request.json << 'EOF'
Content-Length: 140

{"jsonrpc":"2.0","id":1,"method":"workspace/executeCommand","params":{"command":"perl.runCritic","arguments":["/tmp/test.pl"]}}
EOF

perllsp --stdio < /tmp/test_request.json
```

3. **Check editor-specific integration**:

**VS Code**:

- Verify the Perl extension is installed and enabled.
- Check the Output panel for LSP errors.
- Restart the LSP server from the Command Palette.

**Neovim**:

```lua
:lua print(vim.inspect(vim.lsp.get_active_clients()))
:lua vim.lsp.buf.execute_command({command="perl.runCritic", arguments={vim.fn.expand("%:p")}})
```

**Emacs**:

```elisp
(lsp-describe-session)
(lsp-execute-command "perl.runCritic" (list (buffer-file-name)))
```

### Problem: Permission and Security Issues

**Scenario**: `executeCommand` fails due to file permissions or path security
restrictions.

**Diagnostic Steps**:

```bash
ls -la your_file.pl
cat your_file.pl > /dev/null && echo "File readable" || echo "File not readable"
cargo test -p perl-parser --test file_completion_tests -- basic_security_test_rejects_path_traversal
```

**Solutions**:

- Fix permissions with `chmod 644 your_file.pl`.
- Use absolute paths when testing direct command calls.
- Verify the LSP server has workspace access to the target project.

## Advanced Configuration Scenarios

### Scenario: CI/CD Integration

**Problem**: You want automated quality analysis that matches editor behavior.

**Solution**:

1. **Use native critic proof commands in CI**:

```yaml
name: Perl Native Critic
on: [push, pull_request]

jobs:
  analyze:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
    - name: Native critic diagnostics
      run: |
        cargo test -p perl-lsp-rs-core native_critic --profile agent --locked -- --nocapture
        cargo test -p perl-lsp-rs native_critic_engine --profile agent --locked --lib -- --nocapture
```

2. **Attach compatibility receipts for migration PRs**:

```bash
cargo xtask native-tooling perlcritic-compat \
  --profile .perlcriticrc \
  --receipt target/receipts/native-tooling/perlcritic-compat.json \
  --summary target/receipts/native-tooling/perlcritic-compat.md
```

3. **Use external compatibility only for parity checks**:

Install `perlcritic` in CI only when a job intentionally exercises the legacy
adapter or compares native coverage against a `.perlcriticrc`.

### Scenario: Multiple Project Configuration

**Problem**: Different projects need different critic policies.

**Solution**:

1. **Project-specific native profiles**:

```toml
# legacy_project/.perl-lsp.toml
[critic]
engine = "native"
profile = "recommended"

[diagnostics]
perlcritic_severity = 4

# new_project/.perl-lsp.toml
[critic]
engine = "native"
profile = "strict"

[diagnostics]
perlcritic_severity = 2
```

2. **Migration compatibility receipts**:

```bash
cargo xtask native-tooling perlcritic-compat \
  --profile legacy_project/.perlcriticrc \
  --receipt target/receipts/native-tooling/legacy-perlcritic-compat.json \
  --summary target/receipts/native-tooling/legacy-perlcritic-compat.md
```

### Scenario: Custom Perl::Critic Policy Integration

**Problem**: You still depend on custom Perl::Critic policies.

**Solution**:

1. **Keep custom policies in explicit external legacy mode**:

```toml
[critic]
engine = "legacy"
```

2. **Document the gap in the compatibility report**:

```bash
cargo xtask native-tooling perlcritic-compat \
  --profile .perlcriticrc \
  --receipt target/receipts/native-tooling/perlcritic-compat.json \
  --summary target/receipts/native-tooling/perlcritic-compat.md
```

3. **Promote high-value custom policies into native rules deliberately**:

New native rules should have stable rule IDs, precise spans, suppression tests,
severity/config coverage, diagnostics coverage, and code-action coverage when a
fix is safe.

## Monitoring and Maintenance

### Performance Monitoring

**Set up performance monitoring**:

```bash
cargo test -p perl-lsp-rs --test lsp_performance_tests -- test_execute_command_latency
RUST_LOG=debug cargo test -p perl-lsp-rs native_critic_engine --profile agent --locked --lib -- --nocapture
```

### Health Checks

**Regular validation procedures**:

```bash
#!/bin/bash
set -euo pipefail

echo "Testing executeCommand and native critic health..."

cargo xtask native-tooling check-defaults
cargo test -p perl-lsp-rs-core native_critic --profile agent --locked -- --nocapture
cargo test -p perl-lsp-rs native_critic_engine --profile agent --locked --lib -- --nocapture
cargo test -p perl-lsp-rs diagnostics --profile agent --locked --lib -- --nocapture

echo "Native executeCommand health checks passed."
```

## Summary

This guide covers:

- Native `perl.runCritic` configuration and profiles
- Compatibility reporting for existing `.perlcriticrc` files
- Explicit external `perlcritic` adapter troubleshooting
- Suppressions, severity filters, and missing-result triage
- CI patterns that match editor diagnostics
- Maintenance checks for native tooling defaults

The default executeCommand critic path is native. External `perlcritic` is a
compatibility adapter for migration and custom policy stacks, not an operational
dependency for normal perl-lsp editing or CI proof.

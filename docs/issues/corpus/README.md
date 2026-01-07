# Corpus Audit Tooling

## Overview

The corpus audit tooling provides systematic analysis of corpus coverage and gap detection for the Perl parser. This tool helps identify areas where the test corpus may be insufficient to ensure comprehensive parser validation.

## Purpose

The corpus audit tooling infrastructure serves several key purposes:

1. **Coverage Analysis**: Systematically analyze how well the corpus covers all parser features
2. **Gap Detection**: Identify NodeKinds and GA features that lack test coverage
3. **Risk Assessment**: Detect potential timeout/hang risks in corpus files
4. **CI Integration**: Provide automated checks for corpus health in CI pipelines
5. **Report Generation**: Generate machine-readable reports for tracking and analysis

## Architecture

The corpus audit tooling is organized into several modules:

### Core Modules

- **`corpus_audit.rs`**: Main orchestration module that coordinates all audit phases
- **`corpus.rs`**: Corpus file parsing and inventory management
- **`timeout_detection.rs`**: Timeout and hang risk detection with bounded iteration limits
- **`nodekind_analysis.rs`**: NodeKind reachability analysis and frequency tracking
- **`ga_alignment.rs`**: GA feature-to-fixture alignment checking
- **`report.rs`**: Report generation with comprehensive audit findings

### Corpus Layers

The tool analyzes four corpus layers:

1. **TreeSitter Corpus** (`c/test/corpus`): Tree-sitter test cases
2. **Highlight Fixtures** (`c/test/highlight`): Syntax highlighting test fixtures
3. **Test Corpus** (`test_corpus`): General test corpus files
4. **PerlCorpus Generators** (`crates/perl-corpus/src/generators`): Property-based test generators

## Usage

### Running the Audit

```bash
# Run corpus audit (generates corpus_audit_report.json)
just corpus-audit

# Run audit in CI check mode (fails if issues found)
just corpus-audit-check

# Run audit with fresh report regeneration
just corpus-audit-fresh

# Direct xtask invocation
cd xtask && cargo run --no-default-features -- corpus-audit
cd xtask && cargo run --no-default-features -- corpus-audit --check
cd xtask && cargo run --no-default-features -- corpus-audit --fresh
```

### Command Options

- **`--check`**: Run in CI check mode - fails if any issues are found
- **`--fresh`**: Regenerate report even if it exists
- **`--corpus-path <path>`**: Specify custom corpus directory (default: `.`)
- **`--output <path>`**: Specify custom output file (default: `corpus_audit_report.json`)

## Audit Phases

The corpus audit runs through five main phases:

### Phase 1: Corpus Inventory

Collects and analyzes all corpus files:

- Total file count
- Files per corpus layer
- Total size (bytes)
- Total line count
- File metadata (path, layer, size, lines)

### Phase 2: Parse Outcomes

Parses each corpus file with timeout protection:

- **Ok**: Successful parse (records duration)
- **Error**: Parse error (records error message)
- **Timeout**: Parse exceeded timeout (records timeout duration)
- **Panic**: Parse panicked (records panic message)

### Phase 3: NodeKind Coverage

Analyzes AST for NodeKind coverage:

- Frequency table of all NodeKinds encountered
- Never-seen NodeKinds (0 occurrences)
- At-risk NodeKinds (<5 occurrences)
- Coverage percentage

### Phase 4: GA Feature Alignment

Checks GA feature-to-fixture alignment:

- Coverage matrix of GA features
- Features with no coverage
- Features with partial coverage
- Overall coverage percentage

### Phase 5: Timeout/Hang Risk Detection

Identifies potential timeout/hang risks:

- Deep nesting (exceeds MAX_NESTING_DEPTH)
- Complex regex (exceeds MAX_REGEX_OPERATIONS)
- Large heredocs (exceeds MAX_HEREDOC_SIZE)
- Nested heredocs (exceeds MAX_HEREDOC_DEPTH)

## Timeout Configuration

The audit tool uses bounded iteration limits to prevent hangs:

| Constant | Value | Purpose |
|----------|--------|---------|
| `DEFAULT_TIMEOUT` | 30s | Maximum time per file parse |
| `MAX_NESTING_DEPTH` | 100 | Maximum nesting depth |
| `MAX_REGEX_OPERATIONS` | 10,000 | Maximum regex operations |
| `MAX_HEREDOC_DEPTH` | 10 | Maximum heredoc nesting |
| `MAX_HEREDOC_SIZE` | 1,000,000 | Maximum heredoc size (1MB) |

## Report Structure

The audit report is generated in JSON format with the following structure:

```json
{
  "metadata": {
    "generated_at": "2026-01-07T10:00:00Z",
    "version": "0.8.8",
    "duration_secs": 15.5
  },
  "inventory": {
    "total_files": 150,
    "files_by_layer": {
      "tree_sitter": 50,
      "highlight": 30,
      "test_corpus": 40,
      "perl_corpus": 30
    },
    "total_size_bytes": 500000,
    "total_lines": 15000
  },
  "parse_outcomes": {
    "total": 150,
    "ok": 145,
    "error": 3,
    "timeout": 1,
    "panic": 1
  },
  "nodekind_coverage": {
    "total_count": 50,
    "covered_count": 46,
    "coverage_percentage": 92.0,
    "never_seen": [
      "NodeKind::MatchExpression",
      "NodeKind::GivenBlock",
      "NodeKind::FormatStatement",
      "NodeKind::SprintfStatement"
    ],
    "at_risk": [
      {
        "name": "NodeKind::SubroutineDeclaration",
        "count": 3,
        "risk_level": "Medium"
      }
    ],
    "frequency": {
      "NodeKind::Statement": 500,
      "NodeKind::Expression": 400,
      ...
    }
  },
  "ga_coverage": {
    "total_count": 12,
    "covered_count": 8,
    "coverage_percentage": 66.7,
    "features": [
      {
        "feature": {
          "id": "control-flow",
          "name": "Control Flow",
          "priority": "P0",
          "expected_nodekinds": ["NodeKind::IfStatement", ...],
          "description": "If, unless, while, until, for, foreach"
        },
        "covered": true,
        "covering_files": ["file1.pl", "file2.pl"],
        "coverage_percentage": 100.0
      },
      ...
    ],
    "uncovered_critical": [
      {
        "id": "match-given",
        "name": "Match/Given",
        "priority": "P0",
        "description": "Modern match/given syntax"
      }
    ],
    "uncovered_partial": [
      {
        "id": "heredocs",
        "name": "Heredocs",
        "priority": "P1",
        "description": "Heredoc syntax and edge cases"
      }
    ]
  },
  "timeout_risks": [
    {
      "priority": "P0",
      "description": "Deep nesting detected (depth: 150)",
      "file_path": "c/test/corpus/deep_nesting.pl",
      "line_number": 50,
      "mitigation": "Consider splitting into smaller test cases"
    },
    ...
  ]
}
```

## CI Integration

The corpus audit can be integrated into CI pipelines:

```yaml
# Example GitHub Actions workflow
- name: Run corpus audit
  run: just corpus-audit-check

- name: Upload audit report
  if: always()
  uses: actions/upload-artifact@v3
  with:
    name: corpus-audit-report
    path: corpus_audit_report.json
```

### CI Check Mode

The `--check` mode validates the audit results and exits with an error if:

- Any parse errors are found
- Any parse timeouts are detected
- Any parse panics occur
- Any P0 critical timeout risks are identified
- GA feature coverage is below 80%

## Performance Requirements

The corpus audit tool is designed for efficient execution:

- **Target duration**: <30s for typical corpus
- **Memory usage**: <100MB for large files
- **No hangs**: Graceful degradation on timeout

## Troubleshooting

### Common Issues

**Issue**: Audit takes too long

**Solution**: 
- Reduce corpus size by excluding non-essential files
- Increase timeout with `--timeout` flag
- Check for files with excessive complexity

**Issue**: False positive timeout risks

**Solution**:
- Adjust `MAX_*` constants in `corpus_audit.rs`
- Review specific files marked as at-risk
- Add whitelist for known safe patterns

**Issue**: NodeKind coverage seems low

**Solution**:
- Verify corpus files are being discovered
- Check AST traversal logic for completeness
- Review NodeKind definitions for accuracy

## Related Documentation

- [Corpus Gap Issues](./gaps/) - Detailed gap analysis and issue files
- [Commands Reference](../../COMMANDS_REFERENCE.md) - Build and test commands
- [Parser Status](../../crates/perl-parser/PARSER_STATUS.md) - Parser implementation status

## Contributing

When adding new corpus files or parser features:

1. Run `just corpus-audit` to check coverage
2. Review the generated report for gaps
3. Add test cases for uncovered NodeKinds or GA features
4. Re-run audit to verify improvements

## Future Enhancements

Potential improvements to the corpus audit tooling:

- **Historical Tracking**: Track coverage trends over time
- **Visual Reports**: Generate HTML/Markdown reports with charts
- **Automated Fix Suggestions**: Suggest test cases for gaps
- **Integration with Issue Tracking**: Auto-create issues for critical gaps
- **Corpus Quality Metrics**: Analyze test quality beyond coverage

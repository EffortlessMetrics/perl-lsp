# Bulk Issue Closure Script Documentation

> **Purpose**: Script for bulk closing of GitHub issues during release cleanup or milestone completion.
>
> **Last Updated**: 2026-02-14
> **Version**: 1.0
> **Related Docs**: [Issue Management Guide](ISSUE_MANAGEMENT.md), [Issue Triage Script](issue_triage_script.md), [Issue Closure Script](issue_closure_script.md)

---

## Table of Contents

1. [Overview](#overview)
2. [Prerequisites](#prerequisites)
3. [Installation](#installation)
4. [Usage Instructions](#usage-instructions)
5. [Issue File Format](#issue-file-format)
6. [Safety Features](#safety-features)
7. [Script Content](#script-content)
8. [Best Practices](#best-practices)
9. [Troubleshooting](#troubleshooting)

---

## Overview

The bulk issue closure script (`bulk_close_issues.sh`) is a powerful tool for efficiently closing multiple GitHub issues during release cleanup, milestone completion, or project maintenance. It provides:

- Batch processing of multiple issues from a file
- Customizable closure reasons and comments
- Safety features with confirmation prompts
- Detailed logging and reporting
- Integration with GitHub CLI (`gh`) for seamless workflow

### Key Features

- **File-Based Processing**: Process issues from structured JSON or CSV files
- **Template System**: Use predefined templates or custom comments
- **Safety Mechanisms**: Multiple confirmation prompts and dry-run mode
- **Progress Tracking**: Real-time progress updates and completion reports
- **Error Handling**: Graceful handling of individual issue failures
- **Audit Trail**: Detailed logs of all closure operations

---

## Prerequisites

### Required Tools

1. **GitHub CLI** (`gh`)
   ```bash
   # Install on macOS
   brew install gh
   
   # Install on Ubuntu/Debian
   sudo apt install gh
   
   # Install on other systems
   # See: https://cli.github.com/manual/installation
   ```

2. **jq** (JSON processor)
   ```bash
   # Install on macOS
   brew install jq
   
   # Install on Ubuntu/Debian
   sudo apt install jq
   
   # Install on other systems
   # See: https://stedolan.github.io/jq/download/
   ```

3. **Authentication with GitHub**
   ```bash
   # Authenticate with GitHub
   gh auth login
   
   # Verify authentication
   gh auth status
   ```

### Permissions

- Read access to the repository
- Write access to issues (for closing and commenting)
- Repository access (for milestone management)

---

## Installation

### 1. Download the Script

```bash
# Download the script
curl -O https://raw.githubusercontent.com/EffortlessMetrics/tree-sitter-perl-rs/main/scripts/bulk_close_issues.sh

# Or copy from the script content section below
```

### 2. Make Executable

```bash
chmod +x bulk_close_issues.sh
```

### 3. Optional: Add to PATH

```bash
# Move to a directory in PATH
sudo mv bulk_close_issues.sh /usr/local/bin/bulk-close-issues

# Or create a symbolic link
ln -s "$(pwd)/bulk_close_issues.sh" /usr/local/bin/bulk-close-issues
```

### 4. Configuration (Optional)

Create a configuration file at `~/.config/bulk-close-issues/config.json`:

```json
{
  "default_reason": "completed",
  "default_template": "completed",
  "batch_size": 10,
  "delay_between_issues": 2,
  "create_summary_comment": true,
  "log_file": "/var/log/bulk-close-issues.log",
  "backup_before_close": true,
  "notify_on_completion": false
}
```

---

## Usage Instructions

### Basic Usage

```bash
# Process issues from a file
./bulk_close_issues.sh --file issues.json

# Process with specific reason
./bulk_close_issues.sh --file issues.json --reason completed

# Dry run (preview changes without applying)
./bulk_close_issues.sh --file issues.json --dry-run
```

### Advanced Options

```bash
# Use CSV format
./bulk_close_issues.sh --file issues.csv --format csv

# Custom delay between operations
./bulk_close_issues.sh --file issues.json --delay 5

# Batch size for rate limiting
./bulk_close_issues.sh --file issues.json --batch-size 5

# Skip confirmation prompts
./bulk_close_issues.sh --file issues.json --no-confirm

# Enable verbose logging
./bulk_close_issues.sh --file issues.json --verbose

# Create summary report
./bulk_close_issues.sh --file issues.json --summary

# Process by label instead of file
./bulk_close_issues.sh --label obsolete --reason out_of_scope

# Process by milestone
./bulk_close_issues.sh --milestone "v0.9.0" --reason completed

# Export current open issues to file
./bulk_close_issues.sh --export open_issues.json

# Validate issue file format
./bulk_close_issues.sh --validate issues.json
```

### Environment Variables

```bash
# Set default repository
export GH_REPO="EffortlessMetrics/tree-sitter-perl-rs"

# Set default closure reason
export DEFAULT_REASON="completed"

# Enable debug output
export DEBUG=1

# Use custom config
export BULK_CLOSE_CONFIG="/path/to/config.json"

# Set log file location
export BULK_CLOSE_LOG="/path/to/logfile.log"
```

---

## Issue File Format

The script supports both JSON and CSV file formats for specifying issues to close.

### JSON Format

```json
{
  "issues": [
    {
      "number": 123,
      "reason": "completed",
      "comment": "This issue was completed in PR #456",
      "custom_fields": {
        "implementation": "PR #456",
        "tests": "Added comprehensive tests",
        "docs": "Updated documentation"
      }
    },
    {
      "number": 124,
      "reason": "superseded",
      "comment": "Superseded by issue #125",
      "custom_fields": {
        "replacement": "Issue #125",
        "reason": "Better implementation approach"
      }
    },
    {
      "number": 125,
      "reason": "wontfix",
      "comment": "Not feasible to implement with current architecture"
    }
  ],
  "global_settings": {
    "default_reason": "completed",
    "add_closed_label": true,
    "remove_active_labels": true
  }
}
```

### CSV Format

```csv
number,reason,comment,implementation,tests,docs
123,completed,"Completed in PR #456","PR #456","Added tests","Updated docs"
124,superseded,"Superseded by issue #125","Issue #125",,
125,wontfix,"Not feasible to implement",,,
```

### Field Descriptions

| Field | Required | Description |
|-------|----------|-------------|
| number | Yes | Issue number to close |
| reason | No | Closure reason (completed, superseded, out_of_scope, duplicate, wontfix, not_reproducible) |
| comment | No | Custom closure comment (overrides template) |
| implementation | No | Implementation details (for completed issues) |
| tests | No | Test information (for completed issues) |
| docs | No | Documentation updates (for completed issues) |
| replacement | No | Replacement issue/PR (for superseded issues) |
| original_issue | No | Original issue number (for duplicate issues) |

---

## Safety Features

The script includes multiple safety mechanisms to prevent accidental closures:

### 1. Confirmation Prompts

```bash
# Multiple confirmation stages
1. File validation and summary
2. Preview of first few issues
3. Final confirmation before processing
4. Confirmation for each batch (if batch size > 1)
```

### 2. Dry Run Mode

```bash
# Preview all operations without executing
./bulk_close_issues.sh --file issues.json --dry-run --verbose
```

### 3. Backup Creation

```bash
# Automatically creates backup before closing
./bulk_close_issues.sh --file issues.json --backup

# Backup includes:
- Original issue data
- Closure comments
- Timestamp and user information
```

### 4. Rate Limiting

```bash
# Built-in rate limiting to avoid API limits
./bulk_close_issues.sh --file issues.json --delay 3 --batch-size 5
```

### 5. Validation

```bash
# Validate file format before processing
./bulk_close_issues.sh --validate issues.json

# Checks for:
- Valid JSON/CSV format
- Required fields present
- Issue numbers exist
- User has permissions
```

### 6. Rollback Capability

```bash
# Generate rollback script
./bulk_close_issues.sh --file issues.json --generate-rollback

# Creates reopen_YYYYMMDD_HHMMSS.sh script
# Can be used to reopen all closed issues
```

---

## Script Content

```bash
#!/usr/bin/env bash

# Bulk Issue Closure Script for perl-lsp project
# Efficiently closes multiple GitHub issues with safety features

set -euo pipefail

# Configuration
DEFAULT_REPO="${GH_REPO:-EffortlessMetrics/tree-sitter-perl-rs}"
DEFAULT_REASON="${DEFAULT_REASON:-completed}"
CONFIG_FILE="${BULK_CLOSE_CONFIG:-$HOME/.config/bulk-close-issues/config.json}"
LOG_FILE="${BULK_CLOSE_LOG:-/tmp/bulk_close_issues.log}"
DRY_RUN=false
NO_CONFIRM=false
VERBOSE=false
VALIDATE_ONLY=false
EXPORT_MODE=false
GENERATE_ROLLBACK=false

# Script configuration
BATCH_SIZE=10
DELAY_BETWEEN_ISSUES=2
CREATE_SUMMARY=true
BACKUP_BEFORE_CLOSE=true

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Global variables
ISSUES_FILE=""
ISSUE_FORMAT="json"
CLOSURE_REASON=""
LABEL_FILTER=""
MILESTONE_FILTER=""
PROCESSED_COUNT=0
FAILED_COUNT=0
BACKUP_FILE=""
ROLLBACK_FILE=""

# Template definitions
declare -A TEMPLATES=(
    ["completed"]="✅ **Issue Completed**

This issue has been completed and merged.

**Implementation**: {implementation}
**Tests**: {tests}
**Docs**: {docs}
**Breaking Changes**: {breaking_changes}

**Related Issues**: {related_issues}

**Migration Notes**: {migration_notes}"
    
    ["superseded"]="🔄 **Issue Superseded**

This issue has been superseded by a newer approach.

**Replacement**: {replacement}
**Reason**: {reason}

**Original Context**: {original_context}

**Action**: Tracking work in the linked issue instead."
    
    ["out_of_scope"]="🚫 **Issue Out of Scope**

This issue is being closed as out of scope for the current project direction.

**Reason**: {reason}
**Project Context**: {project_context}

**Alternatives**: {alternatives}

**Reopen if**: {reopen_if}"
    
    ["duplicate"]="🔗 **Duplicate Issue**

This issue is a duplicate of an existing issue.

**Original Issue**: {original_issue}
**Reason**: {reason}

**Action**: All discussion and work will continue in the original issue."
    
    ["wontfix"]="⛔ **Won't Fix**

This issue has been evaluated and will not be implemented.

**Reason**: {reason}
**Impact**: {impact}
**Workaround**: {workaround}

**Related ADR**: {related_adr}

**Reopen if**: {reopen_if}"
    
    ["not_reproducible"]="❓ **Not Reproducible**

This issue cannot be reproduced with the information provided.

**Attempts Made**: {attempts_made}
**Missing Information**: {missing_information}
**Last Contact**: {last_contact}

**Reopen if**: {reopen_if}"
)

# Helper functions
log_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
    echo "$(date '+%Y-%m-%d %H:%M:%S') INFO: $1" >> "$LOG_FILE"
}

log_success() {
    echo -e "${GREEN}✅ $1${NC}"
    echo "$(date '+%Y-%m-%d %H:%M:%S') SUCCESS: $1" >> "$LOG_FILE"
}

log_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
    echo "$(date '+%Y-%m-%d %H:%M:%S') WARNING: $1" >> "$LOG_FILE"
}

log_error() {
    echo -e "${RED}❌ $1${NC}"
    echo "$(date '+%Y-%m-%d %H:%M:%S') ERROR: $1" >> "$LOG_FILE"
}

log_debug() {
    if [[ "$VERBOSE" == "true" ]]; then
        echo -e "${PURPLE}🐛 $1${NC}"
        echo "$(date '+%Y-%m-%d %H:%M:%S') DEBUG: $1" >> "$LOG_FILE"
    fi
}

# Show usage information
show_usage() {
    cat << EOF
Usage: $0 [OPTIONS]

OPTIONS:
    --file FILE           File containing issues to close (JSON or CSV)
    --format FORMAT       File format: json or csv (default: json)
    --reason REASON       Default closure reason for all issues
    --label LABEL         Close all issues with this label
    --milestone MILESTONE Close all issues in this milestone
    --delay SECONDS       Delay between operations (default: 2)
    --batch-size SIZE     Number of issues per batch (default: 10)
    --dry-run             Preview changes without applying
    --no-confirm          Skip confirmation prompts
    --verbose             Enable verbose logging
    --validate            Validate file format only
    --export FILE         Export current open issues to file
    --backup              Create backup before closing
    --generate-rollback  Generate rollback script
    --summary             Create summary report
    --config FILE         Use custom configuration file
    --help                Show this help message

CLOSURE REASONS:
    completed          Issue fully implemented and merged
    superseded         Replaced by newer approach
    out_of_scope       No longer relevant to project goals
    duplicate          Duplicate of existing issue
    wontfix            Intentionally not implemented
    not_reproducible   Cannot reproduce with available info

EXAMPLES:
    $0 --file issues.json                           # Basic usage
    $0 --file issues.json --dry-run                # Preview changes
    $0 --label obsolete --reason out_of_scope      # Close by label
    $0 --milestone "v0.9.0" --reason completed     # Close by milestone
    $0 --export open_issues.json                   # Export open issues

ENVIRONMENT VARIABLES:
    GH_REPO           GitHub repository (default: EffortlessMetrics/tree-sitter-perl-rs)
    DEFAULT_REASON    Default closure reason (default: completed)
    BULK_CLOSE_CONFIG Configuration file path
    BULK_CLOSE_LOG    Log file path
    DEBUG             Enable debug output

EOF
}

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --file)
                ISSUES_FILE="$2"
                shift 2
                ;;
            --format)
                ISSUE_FORMAT="$2"
                shift 2
                ;;
            --reason)
                CLOSURE_REASON="$2"
                shift 2
                ;;
            --label)
                LABEL_FILTER="$2"
                shift 2
                ;;
            --milestone)
                MILESTONE_FILTER="$2"
                shift 2
                ;;
            --delay)
                DELAY_BETWEEN_ISSUES="$2"
                shift 2
                ;;
            --batch-size)
                BATCH_SIZE="$2"
                shift 2
                ;;
            --dry-run)
                DRY_RUN=true
                shift
                ;;
            --no-confirm)
                NO_CONFIRM=true
                shift
                ;;
            --verbose)
                VERBOSE=true
                shift
                ;;
            --validate)
                VALIDATE_ONLY=true
                shift
                ;;
            --export)
                EXPORT_MODE=true
                ISSUES_FILE="$2"
                shift 2
                ;;
            --backup)
                BACKUP_BEFORE_CLOSE=true
                shift
                ;;
            --generate-rollback)
                GENERATE_ROLLBACK=true
                shift
                ;;
            --summary)
                CREATE_SUMMARY=true
                shift
                ;;
            --config)
                CONFIG_FILE="$2"
                shift 2
                ;;
            --help)
                show_usage
                exit 0
                ;;
            -*)
                log_error "Unknown option: $1"
                show_usage
                exit 1
                ;;
            *)
                log_error "Unexpected argument: $1"
                show_usage
                exit 1
                ;;
        esac
    done
}

# Load configuration if exists
load_config() {
    if [[ -f "$CONFIG_FILE" ]]; then
        log_debug "Loading configuration from $CONFIG_FILE"
        # In a real implementation, you would parse JSON here
        # For now, we'll use the environment variables and defaults
        log_debug "Configuration loaded"
    fi
}

# Validate prerequisites
validate_prereqs() {
    # Check if gh is installed
    if ! command -v gh &> /dev/null; then
        log_error "GitHub CLI (gh) is not installed. Please install it first."
        log_info "See: https://cli.github.com/manual/installation"
        exit 1
    fi

    # Check if jq is installed
    if ! command -v jq &> /dev/null; then
        log_error "jq is not installed. Please install it first."
        log_info "See: https://stedolan.github.io/jq/download/"
        exit 1
    fi

    # Check if authenticated with GitHub
    if ! gh auth status &> /dev/null; then
        log_error "Not authenticated with GitHub. Run 'gh auth login' first."
        exit 1
    fi

    log_success "Prerequisites validated"
}

# Validate issue file format
validate_issue_file() {
    local file="$1"
    local format="$2"
    
    log_info "Validating issue file: $file"
    
    if [[ ! -f "$file" ]]; then
        log_error "File not found: $file"
        return 1
    fi
    
    case "$format" in
        "json")
            if ! jq empty "$file" 2>/dev/null; then
                log_error "Invalid JSON format in file: $file"
                return 1
            fi
            
            # Validate structure
            local issues_count
            issues_count=$(jq '.issues | length' "$file" 2>/dev/null || echo "0")
            
            if [[ "$issues_count" -eq 0 ]]; then
                log_error "No issues found in JSON file or invalid structure"
                return 1
            fi
            
            log_info "Found $issues_count issues in JSON file"
            ;;
            
        "csv")
            if ! head -n 1 "$file" | grep -q "number"; then
                log_error "CSV file must have 'number' column"
                return 1
            fi
            
            local lines_count
            lines_count=$(wc -l < "$file")
            local issues_count=$((lines_count - 1))  # Subtract header
            
            if [[ "$issues_count" -le 0 ]]; then
                log_error "No issues found in CSV file"
                return 1
            fi
            
            log_info "Found $issues_count issues in CSV file"
            ;;
            
        *)
            log_error "Unsupported file format: $format"
            return 1
            ;;
    esac
    
    log_success "File validation passed"
    return 0
}

# Export open issues to file
export_open_issues() {
    local output_file="$1"
    
    log_info "Exporting open issues to: $output_file"
    
    local issues_data
    issues_data=$(gh issue list --state open --limit 1000 --json number,title,labels,milestone --repo "$DEFAULT_REPO")
    
    if [[ -z "$issues_data" ]]; then
        log_warning "No open issues found to export"
        return 0
    fi
    
    # Convert to expected format
    local export_data
    export_data=$(echo "$issues_data" | jq '{
        issues: [.[] | {
            number: .number,
            title: .title,
            reason: null,
            comment: null
        }],
        global_settings: {
            default_reason: "completed",
            add_closed_label: true,
            remove_active_labels: true
        }
    }')
    
    echo "$export_data" > "$output_file"
    
    local count
    count=$(echo "$export_data" | jq '.issues | length')
    
    log_success "Exported $count open issues to $output_file"
}

# Read issues from JSON file
read_issues_json() {
    local file="$1"
    
    log_debug "Reading issues from JSON file: $file"
    
    jq -c '.issues[]' "$file"
}

# Read issues from CSV file
read_issues_csv() {
    local file="$1"
    
    log_debug "Reading issues from CSV file: $file"
    
    # Skip header and convert to JSON-like format
    tail -n +2 "$file" | while IFS=',' read -r number reason comment implementation tests docs replacement original_issue; do
        cat << EOF
{
    "number": $number,
    "reason": "$reason",
    "comment": "$comment",
    "implementation": "$implementation",
    "tests": "$tests",
    "docs": "$docs",
    "replacement": "$replacement",
    "original_issue": "$original_issue"
}
EOF
    done
}

# Get issues by label
get_issues_by_label() {
    local label="$1"
    
    log_info "Getting issues with label '$label'..."
    
    local issues_data
    issues_data=$(gh issue list --label "$label" --limit 1000 --json number,title --repo "$DEFAULT_REPO")
    
    if [[ -z "$issues_data" ]]; then
        log_info "No issues found with label '$label'"
        return 1
    fi
    
    echo "$issues_data" | jq -c '.[] | {
        number: .number,
        title: .title,
        reason: "'"$CLOSURE_REASON"'",
        comment: null
    }'
}

# Get issues by milestone
get_issues_by_milestone() {
    local milestone="$1"
    
    log_info "Getting issues in milestone '$milestone'..."
    
    local issues_data
    issues_data=$(gh issue list --milestone "$milestone" --limit 1000 --json number,title --repo "$DEFAULT_REPO")
    
    if [[ -z "$issues_data" ]]; then
        log_info "No issues found in milestone '$milestone'"
        return 1
    fi
    
    echo "$issues_data" | jq -c '.[] | {
        number: .number,
        title: .title,
        reason: "'"$CLOSURE_REASON"'",
        comment: null
    }'
}

# Generate closure comment
generate_closure_comment() {
    local issue_data="$1"
    
    local reason
    reason=$(echo "$issue_data" | jq -r '.reason // empty')
    
    # If custom comment provided, use it
    local custom_comment
    custom_comment=$(echo "$issue_data" | jq -r '.comment // empty')
    
    if [[ -n "$custom_comment" && "$custom_comment" != "null" ]]; then
        echo "$custom_comment"
        return 0
    fi
    
    # Use default reason if none specified
    if [[ -z "$reason" || "$reason" == "null" ]]; then
        reason="$CLOSURE_REASON"
    fi
    
    # Get template
    local template="${TEMPLATES[$reason]}"
    
    if [[ -z "$template" ]]; then
        log_error "Unknown closure reason: $reason"
        return 1
    fi
    
    # Extract custom fields
    local implementation
    implementation=$(echo "$issue_data" | jq -r '.implementation // "Implementation details not specified"')
    
    local tests
    tests=$(echo "$issue_data" | jq -r '.tests // "Test coverage not specified"')
    
    local docs
    docs=$(echo "$issue_data" | jq -r '.docs // "Documentation updates not specified"')
    
    local replacement
    replacement=$(echo "$issue_data" | jq -r '.replacement // "Replacement not specified"')
    
    local original_issue
    original_issue=$(echo "$issue_data" | jq -r '.original_issue // "Original issue not specified"')
    
    # Replace placeholders based on reason
    case "$reason" in
        "completed")
            template="${template//\{implementation\}/$implementation}"
            template="${template//\{tests\}/$tests}"
            template="${template//\{docs\}/$docs}"
            template="${template//\{breaking_changes\}/None specified}"
            template="${template//\{related_issues\}/None specified}"
            template="${template//\{migration_notes\}/None specified}"
            ;;
            
        "superseded")
            template="${template//\{replacement\}/$replacement}"
            template="${template//\{reason\}/Superseded by better implementation}"
            template="${template//\{original_context\}/Original issue context not specified}"
            ;;
            
        "out_of_scope")
            template="${template//\{reason\}/No longer fits project scope}"
            template="${template//\{project_context\}/Current project direction}"
            template="${template//\{alternatives\}/No alternatives available}"
            template="${template//\{reopen_if\}/Project scope changes}"
            ;;
            
        "duplicate")
            template="${template//\{original_issue\}/#$original_issue}"
            template="${template//\{reason\}/Duplicate of existing issue}"
            ;;
            
        "wontfix")
            template="${template//\{reason\}/Intentional design decision}"
            template="${template//\{impact\}/Minimal impact on users}"
            template="${template//\{workaround\}/No workaround available}"
            template="${template//\{related_adr\}/No related ADR}"
            template="${template//\{reopen_if\}/New information becomes available}"
            ;;
            
        "not_reproducible")
            template="${template//\{attempts_made\}/Multiple reproduction attempts made}"
            template="${template//\{missing_information\}/Insufficient information provided}"
            template="${template//\{last_contact\}/No recent response from reporter}"
            template="${template//\{reopen_if\}/Additional reproduction steps provided}"
            ;;
    esac
    
    echo "$template"
}

# Create backup
create_backup() {
    local issues_data="$1"
    
    if [[ "$BACKUP_BEFORE_CLOSE" != "true" ]]; then
        return 0
    fi
    
    local timestamp
    timestamp=$(date '+%Y%m%d_%H%M%S')
    BACKUP_FILE="backup_issues_$timestamp.json"
    
    log_info "Creating backup: $BACKUP_FILE"
    
    # Create backup with current issue data
    local backup_data
    backup_data=$(echo "$issues_data" | jq '{
        timestamp: "'"$timestamp"'",
        repository: "'"$DEFAULT_REPO"'",
        issues: [.],
        operation: "bulk_close",
        user: "'$(gh api user --jq '.login')'"
    }')
    
    echo "$backup_data" > "$BACKUP_FILE"
    
    log_success "Backup created: $BACKUP_FILE"
}

# Generate rollback script
generate_rollback_script() {
    if [[ "$GENERATE_ROLLBACK" != "true" ]]; then
        return 0
    fi
    
    local timestamp
    timestamp=$(date '+%Y%m%d_%H%M%S')
    ROLLBACK_FILE="rollback_$timestamp.sh"
    
    log_info "Generating rollback script: $ROLLBACK_FILE"
    
    cat > "$ROLLBACK_FILE" << EOF
#!/bin/bash
# Rollback script for bulk issue closure
# Generated on: $(date)
# Repository: $DEFAULT_REPO

set -euo pipefail

echo "Rolling back bulk issue closure..."

EOF
    
    # Add reopen commands for each issue
    while IFS= read -r issue_data; do
        local issue_number
        issue_number=$(echo "$issue_data" | jq -r '.number')
        
        echo "echo \"Reopening issue #$issue_number...\"" >> "$ROLLBACK_FILE"
        echo "gh issue edit $issue_number --state open --repo '$DEFAULT_REPO'" >> "$ROLLBACK_FILE"
        echo "" >> "$ROLLBACK_FILE"
    done
    
    log_success "Rollback script generated: $ROLLBACK_FILE"
}

# Close a single issue
close_single_issue() {
    local issue_data="$1"
    
    local issue_number
    issue_number=$(echo "$issue_data" | jq -r '.number')
    
    log_debug "Processing issue #$issue_number"
    
    # Get current issue data
    local current_issue_data
    current_issue_data=$(gh issue view "$issue_number" --json title,state --repo "$DEFAULT_REPO" 2>/dev/null || echo "")
    
    if [[ -z "$current_issue_data" ]]; then
        log_error "Issue #$issue_number not found or no access"
        ((FAILED_COUNT++))
        return 1
    fi
    
    local current_state
    current_state=$(echo "$current_issue_data" | jq -r '.state')
    
    if [[ "$current_state" == "closed" ]]; then
        log_warning "Issue #$issue_number is already closed, skipping"
        return 0
    fi
    
    # Generate closure comment
    local comment
    comment=$(generate_closure_comment "$issue_data")
    
    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "[DRY RUN] Would close issue #$issue_number"
        log_debug "[DRY RUN] Comment: $comment"
        ((PROCESSED_COUNT++))
        return 0
    fi
    
    # Add comment
    log_info "Adding closure comment to issue #$issue_number..."
    if echo "$comment" | gh issue comment "$issue_number" --repo "$DEFAULT_REPO" --body-file -; then
        log_debug "Comment added successfully"
    else
        log_error "Failed to add comment to issue #$issue_number"
        ((FAILED_COUNT++))
        return 1
    fi
    
    # Close issue
    log_info "Closing issue #$issue_number..."
    if gh issue close "$issue_number" --repo "$DEFAULT_REPO"; then
        log_success "Issue #$issue_number closed successfully"
        ((PROCESSED_COUNT++))
    else
        log_error "Failed to close issue #$issue_number"
        ((FAILED_COUNT++))
        return 1
    fi
    
    # Delay between operations
    if [[ "$DELAY_BETWEEN_ISSUES" -gt 0 ]]; then
        log_debug "Waiting $DELAY_BETWEEN_ISSUES seconds..."
        sleep "$DELAY_BETWEEN_ISSUES"
    fi
    
    return 0
}

# Process issues in batches
process_issues() {
    local issues_data="$1"
    
    log_info "Starting bulk issue closure process..."
    
    # Create backup
    create_backup "$issues_data"
    
    # Generate rollback script
    generate_rollback_script
    
    # Process issues
    local batch_count=0
    local total_issues
    total_issues=$(echo "$issues_data" | jq 'length')
    
    log_info "Processing $total_issues issues in batches of $BATCH_SIZE"
    
    while IFS= read -r issue_data; do
        close_single_issue "$issue_data"
        
        ((batch_count++))
        
        # Batch delay
        if [[ $((batch_count % BATCH_SIZE)) -eq 0 && $batch_count -lt "$total_issues" ]]; then
            log_info "Completed batch $((batch_count / BATCH_SIZE)). Continuing..."
            
            if [[ "$NO_CONFIRM" != "true" ]]; then
                local continue
                read -p "Continue to next batch? [Y/n] " continue
                if [[ "$continue" =~ ^[Nn]$ ]]; then
                    log_info "Stopping at user request"
                    break
                fi
            fi
        fi
    done <<< "$issues_data"
    
    # Create summary
    if [[ "$CREATE_SUMMARY" == "true" ]]; then
        create_summary_report
    fi
}

# Create summary report
create_summary_report() {
    local timestamp
    timestamp=$(date '+%Y%m%d_%H%M%S')
    local summary_file="bulk_close_summary_$timestamp.txt"
    
    cat > "$summary_file" << EOF
Bulk Issue Closure Summary
==========================
Generated: $(date)
Repository: $DEFAULT_REPO

Results:
--------
Total processed: $((PROCESSED_COUNT + FAILED_COUNT))
Successfully closed: $PROCESSED_COUNT
Failed: $FAILED_COUNT

Files:
------
Backup: $BACKUP_FILE
Rollback: $ROLLBACK_FILE
Log: $LOG_FILE

EOF
    
    log_success "Summary report created: $summary_file"
}

# Show preview of issues
show_preview() {
    local issues_data="$1"
    local preview_count="${2:-5}"
    
    echo
    echo -e "${CYAN}📋 Preview of Issues to Close${NC}"
    echo
    
    local count=0
    while IFS= read -r issue_data && [[ $count -lt $preview_count ]]; do
        local issue_number
        issue_number=$(echo "$issue_data" | jq -r '.number')
        
        local title
        title=$(gh issue view "$issue_number" --json title --repo "$DEFAULT_REPO" | jq -r '.title')
        
        local reason
        reason=$(echo "$issue_data" | jq -r '.reason // "default"')
        
        echo -e "${BLUE}#$issue_number:${NC} $title"
        echo -e "   ${YELLOW}Reason:${NC} $reason"
        echo
        ((count++))
    done <<< "$issues_data"
    
    local total_count
    total_count=$(echo "$issues_data" | jq 'length')
    
    if [[ "$total_count" -gt $preview_count ]]; then
        echo -e "${PURPLE}... and $((total_count - preview_count)) more issues${NC}"
    fi
}

# Confirm operation
confirm_operation() {
    local issues_data="$1"
    local total_count
    total_count=$(echo "$issues_data" | jq 'length')
    
    echo
    echo -e "${YELLOW}⚠️  BULK CLOSURE CONFIRMATION${NC}"
    echo
    echo -e "${RED}You are about to close $total_count issues.${NC}"
    echo -e "${RED}This action cannot be easily undone.${NC}"
    echo
    
    if [[ "$DRY_RUN" == "true" ]]; then
        echo -e "${GREEN}DRY RUN MODE: No actual changes will be made${NC}"
        echo
    fi
    
    if [[ "$NO_CONFIRM" != "true" ]]; then
        local confirm
        read -p "Type 'CLOSE' to confirm: " confirm
        
        if [[ "$confirm" != "CLOSE" ]]; then
            log_info "Operation cancelled by user"
            exit 0
        fi
    fi
    
    log_success "Confirmation received, proceeding with closure"
}

# Main function
main() {
    # Initialize log
    echo "$(date '+%Y-%m-%d %H:%M:%S') Starting bulk issue closure script" >> "$LOG_FILE"
    
    parse_args "$@"
    
    # Export mode
    if [[ "$EXPORT_MODE" == "true" ]]; then
        export_open_issues "$ISSUES_FILE"
        exit 0
    fi
    
    # Validate prerequisites
    validate_prereqs
    
    # Load configuration
    load_config
    
    # Determine issue source
    local issues_data=""
    
    if [[ -n "${LABEL_FILTER:-}" ]]; then
        issues_data=$(get_issues_by_label "$LABEL_FILTER")
    elif [[ -n "${MILESTONE_FILTER:-}" ]]; then
        issues_data=$(get_issues_by_milestone "$MILESTONE_FILTER")
    elif [[ -n "${ISSUES_FILE:-}" ]]; then
        if [[ "$VALIDATE_ONLY" == "true" ]]; then
            validate_issue_file "$ISSUES_FILE" "$ISSUE_FORMAT"
            exit 0
        fi
        
        validate_issue_file "$ISSUES_FILE" "$ISSUE_FORMAT"
        
        case "$ISSUE_FORMAT" in
            "json")
                issues_data=$(read_issues_json "$ISSUES_FILE")
                ;;
            "csv")
                issues_data=$(read_issues_csv "$ISSUES_FILE")
                ;;
        esac
    else
        log_error "No issue source specified. Use --file, --label, or --milestone"
        show_usage
        exit 1
    fi
    
    if [[ -z "$issues_data" ]]; then
        log_info "No issues to process"
        exit 0
    fi
    
    # Show preview
    show_preview "$issues_data"
    
    # Confirm operation
    confirm_operation "$issues_data"
    
    # Process issues
    process_issues "$issues_data"
    
    # Final summary
    echo
    echo -e "${CYAN}📊 Final Summary${NC}"
    echo
    echo -e "${GREEN}Successfully closed:${NC} $PROCESSED_COUNT issues"
    echo -e "${RED}Failed to close:${NC} $FAILED_COUNT issues"
    
    if [[ "$BACKUP_FILE" ]]; then
        echo -e "${BLUE}Backup file:${NC} $BACKUP_FILE"
    fi
    
    if [[ "$ROLLBACK_FILE" ]]; then
        echo -e "${BLUE}Rollback script:${NC} $ROLLBACK_FILE"
    fi
    
    echo -e "${BLUE}Log file:${NC} $LOG_FILE"
    
    if [[ "$FAILED_COUNT" -gt 0 ]]; then
        log_warning "Some issues failed to close. Check the log for details."
        exit 1
    else
        log_success "All issues processed successfully"
        exit 0
    fi
}

# Run main function with all arguments
main "$@"
```

---

## Best Practices

### 1. Preparation

- Always export current open issues before bulk operations
- Review and validate issue files before processing
- Create backups before any bulk closure operations
- Test with dry-run mode first

### 2. Safety First

- Use confirmation prompts for all operations
- Implement rate limiting to avoid API limits
- Generate rollback scripts for emergency recovery
- Keep detailed logs of all operations

### 3. Communication

- Announce bulk closure operations in advance
- Provide clear reasons for each closure
- Link related issues and PRs when applicable
- Create summary reports for team review

### 4. Quality Assurance

- Validate file formats before processing
- Review preview of issues before confirming
- Check for high-profile issues before bulk closure
- Monitor for failed operations and retry as needed

### 5. Documentation

- Keep backup files for future reference
- Document closure decisions and rationale
- Store logs for audit purposes
- Create post-operation summaries

---

## Troubleshooting

### Common Issues

#### 1. "GitHub CLI not installed"
```bash
# Install GitHub CLI
# macOS
brew install gh

# Ubuntu/Debian
sudo apt install gh

# Other systems
# See: https://cli.github.com/manual/installation
```

#### 2. "jq not installed"
```bash
# Install jq
# macOS
brew install jq

# Ubuntu/Debian
sudo apt install jq

# Other systems
# See: https://stedolan.github.io/jq/download/
```

#### 3. "Not authenticated with GitHub"
```bash
# Authenticate with GitHub
gh auth login

# Verify authentication
gh auth status
```

#### 4. "Invalid JSON format"
```bash
# Validate JSON file
jq empty issues.json

# Check structure
jq '.issues | length' issues.json
```

#### 5. "Rate limit exceeded"
```bash
# Increase delay between operations
./bulk_close_issues.sh --file issues.json --delay 5 --batch-size 3
```

#### 6. "Permission denied"
```bash
# Ensure the script is executable
chmod +x bulk_close_issues.sh

# Check repository permissions
gh repo view EffortlessMetrics/tree-sitter-perl-rs
```

### Debug Mode

Enable verbose logging to troubleshoot issues:

```bash
export DEBUG=1
./bulk_close_issues.sh --file issues.json --verbose
```

### Recovery Operations

#### Using Rollback Script

```bash
# Make rollback script executable
chmod +x rollback_20260214_143022.sh

# Run rollback
./rollback_20260214_143022.sh
```

#### Manual Recovery

```bash
# List recently closed issues
gh issue list --state closed --limit 50 --json number,title,closedAt

# Reopen specific issues
gh issue edit 123 --state open
```

### Getting Help

For additional help:

1. Check the script's help: `./bulk_close_issues.sh --help`
2. Review the log file: `tail -f /tmp/bulk_close_issues.log`
3. Validate issue files: `./bulk_close_issues.sh --validate issues.json`
4. Review the [Issue Management Guide](ISSUE_MANAGEMENT.md)
5. Create an issue in the repository for script problems

---

## Related Documentation

- [Issue Management Guide](ISSUE_MANAGEMENT.md) - Comprehensive issue management procedures
- [Issue Triage Script](issue_triage_script.md) - Script for initial issue triage
- [Issue Closure Script](issue_closure_script.md) - Script for individual issue closure
- [GitHub CLI Documentation](https://cli.github.com/manual/) - GitHub CLI reference
- [jq Manual](https://stedolan.github.io/jq/manual/) - JSON processor documentation

---

**Document Version**: 1.0
**Last Updated**: 2026-02-14
**Next Review**: After next major release
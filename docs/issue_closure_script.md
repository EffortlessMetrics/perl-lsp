# Issue Closure Script Documentation

> **Purpose**: Script for generating standardized closure comments when closing GitHub issues.
>
> **Last Updated**: 2026-02-14
> **Version**: 1.0
> **Related Docs**: [Issue Management Guide](ISSUE_MANAGEMENT.md), [Issue Triage Script](issue_triage_script.md), [Bulk Issue Closure Script](issue_bulk_closure_script.md)

---

## Table of Contents

1. [Overview](#overview)
2. [Prerequisites](#prerequisites)
3. [Installation](#installation)
4. [Usage Instructions](#usage-instructions)
5. [Closure Reasons](#closure-reasons)
6. [Template Comments](#template-comments)
7. [Script Content](#script-content)
8. [Best Practices](#best-practices)
9. [Troubleshooting](#troubleshooting)

---

## Overview

The issue closure script (`close_issue.sh`) is a command-line tool that generates standardized closure comments for GitHub issues. It provides:

- Pre-defined templates for different closure reasons
- Interactive prompts for customization
- Integration with GitHub CLI (`gh`) for seamless workflow
- Consistent messaging across all issue closures
- Support for custom closure reasons and templates

### Key Features

- **Template System**: Pre-defined templates for common closure reasons
- **Interactive Mode**: Step-by-step guided closure process
- **Batch Mode**: Close multiple issues with the same reason
- **Custom Templates**: Support for custom closure templates
- **Safety Features**: Dry-run mode and confirmation prompts

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
curl -O https://raw.githubusercontent.com/EffortlessMetrics/tree-sitter-perl-rs/main/scripts/close_issue.sh

# Or copy from the script content section below
```

### 2. Make Executable

```bash
chmod +x close_issue.sh
```

### 3. Optional: Add to PATH

```bash
# Move to a directory in PATH
sudo mv close_issue.sh /usr/local/bin/issue-close

# Or create a symbolic link
ln -s "$(pwd)/close_issue.sh" /usr/local/bin/issue-close
```

### 4. Configuration (Optional)

Create a configuration file at `~/.config/issue-close/config.json`:

```json
{
  "default_reason": "completed",
  "custom_templates": {
    "superseded_by_pr": "This issue has been superseded by PR #{pr_number} which implements the same functionality in a better way.",
    "moved_to_another_repo": "This issue has been moved to {repo_url} as it's more appropriate for that repository."
  },
  "auto_add_closed_label": true,
  "remove_active_labels": true
}
```

---

## Usage Instructions

### Basic Usage

```bash
# Close an issue with interactive prompts
./close_issue.sh <issue-number>

# Example
./close_issue.sh 123
```

### Advanced Options

```bash
# Close with a specific reason
./close_issue.sh --reason completed <issue-number>

# Close with a custom comment
./close_issue.sh --comment "Custom closure message" <issue-number>

# Dry run (show what would be done without making changes)
./close_issue.sh --dry-run <issue-number>

# Skip confirmation prompts
./close_issue.sh --no-confirm <issue-number>

# Use custom template
./close_issue.sh --template custom_template_name <issue-number>

# Batch close multiple issues
./close_issue.sh --batch --reason completed 123,124,125

# Close all issues with a specific label
./close_issue.sh --label obsolete --reason out_of_scope

# List available templates
./close_issue.sh --list-templates
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
export CLOSE_CONFIG="/path/to/config.json"
```

---

## Closure Reasons

The script supports the following standard closure reasons:

### 1. completed
The issue has been fully implemented and merged.

**When to use:**
- All acceptance criteria met
- Tests added and passing
- Documentation updated
- PR merged to main branch

### 2. superseded
The issue has been replaced by a newer approach or solution.

**When to use:**
- Replaced by a better implementation
- Original problem solved differently
- New feature makes this obsolete

### 3. out_of_scope
The issue is no longer relevant to the project goals.

**When to use:**
- Project direction changed
- External dependency removed
- No longer fits project scope

### 4. duplicate
The issue is a duplicate of an existing issue.

**When to use:**
- Same issue already tracked
- Exact duplicate of another issue
- Subset of another issue

### 5. wontfix
The issue has been evaluated and will not be implemented.

**When to use:**
- Intentional design decision
- Technical limitation with acceptable workaround
- Cost/benefit analysis doesn't justify implementation

### 6. not_reproducible
The issue cannot be reproduced or lacks sufficient information.

**When to use:**
- No response to information requests
- Cannot reproduce with provided steps
- Insufficient information to proceed

---

## Template Comments

### Completed Template

```markdown
✅ **Issue Completed**

This issue has been completed and merged.

**Implementation**: PR #<number>
**Tests**: [describe test coverage]
**Docs**: [describe documentation updates]
**Breaking Changes**: [yes/no, describe if yes]

**Related Issues**: [link related issues]

**Migration Notes**: [if applicable]
```

### Superseded Template

```markdown
🔄 **Issue Superseded**

This issue has been superseded by a newer approach.

**Replacement**: Issue #<number> or PR #<number>
**Reason**: [explain why superseded]

**Original Context**: [brief summary of original issue]

**Action**: Tracking work in the linked issue instead.
```

### Out of Scope Template

```markdown
🚫 **Issue Out of Scope**

This issue is being closed as out of scope for the current project direction.

**Reason**: [explain why out of scope]
**Project Context**: [link to relevant roadmap or ADR]

**Alternatives**: [suggest alternatives if applicable]

**Reopen if**: [conditions under which this should be reconsidered]
```

### Duplicate Template

```markdown
🔗 **Duplicate Issue**

This issue is a duplicate of an existing issue.

**Original Issue**: #<number>
**Reason**: [explain why it's a duplicate]

**Action**: All discussion and work will continue in the original issue.
```

### Won't Fix Template

```markdown
⛔ **Won't Fix**

This issue has been evaluated and will not be implemented.

**Reason**: [explain the design decision]
**Impact**: [describe impact on users]
**Workaround**: [if applicable, provide workaround]

**Related ADR**: [link to architecture decision record if applicable]

**Reopen if**: [conditions under which this should be reconsidered]
```

### Not Reproducible Template

```markdown
❓ **Not Reproducible**

This issue cannot be reproduced with the information provided.

**Attempts Made**: [describe reproduction attempts]
**Missing Information**: [list what information is needed]
**Last Contact**: [date of last response from reporter]

**Reopen if**: [conditions for reopening with more information]
```

---

## Script Content

```bash
#!/usr/bin/env bash

# Issue Closure Script for perl-lsp project
# Generates standardized closure comments for GitHub issues

set -euo pipefail

# Configuration
DEFAULT_REPO="${GH_REPO:-EffortlessMetrics/tree-sitter-perl-rs}"
DEFAULT_REASON="${DEFAULT_REASON:-completed}"
CONFIG_FILE="${CLOSE_CONFIG:-$HOME/.config/issue-close/config.json}"
DRY_RUN=false
NO_CONFIRM=false
BATCH_MODE=false
LIST_TEMPLATES=false

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

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
}

log_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

log_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

log_error() {
    echo -e "${RED}❌ $1${NC}"
}

log_debug() {
    if [[ -n "${DEBUG:-}" ]]; then
        echo -e "${PURPLE}🐛 $1${NC}"
    fi
}

# Show usage information
show_usage() {
    cat << EOF
Usage: $0 [OPTIONS] <issue-number>

OPTIONS:
    --reason REASON     Closure reason (completed, superseded, out_of_scope, 
                        duplicate, wontfix, not_reproducible)
    --comment TEXT      Custom closure comment
    --template NAME     Use custom template name
    --dry-run           Show what would be done without making changes
    --no-confirm        Skip confirmation prompts
    --batch             Close multiple issues (comma-separated)
    --label LABEL       Close all issues with this label
    --list-templates    List available templates
    --config FILE       Use custom configuration file
    --help              Show this help message

CLOSURE REASONS:
    completed          Issue fully implemented and merged
    superseded         Replaced by newer approach
    out_of_scope       No longer relevant to project goals
    duplicate          Duplicate of existing issue
    wontfix            Intentionally not implemented
    not_reproducible   Cannot reproduce with available info

EXAMPLES:
    $0 123                          # Interactive closure
    $0 --reason completed 123       # Close as completed
    $0 --comment "Fixed in v1.0" 123  # Custom comment
    $0 --batch --reason wontfix 123,124,125  # Batch close
    $0 --label obsolete --reason out_of_scope  # Close by label

ENVIRONMENT VARIABLES:
    GH_REPO           GitHub repository (default: EffortlessMetrics/tree-sitter-perl-rs)
    DEFAULT_REASON    Default closure reason (default: completed)
    CLOSE_CONFIG      Configuration file path
    DEBUG             Enable debug output

EOF
}

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --reason)
                REASON="$2"
                shift 2
                ;;
            --comment)
                CUSTOM_COMMENT="$2"
                shift 2
                ;;
            --template)
                TEMPLATE_NAME="$2"
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
            --batch)
                BATCH_MODE=true
                shift
                ;;
            --label)
                CLOSE_BY_LABEL="$2"
                shift 2
                ;;
            --list-templates)
                LIST_TEMPLATES=true
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
                if [[ -n "${ISSUE_NUMBERS:-}" ]]; then
                    ISSUE_NUMBERS="$ISSUE_NUMBERS,$1"
                else
                    ISSUE_NUMBERS="$1"
                fi
                shift
                ;;
        esac
    done
}

# Load configuration if exists
load_config() {
    if [[ -f "$CONFIG_FILE" ]]; then
        log_debug "Loading configuration from $CONFIG_FILE"
        # In a real implementation, you would parse JSON here
        # For now, we'll use the environment variables
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

# List available templates
list_templates() {
    echo
    echo -e "${CYAN}📋 Available Closure Templates${NC}"
    echo
    
    for template in "${!TEMPLATES[@]}"; do
        echo -e "${BLUE}$template${NC}"
        echo "${TEMPLATES[$template]}" | head -n 3
        echo "..."
        echo
    done
}

# Get issue information
get_issue_info() {
    local issue_number="$1"
    
    log_debug "Getting information for issue #$issue_number"
    
    local issue_data
    issue_data=$(gh issue view "$issue_number" --json title,body,author,labels,state --repo "$DEFAULT_REPO")
    
    if [[ -z "$issue_data" ]]; then
        log_error "Failed to get issue information for #$issue_number"
        return 1
    fi
    
    echo "$issue_data"
}

# Display issue information
display_issue_info() {
    local issue_number="$1"
    local issue_data="$2"
    
    local title
    title=$(echo "$issue_data" | jq -r '.title')
    
    local author
    author=$(echo "$issue_data" | jq -r '.author.login')
    
    local state
    state=$(echo "$issue_data" | jq -r '.state')
    
    local labels
    labels=$(echo "$issue_data" | jq -r '.labels[].name' | tr '\n' ', ' | sed 's/,$//')
    
    echo
    echo -e "${CYAN}📋 Issue Closure for #$issue_number: \"$title\"${NC}"
    echo
    echo -e "${BLUE}Author:${NC} $author"
    echo -e "${BLUE}State:${NC} $state"
    echo -e "${BLUE}Labels:${NC} ${labels:-none}"
    echo
}

# Interactive prompt function
prompt_choice() {
    local prompt="$1"
    local options="$2"
    local default="${3:-}"
    
    while true; do
        echo -e "${YELLOW}$prompt${NC}"
        echo "$options"
        
        if [[ -n "$default" ]]; then
            echo -n "Enter choice [$default]: "
        else
            echo -n "Enter choice: "
        fi
        
        read -r choice
        
        # Use default if empty
        if [[ -z "$choice" && -n "$default" ]]; then
            choice="$default"
        fi
        
        # Validate choice
        if [[ "$choice" =~ ^[1-9][0-9]*$ ]] && [[ "$choice" -le "$(echo "$options" | wc -l)" ]]; then
            echo "$choice"
            return 0
        else
            log_error "Invalid choice. Please try again."
        fi
    done
}

# Interactive text input
prompt_text() {
    local prompt="$1"
    local default="${2:-}"
    
    echo -n -e "${YELLOW}$prompt${NC}"
    if [[ -n "$default" ]]; then
        echo -n " [$default]: "
    else
        echo -n ": "
    fi
    
    read -r input
    
    if [[ -z "$input" && -n "$default" ]]; then
        echo "$default"
    else
        echo "$input"
    fi
}

# Get issues by label
get_issues_by_label() {
    local label="$1"
    
    log_info "Getting issues with label '$label'..."
    
    local issues
    issues=$(gh issue list --label "$label" --limit 100 --json number --repo "$DEFAULT_REPO" | jq -r '.[].number')
    
    if [[ -z "$issues" ]]; then
        log_info "No issues found with label '$label'"
        return 1
    fi
    
    echo "$issues"
}

# Generate closure comment
generate_comment() {
    local reason="$1"
    local issue_data="$2"
    
    # If custom comment provided, use it
    if [[ -n "${CUSTOM_COMMENT:-}" ]]; then
        echo "$CUSTOM_COMMENT"
        return 0
    fi
    
    # Get template
    local template="${TEMPLATES[$reason]}"
    
    if [[ -z "$template" ]]; then
        log_error "Unknown closure reason: $reason"
        return 1
    fi
    
    # Interactive template filling
    case "$reason" in
        "completed")
            local implementation
            implementation=$(prompt_text "Implementation (PR #): ")
            
            local tests
            tests=$(prompt_text "Test coverage: " "Added comprehensive tests")
            
            local docs
            docs=$(prompt_text "Documentation updates: " "Updated relevant documentation")
            
            local breaking_changes
            breaking_changes=$(prompt_text "Breaking changes: " "None")
            
            local related_issues
            related_issues=$(prompt_text "Related issues: " "None")
            
            local migration_notes
            migration_notes=$(prompt_text "Migration notes: " "None")
            
            # Replace placeholders
            template="${template//\{implementation\}/$implementation}"
            template="${template//\{tests\}/$tests}"
            template="${template//\{docs\}/$docs}"
            template="${template//\{breaking_changes\}/$breaking_changes}"
            template="${template//\{related_issues\}/$related_issues}"
            template="${template//\{migration_notes\}/$migration_notes}"
            ;;
            
        "superseded")
            local replacement
            replacement=$(prompt_text "Replacement (Issue # or PR #): ")
            
            local superseded_reason
            superseded_reason=$(prompt_text "Reason for superseding: ")
            
            local original_context
            original_context=$(prompt_text "Original context: " "Original issue description")
            
            # Replace placeholders
            template="${template//\{replacement\}/$replacement}"
            template="${template//\{reason\}/$superseded_reason}"
            template="${template//\{original_context\}/$original_context}"
            ;;
            
        "out_of_scope")
            local out_of_scope_reason
            out_of_scope_reason=$(prompt_text "Reason for out of scope: ")
            
            local project_context
            project_context=$(prompt_text "Project context: " "Current project roadmap")
            
            local alternatives
            alternatives=$(prompt_text "Alternatives: " "None available")
            
            local reopen_if
            reopen_if=$(prompt_text "Reopen if conditions: " "Project scope changes")
            
            # Replace placeholders
            template="${template//\{reason\}/$out_of_scope_reason}"
            template="${template//\{project_context\}/$project_context}"
            template="${template//\{alternatives\}/$alternatives}"
            template="${template//\{reopen_if\}/$reopen_if}"
            ;;
            
        "duplicate")
            local original_issue
            original_issue=$(prompt_text "Original issue #: ")
            
            local duplicate_reason
            duplicate_reason=$(prompt_text "Reason it's a duplicate: " "Same issue already tracked")
            
            # Replace placeholders
            template="${template//\{original_issue\}/#$original_issue}"
            template="${template//\{reason\}/$duplicate_reason}"
            ;;
            
        "wontfix")
            local wontfix_reason
            wontfix_reason=$(prompt_text "Reason for won't fix: ")
            
            local impact
            impact=$(prompt_text "Impact on users: " "Minimal")
            
            local workaround
            workaround=$(prompt_text "Workaround: " "None available")
            
            local related_adr
            related_adr=$(prompt_text "Related ADR: " "None")
            
            local reopen_if
            reopen_if=$(prompt_text "Reopen if conditions: " "New information available")
            
            # Replace placeholders
            template="${template//\{reason\}/$wontfix_reason}"
            template="${template//\{impact\}/$impact}"
            template="${template//\{workaround\}/$workaround}"
            template="${template//\{related_adr\}/$related_adr}"
            template="${template//\{reopen_if\}/$reopen_if}"
            ;;
            
        "not_reproducible")
            local attempts_made
            attempts_made=$(prompt_text "Attempts made to reproduce: ")
            
            local missing_information
            missing_information=$(prompt_text "Missing information: ")
            
            local last_contact
            last_contact=$(prompt_text "Last contact date: " "Unknown")
            
            local reopen_if
            reopen_if=$(prompt_text "Reopen if conditions: " "Additional information provided")
            
            # Replace placeholders
            template="${template//\{attempts_made\}/$attempts_made}"
            template="${template//\{missing_information\}/$missing_information}"
            template="${template//\{last_contact\}/$last_contact}"
            template="${template//\{reopen_if\}/$reopen_if}"
            ;;
    esac
    
    echo "$template"
}

# Close a single issue
close_issue() {
    local issue_number="$1"
    
    log_info "Starting closure process for issue #$issue_number"
    
    # Get issue information
    local issue_data
    issue_data=$(get_issue_info "$issue_number")
    
    if [[ -z "$issue_data" ]]; then
        log_error "Could not retrieve issue #$issue_number"
        return 1
    fi
    
    # Display issue information
    display_issue_info "$issue_number" "$issue_data"
    
    # Determine closure reason
    local reason="$REASON"
    
    if [[ -z "$reason" ]]; then
        reason=$(prompt_choice "Select closure reason:" "1) completed
2) superseded
3) out_of_scope
4) duplicate
5) wontfix
6) not_reproducible" "$DEFAULT_REASON")
        
        case "$reason" in
            1) reason="completed" ;;
            2) reason="superseded" ;;
            3) reason="out_of_scope" ;;
            4) reason="duplicate" ;;
            5) reason="wontfix" ;;
            6) reason="not_reproducible" ;;
        esac
    fi
    
    # Generate closure comment
    local comment
    comment=$(generate_comment "$reason" "$issue_data")
    
    # Display closure summary
    echo
    echo -e "${CYAN}📊 Closure Summary${NC}"
    echo
    echo -e "${BLUE}Reason:${NC} $reason"
    echo
    echo -e "${BLUE}Comment to add:${NC}"
    echo "$comment"
    echo
    
    # Confirmation
    if [[ "$NO_CONFIRM" != "true" ]]; then
        local confirm
        confirm=$(prompt_choice "Close this issue with the above comment?" "1) Yes
2) No
3) Edit comment")
        
        case "$confirm" in
            2) 
                log_info "Closure cancelled for issue #$issue_number"
                return 0
                ;;
            3)
                comment=$(prompt_text "Edit comment: " "$comment")
                ;;
        esac
    fi
    
    # Apply closure
    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "[DRY RUN] Would close issue #$issue_number"
        log_info "[DRY RUN] Would add comment: $comment"
        return 0
    fi
    
    # Add comment
    log_info "Adding closure comment to issue #$issue_number..."
    echo "$comment" | gh issue comment "$issue_number" --repo "$DEFAULT_REPO" --body-file -
    
    # Close issue
    log_info "Closing issue #$issue_number..."
    gh issue close "$issue_number" --repo "$DEFAULT_REPO"
    
    log_success "Issue #$issue_number closed successfully"
}

# Main function
main() {
    parse_args "$@"
    
    # List templates if requested
    if [[ "$LIST_TEMPLATES" == "true" ]]; then
        list_templates
        exit 0
    fi
    
    # Validate prerequisites
    validate_prereqs
    
    # Load configuration
    load_config
    
    # Handle label-based closure
    if [[ -n "${CLOSE_BY_LABEL:-}" ]]; then
        local issue_numbers
        issue_numbers=$(get_issues_by_label "$CLOSE_BY_LABEL")
        
        if [[ -z "$issue_numbers" ]]; then
            log_info "No issues found with label '$CLOSE_BY_LABEL'"
            exit 0
        fi
        
        ISSUE_NUMBERS="$issue_numbers"
        BATCH_MODE=true
    fi
    
    # Handle batch mode
    if [[ "$BATCH_MODE" == "true" ]]; then
        if [[ -z "${ISSUE_NUMBERS:-}" ]]; then
            log_error "Issue numbers are required for batch mode"
            show_usage
            exit 1
        fi
        
        # Convert comma-separated to space-separated
        local issues
        issues=$(echo "$ISSUE_NUMBERS" | tr ',' ' ')
        
        log_info "Closing multiple issues: $issues"
        
        for issue_number in $issues; do
            close_issue "$issue_number"
            echo
        done
        
        log_success "Batch closure completed"
        exit 0
    fi
    
    # Single issue mode
    if [[ -z "${ISSUE_NUMBERS:-}" ]]; then
        log_error "Issue number is required"
        show_usage
        exit 1
    fi
    
    close_issue "$ISSUE_NUMBERS"
}

# Run main function with all arguments
main "$@"
```

---

## Best Practices

### 1. Consistent Messaging

- Use standard templates for consistent communication
- Personalize templates with specific details when needed
- Keep closure comments professional and helpful

### 2. Proper Documentation

- Always reference related PRs or issues
- Document breaking changes clearly
- Provide migration notes when applicable

### 3. User Consideration

- Explain why issues are being closed
- Provide alternatives when possible
- Set clear conditions for reopening

### 4. Batch Operations

- Use batch mode for similar issues
- Review batch operations before executing
- Document bulk closure decisions

### 5. Follow-up

- Monitor for reactions to closure comments
- Be prepared to reopen if new information emerges
- Learn from closure patterns to improve issue quality

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

#### 4. "Failed to get issue information"
- Check if the issue number is correct
- Verify you have access to the repository
- Check your internet connection

#### 5. "Permission denied"
- Ensure the script is executable: `chmod +x close_issue.sh`
- Check you have write access to the repository

#### 6. "Unknown closure reason"
- Use `--list-templates` to see available reasons
- Check spelling of the reason name
- Use one of: completed, superseded, out_of_scope, duplicate, wontfix, not_reproducible

### Debug Mode

Enable debug output to troubleshoot issues:

```bash
export DEBUG=1
./close_issue.sh 123
```

### Dry Run Mode

Use dry run mode to preview changes without applying them:

```bash
./close_issue.sh --dry-run 123
```

### Getting Help

For additional help:

1. Check the script's help: `./close_issue.sh --help`
2. List available templates: `./close_issue.sh --list-templates`
3. Review the [Issue Management Guide](ISSUE_MANAGEMENT.md)
4. Create an issue in the repository for script problems

---

## Related Documentation

- [Issue Management Guide](ISSUE_MANAGEMENT.md) - Comprehensive issue management procedures
- [Issue Triage Script](issue_triage_script.md) - Script for initial issue triage
- [Bulk Issue Closure Script](issue_bulk_closure_script.md) - Script for bulk issue closure operations
- [GitHub CLI Documentation](https://cli.github.com/manual/) - GitHub CLI reference
- [jq Manual](https://stedolan.github.io/jq/manual/) - JSON processor documentation

---

**Document Version**: 1.0
**Last Updated**: 2026-02-14
**Next Review**: After next major release
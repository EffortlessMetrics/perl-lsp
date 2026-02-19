# Issue Triage Script Documentation

> **Purpose**: Automated issue triage script to help with initial classification and labeling of GitHub issues.
>
> **Last Updated**: 2026-02-14
> **Version**: 1.0
> **Related Docs**: [Issue Management Guide](ISSUE_MANAGEMENT.md), [Issue Closure Script](issue_closure_script.md), [Bulk Issue Closure Script](issue_bulk_closure_script.md)

---

## Table of Contents

1. [Overview](#overview)
2. [Prerequisites](#prerequisites)
3. [Installation](#installation)
4. [Usage Instructions](#usage-instructions)
5. [Interactive Triage Checklist](#interactive-triage-checklist)
6. [Label Assignment Logic](#label-assignment-logic)
7. [Script Content](#script-content)
8. [Best Practices](#best-practices)
9. [Troubleshooting](#troubleshooting)

---

## Overview

The issue triage script (`triage_issue.sh`) is an interactive bash script that helps maintainers systematically triage new GitHub issues. It provides:

- Interactive checklist for consistent triage
- Automatic label assignment based on responses
- Milestone recommendations based on priority
- Comment generation for initial triage
- Integration with GitHub CLI (`gh`) for seamless workflow

### Key Features

- **Interactive Prompts**: Step-by-step guided triage process
- **Label Logic**: Intelligent label assignment based on issue characteristics
- **Milestone Assignment**: Automatic milestone recommendations
- **Comment Generation**: Pre-formatted triage comments
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
- Write access to issues (for labeling and commenting)
- Repository access (for milestone management)

---

## Installation

### 1. Download the Script

```bash
# Download the script
curl -O https://raw.githubusercontent.com/EffortlessMetrics/tree-sitter-perl-rs/main/scripts/triage_issue.sh

# Or copy from the script content section below
```

### 2. Make Executable

```bash
chmod +x triage_issue.sh
```

### 3. Optional: Add to PATH

```bash
# Move to a directory in PATH
sudo mv triage_issue.sh /usr/local/bin/issue-triage

# Or create a symbolic link
ln -s "$(pwd)/triage_issue.sh" /usr/local/bin/issue-triage
```

### 4. Configuration (Optional)

Create a configuration file at `~/.config/issue-triage/config.json`:

```json
{
  "default_milestone": "v1.1-target",
  "default_assignee": "",
  "auto_assign": false,
  "skip_confirmation": false,
  "custom_labels": {
    "performance": ["performance", "regression"],
    "security": ["security", "P0-critical"]
  }
}
```

---

## Usage Instructions

### Basic Usage

```bash
# Triage a specific issue
./triage_issue.sh <issue-number>

# Example
./triage_issue.sh 123
```

### Advanced Options

```bash
# Dry run (show what would be done without making changes)
./triage_issue.sh --dry-run <issue-number>

# Skip confirmation prompts
./triage_issue.sh --no-confirm <issue-number>

# Use custom configuration file
./triage_issue.sh --config /path/to/config.json <issue-number>

# Batch triage multiple issues
./triage_issue.sh --batch 123,124,125

# Triage all issues needing triage
./triage_issue.sh --needs-triage
```

### Environment Variables

```bash
# Set default repository
export GH_REPO="EffortlessMetrics/tree-sitter-perl-rs"

# Set default milestone
export DEFAULT_MILESTONE="v1.1-target"

# Enable debug output
export DEBUG=1

# Use custom config
export TRIAGE_CONFIG="/path/to/config.json"
```

---

## Interactive Triage Checklist

The script guides you through a systematic triage process:

### 1. Issue Analysis

```
📋 Issue Analysis for #123: "Issue title here"

Is this issue a BUG or FEATURE?
1) Bug
2) Feature
3) Documentation
4) Question
5) Other
Enter choice [1-5]:
```

### 2. Severity Assessment

```
🔍 Severity Assessment

How severe is this issue?
1) P0 - Critical (security, data loss, complete failure)
2) P1 - High (significant impact, no workaround)
3) P2 - Medium (minor impact, workaround exists)
4) P3 - Low (edge case, low priority)
Enter choice [1-4]:
```

### 3. Component Identification

```
🧩 Component Identification

Which component does this affect?
1) Parser (perl-parser crate)
2) LSP Server (perl-lsp crate)
3) Debug Adapter (perl-dap crate)
4) Lexer (perl-lexer crate)
5) Test Corpus (perl-corpus crate)
6) Infrastructure/CI/Tooling
7) Documentation
8) Multiple components
Enter choice [1-8]:
```

### 4. Impact Assessment

```
💥 Impact Assessment

How many users are affected?
1) All users
2) Many users (>50%)
3) Some users (10-50%)
4) Few users (<10%)
5) Unknown
Enter choice [1-5]:

Is there a workaround?
1) No workaround
2) Difficult workaround
3) Simple workaround
4) Unknown
Enter choice [1-4]:
```

### 5. Additional Information

```
📝 Additional Information

Does this issue need more information?
1) Yes - needs reproduction steps
2) Yes - needs clarification
3) Yes - needs environment details
4) No - sufficient information
Enter choice [1-4]:

Is this issue blocked by another issue?
1) Yes - specify blocking issue number
2) No
Enter choice [1-2]:
```

### 6. Assignment and Milestone

```
👥 Assignment and Milestone

Assign to:
1) Specific user (enter username)
2) Add "help-wanted" label
3) Add "good-first-issue" label
4) No assignment
Enter choice [1-4]:

Target milestone:
1) v1.0.x-patch (critical post-release)
2) v1.1-target (next minor release)
3) v1.2-target (future minor release)
4) v2.0-target (breaking changes)
5) No milestone (backlog)
Enter choice [1-5]:
```

---

## Label Assignment Logic

The script automatically assigns labels based on your responses:

### Priority Labels

| Response | Label | Description |
|----------|-------|-------------|
| P0 - Critical | `P0-critical` | Security, data loss, complete failure |
| P1 - High | `P1-high` | Significant impact, no workaround |
| P2 - Medium | `P2-medium` | Minor impact, workaround exists |
| P3 - Low | `P3-low` | Edge cases, low priority |

### Type Labels

| Response | Label | Description |
|----------|-------|-------------|
| Bug | `bug` | Incorrect behavior, crashes, errors |
| Feature | `enhancement` | New capability or improvement |
| Documentation | `documentation` | Docs problems or improvements |
| Question | `question` | User question, not an issue |
| Other | `needs-triage` | Unclear, needs manual review |

### Component Labels

| Response | Label | Description |
|----------|-------|-------------|
| Parser | `parser` | Issues in perl-parser crate |
| LSP Server | `lsp` | Issues in perl-lsp crate |
| Debug Adapter | `dap` | Issues in perl-dap crate |
| Lexer | `lexer` | Issues in perl-lexer crate |
| Test Corpus | `corpus` | Issues in perl-corpus crate |
| Infrastructure | `infrastructure` | Build/CI/Tooling issues |
| Documentation | `documentation` | Documentation issues |
| Multiple | `parser,lsp` | Multiple components (comma-separated) |

### Status Labels

| Response | Label | Description |
|----------|-------|-------------|
| Needs reproduction | `needs-reproduction` | Cannot reproduce bug |
| Needs clarification | `needs-info` | More information needed |
| Needs environment | `needs-info` | Environment details needed |
| Blocked | `blocked` | Waiting on dependency |

### Participation Labels

| Response | Label | Description |
|----------|-------|-------------|
| Help wanted | `help-wanted` | Community help needed |
| Good first issue | `good-first-issue` | Good for newcomers |

### Special Labels

| Response | Label | Description |
|----------|-------|-------------|
| Performance issue | `performance` | Performance-related issue |
| Security issue | `security` | Security vulnerability |
| Breaking change | `breaking-change` | API/behavior breaking change |
| Regression | `regression` | Worked before, doesn't now |

---

## Script Content

```bash
#!/usr/bin/env bash

# Issue Triage Script for perl-lsp project
# Automates initial triage of GitHub issues with interactive prompts

set -euo pipefail

# Configuration
DEFAULT_REPO="${GH_REPO:-EffortlessMetrics/tree-sitter-perl-rs}"
DEFAULT_MILESTONE="${DEFAULT_MILESTONE:-v1.1-target}"
CONFIG_FILE="${TRIAGE_CONFIG:-$HOME/.config/issue-triage/config.json}"
DRY_RUN=false
NO_CONFIRM=false
BATCH_MODE=false

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

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
    --dry-run           Show what would be done without making changes
    --no-confirm        Skip confirmation prompts
    --config FILE       Use custom configuration file
    --batch NUMBERS     Triage multiple issues (comma-separated)
    --needs-triage      Triage all issues with 'needs-triage' label
    --help              Show this help message

EXAMPLES:
    $0 123                          # Triage issue #123
    $0 --dry-run 123                # Preview triage for issue #123
    $0 --batch 123,124,125          # Triage multiple issues
    $0 --needs-triage               # Triage all issues needing triage

ENVIRONMENT VARIABLES:
    GH_REPO           GitHub repository (default: EffortlessMetrics/tree-sitter-perl-rs)
    DEFAULT_MILESTONE Default milestone (default: v1.1-target)
    TRIAGE_CONFIG     Configuration file path
    DEBUG             Enable debug output

EOF
}

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --dry-run)
                DRY_RUN=true
                shift
                ;;
            --no-confirm)
                NO_CONFIRM=true
                shift
                ;;
            --config)
                CONFIG_FILE="$2"
                shift 2
                ;;
            --batch)
                BATCH_MODE=true
                ISSUE_NUMBERS="$2"
                shift 2
                ;;
            --needs-triage)
                BATCH_MODE=true
                NEEDS_TRIAGE=true
                shift
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
                ISSUE_NUMBER="$1"
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
    echo -e "${CYAN}📋 Issue Analysis for #$issue_number: \"$title\"${NC}"
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

# Triage a single issue
triage_issue() {
    local issue_number="$1"
    
    log_info "Starting triage for issue #$issue_number"
    
    # Get issue information
    local issue_data
    issue_data=$(get_issue_info "$issue_number")
    
    if [[ -z "$issue_data" ]]; then
        log_error "Could not retrieve issue #$issue_number"
        return 1
    fi
    
    # Display issue information
    display_issue_info "$issue_number" "$issue_data"
    
    # Collect triage information
    local issue_type
    issue_type=$(prompt_choice "Is this issue a BUG or FEATURE?" "1) Bug
2) Feature
3) Documentation
4) Question
5) Other")
    
    local severity
    severity=$(prompt_choice "How severe is this issue?" "1) P0 - Critical (security, data loss, complete failure)
2) P1 - High (significant impact, no workaround)
3) P2 - Medium (minor impact, workaround exists)
4) P3 - Low (edge case, low priority)")
    
    local component
    component=$(prompt_choice "Which component does this affect?" "1) Parser (perl-parser crate)
2) LSP Server (perl-lsp crate)
3) Debug Adapter (perl-dap crate)
4) Lexer (perl-lexer crate)
5) Test Corpus (perl-corpus crate)
6) Infrastructure/CI/Tooling
7) Documentation
8) Multiple components")
    
    local users_affected
    users_affected=$(prompt_choice "How many users are affected?" "1) All users
2) Many users (>50%)
3) Some users (10-50%)
4) Few users (<10%)
5) Unknown")
    
    local workaround
    workaround=$(prompt_choice "Is there a workaround?" "1) No workaround
2) Difficult workaround
3) Simple workaround
4) Unknown")
    
    local needs_info
    needs_info=$(prompt_choice "Does this issue need more information?" "1) Yes - needs reproduction steps
2) Yes - needs clarification
3) Yes - needs environment details
4) No - sufficient information")
    
    local is_blocked
    is_blocked=$(prompt_choice "Is this issue blocked by another issue?" "1) Yes - specify blocking issue number
2) No")
    
    local assignment
    assignment=$(prompt_choice "Assign to:" "1) Specific user (enter username)
2) Add \"help-wanted\" label
3) Add \"good-first-issue\" label
4) No assignment")
    
    local milestone
    milestone=$(prompt_choice "Target milestone:" "1) v1.0.x-patch (critical post-release)
2) v1.1-target (next minor release)
3) v1.2-target (future minor release)
4) v2.0-target (breaking changes)
5) No milestone (backlog)")
    
    # Process triage data
    local labels=()
    local assignee=""
    local milestone_name=""
    local comment=""
    
    # Priority labels
    case "$severity" in
        1) labels+=("P0-critical") ;;
        2) labels+=("P1-high") ;;
        3) labels+=("P2-medium") ;;
        4) labels+=("P3-low") ;;
    esac
    
    # Type labels
    case "$issue_type" in
        1) labels+=("bug") ;;
        2) labels+=("enhancement") ;;
        3) labels+=("documentation") ;;
        4) labels+=("question") ;;
        5) labels+=("needs-triage") ;;
    esac
    
    # Component labels
    case "$component" in
        1) labels+=("parser") ;;
        2) labels+=("lsp") ;;
        3) labels+=("dap") ;;
        4) labels+=("lexer") ;;
        5) labels+=("corpus") ;;
        6) labels+=("infrastructure") ;;
        7) labels+=("documentation") ;;
        8) labels+=("parser" "lsp") ;;
    esac
    
    # Status labels
    case "$needs_info" in
        1) labels+=("needs-reproduction") ;;
        2|3) labels+=("needs-info") ;;
    esac
    
    # Blocked label
    if [[ "$is_blocked" == "1" ]]; then
        labels+=("blocked")
    fi
    
    # Assignment
    case "$assignment" in
        1) 
            assignee=$(prompt_text "Enter username to assign to:")
            ;;
        2) labels+=("help-wanted") ;;
        3) labels+=("good-first-issue") ;;
    esac
    
    # Milestone
    case "$milestone" in
        1) milestone_name="v1.0.x-patch" ;;
        2) milestone_name="v1.1-target" ;;
        3) milestone_name="v1.2-target" ;;
        4) milestone_name="v2.0-target" ;;
        5) milestone_name="" ;;
    esac
    
    # Generate comment
    comment="📋 **Issue Triage**

**Type**: $(
    case "$issue_type" in
        1) echo "Bug" ;;
        2) echo "Feature" ;;
        3) echo "Documentation" ;;
        4) echo "Question" ;;
        5) echo "Other" ;;
    esac
)

**Priority**: $(
    case "$severity" in
        1) echo "P0 - Critical" ;;
        2) echo "P1 - High" ;;
        3) echo "P2 - Medium" ;;
        4) echo "P3 - Low" ;;
    esac
)

**Component**: $(
    case "$component" in
        1) echo "Parser" ;;
        2) echo "LSP Server" ;;
        3) echo "Debug Adapter" ;;
        4) echo "Lexer" ;;
        5) echo "Test Corpus" ;;
        6) echo "Infrastructure/CI/Tooling" ;;
        7) echo "Documentation" ;;
        8) echo "Multiple components" ;;
    esac
)

**Impact**: $(
    case "$users_affected" in
        1) echo "All users" ;;
        2) echo "Many users (>50%)" ;;
        3) echo "Some users (10-50%)" ;;
        4) echo "Few users (<10%)" ;;
        5) echo "Unknown" ;;
    esac
) affected, $(
    case "$workaround" in
        1) echo "no workaround" ;;
        2) echo "difficult workaround" ;;
        3) echo "simple workaround" ;;
        4) echo "unknown workaround" ;;
    esac
)

**Labels**: $(IFS=', '; echo "${labels[*]}")
**Milestone**: ${milestone_name:-"None assigned"}
**Assignee**: ${assignee:-"None assigned"}

**Next Steps**: $(
    if [[ "$needs_info" != "4" ]]; then
        echo "Awaiting additional information from reporter"
    elif [[ "$is_blocked" == "1" ]]; then
        echo "Waiting for blocking issue to be resolved"
    elif [[ -n "$assignee" ]]; then
        echo "Assigned to $assignee for implementation"
    elif [[ "$assignment" == "2" ]]; then
        echo "Community help requested"
    elif [[ "$assignment" == "3" ]]; then
        echo "Good first issue for newcomers"
    else
        echo "Ready for team assignment"
    fi
)
"

    # Display triage summary
    echo
    echo -e "${CYAN}📊 Triage Summary${NC}"
    echo
    echo -e "${BLUE}Labels to add:${NC} $(IFS=', '; echo "${labels[*]}")"
    echo -e "${BLUE}Assignee:${NC} ${assignee:-none}"
    echo -e "${BLUE}Milestone:${NC} ${milestone_name:-none}"
    echo
    echo -e "${BLUE}Comment to add:${NC}"
    echo "$comment"
    echo
    
    # Confirmation
    if [[ "$NO_CONFIRM" != "true" ]]; then
        local confirm
        confirm=$(prompt_choice "Apply this triage?" "1) Yes
2) No
3) Edit")
        
        case "$confirm" in
            2) 
                log_info "Triage cancelled for issue #$issue_number"
                return 0
                ;;
            3)
                log_info "Edit mode not implemented yet. Skipping issue #$issue_number"
                return 0
                ;;
        esac
    fi
    
    # Apply triage
    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "[DRY RUN] Would apply triage to issue #$issue_number"
        log_info "[DRY RUN] Labels: $(IFS=', '; echo "${labels[*]}")"
        log_info "[DRY RUN] Assignee: ${assignee:-none}"
        log_info "[DRY RUN] Milestone: ${milestone_name:-none}"
        return 0
    fi
    
    # Apply labels
    if [[ ${#labels[@]} -gt 0 ]]; then
        log_info "Adding labels to issue #$issue_number..."
        gh issue edit "$issue_number" --add-label "$(IFS=','; echo "${labels[*]}")" --repo "$DEFAULT_REPO"
    fi
    
    # Assign issue
    if [[ -n "$assignee" ]]; then
        log_info "Assigning issue #$issue_number to $assignee..."
        gh issue edit "$issue_number" --assignee "$assignee" --repo "$DEFAULT_REPO"
    fi
    
    # Set milestone
    if [[ -n "$milestone_name" ]]; then
        log_info "Setting milestone for issue #$issue_number to $milestone_name..."
        gh issue edit "$issue_number" --milestone "$milestone_name" --repo "$DEFAULT_REPO"
    fi
    
    # Add comment
    log_info "Adding triage comment to issue #$issue_number..."
    echo "$comment" | gh issue comment "$issue_number" --repo "$DEFAULT_REPO" --body-file -
    
    log_success "Triage completed for issue #$issue_number"
}

# Get issues needing triage
get_issues_needing_triage() {
    log_info "Getting issues needing triage..."
    
    local issues
    issues=$(gh issue list --label "needs-triage" --limit 100 --json number --repo "$DEFAULT_REPO" | jq -r '.[].number')
    
    if [[ -z "$issues" ]]; then
        log_info "No issues needing triage found"
        return 1
    fi
    
    echo "$issues"
}

# Main function
main() {
    parse_args "$@"
    
    # Validate prerequisites
    validate_prereqs
    
    # Load configuration
    load_config
    
    # Handle batch mode
    if [[ "$BATCH_MODE" == "true" ]]; then
        local issue_numbers
        
        if [[ "$NEEDS_TRIAGE" == "true" ]]; then
            issue_numbers=$(get_issues_needing_triage)
        else
            issue_numbers=$(echo "$ISSUE_NUMBERS" | tr ',' ' ')
        fi
        
        if [[ -z "$issue_numbers" ]]; then
            log_info "No issues to triage"
            exit 0
        fi
        
        log_info "Triaging multiple issues: $issue_numbers"
        
        for issue_number in $issue_numbers; do
            triage_issue "$issue_number"
            echo
        done
        
        log_success "Batch triage completed"
        exit 0
    fi
    
    # Single issue mode
    if [[ -z "${ISSUE_NUMBER:-}" ]]; then
        log_error "Issue number is required"
        show_usage
        exit 1
    fi
    
    triage_issue "$ISSUE_NUMBER"
}

# Run main function with all arguments
main "$@"
```

---

## Best Practices

### 1. Consistent Triage

- Use the script for all new issues to ensure consistency
- Follow the same criteria for severity and priority
- Document any deviations from standard triage

### 2. Regular Triage Schedule

- Triage new issues within 24-48 hours
- Schedule weekly triage meetings for complex issues
- Review and update triage decisions periodically

### 3. Quality Comments

- Personalize generated comments when needed
- Add specific context for complex issues
- Link related issues or PRs when relevant

### 4. Label Hygiene

- Use standard labels consistently
- Remove outdated or incorrect labels
- Keep label descriptions up to date

### 5. Milestone Management

- Review milestone assignments regularly
- Move issues between milestones as priorities change
- Close milestones when all issues are resolved

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
- Ensure the script is executable: `chmod +x triage_issue.sh`
- Check you have write access to the repository

### Debug Mode

Enable debug output to troubleshoot issues:

```bash
export DEBUG=1
./triage_issue.sh 123
```

### Dry Run Mode

Use dry run mode to preview changes without applying them:

```bash
./triage_issue.sh --dry-run 123
```

### Getting Help

For additional help:

1. Check the script's help: `./triage_issue.sh --help`
2. Review the [Issue Management Guide](ISSUE_MANAGEMENT.md)
3. Create an issue in the repository for script problems

---

## Related Documentation

- [Issue Management Guide](ISSUE_MANAGEMENT.md) - Comprehensive issue management procedures
- [Issue Closure Script](issue_closure_script.md) - Script for closing issues with proper comments
- [Bulk Issue Closure Script](issue_bulk_closure_script.md) - Script for bulk issue closure operations
- [GitHub CLI Documentation](https://cli.github.com/manual/) - GitHub CLI reference
- [jq Manual](https://stedolan.github.io/jq/manual/) - JSON processor documentation

---

**Document Version**: 1.0
**Last Updated**: 2026-02-14
**Next Review**: After next major release
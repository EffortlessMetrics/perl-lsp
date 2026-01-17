# GitHub Agent Integration Guide

This document provides practical implementation details for integrating Claude agents with GitHub PR workflows.

## GitHub API Integration

### Required Permissions
- **Issues**: write (for PR comments)
- **Pull requests**: write (for status updates, reviews)
- **Contents**: read (for file access)
- **Actions**: read (for workflow status)

### Authentication
```bash
# Set up GitHub CLI for agent use
gh auth login --scopes "repo,write:discussion"

# Verify permissions
gh api user -q .login
```

## Agent GitHub Commands

### pr-initial-reviewer Integration

```bash
# Leave initial review comment
gh pr comment $PR_NUMBER --body "$(cat <<'EOF'
## Initial Review Summary

**Analysis**: Fast review completed
**Findings**: 
- 3 compilation warnings detected
- 1 test file needs updating  
- Code style looks good

**Recommended Flow**: 
→ `test-runner-analyzer` - Run tests to verify current state
→ `pr-cleanup-agent` - Address compilation warnings

**Status**: Ready for detailed review
EOF
)"

# Add review labels
gh pr edit $PR_NUMBER --add-label "needs-testing,ready-for-review"
```

### test-runner-analyzer Integration

```bash
# Update PR status check
gh api repos/{owner}/{repo}/statuses/{sha} \
  --method POST \
  --field state="pending" \
  --field description="Running local test suite" \
  --field context="claude/test-runner"

# After tests complete
if [ $TEST_EXIT_CODE -eq 0 ]; then
  gh api repos/{owner}/{repo}/statuses/{sha} \
    --method POST \
    --field state="success" \
    --field description="All tests passing locally" \
    --field context="claude/test-runner"
else
  gh api repos/{owner}/{repo}/statuses/{sha} \
    --method POST \
    --field state="failure" \
    --field description="Tests failing - see comment for details" \
    --field context="claude/test-runner"
fi

# Comment with test results
gh pr comment $PR_NUMBER --body "$(cat <<EOF
## Test Results 🧪

**Local Test Suite**: $([ $TEST_EXIT_CODE -eq 0 ] && echo "✅ PASSED" || echo "❌ FAILED")

**Summary**:
- Unit tests: $(echo "$TEST_OUTPUT" | grep -c "test result: ok")
- Integration tests: $(echo "$TEST_OUTPUT" | grep -c "running.*integration")
- Performance tests: $(echo "$TEST_OUTPUT" | grep -c "bench")

$([ $TEST_EXIT_CODE -ne 0 ] && echo "**Failures**:" && echo '```' && echo "$TEST_OUTPUT" && echo '```')

**Next Steps**: 
$([ $TEST_EXIT_CODE -eq 0 ] && echo "→ \`pr-finalize\` - Ready for final review" || echo "→ \`pr-cleanup-agent\` - Fix test failures")
EOF
)"
```

### context-scout Integration

```bash
# Update investigation status
gh pr comment $PR_NUMBER --body "$(cat <<'EOF'
## Code Context Analysis 🔍

**Investigation**: Analyzing parser architecture impact

**Findings**:
- Changes affect 3 core parser modules
- Potential impact on LSP server components  
- No breaking API changes detected
- Test coverage adequate for changed areas

**Architecture Assessment**:
- Parser changes are well-isolated
- Backward compatibility maintained
- Performance impact: minimal expected

**Recommendation**: 
→ `test-runner-analyzer` - Verify performance benchmarks
→ `pr-cleanup-agent` - Minor documentation updates needed
EOF
)"
```

### pr-cleanup-agent Integration

```bash
# Track cleanup progress
CLEANUP_COMMENT_ID=$(gh pr comment $PR_NUMBER --body "## Cleanup Progress 🔧

**Issues Addressed**:
- [ ] Fix compilation warnings
- [ ] Update test files
- [ ] Address reviewer feedback
- [ ] Update documentation

**Status**: In progress..." --jq .id)

# Update progress as issues are fixed
gh api repos/{owner}/{repo}/issues/comments/$CLEANUP_COMMENT_ID \
  --method PATCH \
  --field body="## Cleanup Progress 🔧

**Issues Addressed**:
- [x] Fix compilation warnings ✅
- [x] Update test files ✅  
- [ ] Address reviewer feedback
- [ ] Update documentation

**Status**: 50% complete

**Next**: → \`test-runner-analyzer\` - Verify fixes"
```

### pr-finalize Integration

```bash
# Final approval comment
gh pr comment $PR_NUMBER --body "$(cat <<'EOF'
## Final Review Complete ✅

**Quality Checks**:
- ✅ All tests passing
- ✅ No compilation warnings
- ✅ Documentation updated
- ✅ API contracts verified
- ✅ Performance benchmarks stable

**Merge Strategy**: Squash merge recommended
**Breaking Changes**: None
**Release Impact**: Patch version bump suggested

**Status**: Ready for merge 🚀
→ `pr-merger` - Execute merge
EOF
)"

# Add final approval
gh pr review $PR_NUMBER --approve --body "Comprehensive review complete. All quality gates passed."
```

### pr-merger Integration

```bash
# Execute merge
MERGE_SHA=$(gh pr merge $PR_NUMBER --squash --auto --delete-branch --body "$(cat <<'EOF'
Comprehensive review completed by Claude agents:
- Initial review: Issues identified and resolved
- Testing: All local tests passing  
- Context analysis: No architectural concerns
- Cleanup: All feedback addressed
- Final review: Quality gates passed

Co-authored-by: claude-agents <claude@anthropic.com>
EOF
)")

# Update with merge details
gh pr comment $PR_NUMBER --body "## Merge Complete 🎉

**Merge SHA**: $MERGE_SHA
**Strategy**: Squash merge
**Branch**: Deleted automatically

**Post-merge Actions**:
- [ ] Documentation updates
- [ ] Release notes preparation
- [ ] Integration verification

**Next**: → \`pr-doc-finalize\` - Comprehensive documentation review"
```

### pr-doc-finalize Integration

```bash
# Document improvements made
gh pr comment $PR_NUMBER --body "$(cat <<'EOF'
## Documentation Finalized 📚

**Updates Made**:
- Updated API reference for new parser methods
- Enhanced tutorial with new features
- Added troubleshooting guide entries
- Improved code examples

**Diataxis Framework Applied**:
- **Tutorial**: Updated getting started guide
- **How-to**: Added parser configuration guide  
- **Reference**: Updated API documentation
- **Explanation**: Enhanced architecture docs

**Quality Improvements**:
- Fixed 3 broken internal links
- Standardized code formatting
- Added missing parameter descriptions
- Improved navigation structure

**Status**: Documentation review complete ✨
EOF
)"
```

## Workflow Status Management

### PR Status Tracking
```bash
# Set status for agent workflow
function set_agent_status() {
  local agent=$1
  local state=$2  # pending, success, failure
  local description=$3
  
  gh api repos/{owner}/{repo}/statuses/$SHA \
    --method POST \
    --field state="$state" \
    --field description="$description" \
    --field context="claude/$agent"
}

# Usage examples
set_agent_status "pr-initial" "success" "Initial review complete"
set_agent_status "test-runner" "pending" "Running test suite"
set_agent_status "cleanup" "success" "All issues resolved"
```

### Progress Labels
```bash
# Workflow stage labels
gh pr edit $PR_NUMBER --add-label "claude/initial-review"
gh pr edit $PR_NUMBER --remove-label "claude/initial-review" --add-label "claude/testing"  
gh pr edit $PR_NUMBER --remove-label "claude/testing" --add-label "claude/cleanup"
gh pr edit $PR_NUMBER --remove-label "claude/cleanup" --add-label "claude/finalized"
gh pr edit $PR_NUMBER --remove-label "claude/finalized" --add-label "claude/merged"
```

## Error Handling and Recovery

### Agent Failure Recovery
```bash
# When agent cannot complete task
function agent_handoff() {
  local agent=$1
  local reason=$2
  local next_steps=$3
  
  gh pr comment $PR_NUMBER --body "## Agent Handoff Required 🔄

**Agent**: $agent
**Status**: Unable to complete task
**Reason**: $reason

**Current Progress**: Saved to branch
**Recovery Steps**: $next_steps

**Manual Intervention**: Required
**Resume Point**: Available for continuation"

  # Set failure status
  set_agent_status "$agent" "failure" "Manual intervention required"
  
  # Push current progress
  git push origin HEAD
}

# Usage
agent_handoff "pr-cleanup" "Complex merge conflicts" "Resolve conflicts manually, then resume with test-runner-analyzer"
```

### Pause and Resume Workflow
```bash
# Pause workflow
function pause_workflow() {
  local reason=$1
  
  gh pr comment $PR_NUMBER --body "## Workflow Paused ⏸️

**Reason**: $reason
**Current State**: Progress saved
**Resume Instructions**: Use appropriate agent command to continue

**Available Resume Points**:
- \`pr-initial-reviewer\` - Start fresh review
- \`test-runner-analyzer\` - Run tests from current state  
- \`pr-cleanup-agent\` - Continue cleanup from current progress
- \`context-scout\` - Gather additional context"

  # Add pause label
  gh pr edit $PR_NUMBER --add-label "claude/paused"
}

# Resume workflow
function resume_workflow() {
  gh pr edit $PR_NUMBER --remove-label "claude/paused"
  gh pr comment $PR_NUMBER --body "## Workflow Resumed ▶️

**Status**: Continuing from previous state
**Next Agent**: Will be determined based on current conditions"
}
```

## Agent Orchestration Patterns

### Sequential Flow
```bash
# Standard flow execution
run_initial_review() {
  pr-initial-reviewer && 
  determine_next_agent &&
  execute_next_agent
}

determine_next_agent() {
  # Logic based on initial review results
  if [[ $TESTS_NEEDED == "true" ]]; then
    echo "test-runner-analyzer"
  elif [[ $CONTEXT_NEEDED == "true" ]]; then
    echo "context-scout"
  elif [[ $CLEANUP_NEEDED == "true" ]]; then
    echo "pr-cleanup-agent"  
  else
    echo "pr-finalize"
  fi
}
```

### Loop Handling
```bash
# Cleanup loop with circuit breaker
CLEANUP_ITERATIONS=0
MAX_CLEANUP_ATTEMPTS=3

cleanup_loop() {
  while [[ $CLEANUP_ITERATIONS -lt $MAX_CLEANUP_ATTEMPTS ]]; do
    CLEANUP_ITERATIONS=$((CLEANUP_ITERATIONS + 1))
    
    if pr-cleanup-agent; then
      if test-runner-analyzer; then
        break  # Success, exit loop
      else
        continue  # Tests failed, try cleanup again
      fi
    else
      # Cleanup failed, need manual intervention
      agent_handoff "pr-cleanup" "Repeated cleanup failures" "Review cleanup agent logs and resolve manually"
      return 1
    fi
  done
  
  if [[ $CLEANUP_ITERATIONS -eq $MAX_CLEANUP_ATTEMPTS ]]; then
    agent_handoff "cleanup-loop" "Maximum attempts reached" "Manual review and intervention required"
    return 1
  fi
}
```

This integration guide provides the practical GitHub API commands and workflow patterns needed to implement the agent flow described in the main design document.
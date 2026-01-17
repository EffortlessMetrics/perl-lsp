# Agent Orchestration Guide

This document provides practical guidance for orchestrating Claude agents in the PR review flow.

## Core Orchestration Principles

### 1. Agent Responsibility
Each agent should:
- **Complete its primary function** fully before recommending next steps
- **Leave clear guidance** for the next agent in the chain
- **Update GitHub status** to reflect progress and decisions
- **Provide context** about what was learned/accomplished
- **Be flexible** in recommendations while providing clear default paths

### 2. Flow Direction Logic
Agents guide the orchestrator using this decision framework:

```
Current State + Conditions → Recommended Next Agent

Examples:
- Initial Review + "Tests likely to pass" → pr-finalize  
- Initial Review + "Obvious issues found" → pr-cleanup-agent
- Tests Failing + "Need understanding" → context-scout
- Context Gathered + "Issues identified" → pr-cleanup-agent  
- Cleanup Complete + "Ready to verify" → test-runner-analyzer
- All Green + "No issues" → pr-finalize
```

### 3. Flexibility Over Rigidity
- Agents provide **recommendations**, not commands
- Orchestrator can **override** based on new information  
- Support **jumping ahead** when appropriate (e.g., skip testing if changes are documentation-only)
- Enable **parallel execution** when beneficial
- Allow **human intervention** at any point

## Orchestration Patterns

### Pattern 1: Standard Flow
```
pr-initial-reviewer → test-runner-analyzer → pr-finalize → pr-merger → pr-doc-finalize
```

**When to use**: Clean PRs with no obvious issues
**Agent guidance**: Each agent confirms "proceed to next" in their recommendation

### Pattern 2: Issue Resolution Loop  
```
pr-initial-reviewer → [test-runner-analyzer → context-scout → pr-cleanup-agent]* → pr-finalize
```

**When to use**: PRs with test failures or reviewer feedback
**Loop control**: Circuit breaker after 3 iterations, then pause for manual review

### Pattern 3: Context-First Flow
```
pr-initial-reviewer → context-scout → test-runner-analyzer → pr-finalize  
```

**When to use**: Complex architectural changes needing understanding first
**Agent guidance**: context-scout determines if testing or cleanup needed next

### Pattern 4: Documentation-Only Fast Track
```
pr-initial-reviewer → pr-finalize → pr-merger → pr-doc-finalize
```

**When to use**: Documentation-only changes with no code impact
**Skip condition**: pr-initial-reviewer confirms "no testing needed"

## Agent Communication Protocol

### Handoff Format
Each agent should end with a structured handoff:

```markdown
## Agent Handoff

**Agent**: pr-initial-reviewer  
**Status**: Completed successfully
**Key Findings**: 
- 2 compilation warnings detected
- Test coverage looks adequate  
- No obvious architectural concerns

**Recommended Flow**:
1. **Primary**: test-runner-analyzer (verify compilation warnings don't break tests)
2. **Alternative**: pr-cleanup-agent (fix warnings first, then test)  
3. **Rationale**: Warnings are minor, tests should pass, but good to verify

**GitHub Status**: Updated PR with initial review summary
**Context for Next Agent**: Warnings are in lexer.rs:45 and parser.rs:112
```

### Context Passing
Agents should pass relevant context:

```markdown  
**Context for Next Agent**:
- Files modified: src/lexer.rs, src/parser.rs, tests/integration_tests.rs
- Test scope needed: lexer_tests, parser_tests, integration_tests
- Known issues: Compilation warnings (non-blocking)  
- Performance impact: None expected
- Reviewer concerns: None yet raised
```

### Status Updates
Each agent updates GitHub with:
- **Comment**: Detailed findings and next steps
- **Status check**: Success/failure/pending for their phase
- **Labels**: Add/remove workflow stage labels
- **Assignees**: Tag relevant reviewers if human input needed

## Decision Trees for Orchestration

### Initial Review Decision Tree
```
pr-initial-reviewer completes:
├── "Tests likely to fail" → test-runner-analyzer  
├── "Architecture change unclear" → context-scout
├── "Obvious fixes needed" → pr-cleanup-agent
├── "Documentation only" → pr-finalize
└── "Clean, needs verification" → test-runner-analyzer
```

### Test Results Decision Tree  
```
test-runner-analyzer completes:
├── Tests Pass
│   ├── "Clean PR" → pr-finalize
│   └── "Minor issues noted" → pr-cleanup-agent  
└── Tests Fail
    ├── "Clear failure reason" → pr-cleanup-agent
    ├── "Need context to understand" → context-scout  
    └── "Fundamental issues" → pause workflow, manual review
```

### Cleanup Results Decision Tree
```  
pr-cleanup-agent completes:
├── "All issues resolved" → test-runner-analyzer
├── "Partial resolution" → test-runner-analyzer (verify progress)
├── "Need more context" → context-scout  
└── "Cannot resolve" → pause workflow, manual handoff
```

### Context Analysis Decision Tree
```
context-scout completes:
├── "Clear path to fixes" → pr-cleanup-agent
├── "Need to verify current state" → test-runner-analyzer
├── "Ready for review" → pr-finalize  
└── "Complex architectural concerns" → pause for architect review
```

## Error Handling and Recovery

### Circuit Breaker Pattern
```bash  
# Prevent infinite loops
MAX_ITERATIONS=3
CURRENT_ITERATION=0

while [[ $CURRENT_ITERATION -lt $MAX_ITERATIONS ]]; do
  CURRENT_ITERATION=$((CURRENT_ITERATION + 1))
  
  # Run cleanup → test cycle
  if pr-cleanup-agent && test-runner-analyzer; then
    # Success, break out of loop  
    break
  fi
  
  # If final iteration and still failing
  if [[ $CURRENT_ITERATION -eq $MAX_ITERATIONS ]]; then
    pause_workflow "Maximum cleanup iterations reached"
    exit 1
  fi
done
```

### Graceful Degradation
When agents encounter insurmountable issues:

1. **Save Progress**: Push current branch state
2. **Document State**: Leave detailed GitHub comment about what was attempted
3. **Clear Handoff**: Specify exactly what needs manual intervention
4. **Preserve Context**: Ensure next agent/human has all necessary information
5. **Set Status**: Update GitHub status to indicate manual intervention needed

### Recovery Points  
Define clear recovery points where workflow can be resumed:

- **After Initial Review**: Resume with any agent based on current state
- **After Testing**: Resume with cleanup or finalization
- **After Context Gathering**: Resume with testing or cleanup  
- **After Partial Cleanup**: Resume testing to verify progress
- **Before Finalization**: Re-run final checks
- **Before Merge**: Verify all conditions still met

## Implementation Examples

### Orchestrator Logic (Pseudocode)
```python
def orchestrate_pr_review(pr_number):
    state = PRState(pr_number)
    
    # Always start with initial review
    result = run_agent("pr-initial-reviewer", state)
    
    while not state.is_complete():
        next_agent = determine_next_agent(result, state)
        
        if next_agent == "pause":
            pause_workflow(state, result.reason)
            break
            
        try:
            result = run_agent(next_agent, state)
            state.update(result)
            
            # Circuit breaker
            if state.loop_count > MAX_LOOPS:
                pause_workflow(state, "Maximum iterations reached")
                break
                
        except AgentFailure as e:
            handle_agent_failure(e, state)
            break
    
    return state

def determine_next_agent(result, state):
    """Determine next agent based on current result and state"""  
    recommendations = result.recommended_flow
    
    # Primary recommendation
    if recommendations.primary and should_follow_primary(state):
        return recommendations.primary
        
    # Alternative path  
    if recommendations.alternative and has_alternative_condition(state):
        return recommendations.alternative
        
    # Default fallback
    return "pr-finalize" if state.all_checks_pass() else "pause"
```

### Human Override Interface
```bash
# Allow human to override agent recommendations
function override_flow() {
  local current_agent=$1
  local recommended_agent=$2
  local override_agent=$3
  
  echo "Agent $current_agent recommends: $recommended_agent"
  echo "Override to: $override_agent? (y/N)"
  
  read -r response
  if [[ $response =~ ^[Yy]$ ]]; then
    gh pr comment $PR_NUMBER --body "**Manual Override**: Proceeding with $override_agent instead of recommended $recommended_agent"
    return $override_agent  
  else
    return $recommended_agent
  fi
}
```

This orchestration framework balances automation with flexibility, ensuring agents can work together effectively while providing clear control points for human intervention when needed.
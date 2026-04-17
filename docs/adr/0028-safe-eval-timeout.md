# ADR-0028: Safe Evaluation Timeout Policy

**Status**: Accepted
**Date**: 2025-02-20
**Decision Makers**: Perl LSP Architecture Team
**Related**: [ADR-0019](0019-security-first-dap.md)

## Context

The Debug Adapter Protocol supports expression evaluation during debugging sessions. This powerful feature allows developers to inspect variables, call functions, and test code snippets. However, it also introduces security and stability risks:

1. **Denial of Service**: Malicious or buggy expressions can hang indefinitely
   ```perl
   # Infinite loop
   while(1) {}
   
   # Expensive computation
   my $n = 1; $n *= ++$n while $n < 1e100;
   ```

2. **Resource Exhaustion**: Expressions can consume excessive memory or CPU
   ```perl
   # Memory bomb
   my @arr = (1) x 1_000_000_000;
   ```

3. **Interactive Debugging**: Long evaluations block the debugging session

### The Timeout Dilemma

Setting appropriate timeouts requires balancing:

| Concern | Short Timeout | Long Timeout |
|---------|---------------|--------------|
| DoS Prevention | ✅ Better | ❌ Worse |
| Complex Expressions | ❌ May timeout | ✅ More time |
| User Experience | ❌ Frustrating | ✅ Flexible |
| Server Stability | ✅ Protected | ❌ Vulnerable |

## Decision

**We enforce a 5-second default timeout with 300-second (5-minute) maximum for expression evaluation, preventing DoS while maintaining debugging usability.**

### Timeout Configuration

```rust
/// Default timeout in milliseconds (5 seconds)
pub const DEFAULT_TIMEOUT_MS: u32 = 5_000;

/// Maximum allowed timeout in milliseconds (5 minutes)
pub const MAX_TIMEOUT_MS: u32 = 300_000;

/// Evaluation timeout configuration
#[derive(Clone, Debug)]
pub struct EvaluationTimeout {
    /// Configured timeout in milliseconds
    pub timeout_ms: u32,
    /// Whether timeout is user-configurable
    pub configurable: bool,
}

impl Default for EvaluationTimeout {
    fn default() -> Self {
        Self {
            timeout_ms: DEFAULT_TIMEOUT_MS,
            configurable: true,
        }
    }
}

impl EvaluationTimeout {
    /// Create timeout with validation
    pub fn new(timeout_ms: u32) -> Result<Self, String> {
        if timeout_ms > MAX_TIMEOUT_MS {
            return Err(format!(
                "Timeout cannot exceed {} milliseconds (5 minutes)",
                MAX_TIMEOUT_MS
            ));
        }
        Ok(Self {
            timeout_ms,
            configurable: true,
        })
    }
}
```

### Enforcement Implementation

```rust
impl DebugAdapter {
    /// Handle evaluate request with safe evaluation mode and timeout enforcement
    pub fn handle_evaluate(
        &mut self,
        args: EvaluateArguments,
    ) -> Result<EvaluateResponse, DapError> {
        // Validate timeout
        let timeout_ms = args.timeout_ms
            .unwrap_or(DEFAULT_TIMEOUT_MS)
            .min(MAX_TIMEOUT_MS);
        
        // Create timeout context
        let timeout = Duration::from_millis(timeout_ms as u64);
        
        // Execute with timeout
        let result = self.execute_with_timeout(&args.expression, timeout)?;
        
        Ok(EvaluateResponse {
            result,
            variables_reference: 0,
            ..Default::default()
        })
    }
    
    fn execute_with_timeout(
        &self,
        expression: &str,
        timeout: Duration,
    ) -> Result<String, DapError> {
        // Use crossbeam for timeout enforcement
        crossbeam::scope(|s| {
            let handle = s.spawn(|_| {
                self.perl_eval(expression)
            });
            
            match handle.join_timeout(timeout) {
                Ok(result) => result,
                Err(_) => Err(DapError::Timeout {
                    message: format!("evaluate timed out after {}ms", timeout.as_millis()),
                }),
            }
        }).map_err(|_| DapError::Timeout {
            message: "Evaluation thread panicked".to_string(),
        })?
    }
}
```

### Error Response

```rust
/// Timeout error response
fn timeout_error(timeout_ms: u32) -> Message {
    Message {
        typ: MessageType::ERROR,
        message: Some(format!(
            "Timeout cannot exceed {} milliseconds (5 minutes)",
            timeout_ms
        )),
        ..Default::default()
    }
}
```

### Timeout Values Rationale

| Timeout | Use Case | Rationale |
|---------|----------|-----------|
| 5s (default) | Most evaluations | Covers typical variable inspection, simple expressions |
| 30s | Complex expressions | Allows for database queries, file operations |
| 60s | Heavy computation | Complex data transformations |
| 300s (max) | Extreme cases | Upper bound to prevent indefinite hangs |

### Client Configuration

```json
{
  "perl.debug.evaluationTimeout": 5000,
  "perl.debug.maxEvaluationTimeout": 300000
}
```

### Security Limitation

**Important**: Safe evaluation mode provides syntactic validation (admission control) for expressions,
blocking known dangerous operations. However, it does **not provide interpreter isolation** or OS-level
sandboxing. It is one layer of defense alongside timeout enforcement.

## Consequences

### Positive

- **DoS Prevention**: Malicious expressions cannot hang the server
- **Resource Protection**: CPU and memory bounded by timeout
- **Predictable Behavior**: Clear timeout expectations for users
- **Configurable**: Users can adjust for legitimate complex evaluations
- **Safe Default**: 5-second default covers most debugging scenarios

### Negative

- **False Timeouts**: Complex legitimate expressions may timeout
- **Configuration Burden**: Users may need to adjust for specific cases
- **Incomplete Evaluation**: Long-running evaluations return partial results
- **Thread Management**: Timeout enforcement requires thread coordination

### Mitigations

- Clear error messages explaining timeout and how to increase
- Progress reporting for long evaluations
- Cancellation support for user-aborted evaluations
- Documentation of timeout configuration options

## References

- [crates/perl-dap-security/src/lib.rs](../../crates/perl-dap-security/src/lib.rs) - Security constants
- [crates/perl-dap/src/debug_adapter.rs](../../crates/perl-dap/src/debug_adapter.rs) - Timeout enforcement
- [crates/perl-dap/src/configuration.rs](../../crates/perl-dap/src/configuration.rs) - Configuration validation
- [ADR-0019: Security-First DAP](0019-security-first-dap.md) - Security framework
- [DAP Security Specification](../DAP_SECURITY_SPECIFICATION.md)

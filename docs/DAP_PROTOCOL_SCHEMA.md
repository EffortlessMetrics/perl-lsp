# DAP Protocol Schema Specification
<!-- Labels: protocol:dap, schema:json-rpc, specification:complete -->

**Issue**: #207 - Debug Adapter Protocol Support
**Status**: Schema Definitions Complete
**Version**: 0.9.x (Production-Ready)
**Date**: 2025-10-04

---

## Executive Summary

This specification defines the complete JSON-RPC 2.0 message schemas for the Debug Adapter Protocol (DAP) implementation. All schemas follow the DAP 1.x specification with extensions for Perl-specific features.

**Transport**: JSON-RPC 2.0 over stdio with `Content-Length` headers
**Message Types**: Request, Response, Event
**Core Requests**: 15 request types (initialize, launch, attach, breakpoints, control flow, variables, evaluate)
**Events**: 5 event types (initialized, stopped, continued, terminated, output)

---

## 1. Base Protocol Types

### 1.1 Message Transport

```typescript
// Content-Length header format
Content-Length: <length>\r\n
\r\n
<JSON message>

// Example:
Content-Length: 123\r\n
\r\n
{"seq":1,"type":"request","command":"initialize","arguments":{}}
```

### 1.2 Request Message

```json
{
  "seq": 1,
  "type": "request",
  "command": "initialize",
  "arguments": {
    // Command-specific arguments
  }
}
```

**JSON Schema**:
```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "DapRequest",
  "type": "object",
  "required": ["seq", "type", "command"],
  "properties": {
    "seq": {
      "type": "integer",
      "description": "Sequence number (incremented for each message)"
    },
    "type": {
      "type": "string",
      "const": "request"
    },
    "command": {
      "type": "string",
      "description": "Command name (e.g., initialize, launch, setBreakpoints)"
    },
    "arguments": {
      "type": "object",
      "description": "Command-specific arguments"
    }
  }
}
```

### 1.3 Response Message

```json
{
  "seq": 1,
  "type": "response",
  "request_seq": 1,
  "success": true,
  "command": "initialize",
  "message": "Optional error message",
  "body": {
    // Command-specific response body
  }
}
```

**JSON Schema**:
```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "DapResponse",
  "type": "object",
  "required": ["seq", "type", "request_seq", "success", "command"],
  "properties": {
    "seq": {
      "type": "integer"
    },
    "type": {
      "type": "string",
      "const": "response"
    },
    "request_seq": {
      "type": "integer",
      "description": "Sequence number of corresponding request"
    },
    "success": {
      "type": "boolean"
    },
    "command": {
      "type": "string"
    },
    "message": {
      "type": "string",
      "description": "Error message (if success=false)"
    },
    "body": {
      "type": "object",
      "description": "Command-specific response body"
    }
  }
}
```

### 1.4 Event Message

```json
{
  "seq": 2,
  "type": "event",
  "event": "stopped",
  "body": {
    // Event-specific body
  }
}
```

**JSON Schema**:
```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "DapEvent",
  "type": "object",
  "required": ["seq", "type", "event"],
  "properties": {
    "seq": {
      "type": "integer"
    },
    "type": {
      "type": "string",
      "const": "event"
    },
    "event": {
      "type": "string",
      "description": "Event name (e.g., initialized, stopped, terminated)"
    },
    "body": {
      "type": "object",
      "description": "Event-specific body"
    }
  }
}
```

---

## 2. Initialization Requests

### 2.1 initialize Request

**Purpose**: Capability negotiation between adapter and client

```json
{
  "seq": 1,
  "type": "request",
  "command": "initialize",
  "arguments": {
    "clientID": "vscode",
    "clientName": "Visual Studio Code",
    "adapterID": "perl-rs",
    "locale": "en-US",
    "linesStartAt1": true,
    "columnsStartAt1": true,
    "pathFormat": "path",
    "supportsVariableType": true,
    "supportsVariablePaging": false,
    "supportsRunInTerminalRequest": false,
    "supportsMemoryReferences": false,
    "supportsProgressReporting": false,
    "supportsInvalidatedEvent": false
  }
}
```

**Response**:
```json
{
  "seq": 1,
  "type": "response",
  "request_seq": 1,
  "success": true,
  "command": "initialize",
  "body": {
    "supportsConfigurationDoneRequest": true,
    "supportsEvaluateForHovers": true,
    "supportsStepInTargetsRequest": false,
    "supportsSetVariable": false,
    "supportsConditionalBreakpoints": false,
    "supportsExceptionBreakpoints": false,
    "supportsDataBreakpoints": false,
    "supportsLogPoints": false,
    "supportsTerminateRequest": true,
    "supportsRestartRequest": false
  }
}
```

### 2.2 initialized Event

**Purpose**: Signals adapter is ready to accept configuration requests

```json
{
  "seq": 2,
  "type": "event",
  "event": "initialized"
}
```

---

## 3. Launch/Attach Requests

### 3.1 launch Request

**Purpose**: Start debugging session by launching Perl script

```json
{
  "seq": 3,
  "type": "request",
  "command": "launch",
  "arguments": {
    "program": "/workspace/script.pl",
    "args": ["--verbose", "input.txt"],
    "perlPath": "/usr/bin/perl",
    "includePaths": ["/workspace/lib", "/custom/lib"],
    "env": {
      "PERL5LIB": "/custom/lib",
      "DEBUG": "1"
    },
    "cwd": "/workspace",
    "stopOnEntry": false,
    "__restart": null
  }
}
```

**Arguments Schema**:
```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "LaunchRequestArguments",
  "type": "object",
  "required": ["program"],
  "properties": {
    "program": {
      "type": "string",
      "description": "Absolute path to Perl script to debug"
    },
    "args": {
      "type": "array",
      "items": {"type": "string"},
      "description": "Command-line arguments passed to script"
    },
    "perlPath": {
      "type": "string",
      "default": "perl",
      "description": "Path to Perl executable"
    },
    "includePaths": {
      "type": "array",
      "items": {"type": "string"},
      "description": "Additional @INC paths"
    },
    "env": {
      "type": "object",
      "additionalProperties": {"type": "string"},
      "description": "Environment variables"
    },
    "cwd": {
      "type": "string",
      "description": "Working directory"
    },
    "stopOnEntry": {
      "type": "boolean",
      "default": false,
      "description": "Automatically stop after launch"
    }
  }
}
```

**Response**:
```json
{
  "seq": 3,
  "type": "response",
  "request_seq": 3,
  "success": true,
  "command": "launch"
}
```

### 3.2 attach Request

**Purpose**: Attach to running Perl process (TCP mode)

```json
{
  "seq": 4,
  "type": "request",
  "command": "attach",
  "arguments": {
    "host": "localhost",
    "port": 5000,
    "pathMapping": {
      "/workspace": "/remote/workspace"
    }
  }
}
```

---

## 4. Breakpoint Requests

### 4.1 setBreakpoints Request

**Purpose**: Set/clear breakpoints in source file

```json
{
  "seq": 5,
  "type": "request",
  "command": "setBreakpoints",
  "arguments": {
    "source": {
      "path": "/workspace/lib/Module.pm",
      "name": "Module.pm"
    },
    "breakpoints": [
      {"line": 10, "column": 0},
      {"line": 25, "column": 0},
      {"line": 100, "column": 0}
    ],
    "sourceModified": false
  }
}
```

**Response** (AC7 - Breakpoint verification):
```json
{
  "seq": 5,
  "type": "response",
  "request_seq": 5,
  "success": true,
  "command": "setBreakpoints",
  "body": {
    "breakpoints": [
      {
        "id": 1,
        "verified": true,
        "line": 10,
        "column": 0
      },
      {
        "id": 2,
        "verified": true,
        "line": 25,
        "column": 0
      },
      {
        "id": 3,
        "verified": false,
        "line": 100,
        "column": 0,
        "message": "Line contains only comments"
      }
    ]
  }
}
```

**Breakpoint Schema**:
```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "Breakpoint",
  "type": "object",
  "required": ["id", "verified", "line"],
  "properties": {
    "id": {
      "type": "integer",
      "description": "Unique breakpoint identifier"
    },
    "verified": {
      "type": "boolean",
      "description": "Breakpoint successfully set"
    },
    "line": {
      "type": "integer",
      "description": "Actual line number (may differ from requested)"
    },
    "column": {
      "type": "integer",
      "description": "Actual column (optional)"
    },
    "message": {
      "type": "string",
      "description": "Error message if not verified"
    }
  }
}
```

### 4.2 breakpoint Event

**Purpose**: Notify client of breakpoint verification changes

```json
{
  "seq": 6,
  "type": "event",
  "event": "breakpoint",
  "body": {
    "reason": "changed",
    "breakpoint": {
      "id": 1,
      "verified": true,
      "line": 10
    }
  }
}
```

---

## 5. Execution Control Requests

### 5.1 continue Request

**Purpose**: Resume execution until next breakpoint

```json
{
  "seq": 7,
  "type": "request",
  "command": "continue",
  "arguments": {
    "threadId": 1
  }
}
```

**Response** (AC9 - <100ms p95 latency):
```json
{
  "seq": 7,
  "type": "response",
  "request_seq": 7,
  "success": true,
  "command": "continue",
  "body": {
    "allThreadsContinued": true
  }
}
```

### 5.2 next Request

**Purpose**: Step over (execute next line)

```json
{
  "seq": 8,
  "type": "request",
  "command": "next",
  "arguments": {
    "threadId": 1,
    "granularity": "line"
  }
}
```

### 5.3 stepIn Request

**Purpose**: Step into subroutine

```json
{
  "seq": 9,
  "type": "request",
  "command": "stepIn",
  "arguments": {
    "threadId": 1,
    "granularity": "line"
  }
}
```

### 5.4 stepOut Request

**Purpose**: Step out of subroutine

```json
{
  "seq": 10,
  "type": "request",
  "command": "stepOut",
  "arguments": {
    "threadId": 1,
    "granularity": "line"
  }
}
```

### 5.5 pause Request

**Purpose**: Interrupt execution

```json
{
  "seq": 11,
  "type": "request",
  "command": "pause",
  "arguments": {
    "threadId": 1
  }
}
```

---

## 6. Execution State Events

### 6.1 stopped Event

**Purpose**: Notify client that execution has stopped

```json
{
  "seq": 12,
  "type": "event",
  "event": "stopped",
  "body": {
    "reason": "breakpoint",
    "threadId": 1,
    "preserveFocusHint": false,
    "allThreadsStopped": true,
    "hitBreakpointIds": [1]
  }
}
```

**Stopped Reasons**:
- `"breakpoint"`: Hit breakpoint
- `"step"`: Step operation completed
- `"pause"`: Pause request completed
- `"entry"`: stopOnEntry launch option
- `"exception"`: Unhandled exception (future)

### 6.2 continued Event

**Purpose**: Notify client that execution has resumed

```json
{
  "seq": 13,
  "type": "event",
  "event": "continued",
  "body": {
    "threadId": 1,
    "allThreadsContinued": true
  }
}
```

---

## 7. Stack Trace Requests

### 7.1 threads Request

**Purpose**: Retrieve list of threads (Perl: single "Main Thread")

```json
{
  "seq": 14,
  "type": "request",
  "command": "threads"
}
```

**Response**:
```json
{
  "seq": 14,
  "type": "response",
  "request_seq": 14,
  "success": true,
  "command": "threads",
  "body": {
    "threads": [
      {
        "id": 1,
        "name": "Main Thread"
      }
    ]
  }
}
```

### 7.2 stackTrace Request

**Purpose**: Retrieve call stack (AC8)

```json
{
  "seq": 15,
  "type": "request",
  "command": "stackTrace",
  "arguments": {
    "threadId": 1,
    "startFrame": 0,
    "levels": 20
  }
}
```

**Response**:
```json
{
  "seq": 15,
  "type": "response",
  "request_seq": 15,
  "success": true,
  "command": "stackTrace",
  "body": {
    "stackFrames": [
      {
        "id": 1001,
        "name": "Package::subroutine",
        "source": {
          "path": "/workspace/lib/Package.pm",
          "name": "Package.pm"
        },
        "line": 42,
        "column": 0,
        "presentationHint": "normal"
      },
      {
        "id": 1002,
        "name": "main::run",
        "source": {
          "path": "/workspace/script.pl",
          "name": "script.pl"
        },
        "line": 10,
        "column": 0,
        "presentationHint": "normal"
      }
    ],
    "totalFrames": 2
  }
}
```

---

## 8. Variable Inspection Requests

### 8.1 scopes Request

**Purpose**: Get variable scopes for stack frame

```json
{
  "seq": 16,
  "type": "request",
  "command": "scopes",
  "arguments": {
    "frameId": 1001
  }
}
```

**Response**:
```json
{
  "seq": 16,
  "type": "response",
  "request_seq": 16,
  "success": true,
  "command": "scopes",
  "body": {
    "scopes": [
      {
        "name": "Locals",
        "variablesReference": 2001,
        "expensive": false
      },
      {
        "name": "Package",
        "variablesReference": 2002,
        "expensive": false
      }
    ]
  }
}
```

### 8.2 variables Request

**Purpose**: Retrieve variable values with lazy expansion (AC8)

```json
{
  "seq": 17,
  "type": "request",
  "command": "variables",
  "arguments": {
    "variablesReference": 2001,
    "start": 0,
    "count": 100
  }
}
```

**Response** (AC8 - <200ms initial, <100ms per child):
```json
{
  "seq": 17,
  "type": "response",
  "request_seq": 17,
  "success": true,
  "command": "variables",
  "body": {
    "variables": [
      {
        "name": "$x",
        "value": "42",
        "type": "scalar",
        "variablesReference": 0
      },
      {
        "name": "@array",
        "value": "[10 items]",
        "type": "array",
        "variablesReference": 3001
      },
      {
        "name": "%hash",
        "value": "{5 keys}",
        "type": "hash",
        "variablesReference": 3002
      },
      {
        "name": "$large_scalar",
        "value": "This is a very long string that has been truncated…",
        "type": "scalar",
        "variablesReference": 0
      },
      {
        "name": "&coderef",
        "value": "sub { my ($x) = @_; return $x * 2; }",
        "type": "code",
        "variablesReference": 0
      }
    ]
  }
}
```

**Variable Schema**:
```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "Variable",
  "type": "object",
  "required": ["name", "value", "variablesReference"],
  "properties": {
    "name": {
      "type": "string",
      "description": "Variable name ($x, @array, %hash, &coderef)"
    },
    "value": {
      "type": "string",
      "description": "Variable value (truncated to 1KB max)"
    },
    "type": {
      "type": "string",
      "enum": ["scalar", "array", "hash", "code"],
      "description": "Perl variable type"
    },
    "variablesReference": {
      "type": "integer",
      "description": "Reference for child expansion (0 = not expandable)"
    }
  }
}
```

---

## 9. Evaluate Request

### 9.1 evaluate Request

**Purpose**: Evaluate expression in stack frame context (AC10)

```json
{
  "seq": 18,
  "type": "request",
  "command": "evaluate",
  "arguments": {
    "expression": "$x + $y",
    "frameId": 1001,
    "context": "watch",
    "allowSideEffects": false
  }
}
```

**Contexts**:
- `"watch"`: Watch expression
- `"repl"`: REPL console
- `"hover"`: Hover evaluation

**Response** (AC10 - Safe evaluation with timeout):
```json
{
  "seq": 18,
  "type": "response",
  "request_seq": 18,
  "success": true,
  "command": "evaluate",
  "body": {
    "result": "62",
    "type": "scalar",
    "variablesReference": 0
  }
}
```

**Error Response** (side effects without opt-in):
```json
{
  "seq": 19,
  "type": "response",
  "request_seq": 19,
  "success": false,
  "command": "evaluate",
  "message": "Side effects not allowed without allowSideEffects flag",
  "body": {
    "error": {
      "id": 1002,
      "format": "Expression '{expression}' requires side effects",
      "variables": {
        "expression": "$x = 42"
      },
      "showUser": true
    }
  }
}
```

---

## 10. Output Events

### 10.1 output Event

**Purpose**: Send stdout/stderr/console output to client

```json
{
  "seq": 20,
  "type": "event",
  "event": "output",
  "body": {
    "category": "stdout",
    "output": "Hello, world!\n",
    "source": {
      "path": "/workspace/script.pl"
    },
    "line": 5,
    "column": 0
  }
}
```

**Categories**:
- `"stdout"`: Standard output
- `"stderr"`: Standard error
- `"console"`: Debug console
- `"telemetry"`: Telemetry data

---

## 11. Session Termination

### 11.1 disconnect Request

**Purpose**: Terminate debugging session

```json
{
  "seq": 21,
  "type": "request",
  "command": "disconnect",
  "arguments": {
    "restart": false,
    "terminateDebuggee": true
  }
}
```

**Response**:
```json
{
  "seq": 21,
  "type": "response",
  "request_seq": 21,
  "success": true,
  "command": "disconnect"
}
```

### 11.2 terminated Event

**Purpose**: Notify client that debuggee has terminated

```json
{
  "seq": 22,
  "type": "event",
  "event": "terminated",
  "body": {
    "restart": false
  }
}
```

---

## 12. Error Handling

### 12.1 Error Response Format

```json
{
  "seq": 23,
  "type": "response",
  "request_seq": 23,
  "success": false,
  "command": "setBreakpoints",
  "message": "Path traversal detected in breakpoint path",
  "body": {
    "error": {
      "id": 1001,
      "format": "Security violation: attempted path traversal in '{path}'",
      "variables": {
        "path": "/workspace/../../../etc/passwd"
      },
      "showUser": true,
      "sendTelemetry": false
    }
  }
}
```

### 12.2 Error IDs (AC16 - Security validation)

| ID | Category | Description |
|----|----------|-------------|
| 1001 | Security | Path traversal attempt |
| 1002 | Security | Side effects not allowed |
| 1003 | Security | Evaluation timeout (>5s) |
| 1004 | Protocol | Invalid request format |
| 1005 | Protocol | Unknown command |
| 1006 | Runtime | Debuggee terminated |
| 1007 | Runtime | Breakpoint verification failed |

---

## 13. Perl-Specific Extensions

### 13.1 Perl Variable Types

**Standard Perl Sigils**:
- `$scalar`: Scalar variable
- `@array`: Array variable
- `%hash`: Hash variable
- `&coderef`: Subroutine reference
- `*typeglob`: Typeglob (not expanded by default)

### 13.2 Variable Rendering Rules

**Truncation** (AC8):
- Scalars: 1KB max with `…` suffix
- Arrays: `[N items]` summary, lazy child expansion
- Hashes: `{N keys}` summary, lazy child expansion
- Coderefs: B::Deparse representation

**Unicode Safety** (AC16):
- UTF-16 boundary validation (PR #153 infrastructure)
- Emoji and multi-byte character support

### 13.3 Breakpoint Validation (AC7)

**Invalid Breakpoint Locations**:
- Comment-only lines
- Blank lines
- Inside heredocs
- Inside POD documentation
- Inside string literals

**Adjustment Strategy**:
- Search forward (max 5 lines) for executable code
- Return adjusted line number in verification response

---

## 14. References

- [Debug Adapter Protocol Specification](https://microsoft.github.io/debug-adapter-protocol/)
- [JSON-RPC 2.0 Specification](https://www.jsonrpc.org/specification)
- [DAP Implementation Specification](DAP_IMPLEMENTATION_SPECIFICATION.md)
- [DAP Crate Architecture](CRATE_ARCHITECTURE_DAP.md)
- [DAP Security Specification](DAP_SECURITY_SPECIFICATION.md)

---

**End of DAP Protocol Schema Specification**

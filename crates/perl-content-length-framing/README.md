# perl-content-length-framing

Shared Content-Length frame parsing and encoding for JSON-RPC transports.

## Scope

- Accumulate transport bytes and extract complete `Content-Length` frames
- Encode outbound payload bytes into `Content-Length` framed messages
- Apply resynchronization guardrails for malformed or desynchronized streams

## API

- `ContentLengthFramer::push(bytes)`
- `ContentLengthFramer::try_next()`
- `frame(body)`

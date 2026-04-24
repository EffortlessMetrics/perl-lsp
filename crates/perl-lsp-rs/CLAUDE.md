# perl-lsp

LSP server binary and integration tests.

## Test Threading
ALWAYS use threading constraints:
```bash
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs -- --test-threads=2
```

## After Async Migration
- LspServer no longer needs &mut self — use `let server` not `let mut server`
- Test harness uses interior mutability (Arc<Mutex>)

## Verify
```bash
cargo fmt --all
cargo clippy -p perl-lsp-rs --tests
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs -- --test-threads=2
```

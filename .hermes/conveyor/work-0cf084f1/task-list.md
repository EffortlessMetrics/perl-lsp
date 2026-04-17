# Task List — work-0cf084f1

## Remaining Work: Fix Test Compilation Error

The implementation is already done in `workspace.rs:1613-1670` (merged in commit `a7344b84`).
The test file has a compilation error that must be fixed to verify the implementation.

### Tasks

- [ ] 1. **Fix `&perms` typo at `workspace_permission_denied_test.rs:92`**
       Change `std::fs::set_permissions(&probe, &perms)` → `std::fs::set_permissions(&probe, perms)`
       The `set_permissions` function takes `&Permissions`, not `&&Permissions`.

- [ ] 2. **Fix `&perms` typo at `workspace_permission_denied_test.rs:97`**
       Same fix — `std::fs::set_permissions(&probe, &perms)` → `std::fs::set_permissions(&probe, perms)`
       This is in the `can_create_permission_denied()` helper cleanup path.

- [ ] 3. **Verify tests compile**
       Run: `cargo test -p perl-lsp-rs --test workspace_permission_denied_test --no-run`
       Confirms the fix resolves the compilation error.

- [ ] 4. **Run tests (non-root environments only)**
       Run: `cargo test -p perl-lsp-rs --test workspace_permission_denied_test`
       Tests skip gracefully when running as root (permission checks bypassed by OS).

### Optional Improvements (Non-Blocking)

- [ ] 5. **Address test timing race**
       The tests use `std::thread::sleep(Duration::from_secs(3))` before draining notifications.
       A polling loop with a timeout would be more robust on loaded CI.

- [ ] 6. **Add guard to `make_workspace_with_permission_denied_file()`**
       This helper doesn't check `can_create_permission_denied()` before creating a misleadingly-writable workspace.
       Should skip or bail early when running as root.

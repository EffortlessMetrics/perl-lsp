//! Test-only helpers for mocking process exit status across platforms.

// Cross-platform helpers for synthesizing `ExitStatus` in tests/mocks.
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt as _;
#[cfg(windows)]
use std::os::windows::process::ExitStatusExt as _;

#[cfg(unix)]
#[inline]
fn raw_exit(code: i32) -> i32 {
    code << 8
}

#[cfg(windows)]
#[inline]
fn raw_exit(code: i32) -> u32 {
    code as u32
}

#[cfg(not(any(unix, windows)))]
compile_error!("Add raw_exit() mapping for this platform.");

#[inline]
pub(super) fn mock_status(code: i32) -> std::process::ExitStatus {
    std::process::ExitStatus::from_raw(raw_exit(code))
}

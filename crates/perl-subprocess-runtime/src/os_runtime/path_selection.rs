use std::path::Path;

/// Priority score for a resolved candidate path: `.exe` beats `.com` beats
/// `.cmd` beats `.bat` beats any other extension beats no extension.
///
/// This function is cross-platform so that `select_path_candidate` can be
/// exercised on Linux CI without requiring a Windows runner.  The extension
/// values are Windows-specific, but the comparison logic is pure Rust.
///
/// On non-Windows builds the function is not called from production code
/// (only from tests) — the `allow(dead_code)` suppresses the resulting lint.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn candidate_priority(candidate: &str) -> u8 {
    match Path::new(candidate)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
    {
        Some(ext) if ext == "exe" => 5,
        Some(ext) if ext == "com" => 4,
        Some(ext) if ext == "cmd" => 3,
        Some(ext) if ext == "bat" => 2,
        Some(_) => 1,
        None => 0,
    }
}

/// Pure selection function: given a list of fully-resolved candidate paths and
/// the current working directory, return the best candidate that is **not**
/// located under `cwd`, applying `candidate_priority` to break ties.
///
/// This function is extracted for unit-testability: callers inject arbitrary
/// candidate lists and CWD values so the security invariant can be verified
/// without touching the real file system or environment.  Being cross-platform
/// also lets the ripr quality gate observe call paths on Linux CI runners.
///
/// ## Security invariant (two layers of defense)
///
/// 1. **Absolute-only**: any candidate that is not an absolute path is dropped
///    immediately.  A relative candidate (e.g. produced by an empty `;;` PATH
///    entry via `dir.join`) resolves against the CWD and is indistinguishable
///    from a planted binary.  This is the primary guard against the
///    empty-PATH-entry bypass.
/// 2. **CWD exclusion**: candidates whose parent directory canonicalizes to `cwd`
///    are dropped.  If canonicalize fails (non-existent dir in a unit test)
///    the raw path is compared instead — production candidates always come from
///    real `.is_file()`-verified PATH dirs where canonicalize succeeds.
///
/// Returns `None` when every candidate is filtered out or when `candidates` is
/// empty (tool genuinely not on PATH; better than running a planted binary).
///
/// On non-Windows builds the function is not called from production code
/// (only from tests) — the `allow(dead_code)` suppresses the resulting lint.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn select_path_candidate(candidates: &[&str], cwd: &Path) -> Option<String> {
    let cwd_canon = std::fs::canonicalize(cwd).unwrap_or_else(|_| cwd.to_path_buf());

    candidates
        .iter()
        .filter(|&&c| {
            // Layer 1: reject any candidate that is not absolute.  A relative
            // path (e.g. produced by an empty `;;` PATH entry via `dir.join`)
            // would silently resolve against the CWD and is indistinguishable
            // from a planted binary.  This is the primary fix for the
            // empty-PATH-entry bypass.
            if !Path::new(c).is_absolute() {
                return false;
            }
            // Layer 2: reject if the parent directory canonicalizes to the CWD.
            // If canonicalize fails (directory does not exist on disk) fall back
            // to the raw path for comparison — the candidate will not have passed
            // `.is_file()` in `resolve_windows_program` anyway, so this path is
            // only reachable in unit tests with synthetic candidate lists.
            let candidate_parent = Path::new(c).parent().unwrap_or(Path::new(""));
            let parent_canon = std::fs::canonicalize(candidate_parent)
                .unwrap_or_else(|_| candidate_parent.to_path_buf());
            parent_canon != cwd_canon
        })
        .max_by_key(|&&c| candidate_priority(c))
        .map(|&s| s.to_string())
}

//! Git staged-tree plumbing for the commit-tier gate (issue #3786).
//!
//! Every commit-tier check must inspect the **staged tree** — the exact set
//! of blobs `git commit` would record right now — never the working tree and
//! never an unstaged edit. The identity of "what's being committed" is
//! `git write-tree`'s output, not `git status` or `fs::read_to_string`.
//!
//! This matters concretely for executable-bit policy: `core.fileMode=false`
//! (set on this repo, and common on Windows checkouts) makes the filesystem's
//! permission bits unreliable — a file can show as executable on disk without
//! that ever being staged, or vice versa. `git ls-tree` on the written tree
//! reports the mode git actually recorded (`100644` vs `100755`), which is
//! the only reliable source for that policy.
//!
//! # Read the captured snapshot, not the live index
//!
//! [`staged_tree_oid`] is called exactly once per run (`plan_gates`) and its
//! result is threaded through every later check invocation
//! (`GatePlan.staged_tree_oid` → `AgentReceipt.staged_tree_oid` →
//! `commit_checks::run_named_check`'s `tree_oid` parameter). [`staged_diff_paths`]
//! and [`read_staged_path_text`] both take an `Option<&str>` tree OID for
//! exactly this reason: passing the captured OID makes them read from that
//! frozen tree object (`git diff HEAD <oid>`, `git show <oid>:path`) instead
//! of the live index (`git diff --cached`, `git show :path`). A concurrent
//! `git add` between `plan_gates` capturing the OID and a check running
//! would otherwise make different checks — or a check and the receipt that
//! records what ran — inspect different states of the same commit. `None`
//! is a defensive fallback for callers outside a real plan (e.g. ad hoc
//! testing); every production call path has a captured OID to pass.

use color_eyre::eyre::{Context, Result, bail};
use std::path::Path;
use std::process::Command;

/// One entry from `git ls-tree -r` over the staged tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedEntry {
    /// Octal mode string as recorded by git, e.g. `"100644"` or `"100755"`.
    pub mode: String,
    pub blob_oid: String,
    /// Repo-relative path, forward-slash separated (git's native form).
    pub path: String,
}

impl StagedEntry {
    pub fn is_executable(&self) -> bool {
        self.mode == "100755"
    }
}

fn run_git_ok(root: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .with_context(|| format!("failed to spawn `git {}`", args.join(" ")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("`git {}` failed: {stderr}", args.join(" "));
    }
    String::from_utf8(output.stdout)
        .with_context(|| format!("`git {}` output was not UTF-8", args.join(" ")))
}

/// The OID of the tree that `git commit` would record right now — i.e.
/// exactly the staged index, independent of the working tree. This is the
/// commit tier's identity: every check keys off this OID, never a working
/// -tree walk.
pub fn staged_tree_oid(root: &Path) -> Result<String> {
    Ok(run_git_ok(root, &["write-tree"])?.trim().to_string())
}

/// List every entry (mode + blob OID + path) in the staged tree.
///
/// This is the *whole* tree, not just what changed in this commit — use
/// [`staged_diff_paths`] to scope a check to the files actually being
/// committed (the common case; scanning the whole tree on every commit would
/// blow the commit-tier time budget on a large repo). Already frozen-tree
/// -based by construction: `tree_oid` names the exact tree object to list,
/// never the live index.
pub fn list_staged_entries(root: &Path, tree_oid: &str) -> Result<Vec<StagedEntry>> {
    let raw = run_git_ok(root, &["ls-tree", "-r", "-z", tree_oid])?;
    let mut entries = Vec::new();
    for record in raw.split('\0') {
        if record.is_empty() {
            continue;
        }
        // Format: "<mode> <type> <oid>\t<path>"
        let Some((meta, path)) = record.split_once('\t') else { continue };
        let mut fields = meta.split(' ');
        let (Some(mode), Some(_kind), Some(oid)) = (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        entries.push(StagedEntry {
            mode: mode.to_string(),
            blob_oid: oid.to_string(),
            path: path.to_string(),
        });
    }
    Ok(entries)
}

/// Paths added/copied/modified/renamed relative to `HEAD` — the files that
/// are actually part of this commit. This is the scoping input every
/// per-file commit-tier check should use so cost stays proportional to the
/// staged change, not the whole repo.
///
/// `tree_oid`: `Some(oid)` diffs `HEAD` against that specific tree object
/// (`git diff HEAD <oid>`) — the captured snapshot, immune to a `git add`
/// that happens after `oid` was written. `None` falls back to `git diff
/// --cached` (the live index) for callers outside a real plan.
pub fn staged_diff_paths(root: &Path, tree_oid: Option<&str>) -> Result<Vec<String>> {
    let raw = match tree_oid {
        Some(oid) => {
            let base = diff_base(root)?;
            run_git_ok(root, &["diff", "--name-only", "--diff-filter=ACMR", "-z", &base, oid])?
        }
        None => run_git_ok(root, &["diff", "--cached", "--name-only", "--diff-filter=ACMR", "-z"])?,
    };
    Ok(raw.split('\0').filter(|s| !s.is_empty()).map(str::to_string).collect())
}

/// Git's well-known empty-tree object hash — a repo-independent constant
/// git recognizes without it needing to be an explicitly stored object (the
/// standard idiom for "diff against nothing", used e.g. to diff a repo's
/// very first commit).
const EMPTY_TREE_OID: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

/// `"HEAD"` when it resolves, else the empty-tree constant — so a
/// tree-oid-pinned diff still works on a brand-new repo with no commits yet
/// (an unborn `HEAD`), matching what `git diff --cached` already handles
/// implicitly for the live-index path.
///
/// `pub(crate)`: also used directly by `commit_checks::whitespace_check_at`,
/// which needs the same base to run `git diff <base> <oid> --check` against
/// the pinned tree object instead of `git diff --cached --check` (the live
/// index) — see that function's doc comment for the TOCTOU this closes.
pub(crate) fn diff_base(root: &Path) -> Result<String> {
    let resolves = Command::new("git")
        .current_dir(root)
        .args(["rev-parse", "--verify", "--quiet", "HEAD"])
        .output()
        .context("failed to check whether HEAD resolves")?
        .status
        .success();
    Ok(if resolves { "HEAD".to_string() } else { EMPTY_TREE_OID.to_string() })
}

/// Byte size of a staged blob without reading its content (`git cat-file -s`).
/// Already frozen-tree-based by construction: `oid` names the exact blob,
/// never the live index.
pub fn blob_size(root: &Path, oid: &str) -> Result<u64> {
    let raw = run_git_ok(root, &["cat-file", "-s", oid])?;
    raw.trim()
        .parse::<u64>()
        .with_context(|| format!("`git cat-file -s {oid}` returned non-numeric size: {raw:?}"))
}

/// Read a staged path's content — works even when the working tree doesn't
/// match (a partially-staged edit), because it never touches the filesystem
/// outside git's object database.
///
/// `tree_oid`: `Some(oid)` reads the blob from that specific tree object
/// (`git show <oid>:path`) — the captured snapshot. `None` reads from the
/// live index (`git show :path`) for callers outside a real plan.
///
/// `Ok(None)` means exactly one thing: `git show` succeeded and the content
/// isn't valid UTF-8 (a legitimate binary file, skipped by text-oriented
/// checks). Any other failure — the path genuinely isn't staged, a
/// corrupted object database, a permissions problem, git itself missing —
/// is a real `Err`, not `Ok(None)`. Conflating the two would make a check
/// silently skip staged content on a git failure instead of surfacing it;
/// callers of this function pass paths from [`staged_diff_paths`], which by
/// construction *are* staged, so an unexpected failure here is worth
/// knowing about, not swallowing.
pub fn read_staged_path_text(
    root: &Path,
    path: &str,
    tree_oid: Option<&str>,
) -> Result<Option<String>> {
    let spec = match tree_oid {
        Some(oid) => format!("{oid}:{path}"),
        None => format!(":{path}"),
    };
    let output = Command::new("git")
        .current_dir(root)
        .args(["show", &spec])
        .output()
        .with_context(|| format!("failed to read staged content for {path}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("failed to read staged content for {path}: {stderr}");
    }
    Ok(String::from_utf8(output.stdout).ok())
}

/// Whether `path` exists at all in the given staged tree object — for a
/// caller like `commit_checks::rustfmt_staged_at` that needs the STAGED
/// version of a config file (`rustfmt.toml`) which may or may not be part of
/// the diff being committed (it might be untouched by this commit, or not
/// tracked at all). [`read_staged_path_text`] deliberately treats "path
/// isn't staged" as a real `Err` (its callers pass paths already known to be
/// staged, from [`staged_diff_paths`]), so a caller that genuinely doesn't
/// know whether an incidental path exists in the tree must check first
/// rather than pattern-matching on `read_staged_path_text`'s error text.
pub fn staged_path_exists(root: &Path, tree_oid: &str, path: &str) -> Result<bool> {
    let status = Command::new("git")
        .current_dir(root)
        .args(["cat-file", "-e", &format!("{tree_oid}:{path}")])
        .status()
        .with_context(|| format!("failed to check whether {path} exists in tree {tree_oid}"))?;
    Ok(status.success())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_staged_entries_parses_ls_tree_records() -> Result<()> {
        // Synthetic ls-tree -z record shape (mode SP type SP oid TAB path NUL).
        // `\x00` (not `\0`) for the NUL separator: `\0` immediately followed
        // by digits reads as an octal escape to a human (and to clippy's
        // octal_escapes lint), even though Rust has no octal escapes and
        // parses it correctly either way.
        let raw = "100644 blob abc123\tfoo.rs\x00100755 blob def456\tscripts/run.sh\x00";
        let mut entries = Vec::new();
        for record in raw.split('\0') {
            if record.is_empty() {
                continue;
            }
            let Some((meta, path)) = record.split_once('\t') else { continue };
            let mut fields = meta.split(' ');
            let (Some(mode), Some(_kind), Some(oid)) =
                (fields.next(), fields.next(), fields.next())
            else {
                continue;
            };
            entries.push(StagedEntry {
                mode: mode.to_string(),
                blob_oid: oid.to_string(),
                path: path.to_string(),
            });
        }

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].path, "foo.rs");
        assert!(!entries[0].is_executable());
        assert_eq!(entries[1].path, "scripts/run.sh");
        assert!(entries[1].is_executable());
        Ok(())
    }

    // -------------------------------------------------------------------
    // Staged-vs-working-tree correctness proof.
    //
    // This is the one property the whole module exists to hold: every read
    // must reflect the STAGED index, never an unstaged working-tree edit and
    // never a pre-image the index has since moved past. Proven against a
    // real temp git repository (never the host repo's own index) in both
    // directions: staging something the working tree doesn't have, and
    // un-staging something the working tree still has.
    // -------------------------------------------------------------------

    use color_eyre::eyre::ContextCompat;

    struct TempRepo {
        dir: tempfile::TempDir,
    }

    impl TempRepo {
        fn init() -> Result<Self> {
            let dir = tempfile::tempdir().context("failed to create temp repo dir")?;
            let root = dir.path();
            for args in [
                vec!["init", "--quiet"],
                vec!["config", "user.email", "test@example.com"],
                vec!["config", "user.name", "Test"],
                // Mirrors this repo's own core.fileMode=false — the exact
                // condition that makes filesystem mode bits unreliable (see
                // module docs).
                vec!["config", "core.fileMode", "false"],
            ] {
                let status = Command::new("git")
                    .current_dir(root)
                    .args(&args)
                    .status()
                    .with_context(|| format!("failed to run git {args:?}"))?;
                if !status.success() {
                    bail!("git {args:?} failed in temp repo setup");
                }
            }
            Ok(Self { dir })
        }

        fn root(&self) -> &Path {
            self.dir.path()
        }

        fn write(&self, rel_path: &str, content: &str) -> Result<()> {
            let path = self.root().join(rel_path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(path, content).context("failed to write file in temp repo")
        }

        fn add(&self, rel_path: &str) -> Result<()> {
            let status = Command::new("git")
                .current_dir(self.root())
                .args(["add", rel_path])
                .status()
                .context("failed to run git add")?;
            if !status.success() {
                bail!("git add {rel_path} failed");
            }
            Ok(())
        }

        fn commit(&self, message: &str) -> Result<()> {
            let status = Command::new("git")
                .current_dir(self.root())
                .args(["commit", "--quiet", "-m", message])
                .status()
                .context("failed to run git commit")?;
            if !status.success() {
                bail!("git commit failed");
            }
            Ok(())
        }
    }

    #[test]
    fn read_staged_path_text_ignores_unstaged_working_tree_edits() -> Result<()> {
        let repo = TempRepo::init()?;
        repo.write("foo.rs", "fn main() {}\n")?;
        repo.add("foo.rs")?;
        repo.commit("initial")?;

        // Dirty the WORKING TREE without staging it.
        repo.write("foo.rs", "fn main() { /* unstaged edit */ }\n")?;

        let staged_text = read_staged_path_text(repo.root(), "foo.rs", None)?
            .context("foo.rs should still be readable from the index")?;
        assert_eq!(
            staged_text, "fn main() {}\n",
            "read_staged_path_text must return the committed/staged blob, not the dirtied \
             working-tree file"
        );
        Ok(())
    }

    #[test]
    fn read_staged_path_text_sees_staged_content_the_working_tree_no_longer_has() -> Result<()> {
        let repo = TempRepo::init()?;
        repo.write("foo.rs", "fn main() {}\n")?;
        repo.add("foo.rs")?;
        repo.commit("initial")?;

        // Stage a change, matching what `git write-tree` would record...
        repo.write("foo.rs", "fn main() { /* staged edit */ }\n")?;
        repo.add("foo.rs")?;
        // ...then revert the WORKING TREE back to the old content. The file
        // on disk no longer has the edit, but the STAGED blob still does.
        repo.write("foo.rs", "fn main() {}\n")?;

        let staged_text = read_staged_path_text(repo.root(), "foo.rs", None)?
            .context("foo.rs should still be readable from the index")?;
        assert_eq!(
            staged_text, "fn main() { /* staged edit */ }\n",
            "read_staged_path_text must return the STAGED blob even though the working tree \
             was reverted underneath it"
        );
        Ok(())
    }

    #[test]
    fn read_staged_path_text_errors_rather_than_silently_skipping_an_unreadable_path() -> Result<()>
    {
        // A `git show :path` failure — the path genuinely isn't staged, a
        // corrupted object, a permissions problem — must be a real `Err`,
        // never `Ok(None)`. `Ok(None)` is reserved for the one legitimate
        // case: git succeeded and the content just isn't valid UTF-8.
        // Conflating "git failed" with "nothing to see here" would make a
        // text check silently skip staged content instead of surfacing the
        // failure — the same class of bug as a broken grep quietly
        // returning zero matches instead of erroring.
        let repo = TempRepo::init()?;
        repo.write("foo.rs", "fn main() {}\n")?;
        repo.add("foo.rs")?;
        repo.commit("initial")?;

        let result = read_staged_path_text(repo.root(), "never-staged.rs", None);

        let err = match result {
            Err(err) => err,
            Ok(value) => bail!(
                "expected an error for a path that was never staged, got Ok({value:?}) instead \
                 of a surfaced failure"
            ),
        };
        let message = format!("{err:#}");
        assert!(
            message.contains("never-staged.rs"),
            "error should name the path that failed: {message}"
        );
        Ok(())
    }

    #[test]
    fn staged_diff_paths_reflects_the_index_not_the_working_tree() -> Result<()> {
        let repo = TempRepo::init()?;
        repo.write("committed.rs", "fn main() {}\n")?;
        repo.add("committed.rs")?;
        repo.commit("initial")?;

        // committed.rs dirtied on disk but NOT staged -> must not appear.
        repo.write("committed.rs", "fn main() { /* dirty */ }\n")?;
        // new_file.rs staged -> must appear.
        repo.write("new_file.rs", "fn main() {}\n")?;
        repo.add("new_file.rs")?;

        let paths = staged_diff_paths(repo.root(), None)?;
        assert!(
            paths.iter().any(|p| p == "new_file.rs"),
            "the newly staged file must be reported: {paths:?}"
        );
        assert!(
            !paths.iter().any(|p| p == "committed.rs"),
            "an unstaged working-tree edit must NOT be reported: {paths:?}"
        );
        Ok(())
    }

    #[test]
    fn staged_tree_oid_changes_with_the_index_not_the_working_tree() -> Result<()> {
        let repo = TempRepo::init()?;
        repo.write("foo.rs", "fn main() {}\n")?;
        repo.add("foo.rs")?;
        repo.commit("initial")?;

        let oid_before = staged_tree_oid(repo.root())?;

        // Dirty the working tree only — the staged tree's OID must not move.
        repo.write("foo.rs", "fn main() { /* dirty, unstaged */ }\n")?;
        let oid_unstaged_dirty = staged_tree_oid(repo.root())?;
        assert_eq!(
            oid_before, oid_unstaged_dirty,
            "an unstaged working-tree edit must not change the staged-tree identity"
        );

        // Now stage the same edit — the OID must move.
        repo.add("foo.rs")?;
        let oid_after_stage = staged_tree_oid(repo.root())?;
        assert_ne!(
            oid_before, oid_after_stage,
            "staging a real content change must change the staged-tree identity"
        );
        Ok(())
    }

    #[test]
    fn list_staged_entries_reads_git_mode_not_filesystem_mode() -> Result<()> {
        let repo = TempRepo::init()?;
        repo.write("crates/foo/src/lib.rs", "fn main() {}\n")?;
        repo.add("crates/foo/src/lib.rs")?;

        // With core.fileMode=false the filesystem's permission bit is not
        // what git records; force the STAGED mode via update-index, the
        // same class of divergence the R1 chmod defect hit.
        let status = Command::new("git")
            .current_dir(repo.root())
            .args(["update-index", "--chmod=+x", "crates/foo/src/lib.rs"])
            .status()
            .context("failed to run git update-index --chmod=+x")?;
        if !status.success() {
            bail!("git update-index --chmod=+x failed");
        }

        let tree_oid = staged_tree_oid(repo.root())?;
        let entries = list_staged_entries(repo.root(), &tree_oid)?;
        let entry = entries
            .iter()
            .find(|e| e.path == "crates/foo/src/lib.rs")
            .context("staged entry should be present")?;
        assert!(
            entry.is_executable(),
            "list_staged_entries must report the git-recorded mode (100755), not whatever the \
             filesystem happens to show under core.fileMode=false"
        );
        Ok(())
    }

    // -------------------------------------------------------------------
    // Frozen-tree-vs-live-index correctness proof.
    //
    // The class of bug a deep review caught in #3786-A: a check must read
    // the SAME snapshot `plan_gates` captured via `staged_tree_oid`, not
    // whatever the index has become by the time the check actually runs. A
    // concurrent `git add` between capture and dispatch must not change
    // what a `Some(oid)`-pinned read reports.
    // -------------------------------------------------------------------

    #[test]
    fn read_staged_path_text_with_pinned_oid_ignores_a_later_git_add() -> Result<()> {
        let repo = TempRepo::init()?;
        repo.write("foo.rs", "fn main() {}\n")?;
        repo.add("foo.rs")?;
        repo.commit("initial")?;

        // Stage an edit and capture the tree OID -- this is what plan_gates
        // does once, up front.
        repo.write("foo.rs", "fn main() { /* captured */ }\n")?;
        repo.add("foo.rs")?;
        let captured_oid = staged_tree_oid(repo.root())?;

        // A concurrent `git add` changes the index AFTER the OID was
        // captured -- simulating another process staging more work while
        // this run's checks are still dispatching.
        repo.write("foo.rs", "fn main() { /* concurrent change */ }\n")?;
        repo.add("foo.rs")?;

        // A read pinned to the captured OID must still see the captured
        // content, not the concurrent change.
        let pinned_text = read_staged_path_text(repo.root(), "foo.rs", Some(&captured_oid))?
            .context("foo.rs should be readable from the pinned tree")?;
        assert_eq!(
            pinned_text, "fn main() { /* captured */ }\n",
            "a Some(oid)-pinned read must ignore a concurrent git add that happened after the \
             OID was captured"
        );

        // For contrast: an unpinned (None) read DOES see the live index --
        // proving the difference is the tree-oid pinning, not some other
        // accident of the test setup.
        let live_text = read_staged_path_text(repo.root(), "foo.rs", None)?
            .context("foo.rs should be readable from the live index")?;
        assert_eq!(
            live_text, "fn main() { /* concurrent change */ }\n",
            "an unpinned (None) read should see the live index, confirming the pinned read's \
             stability comes from the OID, not from git caching"
        );
        Ok(())
    }

    #[test]
    fn staged_diff_paths_with_pinned_oid_ignores_a_later_git_add() -> Result<()> {
        let repo = TempRepo::init()?;
        repo.write("committed.rs", "fn main() {}\n")?;
        repo.add("committed.rs")?;
        repo.commit("initial")?;

        // Stage one new file and capture the tree OID.
        repo.write("captured.rs", "fn main() {}\n")?;
        repo.add("captured.rs")?;
        let captured_oid = staged_tree_oid(repo.root())?;

        // A concurrent `git add` stages a SECOND new file after the OID was
        // captured.
        repo.write("concurrent.rs", "fn main() {}\n")?;
        repo.add("concurrent.rs")?;

        let pinned_paths = staged_diff_paths(repo.root(), Some(&captured_oid))?;
        assert!(
            pinned_paths.iter().any(|p| p == "captured.rs"),
            "the file staged before the OID was captured must be reported: {pinned_paths:?}"
        );
        assert!(
            !pinned_paths.iter().any(|p| p == "concurrent.rs"),
            "a file staged AFTER the OID was captured must NOT be reported by a pinned read: \
             {pinned_paths:?}"
        );

        let live_paths = staged_diff_paths(repo.root(), None)?;
        assert!(
            live_paths.iter().any(|p| p == "concurrent.rs"),
            "an unpinned (None) read should see the concurrently staged file: {live_paths:?}"
        );
        Ok(())
    }
}

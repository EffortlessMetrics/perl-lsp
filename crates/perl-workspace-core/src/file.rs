//! File-level facts: role, digest, parse status.

use serde::{Deserialize, Serialize};

use crate::id::{Digest, FileId};

/// What role a file plays in the distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileRole {
    /// A library module under `lib/` (or a top-level `.pm`).
    Lib,
    /// A test file (`.t`, or under `t/` / `xt/`).
    Test,
    /// An executable script (`.pl`, or under `bin/` / `script/`).
    Script,
    /// Distribution metadata (`META.json`, `Makefile.PL`, `cpanfile`, …).
    DistMetadata,
    /// Standalone POD documentation (`.pod`).
    Pod,
    /// A generated file.
    Generated,
    /// Role could not be determined.
    Unknown,
}

impl FileRole {
    /// Classify a repo-relative path into a role.
    ///
    /// Uses forward-slash paths (the substrate normalizes to POSIX). The
    /// classification is intentionally conservative: unfamiliar shapes map to
    /// [`FileRole::Unknown`] rather than guessing.
    #[must_use]
    pub fn from_path(repo_relative_path: &str) -> Self {
        let path = repo_relative_path;
        let name = path.rsplit('/').next().unwrap_or(path);

        if path.ends_with(".t")
            || path.starts_with("t/")
            || path.starts_with("xt/")
            || path.contains("/t/")
            || path.contains("/xt/")
        {
            return Self::Test;
        }
        if name == "META.json"
            || name == "META.yml"
            || name == "Makefile.PL"
            || name == "Build.PL"
            || name == "dist.ini"
            || name == "cpanfile"
            || name == "MANIFEST"
            || name == "MANIFEST.SKIP"
        {
            return Self::DistMetadata;
        }
        if path.ends_with(".pod") {
            return Self::Pod;
        }
        if path.ends_with(".pm") {
            return Self::Lib;
        }
        if path.ends_with(".pl")
            || path.starts_with("bin/")
            || path.starts_with("script/")
            || path.contains("/bin/")
            || path.contains("/script/")
        {
            return Self::Script;
        }
        Self::Unknown
    }
}

/// Whether a file parsed cleanly, recovered, or failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParseStatus {
    /// Parsed with no errors.
    Clean,
    /// Parsed via error recovery (some facts may be lower-confidence).
    Recovered,
    /// Could not be parsed; the file fact is still emitted, with no symbols.
    Failed,
    /// Not parsed (e.g. a metadata file, or facts not requested).
    NotParsed,
}

/// A file fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileRecord {
    /// Stable identity.
    pub file_id: FileId,
    /// Repo-relative, forward-slash path.
    pub relative_path: String,
    /// The file's role in the distribution.
    pub role: FileRole,
    /// Content digest.
    pub digest: Digest,
    /// Parse outcome.
    pub parse_status: ParseStatus,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_common_roles() {
        assert_eq!(FileRole::from_path("lib/App.pm"), FileRole::Lib);
        assert_eq!(FileRole::from_path("t/basic.t"), FileRole::Test);
        assert_eq!(FileRole::from_path("xt/author/pod.t"), FileRole::Test);
        assert_eq!(FileRole::from_path("bin/app"), FileRole::Script);
        assert_eq!(FileRole::from_path("script/run.pl"), FileRole::Script);
        assert_eq!(FileRole::from_path("cpanfile"), FileRole::DistMetadata);
        assert_eq!(FileRole::from_path("META.json"), FileRole::DistMetadata);
        assert_eq!(FileRole::from_path("lib/App.pod"), FileRole::Pod);
        assert_eq!(FileRole::from_path("Changes"), FileRole::Unknown);
    }

    #[test]
    fn test_role_wins_over_pm_extension() {
        // A `.pm` under t/ is test-support code, classified as Test.
        assert_eq!(FileRole::from_path("t/lib/Helper.pm"), FileRole::Test);
    }

    #[test]
    fn role_serializes_lowercase() {
        assert_eq!(serde_json::to_string(&FileRole::Lib).unwrap(), "\"lib\"");
        assert_eq!(serde_json::to_string(&FileRole::DistMetadata).unwrap(), "\"distmetadata\"");
    }
}

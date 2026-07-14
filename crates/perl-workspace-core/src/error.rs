//! The substrate's error type.

use serde::{Deserialize, Serialize};

/// A failure that prevents building (part of) a project model.
///
/// Per-file read/parse failures are **not** errors — they are recorded as
/// [`ModelLimitation`](crate::model::ModelLimitation)s on the model, so a
/// single bad file never fails the whole build. These variants cover failures
/// that stop the build request itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceCoreError {
    /// The requested root is not a directory that can be walked.
    InvalidRoot {
        /// The offending path.
        path: String,
        /// Why it is invalid.
        reason: String,
    },
    /// No fact classes were requested.
    NoFactClasses,
}

impl std::fmt::Display for WorkspaceCoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidRoot { path, reason } => {
                write!(f, "invalid workspace root `{path}`: {reason}")
            }
            Self::NoFactClasses => write!(f, "at least one fact class must be requested"),
        }
    }
}

impl std::error::Error for WorkspaceCoreError {}

/// A recorded limitation: something the model could not fully determine.
///
/// Limitations keep the model honest. A file that could not be read or parsed,
/// a construct that was not modeled, or a fact class that was requested but is
/// not yet implemented all surface here rather than being silently dropped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLimitation {
    /// A stable, greppable id (e.g. `"parse-failed:lib/App.pm"`).
    pub id: String,
    /// A short machine-readable kind (e.g. `"parse_failure"`).
    pub kind: String,
    /// A human-readable explanation.
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_display_readably() {
        let e = WorkspaceCoreError::InvalidRoot {
            path: "/nope".to_string(),
            reason: "not a directory".to_string(),
        };
        assert!(e.to_string().contains("/nope"));
        assert!(WorkspaceCoreError::NoFactClasses.to_string().contains("fact class"));
    }

    #[test]
    fn limitation_round_trips() {
        let lim = ModelLimitation {
            id: "parse-failed:lib/App.pm".to_string(),
            kind: "parse_failure".to_string(),
            message: "could not parse".to_string(),
        };
        let json = serde_json::to_string(&lim).unwrap();
        let back: ModelLimitation = serde_json::from_str(&json).unwrap();
        assert_eq!(lim, back);
    }
}

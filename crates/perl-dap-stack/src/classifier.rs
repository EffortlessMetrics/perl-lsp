//! Frame classifier for distinguishing user code from library code.
//!
//! This module provides utilities for classifying stack frames based on their
//! source location, helping debuggers present relevant frames to users.

use crate::{StackFrame, StackFramePresentationHint};

/// Categories for stack frame classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameCategory {
    /// User code (the developer's own code)
    User,
    /// Library code (third-party modules)
    Library,
    /// Core Perl code (built-in modules and internals)
    Core,
    /// Eval-generated code
    Eval,
    /// Unknown origin
    Unknown,
}

impl FrameCategory {
    /// Returns the appropriate presentation hint for this category.
    #[must_use]
    pub fn presentation_hint(&self) -> StackFramePresentationHint {
        match self {
            FrameCategory::User => StackFramePresentationHint::Normal,
            FrameCategory::Eval => StackFramePresentationHint::Label,
            FrameCategory::Library | FrameCategory::Core | FrameCategory::Unknown => {
                StackFramePresentationHint::Subtle
            }
        }
    }

    /// Returns true if this category represents user-written code.
    #[must_use]
    pub fn is_user_code(&self) -> bool {
        matches!(self, FrameCategory::User)
    }

    /// Returns true if this category represents external code.
    #[must_use]
    pub fn is_external(&self) -> bool {
        matches!(self, FrameCategory::Library | FrameCategory::Core)
    }
}

/// Trait for classifying stack frames.
///
/// Implementations determine whether a stack frame represents user code,
/// library code, or core Perl internals.
pub trait FrameClassifier {
    /// Classifies a stack frame.
    ///
    /// # Arguments
    ///
    /// * `frame` - The frame to classify
    ///
    /// # Returns
    ///
    /// The classification category for the frame.
    fn classify(&self, frame: &StackFrame) -> FrameCategory;

    /// Applies classification to a frame, setting its presentation hint.
    ///
    /// # Arguments
    ///
    /// * `frame` - The frame to classify and update
    ///
    /// # Returns
    ///
    /// The frame with updated presentation hint.
    fn apply_classification(&self, frame: StackFrame) -> StackFrame {
        let category = self.classify(&frame);
        frame.with_presentation_hint(category.presentation_hint())
    }

    /// Classifies and filters a list of frames.
    ///
    /// # Arguments
    ///
    /// * `frames` - The frames to classify
    /// * `include_external` - Whether to include library/core frames
    ///
    /// # Returns
    ///
    /// Classified frames with appropriate presentation hints.
    fn classify_all(&self, frames: Vec<StackFrame>, include_external: bool) -> Vec<StackFrame> {
        frames
            .into_iter()
            .map(|f| self.apply_classification(f))
            .filter(|f| include_external || f.is_user_code())
            .collect()
    }
}

/// Default Perl frame classifier.
///
/// This classifier uses path-based heuristics to determine frame categories:
///
/// - Core modules: Paths containing `/perl/`, `/perl5/`, or standard module names
/// - Library code: Paths in common library directories (lib, vendor, local)
/// - Eval code: Files named `(eval N)` or with eval origin
/// - User code: Everything else (assumed to be project code)
#[derive(Debug, Default)]
pub struct PerlFrameClassifier {
    /// Paths considered to be user code directories
    user_paths: Vec<String>,
    /// Paths considered to be library directories
    library_paths: Vec<String>,
}

impl PerlFrameClassifier {
    /// Creates a new classifier with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self { user_paths: Vec::new(), library_paths: Vec::new() }
    }

    /// Adds a path that should be considered user code.
    ///
    /// Files under this path will be classified as user code.
    #[must_use]
    pub fn with_user_path(mut self, path: impl Into<String>) -> Self {
        self.user_paths.push(path.into());
        self
    }

    /// Adds a path that should be considered library code.
    ///
    /// Files under this path will be classified as library code.
    #[must_use]
    pub fn with_library_path(mut self, path: impl Into<String>) -> Self {
        self.library_paths.push(path.into());
        self
    }

    /// Checks if a path is under any of the user paths.
    fn is_under_user_path(&self, path: &str) -> bool {
        self.user_paths.iter().any(|user_path| path.starts_with(user_path))
    }

    /// Checks if a path is under any of the library paths.
    fn is_under_library_path(&self, path: &str) -> bool {
        self.library_paths.iter().any(|lib_path| path.starts_with(lib_path))
    }

    /// Checks if a path looks like a Perl core module.
    fn is_core_path(path: &str) -> bool {
        // Common core module path patterns
        let core_patterns = ["/perl/", "/perl5/", "/site_perl/", "/vendor_perl/", "/lib/perl5/"];

        // Core module packages
        let core_packages = [
            "strict.pm",
            "warnings.pm",
            "vars.pm",
            "Exporter.pm",
            "Carp.pm",
            "constant.pm",
            "overload.pm",
            "AutoLoader.pm",
            "base.pm",
            "parent.pm",
            "feature.pm",
            "utf8.pm",
            "encoding.pm",
            "lib.pm",
        ];

        // Check path patterns
        for pattern in &core_patterns {
            if path.contains(pattern) {
                return true;
            }
        }

        // Check if it's a known core module
        for module in &core_packages {
            if path.ends_with(module) {
                return true;
            }
        }

        false
    }

    /// Checks if a path looks like a library module.
    fn is_library_path(path: &str) -> bool {
        // Common library path patterns
        let library_patterns =
            ["/local/lib/", "/vendor/", "/cpan/", "/.cpanm/", "/extlib/", "/fatlib/"];

        for pattern in &library_patterns {
            if path.contains(pattern) {
                return true;
            }
        }

        false
    }

    /// Checks if a path looks like an eval source.
    fn is_eval_source(path: &str) -> bool {
        path.starts_with("(eval") || path.contains("(eval ")
    }
}

impl FrameClassifier for PerlFrameClassifier {
    fn classify(&self, frame: &StackFrame) -> FrameCategory {
        // Check source
        let path = match frame.file_path() {
            Some(p) => p,
            None => return FrameCategory::Unknown,
        };

        // Check for eval
        if Self::is_eval_source(path) {
            return FrameCategory::Eval;
        }

        // Also check source origin
        if frame.source.as_ref().is_some_and(|s| s.is_eval()) {
            return FrameCategory::Eval;
        }

        // Check explicit user paths first
        if self.is_under_user_path(path) {
            return FrameCategory::User;
        }

        // Check explicit library paths
        if self.is_under_library_path(path) {
            return FrameCategory::Library;
        }

        // Check for core modules
        if Self::is_core_path(path) {
            return FrameCategory::Core;
        }

        // Check for library modules
        if Self::is_library_path(path) {
            return FrameCategory::Library;
        }

        // Default to user code if we can't determine otherwise
        // This is intentional: we want to show frames by default rather than hide them
        FrameCategory::User
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Source;

    fn frame_with_path(path: &str) -> StackFrame {
        StackFrame::new(1, "test", Some(Source::new(path)), 1)
    }

    #[test]
    fn test_classify_user_code() {
        let classifier = PerlFrameClassifier::new();

        // Regular project files should be user code
        let frame = frame_with_path("/home/user/project/lib/MyApp/Module.pm");
        assert_eq!(classifier.classify(&frame), FrameCategory::User);

        let frame = frame_with_path("./script.pl");
        assert_eq!(classifier.classify(&frame), FrameCategory::User);
    }

    #[test]
    fn test_classify_core_modules() {
        let classifier = PerlFrameClassifier::new();

        let frame = frame_with_path("/usr/lib/perl5/strict.pm");
        assert_eq!(classifier.classify(&frame), FrameCategory::Core);

        let frame = frame_with_path("/usr/share/perl/5.30/Exporter.pm");
        assert_eq!(classifier.classify(&frame), FrameCategory::Core);

        let frame = frame_with_path("/usr/lib/perl/5.30/warnings.pm");
        assert_eq!(classifier.classify(&frame), FrameCategory::Core);
    }

    #[test]
    fn test_classify_library_code() {
        let classifier = PerlFrameClassifier::new();

        // Use paths that match library patterns but not core patterns
        let frame = frame_with_path("/home/user/project/extlib/Moose.pm");
        assert_eq!(classifier.classify(&frame), FrameCategory::Library);

        let frame = frame_with_path("/home/user/.cpanm/work/1234/Foo-1.0/lib/Foo.pm");
        assert_eq!(classifier.classify(&frame), FrameCategory::Library);
    }

    #[test]
    fn test_classify_eval() {
        let classifier = PerlFrameClassifier::new();

        let frame = frame_with_path("(eval 42)");
        assert_eq!(classifier.classify(&frame), FrameCategory::Eval);

        let frame = frame_with_path("(eval 10)[script.pl:5]");
        assert_eq!(classifier.classify(&frame), FrameCategory::Eval);

        // Frame with eval origin
        let mut frame = frame_with_path("/path/file.pm");
        frame.source = Some(Source::new("/path/file.pm").with_origin("eval"));
        assert_eq!(classifier.classify(&frame), FrameCategory::Eval);
    }

    #[test]
    fn test_classify_no_source() {
        let classifier = PerlFrameClassifier::new();

        let frame = StackFrame::new(1, "test", None, 1);
        assert_eq!(classifier.classify(&frame), FrameCategory::Unknown);
    }

    #[test]
    fn test_explicit_user_path() {
        let classifier = PerlFrameClassifier::new().with_user_path("/my/project/");

        // Should be classified as user even if path looks like library
        let frame = frame_with_path("/my/project/local/lib/perl5/MyModule.pm");
        assert_eq!(classifier.classify(&frame), FrameCategory::User);
    }

    #[test]
    fn test_explicit_library_path() {
        let classifier = PerlFrameClassifier::new().with_library_path("/opt/mylibs/");

        let frame = frame_with_path("/opt/mylibs/SomeModule.pm");
        assert_eq!(classifier.classify(&frame), FrameCategory::Library);
    }

    #[test]
    fn test_frame_category_presentation_hint() {
        assert_eq!(FrameCategory::User.presentation_hint(), StackFramePresentationHint::Normal);
        assert_eq!(FrameCategory::Library.presentation_hint(), StackFramePresentationHint::Subtle);
        assert_eq!(FrameCategory::Core.presentation_hint(), StackFramePresentationHint::Subtle);
        assert_eq!(FrameCategory::Eval.presentation_hint(), StackFramePresentationHint::Label);
    }

    #[test]
    fn test_apply_classification() {
        let classifier = PerlFrameClassifier::new();
        let frame = frame_with_path("/usr/lib/perl5/strict.pm");

        let classified = classifier.apply_classification(frame);
        assert_eq!(classified.presentation_hint, Some(StackFramePresentationHint::Subtle));
    }

    #[test]
    fn test_classify_all() {
        let classifier = PerlFrameClassifier::new();

        let frames = vec![
            frame_with_path("/home/user/project/script.pl"),
            frame_with_path("/usr/lib/perl5/strict.pm"),
            frame_with_path("/home/user/project/lib/App.pm"),
        ];

        // With external frames
        let classified = classifier.classify_all(frames.clone(), true);
        assert_eq!(classified.len(), 3);

        // Without external frames
        let classified = classifier.classify_all(frames, false);
        assert_eq!(classified.len(), 2);
    }

    #[test]
    fn test_is_user_code() {
        assert!(FrameCategory::User.is_user_code());
        assert!(!FrameCategory::Library.is_user_code());
        assert!(!FrameCategory::Core.is_user_code());
        assert!(!FrameCategory::Eval.is_user_code());
    }

    #[test]
    fn test_is_external() {
        assert!(!FrameCategory::User.is_external());
        assert!(FrameCategory::Library.is_external());
        assert!(FrameCategory::Core.is_external());
        assert!(!FrameCategory::Eval.is_external());
    }
}

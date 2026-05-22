//! Conservative Perl core-module detection for definition fallbacks.
//!
//! Keeping this catalog isolated makes the main navigation flow focus on
//! request handling while this module owns the policy for suppressing unresolved
//! definition lookups for common core pragmas/modules.

/// Returns `true` if the module name is a known Perl core pragma or standard module
/// that will never be found on disk in a user's workspace.
///
/// This list covers the pragmas and core modules that every Perl installation ships
/// with and that users most commonly reference with `use` or `require`.  It is
/// intentionally conservative — if a module is not listed here and is not found in
/// the workspace, the definition handler falls through to the normal "not found"
/// path unchanged.
pub(super) fn is_core_perl_module(name: &str) -> bool {
    matches!(
        name,
        "strict"
            | "warnings"
            | "warnings::register"
            | "utf8"
            | "feature"
            | "constant"
            | "vars"
            | "lib"
            | "parent"
            | "base"
            | "overload"
            | "overloading"
            | "Scalar::Util"
            | "List::Util"
            | "Carp"
            | "Exporter"
            | "POSIX"
            | "Data::Dumper"
            | "File::Basename"
            | "File::Path"
            | "File::Spec"
            | "Storable"
            | "Encode"
            | "MIME::Base64"
            | "Digest::MD5"
            | "Digest::SHA"
            | "IO::File"
            | "IO::Handle"
            | "Fcntl"
            | "Socket"
            | "Time::HiRes"
            | "Time::Local"
            | "Getopt::Long"
            | "Pod::Usage"
    )
}

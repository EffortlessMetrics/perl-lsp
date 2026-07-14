//! Deterministic, host-path-free identity for project facts.
//!
//! Every fact carries a stable ID derived only from repo-relative content: no
//! host paths, no timestamps, no traversal-order counters, no random UUIDs (see
//! [PLSP-ADR-0006](../../../docs/adr/PLSP-ADR-0006-perl-workspace-core-facts-substrate.md)).
//! Re-running the builder on unchanged source yields byte-identical IDs.
//!
//! # Digest algorithm
//!
//! IDs and content digests use a **FNV-1a 64-bit** hash with an explicit
//! `fnv64:` prefix — the same dependency-free convention `perl-ripr-facts`
//! uses. A SHA-256 digest would need an added crypto dependency; the prefix is
//! there so a future `sha256:` variant can coexist without ambiguity. The
//! `*Id` newtypes are the single place to change if that day comes.

use serde::{Deserialize, Serialize};

const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Deterministic FNV-1a 64-bit hash of a byte slice.
#[must_use]
pub fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET_BASIS;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Hash a sequence of fields, NUL-separated so `["a", "bc"]` and `["ab", "c"]`
/// hash differently. This is the identity primitive for the `*Id` types.
fn hash_fields(fields: &[&str]) -> u64 {
    let mut hash = FNV_OFFSET_BASIS;
    for (i, field) in fields.iter().enumerate() {
        if i > 0 {
            hash ^= u64::from(b'\0');
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        for &byte in field.as_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(FNV_PRIME);
        }
    }
    hash
}

/// A content digest, formatted `fnv64:<16 hex digits>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Digest(String);

impl Digest {
    /// Digest of a file's source text.
    #[must_use]
    pub fn of(content: &str) -> Self {
        Self(format!("fnv64:{:016x}", fnv1a(content.as_bytes())))
    }

    /// The digest string, including the `fnv64:` prefix.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Digest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Stable identity of a file: `file:fnv64:<hash>` over `path + digest`.
///
/// Two files with the same repo-relative path but different content get
/// different IDs; two builds of the same content at the same path get the same
/// ID.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FileId(String);

impl FileId {
    /// Derive a file ID from its repo-relative path and content digest.
    #[must_use]
    pub fn new(repo_relative_path: &str, digest: &Digest) -> Self {
        Self(format!("file:fnv64:{:016x}", hash_fields(&[repo_relative_path, digest.as_str()])))
    }

    /// The ID string, including the `file:fnv64:` prefix.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for FileId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Stable identity of a package: `pkg:fnv64:<hash>` over
/// `file_id + package_name + declaration_start_byte`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PackageId(String);

impl PackageId {
    /// Derive a package ID from its file, name, and declaration start byte.
    #[must_use]
    pub fn new(file_id: &FileId, package_name: &str, decl_start_byte: u32) -> Self {
        Self(format!(
            "pkg:fnv64:{:016x}",
            hash_fields(&[file_id.as_str(), package_name, &decl_start_byte.to_string()])
        ))
    }

    /// The ID string, including the `pkg:fnv64:` prefix.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for PackageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Stable identity of a symbol: `sym:fnv64:<hash>` over
/// `file_id + kind + qualified_name + start_byte + end_byte`.
///
/// The span is part of the identity, so a symbol is uniquely located
/// independent of traversal order.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SymbolId(String);

impl SymbolId {
    /// Derive a symbol ID from its file, kind tag, qualified name, and span.
    #[must_use]
    pub fn new(
        file_id: &FileId,
        kind_tag: &str,
        qualified_name: &str,
        start_byte: u32,
        end_byte: u32,
    ) -> Self {
        Self(format!(
            "sym:fnv64:{:016x}",
            hash_fields(&[
                file_id.as_str(),
                kind_tag,
                qualified_name,
                &start_byte.to_string(),
                &end_byte.to_string(),
            ])
        ))
    }

    /// The ID string, including the `sym:fnv64:` prefix.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SymbolId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv1a_is_deterministic_and_distinguishing() {
        assert_eq!(fnv1a(b"hello"), fnv1a(b"hello"));
        assert_ne!(fnv1a(b"hello"), fnv1a(b"world"));
        // Known FNV-1a offset basis: empty input hashes to the basis.
        assert_eq!(fnv1a(b""), FNV_OFFSET_BASIS);
    }

    #[test]
    fn digest_has_stable_prefixed_form() {
        let d = Digest::of("package App;\n1;\n");
        assert!(d.as_str().starts_with("fnv64:"), "got {d}");
        assert_eq!(d, Digest::of("package App;\n1;\n"));
        assert_ne!(d, Digest::of("package Other;\n1;\n"));
    }

    #[test]
    fn hash_fields_is_boundary_sensitive() {
        // NUL separation must prevent field-boundary collisions.
        assert_ne!(hash_fields(&["a", "bc"]), hash_fields(&["ab", "c"]));
    }

    #[test]
    fn file_id_depends_on_path_and_content() {
        let digest = Digest::of("x");
        let a = FileId::new("lib/A.pm", &digest);
        let b = FileId::new("lib/B.pm", &digest);
        let c = FileId::new("lib/A.pm", &Digest::of("y"));
        assert_eq!(a, FileId::new("lib/A.pm", &digest), "same inputs → same id");
        assert_ne!(a, b, "different path → different id");
        assert_ne!(a, c, "different content → different id");
        assert!(a.as_str().starts_with("file:fnv64:"));
    }

    #[test]
    fn symbol_id_includes_the_span() {
        let file = FileId::new("lib/A.pm", &Digest::of("x"));
        let a = SymbolId::new(&file, "sub", "A::run", 10, 20);
        let b = SymbolId::new(&file, "sub", "A::run", 10, 21);
        assert_ne!(a, b, "span is part of identity");
        assert_eq!(a, SymbolId::new(&file, "sub", "A::run", 10, 20));
    }

    #[test]
    fn package_id_includes_declaration_start() {
        let file = FileId::new("lib/A.pm", &Digest::of("x"));
        let a = PackageId::new(&file, "App", 0);
        let b = PackageId::new(&file, "App", 42);
        assert_ne!(a, b);
        assert!(a.as_str().starts_with("pkg:fnv64:"));
    }

    #[test]
    fn id_display_impls_match_as_str() {
        // Display is the format!("{id}") path used throughout the codebase's
        // logging/debug output; it must agree with `as_str()` exactly.
        let file = FileId::new("lib/A.pm", &Digest::of("x"));
        assert_eq!(format!("{file}"), file.as_str());

        let package = PackageId::new(&file, "App", 0);
        assert_eq!(format!("{package}"), package.as_str());

        let symbol = SymbolId::new(&file, "sub", "App::run", 0, 10);
        assert_eq!(format!("{symbol}"), symbol.as_str());
    }
}

//! Fact-class selection.
//!
//! Computing every fact class on every request wastes work — asking for one
//! class should not walk, parse, and emit unrelated payloads (the same waste
//! `perl-ripr-facts` guards against by only walking when files/owners are
//! requested). [`FactClasses`] is a small dependency-free bitset the builder
//! honors: it only does the work a request actually asks for.

use serde::{Deserialize, Serialize};

/// A set of fact classes to compute, as a bitset.
///
/// Implemented by hand (no `bitflags` dependency) to keep the substrate's
/// dependency surface minimal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FactClasses(u32);

impl FactClasses {
    /// File-level facts (role, digest, parse status).
    pub const FILES: Self = Self(1 << 0);
    /// Syntax availability (parse success/recovery).
    pub const SYNTAX: Self = Self(1 << 1);
    /// Symbol declarations (packages, subs, methods, …).
    pub const SYMBOLS: Self = Self(1 << 2);
    /// Import facts (`use`/`require`/`no`).
    pub const IMPORTS: Self = Self(1 << 3);
    /// Export facts (Exporter, `@EXPORT`).
    pub const EXPORTS: Self = Self(1 << 4);
    /// POD facts.
    pub const POD: Self = Self(1 << 5);
    /// Test facts.
    pub const TESTS: Self = Self(1 << 6);
    /// Distribution-metadata facts.
    pub const DIST: Self = Self(1 << 7);
    /// Compile-time effect facts.
    pub const COMPILE_EFFECTS: Self = Self(1 << 8);
    /// Relation/edge facts.
    pub const RELATIONS: Self = Self(1 << 9);
    /// Dynamic-boundary facts.
    pub const DYNAMIC_BOUNDARIES: Self = Self(1 << 10);

    /// The empty set.
    pub const NONE: Self = Self(0);

    /// Everything the substrate can currently produce.
    #[must_use]
    pub const fn all() -> Self {
        Self(
            Self::FILES.0
                | Self::SYNTAX.0
                | Self::SYMBOLS.0
                | Self::IMPORTS.0
                | Self::EXPORTS.0
                | Self::POD.0
                | Self::TESTS.0
                | Self::DIST.0
                | Self::COMPILE_EFFECTS.0
                | Self::RELATIONS.0
                | Self::DYNAMIC_BOUNDARIES.0,
        )
    }

    /// True if `self` contains every class in `other`.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// True if `self` contains any class in `other`.
    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        (self.0 & other.0) != 0
    }

    /// The union of two sets.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// True if no classes are selected.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl std::ops::BitOr for FactClasses {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        self.union(rhs)
    }
}

impl std::ops::BitOrAssign for FactClasses {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_and_intersects() {
        let set = FactClasses::FILES | FactClasses::SYMBOLS;
        assert!(set.contains(FactClasses::FILES));
        assert!(set.contains(FactClasses::SYMBOLS));
        assert!(!set.contains(FactClasses::IMPORTS));
        assert!(set.contains(FactClasses::FILES | FactClasses::SYMBOLS));
        assert!(set.intersects(FactClasses::SYMBOLS | FactClasses::IMPORTS));
        assert!(!set.intersects(FactClasses::IMPORTS | FactClasses::EXPORTS));
    }

    #[test]
    fn all_contains_every_class() {
        let all = FactClasses::all();
        for class in [
            FactClasses::FILES,
            FactClasses::SYMBOLS,
            FactClasses::IMPORTS,
            FactClasses::DYNAMIC_BOUNDARIES,
        ] {
            assert!(all.contains(class));
        }
    }

    #[test]
    fn none_is_empty() {
        assert!(FactClasses::NONE.is_empty());
        assert!(!FactClasses::FILES.is_empty());
    }

    #[test]
    fn bitor_assign_accumulates() {
        let mut set = FactClasses::FILES;
        set |= FactClasses::SYMBOLS;
        assert!(set.contains(FactClasses::FILES | FactClasses::SYMBOLS));
    }
}

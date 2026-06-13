//! Type-safe variablesReference codec for the DAP protocol.
//!
//! # Problem
//!
//! The DAP protocol uses a single `i32` field (`variablesReference`) to encode
//! references to three logically distinct spaces:
//!
//! - **Scope** references: identify a scope (locals/package/globals) within a stack frame
//! - **EvalResult** references: identify a structured evaluation result (HASH/ARRAY)
//! - **Child** references: identify a nested child variable within a parent
//!
//! Prior to this module, encoding was done with ad-hoc arithmetic scattered across
//! multiple call sites. Issue #1219 identified a collision hazard: `frame_id * 10 + kind`
//! (where kind ∈ [1,3]) can produce values that overlap EvalResult counters.
//!
//! # Solution: pure-range disjoint bands
//!
//! This module provides a typed enum `VariableReference` with a single encode/decode
//! codec. Each variant occupies a **strictly disjoint** wire range:
//!
//! | Variant | Wire Range | Encoding |
//! |---------|-----------|----------|
//! | `Scope` | [1, 999_999] | `frame_id * 10 + kind` (kind ∈ [1,3], frame_id ∈ [0, 99_999]) |
//! | `EvalResult` | [1_000_000, 1_999_999_999] | `1_000_000 + counter` |
//! | `Child` | [2_000_000_000, i32::MAX] | `2_000_000_000 + (parent << 16 \| index)` |
//!
//! The bands are pairwise disjoint: no Scope wire value can equal any EvalResult or
//! Child wire value, by construction. Decode is pure-range — it tests which band
//! `raw` falls into, with no discriminant disambiguation required.
//!
//! # Scope frame_id bound
//!
//! Scope wire = `frame_id * 10 + kind` must stay in `[1, 999_999]`.
//! With kind ∈ [1, 3], the maximum Scope wire is `99_999 * 10 + 3 = 999_993`.
//! Therefore `frame_id` must be in `[0, 99_999]`.
//! Encoding a Scope with `frame_id > 99_999` returns `None` (safely rejected,
//! never produces a wire value in the EvalResult or Child band).
//!
//! # Decode ordering (pure range)
//!
//! 1. `raw <= 0` → `None`
//! 2. `raw >= 2_000_000_000` → `Child`
//! 3. `raw >= 1_000_000` → `EvalResult` (counter = raw − 1_000_000)
//! 4. `raw in [1, 999_999]` → `Scope` if `raw % 10 ∈ [1,3]`, else `None`
//! 5. Otherwise → `None`
//!
//! No value can match more than one band, so decode is unambiguous without any
//! residue-based disambiguation.
//!
//! # Safety
//!
//! All arithmetic uses saturating operations. Extreme inputs (i32::MAX, u32::MAX)
//! saturate rather than panic or overflow.

use std::fmt;

/// Error type for `TryFrom<i32>` on `ScopeKind`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableReferenceError {
    message: String,
}

impl VariableReferenceError {
    fn new(msg: impl Into<String>) -> Self {
        Self { message: msg.into() }
    }
}

impl fmt::Display for VariableReferenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VariableReferenceError: {}", self.message)
    }
}

impl std::error::Error for VariableReferenceError {}

/// The kind of scope a `Scope` variable reference points to within a stack frame.
///
/// Wire values: Locals=1, Package=2, Globals=3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScopeKind {
    /// Lexical (my) variables in the current frame.
    Locals = 1,
    /// Package (our) variables in the current frame.
    Package = 2,
    /// All global variables.
    Globals = 3,
}

impl TryFrom<i32> for ScopeKind {
    type Error = VariableReferenceError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(ScopeKind::Locals),
            2 => Ok(ScopeKind::Package),
            3 => Ok(ScopeKind::Globals),
            other => Err(VariableReferenceError::new(format!(
                "invalid ScopeKind discriminant: {other}; expected 1 (Locals), 2 (Package), or 3 (Globals)"
            ))),
        }
    }
}

/// A typed, codec-backed reference into the DAP `variablesReference` wire space.
///
/// ## Wire ranges (pairwise disjoint)
///
/// - `Scope`:      [1, 999_999]            — `frame_id * 10 + kind` (frame_id ∈ [0, 99_999], kind ∈ [1,3])
/// - `EvalResult`: [1_000_000, 1_999_999_999] — `1_000_000 + counter`
/// - `Child`:      [2_000_000_000, i32::MAX]  — `2_000_000_000 + (parent << 16 | index)`
///
/// Wire value 0 is reserved/invalid (DAP: 0 = "no children"). Negative values are
/// invalid. Values in [1_000_000..2_000_000_000) not matching any band decode to None.
///
/// ## Encode contract
///
/// `encode()` returns `None` when encoding would bleed a wire value into another variant's band:
/// - `Scope`: `None` when `frame_id > 99_999` (would overflow into the EvalResult band).
/// - `EvalResult`: `None` when `1_000_000 + counter > EVAL_MAX` (counter ≥ 1_999_000_000,
///   practically unreachable, but enforced for type-level correctness).
/// - `Child`: `None` when `parent < 0` (negative parent produces wire in the EvalResult band,
///   violating the disjoint-band invariant; saturating arithmetic for non-negative inputs).
///
/// All fields are primitive scalar types, so `VariableReference` is `Copy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableReference {
    /// A scope reference: variables in `kind` scope at stack frame `frame_id`.
    ///
    /// Valid range: `frame_id ∈ [0, 99_999]`.
    Scope {
        /// Stack frame identifier (0-based; must be ≤ 99_999 for a valid wire encoding).
        frame_id: i32,
        /// Which scope within the frame.
        kind: ScopeKind,
    },
    /// A structured evaluation result reference (HASH or ARRAY from `evaluate`).
    EvalResult {
        /// Monotonically increasing counter allocated per evaluation result.
        counter: i32,
    },
    /// A child variable reference within a parent variable.
    Child {
        /// The parent variable reference (the Scope or EvalResult that owns this child).
        ///
        /// Must be a non-negative, previously encoded `variablesReference` (always ≥ 0
        /// by the DAP spec). Negative values are invalid and encode() returns `None`.
        parent: i32,
        /// Zero-based index of the child within the parent's variable list.
        index: u32,
    },
}

/// Wire range constants — pairwise disjoint bands.
///
/// Scope:      [SCOPE_MIN, SCOPE_MAX]              = [1, 999_999]
/// EvalResult: [EVAL_BASE, EVAL_MAX]               = [1_000_000, 1_999_999_999]
/// Child:      [CHILD_BASE, i32::MAX]              = [2_000_000_000, 2_147_483_647]
const SCOPE_MIN: i32 = 1;
const SCOPE_MAX: i32 = 999_999; // 99_999 * 10 + 3 = 999_993 < 999_999; max Scope wire
const SCOPE_FRAME_ID_MAX: i32 = 99_999; // frame_id bound: 99_999 * 10 + 3 = 999_993 ≤ SCOPE_MAX
const EVAL_BASE: i32 = 1_000_000;
const EVAL_MAX: i32 = 1_999_999_999;
const CHILD_BASE: i32 = 2_000_000_000;

impl VariableReference {
    /// Encode this reference to an i32 wire value.
    ///
    /// Returns `None` when encoding would produce a wire value outside the variant's band:
    ///
    /// - `Scope`: `None` when `frame_id` is out of `[0, 99_999]` (would bleed into EvalResult band).
    /// - `EvalResult`: `None` when `counter` is negative or `1_000_000 + counter > 1_999_999_999`
    ///   (would bleed into Child band; requires counter ≥ 1_999_000_000, practically unreachable).
    /// - `Child`: `None` when `parent < 0` (negative parent is semantically invalid — a
    ///   DAP variablesReference is always non-negative, and negative parents produce wire values
    ///   in the EvalResult band, corrupting the disjoint-band invariant).
    pub fn encode(&self) -> Option<i32> {
        match self {
            VariableReference::Scope { frame_id, kind } => {
                // Scope wire = frame_id * 10 + kind_disc (1-3).
                // frame_id must be in [0, SCOPE_FRAME_ID_MAX] to stay within the Scope band.
                if *frame_id < 0 || *frame_id > SCOPE_FRAME_ID_MAX {
                    return None;
                }
                let kind_disc = *kind as i32;
                // frame_id in [0, 99_999] and kind_disc in [1, 3]:
                // max wire = 99_999 * 10 + 3 = 999_993 ≤ SCOPE_MAX ✓
                Some(frame_id * 10 + kind_disc)
            }
            VariableReference::EvalResult { counter } => {
                // Wire = 1_000_000 + counter, must stay in [EVAL_BASE, EVAL_MAX].
                // Negative counters are invalid. Counters >= 1_999_000_000 would push
                // the wire value into or past CHILD_BASE (2_000_000_000) and be
                // misclassified as Child on decode — reject them explicitly.
                if *counter < 0 {
                    return None;
                }
                let wire = EVAL_BASE.saturating_add(*counter);
                if wire > EVAL_MAX {
                    return None;
                }
                Some(wire)
            }
            VariableReference::Child { parent, index } => {
                // Wire = 2_000_000_000 + (parent << 16 | (index & 0xFFFF))
                // Negative parents produce wire values in the EvalResult band
                // (e.g. parent=-1 → wire 1_999_934_464), violating disjoint-band invariant.
                // A valid DAP variablesReference is always non-negative, so reject here.
                if *parent < 0 {
                    return None;
                }
                // Clamp parent so the multiplication fits in i32.
                let index_truncated = (*index & 0xFFFF) as i32;
                let parent_clamped = (*parent).clamp(0, i32::MAX / 65_536);
                let parent_shifted = parent_clamped * 65_536;
                let packed = parent_shifted.saturating_add(index_truncated);
                Some(CHILD_BASE.saturating_add(packed))
            }
        }
    }

    /// Decode a wire i32 value into a `VariableReference`.
    ///
    /// Uses **pure-range** classification against the three disjoint bands:
    ///
    /// - `raw in [2_000_000_000, i32::MAX]` → `Child`
    /// - `raw in [1_000_000, 1_999_999_999]` → `EvalResult{counter: raw - 1_000_000}`
    /// - `raw in [1, 999_999]` → `Scope` if `raw % 10 ∈ {1,2,3}`, else `None`
    /// - All others (0, negative, gaps) → `None`
    ///
    /// Because the bands are pairwise disjoint, no value can match more than one case.
    /// There is no residue-based disambiguation between bands.
    pub fn decode(raw: i32) -> Option<Self> {
        if raw <= 0 {
            return None;
        }

        // 1. Child band: [2_000_000_000, i32::MAX]
        if raw >= CHILD_BASE {
            let packed = raw - CHILD_BASE;
            let parent = packed >> 16;
            let index = (packed & 0xFFFF) as u32;
            return Some(VariableReference::Child { parent, index });
        }

        // 2. EvalResult band: [1_000_000, 1_999_999_999]
        if (EVAL_BASE..=EVAL_MAX).contains(&raw) {
            let counter = raw - EVAL_BASE;
            return Some(VariableReference::EvalResult { counter });
        }

        // 3. Scope band: [1, 999_999] — kind discriminant must be 1, 2, or 3.
        if (SCOPE_MIN..=SCOPE_MAX).contains(&raw) {
            let kind_disc = raw % 10;
            let frame_id = raw / 10;
            let kind = ScopeKind::try_from(kind_disc).ok()?;
            return Some(VariableReference::Scope { frame_id, kind });
        }

        // Gap or out-of-range (e.g. [2_000_000_000, i32::MAX] already handled above,
        // [1_000_000..2_000_000_000) above 1_999_999_999 falls here → None).
        None
    }
}

#[cfg(test)]
mod codec_unit_tests {
    use super::*;

    #[test]
    fn scope_kind_tryfrom_valid() {
        assert_eq!(ScopeKind::try_from(1), Ok(ScopeKind::Locals));
        assert_eq!(ScopeKind::try_from(2), Ok(ScopeKind::Package));
        assert_eq!(ScopeKind::try_from(3), Ok(ScopeKind::Globals));
    }

    #[test]
    fn scope_kind_tryfrom_invalid() {
        assert!(ScopeKind::try_from(0).is_err());
        assert!(ScopeKind::try_from(4).is_err());
        assert!(ScopeKind::try_from(-1).is_err());
    }

    #[test]
    fn scope_encode_decode_basic() {
        let s = VariableReference::Scope { frame_id: 5000, kind: ScopeKind::Locals };
        let wire = s.encode().expect("frame_id=5000 is in [0,99_999]");
        assert_eq!(wire, 50_001);
        assert_eq!(VariableReference::decode(50_001), Some(s));
    }

    #[test]
    fn evalresult_encode_decode_roundtrip() {
        // counter=0: wire 1_000_000 (EvalResult band)
        let e0 = VariableReference::EvalResult { counter: 0 };
        let w0 = e0.encode().unwrap();
        assert_eq!(w0, 1_000_000);
        assert_eq!(VariableReference::decode(w0), Some(e0));

        // counter=1: wire 1_000_001 — previously misclassified as Scope
        let e1 = VariableReference::EvalResult { counter: 1 };
        let w1 = e1.encode().unwrap();
        assert_eq!(w1, 1_000_001);
        assert_eq!(VariableReference::decode(w1), Some(e1), "counter=1 must decode as EvalResult");

        // counter=3: wire 1_000_003 — kind_disc=3 is Globals under old logic
        let e3 = VariableReference::EvalResult { counter: 3 };
        let w3 = e3.encode().unwrap();
        assert_eq!(w3, 1_000_003);
        assert_eq!(VariableReference::decode(w3), Some(e3), "counter=3 must decode as EvalResult");
    }

    #[test]
    fn scope_frame_id_max_boundary() {
        // frame_id=99_999 is valid; wire = 999_993 (Locals)
        let s_max = VariableReference::Scope { frame_id: 99_999, kind: ScopeKind::Locals };
        let wire = s_max.encode().expect("frame_id=99_999 is the max valid frame_id");
        assert_eq!(wire, 999_991);
        assert_eq!(VariableReference::decode(wire), Some(s_max));
    }

    #[test]
    fn scope_frame_id_over_max_returns_none() {
        // frame_id=100_000 would put wire at 1_000_001 — in the EvalResult band
        let over = VariableReference::Scope { frame_id: 100_000, kind: ScopeKind::Locals };
        assert_eq!(
            over.encode(),
            None,
            "frame_id=100_000 must be rejected (would overflow into EvalResult band)"
        );
    }

    #[test]
    fn child_encode_decode_base() {
        let c = VariableReference::Child { parent: 0, index: 0 };
        let wire = c.encode().unwrap();
        assert_eq!(wire, 2_000_000_000);
        assert_eq!(VariableReference::decode(2_000_000_000), Some(c));
    }

    #[test]
    fn decode_zero_none() {
        assert_eq!(VariableReference::decode(0), None);
    }

    #[test]
    fn decode_negative_none() {
        assert_eq!(VariableReference::decode(-1), None);
        assert_eq!(VariableReference::decode(i32::MIN), None);
    }

    #[test]
    fn decode_invalid_scope_kind_none() {
        // 999_999: frame_id=99_999, kind_disc=9 → None (invalid kind)
        assert_eq!(VariableReference::decode(999_999), None);
    }

    #[test]
    fn bands_are_disjoint_no_scope_wire_in_eval_range() {
        // No valid Scope encoding can fall in [1_000_000, 1_999_999_999].
        // Max Scope wire = 99_999 * 10 + 3 = 999_993 < 1_000_000.
        let max_scope_wire = SCOPE_FRAME_ID_MAX * 10 + 3;
        assert!(
            max_scope_wire < EVAL_BASE,
            "Scope max wire {max_scope_wire} must be < EvalResult base {EVAL_BASE}"
        );
    }

    // --- GUARD-PATH UNIT TESTS (covers --lib patch; integration suite has adversarial depth) ---

    #[test]
    fn evalresult_negative_counter_returns_none() {
        // Guard: if *counter < 0 { return None; } -- added by deep-review fix.
        // Negative counters are semantically invalid (counters are monotonically increasing).
        assert_eq!(
            VariableReference::EvalResult { counter: -1 }.encode(),
            None,
            "negative EvalResult counter must be rejected"
        );
        assert_eq!(
            VariableReference::EvalResult { counter: i32::MIN }.encode(),
            None,
            "i32::MIN EvalResult counter must be rejected"
        );
    }

    #[test]
    fn evalresult_overflow_into_child_band_returns_none() {
        // Guard: if wire > EVAL_MAX { return None; } -- added by deep-review fix.
        // EVAL_BASE = 1_000_000, EVAL_MAX = 1_999_999_999.
        // Max valid counter: EVAL_MAX - EVAL_BASE = 1_998_999_999 (wire = 1_999_999_999).
        // counter = 1_999_000_000 -> wire = 2_000_000_000 = CHILD_BASE -> must be rejected.
        assert_eq!(
            VariableReference::EvalResult { counter: 1_998_999_999 }.encode(),
            Some(1_999_999_999),
            "counter=1_998_999_999 should encode to EVAL_MAX"
        );
        assert_eq!(
            VariableReference::EvalResult { counter: 1_999_000_000 }.encode(),
            None,
            "counter that would push wire into Child band must be rejected"
        );
    }

    #[test]
    fn child_negative_parent_returns_none() {
        // Guard: if *parent < 0 { return None; } -- added by deep-review fix.
        // Negative parents produce wire values in the EvalResult band (disjoint-band violation).
        assert_eq!(
            VariableReference::Child { parent: -1, index: 0 }.encode(),
            None,
            "parent=-1 must be rejected (would land in EvalResult band)"
        );
        assert_eq!(
            VariableReference::Child { parent: i32::MIN, index: 0 }.encode(),
            None,
            "parent=i32::MIN must be rejected"
        );
    }
}

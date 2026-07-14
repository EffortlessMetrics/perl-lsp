//! TAP (Test Anything Protocol) output reader.
//!
//! `perl-lsp` does not run tests itself — `yath`, `prove`, or `perl` do. This
//! module *reads* their TAP output and turns it into structured results the
//! editor can act on: which assertions failed, their source location when the
//! producer reported it, and TODO/SKIP status (which are **not** hard failures).
//!
//! The reader is intentionally conservative. It parses the well-specified TAP
//! grammar (plan line, `ok`/`not ok`, directives, `# diagnostics`, `Bail out!`,
//! nested-subtest indentation) and does not attempt to statically reconstruct
//! anything the producer did not print.

use once_cell::sync::Lazy;
use regex::Regex;

/// A TODO/SKIP directive attached to a test line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TapDirective {
    /// `# TODO reason` — a known-failing test; a `not ok` here is expected and
    /// is not counted as a hard failure.
    Todo(String),
    /// `# SKIP reason` — the test was not run.
    Skip(String),
}

/// A single `ok` / `not ok` line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TapTest {
    /// The 1-based test number, if present.
    pub number: Option<usize>,
    /// Whether the line began with `ok` (as opposed to `not ok`).
    pub ok: bool,
    /// The test description (text after the number, before any directive).
    pub description: String,
    /// A TODO/SKIP directive, if any.
    pub directive: Option<TapDirective>,
    /// Nesting depth from TAP indentation (0 = top level).
    pub depth: usize,
    /// Diagnostic (`#`) lines that followed this test line.
    pub diagnostics: Vec<String>,
    /// Source file parsed from an `# at FILE line N.` diagnostic, if present.
    pub file: Option<String>,
    /// Source line parsed from an `# at FILE line N.` diagnostic, if present.
    pub line: Option<usize>,
    /// `got` value parsed from diagnostics, if present.
    pub got: Option<String>,
    /// `expected` value parsed from diagnostics, if present.
    pub expected: Option<String>,
}

impl TapTest {
    /// Whether this line is a *hard* failure: a `not ok` that is not marked
    /// TODO or SKIP. TODO failures and SKIP lines are not hard failures — this
    /// must agree with `summarize()`, which counts skipped tests separately.
    pub fn is_failure(&self) -> bool {
        !self.ok && !self.is_todo() && !self.is_skipped()
    }

    /// Whether this test was skipped.
    pub fn is_skipped(&self) -> bool {
        matches!(self.directive, Some(TapDirective::Skip(_)))
    }

    /// Whether this test is a TODO (expected-failing) test.
    pub fn is_todo(&self) -> bool {
        matches!(self.directive, Some(TapDirective::Todo(_)))
    }
}

/// The `1..N` plan line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TapPlan {
    /// The number of planned tests (`N` in `1..N`).
    pub count: usize,
    /// The skip-all reason for a `1..0 # SKIP reason` plan, if present.
    pub skip_all: Option<String>,
}

/// Aggregate counts for a parsed TAP stream.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TapSummary {
    /// Total `ok`/`not ok` lines observed (top level and nested).
    pub total: usize,
    /// Hard failures (`not ok` without TODO).
    pub failed: usize,
    /// Passing lines (`ok` without SKIP).
    pub passed: usize,
    /// Skipped lines.
    pub skipped: usize,
    /// TODO lines (both passing and failing).
    pub todo: usize,
}

/// A fully parsed TAP stream.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TapReport {
    /// The plan line, if present.
    pub plan: Option<TapPlan>,
    /// All test lines in order.
    pub tests: Vec<TapTest>,
    /// The `Bail out!` reason, if the run bailed.
    pub bailed_out: Option<String>,
    /// Aggregate counts.
    pub summary: TapSummary,
}

impl TapReport {
    /// The hard failures in this report (`not ok` without TODO).
    pub fn failures(&self) -> Vec<&TapTest> {
        self.tests.iter().filter(|t| t.is_failure()).collect()
    }

    /// Whether the run passed: no hard failures and no bail-out. Note a plan
    /// mismatch is reported via [`Self::plan_mismatch`] separately.
    pub fn passed(&self) -> bool {
        self.bailed_out.is_none() && self.summary.failed == 0
    }

    /// If a plan was declared, whether the number of top-level tests differs
    /// from the plan count. Returns `None` when there is no plan to compare.
    pub fn plan_mismatch(&self) -> Option<(usize, usize)> {
        let plan = self.plan.as_ref()?;
        if plan.skip_all.is_some() {
            return None;
        }
        let top_level = self.tests.iter().filter(|t| t.depth == 0).count();
        (top_level != plan.count).then_some((top_level, plan.count))
    }
}

/// The result of focusing a TAP report on one named subtest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubtestFocus {
    /// The requested subtest name.
    pub name: String,
    /// Whether a subtest summary line with that name was found in the output.
    pub found: bool,
    /// Whether that subtest passed (its summary line was `ok`). Meaningless
    /// when `found` is false.
    pub passed: bool,
    /// Number of nested `not ok` (hard) failures attributed to the subtest.
    pub inner_failed: usize,
}

/// Focus a parsed TAP report on a single subtest by name.
///
/// Test2/Test::More print a buffered subtest as a run of indented (`depth > 0`)
/// assertion lines followed by a `depth == 0` summary line whose description is
/// the subtest name. We match that summary line and attribute the immediately
/// preceding nested failures to it. Returns `None` when no summary line with
/// `name` is present — the caller then reports a whole-file run without focus
/// (e.g. a dynamic subtest name, or a runner that did not label the subtest).
pub fn focus_subtest(report: &TapReport, name: &str) -> Option<SubtestFocus> {
    let summary_idx =
        report.tests.iter().position(|test| test.depth == 0 && test.description == name)?;
    let summary = &report.tests[summary_idx];

    // Count nested failures in the contiguous depth>0 run just before the
    // summary line (buffered subtest body).
    let mut inner_failed = 0;
    for test in report.tests[..summary_idx].iter().rev() {
        if test.depth == 0 {
            break;
        }
        if test.is_failure() {
            inner_failed += 1;
        }
    }

    Some(SubtestFocus {
        name: name.to_string(),
        found: true,
        passed: !summary.is_failure(),
        inner_failed,
    })
}

static PLAN_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^1\.\.(\d+)\s*(?:#\s*[Ss][Kk][Ii][Pp]\S*\s*(.*))?$")
        .unwrap_or_else(|_| unreachable!("static TAP plan pattern is valid"))
});

static TEST_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(not )?ok\b[ \t]*(\d+)?[ \t]*(?:-[ \t]*)?(.*)$")
        .unwrap_or_else(|_| unreachable!("static TAP test pattern is valid"))
});

static DIRECTIVE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)#\s*(todo|skip)\b\s*(.*)$")
        .unwrap_or_else(|_| unreachable!("static TAP directive pattern is valid"))
});

static AT_LINE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\bat\s+(.+?)\s+line\s+(\d+)")
        .unwrap_or_else(|_| unreachable!("static TAP at-line pattern is valid"))
});

static GOT_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\s*(?:got|received):\s*(.*)$")
        .unwrap_or_else(|_| unreachable!("static TAP got pattern is valid"))
});

static EXPECTED_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\s*(?:expected|wanted):\s*(.*)$")
        .unwrap_or_else(|_| unreachable!("static TAP expected pattern is valid"))
});

/// Parse a TAP stream into a structured [`TapReport`].
pub fn parse_tap(output: &str) -> TapReport {
    let mut report = TapReport::default();

    for raw_line in output.lines() {
        let depth = indentation_depth(raw_line);
        let line = raw_line.trim_start();

        if line.is_empty() {
            continue;
        }

        if let Some(reason) = line.strip_prefix("Bail out!") {
            report.bailed_out = Some(reason.trim().to_string());
            continue;
        }

        // TAP version header — ignore.
        if line.starts_with("TAP version") {
            continue;
        }

        // Plan line `1..N`.
        if let Some(caps) = PLAN_RE.captures(line) {
            let count = caps.get(1).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
            let skip_all = caps.get(2).map(|m| m.as_str().trim().to_string()).filter(|s| {
                // A `1..0 # SKIP` marks skip-all; `1..N` (N>0) has no skip text.
                !s.is_empty() || count == 0
            });
            report.plan = Some(TapPlan { count, skip_all });
            continue;
        }

        // Test line.
        if let Some(test) = parse_test_line(line, depth) {
            report.tests.push(test);
            continue;
        }

        // Diagnostic line: attach to the most recent test.
        if let Some(diag) = line.strip_prefix('#') {
            let diag = diag.trim();
            if diag.is_empty() {
                continue;
            }
            if let Some(last) = report.tests.last_mut() {
                apply_diagnostic(last, diag);
            }
            continue;
        }
    }

    report.summary = summarize(&report.tests);
    report
}

/// Number of leading indentation "levels" (4 spaces or a tab each) — TAP nests
/// subtests with 4-space indentation.
fn indentation_depth(line: &str) -> usize {
    let mut spaces = 0usize;
    for ch in line.chars() {
        match ch {
            ' ' => spaces += 1,
            '\t' => spaces += 4,
            _ => break,
        }
    }
    spaces / 4
}

fn parse_test_line(line: &str, depth: usize) -> Option<TapTest> {
    let caps = TEST_RE.captures(line)?;
    let ok = caps.get(1).is_none();
    let number = caps.get(2).and_then(|m| m.as_str().parse().ok());
    let rest = caps.get(3).map(|m| m.as_str()).unwrap_or("").trim();

    // Split off a trailing directive `# TODO ...` / `# SKIP ...`.
    let (description, directive) = if let Some(dcaps) = DIRECTIVE_RE.captures(rest) {
        let whole = dcaps.get(0).map(|m| m.start()).unwrap_or(rest.len());
        let desc = rest[..whole].trim_end().trim_end_matches('#').trim().to_string();
        let kind = dcaps.get(1).map(|m| m.as_str().to_ascii_lowercase()).unwrap_or_default();
        let reason = dcaps.get(2).map(|m| m.as_str().trim().to_string()).unwrap_or_default();
        let directive = match kind.as_str() {
            "todo" => Some(TapDirective::Todo(reason)),
            "skip" => Some(TapDirective::Skip(reason)),
            _ => None,
        };
        (desc, directive)
    } else {
        (rest.to_string(), None)
    };

    Some(TapTest {
        number,
        ok,
        description,
        directive,
        depth,
        diagnostics: Vec::new(),
        file: None,
        line: None,
        got: None,
        expected: None,
    })
}

fn apply_diagnostic(test: &mut TapTest, diag: &str) {
    if let Some(caps) = AT_LINE_RE.captures(diag) {
        if test.file.is_none() {
            test.file = caps.get(1).map(|m| m.as_str().trim().to_string());
        }
        if test.line.is_none() {
            test.line = caps.get(2).and_then(|m| m.as_str().parse().ok());
        }
    }
    if let Some(caps) = GOT_RE.captures(diag) {
        if test.got.is_none() {
            test.got = caps.get(1).map(|m| m.as_str().trim().to_string());
        }
    }
    if let Some(caps) = EXPECTED_RE.captures(diag) {
        if test.expected.is_none() {
            test.expected = caps.get(1).map(|m| m.as_str().trim().to_string());
        }
    }
    test.diagnostics.push(diag.to_string());
}

fn summarize(tests: &[TapTest]) -> TapSummary {
    let mut summary = TapSummary::default();
    for test in tests {
        summary.total += 1;
        if test.is_todo() {
            summary.todo += 1;
        }
        if test.is_skipped() {
            summary.skipped += 1;
        } else if test.is_failure() {
            summary.failed += 1;
        } else if test.ok {
            summary.passed += 1;
        }
    }
    summary
}

#[cfg(test)]
mod tests;

//! Builder for the [`DebugSessionPacket`] — the stable, serializable handoff
//! format an external debugger tool (ptkdb today, any engine tomorrow) can
//! consume regardless of transport (peer protocol, `.ptkdbrc`, or a future
//! ptkdb import command).
//!
//! The packet is deterministic (sorted source-facts keys) so golden tests and
//! reproducible receipts are possible.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::breakpoint_oracle::{AstBreakpointOracle, BreakpointOracle};
use crate::model::{DebugBreakpoint, DebugSessionPacket, DebugSource, SourceDebugFacts};

/// Fluent builder for a [`DebugSessionPacket`].
#[derive(Debug, Clone)]
pub struct DebugSessionPlanBuilder {
    packet: DebugSessionPacket,
}

impl DebugSessionPlanBuilder {
    /// Start a plan for `program`.
    pub fn new(program: impl Into<PathBuf>) -> Self {
        Self { packet: DebugSessionPacket::new(program) }
    }

    /// Set the working directory.
    #[must_use]
    pub fn cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.packet.cwd = Some(cwd.into());
        self
    }

    /// Set `@INC` additions.
    #[must_use]
    pub fn include_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.packet.include_paths = paths;
        self
    }

    /// Add a source breakpoint.
    #[must_use]
    pub fn breakpoint(mut self, breakpoint: DebugBreakpoint) -> Self {
        self.packet.breakpoints.push(breakpoint);
        self
    }

    /// Add a function breakpoint by exact name.
    #[must_use]
    pub fn function_breakpoint(mut self, name: impl Into<String>) -> Self {
        self.packet.function_breakpoints.push(name.into());
        self
    }

    /// Add a function breakpoint by regex over sub names.
    #[must_use]
    pub fn function_breakpoint_regex(mut self, regex: impl Into<String>) -> Self {
        self.packet.function_breakpoint_regexes.push(regex.into());
        self
    }

    /// Add a watch expression.
    #[must_use]
    pub fn watch_expression(mut self, expr: impl Into<String>) -> Self {
        self.packet.watch_expressions.push(expr.into());
        self
    }

    /// Derive and attach [`SourceDebugFacts`] for `source` from its `text`,
    /// using the AST breakpoint oracle. Non-parseable sources are skipped.
    #[must_use]
    pub fn source_facts_from_text(mut self, source: &DebugSource, text: &str) -> Self {
        if let Ok(oracle) = AstBreakpointOracle::new(source.clone(), text) {
            let facts = SourceDebugFacts {
                breakable_line_candidates: oracle.breakable_line_candidates(),
                subroutines: oracle.function_candidates(),
            };
            self.packet.source_facts.insert(source.path.clone(), facts);
        }
        self
    }

    /// Attach pre-computed facts for a source.
    #[must_use]
    pub fn source_facts(mut self, path: impl Into<PathBuf>, facts: SourceDebugFacts) -> Self {
        self.packet.source_facts.insert(path.into(), facts);
        self
    }

    /// Finish and return the packet.
    #[must_use]
    pub fn build(self) -> DebugSessionPacket {
        self.packet
    }

    /// Finish and serialize to pretty JSON.
    ///
    /// # Errors
    /// Returns an error only if serialization fails (should not happen for the
    /// well-formed model types).
    pub fn to_json(self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.packet)
    }
}

/// Merge source facts from multiple sources into a deterministic map.
#[must_use]
pub fn facts_map(entries: Vec<(PathBuf, SourceDebugFacts)>) -> BTreeMap<PathBuf, SourceDebugFacts> {
    entries.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_produces_schema_tagged_packet() {
        let packet = DebugSessionPlanBuilder::new("/work/script.pl")
            .cwd("/work")
            .include_paths(vec![PathBuf::from("lib")])
            .function_breakpoint("main::run")
            .watch_expression("$self")
            .build();
        assert_eq!(packet.schema, DebugSessionPacket::SCHEMA);
        assert_eq!(packet.function_breakpoints, vec!["main::run"]);
        assert_eq!(packet.watch_expressions, vec!["$self"]);
    }

    #[test]
    fn source_facts_are_derived_from_source_text() {
        let source = DebugSource::from_path("/work/script.pl");
        let text = "sub run {\n    my $x = 1;\n    return $x;\n}\n";
        let packet = DebugSessionPlanBuilder::new("/work/script.pl")
            .source_facts_from_text(&source, text)
            .build();
        let facts =
            packet.source_facts.get(&PathBuf::from("/work/script.pl")).expect("facts present");
        assert!(!facts.breakable_line_candidates.is_empty());
        assert!(facts.subroutines.iter().any(|s| s.name == "run"));
    }

    #[test]
    fn json_is_deterministic_across_builds() {
        let build = || {
            DebugSessionPlanBuilder::new("/work/script.pl")
                .source_facts("/z.pl", SourceDebugFacts::default())
                .source_facts("/a.pl", SourceDebugFacts::default())
                .to_json()
                .expect("json")
        };
        assert_eq!(build(), build(), "serialization must be stable");
        let json = build();
        let a = json.find("/a.pl").expect("a");
        let z = json.find("/z.pl").expect("z");
        assert!(a < z, "sources emit in sorted order");
    }
}

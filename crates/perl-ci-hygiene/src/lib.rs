//! Re-export binary functionality for testing.
//!
//! This module is primarily used to enable `cargo test --lib` CI runs.
//! The primary entry point is the binary in `main.rs`.

pub mod version_sync;

/// Categorize an `#[ignore]` reason/context into a policy bucket.
pub fn categorize_ignore(reason: &str, context: &str) -> String {
    let reason = reason.trim().to_lowercase();
    let context = context.to_lowercase();
    let reason_no_space = reason.replace(' ', "");

    if reason.starts_with("manual:")
        || reason.contains("manual ")
        || reason.contains("regenerate")
        || reason.contains("helper")
    {
        return "manual".to_string();
    }
    if reason.starts_with("stress:")
        || reason.contains("stress test")
        || reason.contains("memory.stress")
        || reason.contains("performance.stress")
        || reason.contains("load.test")
        || reason.contains("stack.overflow")
        || reason.contains("designed.to.fail")
    {
        return "stress".to_string();
    }
    if reason.starts_with("bug:")
        || reason.contains("bug:")
        || reason.contains("known.bug")
        || reason.contains("regression")
        || reason.contains("incorrect.behavior")
        || reason.contains("parser.bug")
        || reason.contains("missing.notification")
        || reason.contains("missing.initialize")
        || reason.contains("server.returns.instead")
        || reason.contains("will.kill")
        || reason.contains("known.inconsistencies")
        || reason.contains("mut_")
        || reason.contains("matching.issue")
        || reason.contains("investigate")
        || reason.contains("instead.of.expected")
        || reason.contains("different.error.format")
        || reason.contains("expects")
    {
        return "bug".to_string();
    }
    if reason.starts_with("todo:")
        || reason_no_space.starts_with("todo(#")
        || reason.starts_with("infra:")
        || reason.contains("infra ")
        || reason.contains("fixme")
        || reason.contains("needs")
        || reason.contains("requires")
        || reason.contains("setup")
        || reason.contains("config")
        || reason.contains("environment")
        || reason.contains("run.with")
        || reason.contains("only.run.after")
        || reason.contains("only.run.when")
    {
        return "infra".to_string();
    }
    if reason.starts_with("feature:")
        || reason.contains("feature ")
        || reason.contains("not.implemented")
        || reason.contains("unimplemented")
        || reason.contains("wip")
        || reason.contains("work.in.progress")
        || reason.contains("pending")
        || reason.contains("when.implemented")
        || reason.contains("remove.when")
        || reason.contains("ac:")
        || reason.contains("ac ")
        || reason.contains("not.yet")
        || reason.contains("tdd.scaffold")
        || reason.contains("scaffold")
        || reason.contains("doesn.t.support")
        || reason.contains("doesn't.support")
        || reason.contains("parser.limitation")
        || reason.contains("expected.to.fail")
        || reason.contains("not.fully.supported")
        || reason.contains("enable.after")
        || reason.contains("after.phase")
        || reason.contains("parser.doesn")
        || reason.contains("tracked in #")
    {
        return "feature".to_string();
    }
    if reason.starts_with("brokenpipe:")
        || reason.contains("brokenpipe ")
        || reason.contains("broken.pipe")
        || reason.contains("transport.error")
        || reason.contains("transport.flake")
        || reason.contains("flaky")
    {
        return "brokenpipe".to_string();
    }
    if reason.contains("protocol")
        || reason.contains("lsp")
        || reason.contains("dap")
        || reason.contains("compliance")
        || reason.contains("specification")
    {
        return "protocol".to_string();
    }
    if reason.contains("tracked in #") {
        return "feature".to_string();
    }
    if reason.contains("doesn.t.have.field")
        || reason.contains("may.not.produce")
        || reason.contains("doesn.t.yet")
        || reason.contains("fewer.than.expected")
    {
        return "feature".to_string();
    }
    if reason.contains("recursion.limit.behavior") || reason.contains("behavior.changed") {
        return "feature".to_string();
    }
    if reason.contains("integration.test.that.spawns")
        || reason.contains("spawns.external")
        || reason.contains("burn.down")
        || reason.contains("mutation.hardening")
    {
        return "infra".to_string();
    }
    if reason.contains("clippy.warnings") || reason.contains("warnings.burn") {
        return "infra".to_string();
    }
    if reason.starts_with("ac:") {
        return "feature".to_string();
    }
    if reason.is_empty() || reason == "ignore" {
        return "bare".to_string();
    }
    if context.contains("ac:") {
        return "feature".to_string();
    }
    "other".to_string()
}

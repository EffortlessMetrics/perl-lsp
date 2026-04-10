//! Tests for OOM protection and memory budget tracking
//!
//! Covers: MemoryBudget defaults, MemoryPressure levels, MemoryMonitor
//! accounting, LspLimits memory fields, and degradation thresholds.

use perl_lsp_limits::{
    LspLimits, MemoryBudget, MemoryMonitor, MemoryPressure, ast_cache_max_memory_bytes,
    memory_critical_threshold_bytes, memory_warning_threshold_bytes,
};

#[test]
fn memory_budget_default_has_sensible_values() -> Result<(), Box<dyn std::error::Error>> {
    let budget = MemoryBudget::default();
    assert!(budget.warning_threshold_bytes > 0);
    assert!(budget.critical_threshold_bytes > budget.warning_threshold_bytes);
    assert!(budget.ast_cache_max_bytes > 0);
    Ok(())
}

#[test]
fn memory_budget_constrained_preset_is_smaller() -> Result<(), Box<dyn std::error::Error>> {
    let default = MemoryBudget::default();
    let constrained = MemoryBudget::constrained();
    assert!(constrained.warning_threshold_bytes <= default.warning_threshold_bytes);
    assert!(constrained.ast_cache_max_bytes <= default.ast_cache_max_bytes);
    Ok(())
}

#[test]
fn memory_budget_large_workspace_preset_is_bigger() -> Result<(), Box<dyn std::error::Error>> {
    let default = MemoryBudget::default();
    let large = MemoryBudget::large_workspace();
    assert!(large.warning_threshold_bytes >= default.warning_threshold_bytes);
    assert!(large.ast_cache_max_bytes >= default.ast_cache_max_bytes);
    Ok(())
}

#[test]
fn memory_pressure_normal_is_least_severe() -> Result<(), Box<dyn std::error::Error>> {
    assert!(MemoryPressure::Normal < MemoryPressure::Warning);
    assert!(MemoryPressure::Normal < MemoryPressure::Critical);
    Ok(())
}

#[test]
fn memory_pressure_warning_is_between() -> Result<(), Box<dyn std::error::Error>> {
    assert!(MemoryPressure::Warning > MemoryPressure::Normal);
    assert!(MemoryPressure::Warning < MemoryPressure::Critical);
    Ok(())
}

#[test]
fn memory_pressure_critical_is_most_severe() -> Result<(), Box<dyn std::error::Error>> {
    assert!(MemoryPressure::Critical > MemoryPressure::Normal);
    assert!(MemoryPressure::Critical > MemoryPressure::Warning);
    Ok(())
}

#[test]
fn memory_pressure_should_degrade_at_warning_and_above() -> Result<(), Box<dyn std::error::Error>> {
    assert!(!MemoryPressure::Normal.should_degrade());
    assert!(MemoryPressure::Warning.should_degrade());
    assert!(MemoryPressure::Critical.should_degrade());
    Ok(())
}

#[test]
fn memory_pressure_is_critical_only_at_critical() -> Result<(), Box<dyn std::error::Error>> {
    assert!(!MemoryPressure::Normal.is_critical());
    assert!(!MemoryPressure::Warning.is_critical());
    assert!(MemoryPressure::Critical.is_critical());
    Ok(())
}

#[test]
fn memory_monitor_starts_at_zero() -> Result<(), Box<dyn std::error::Error>> {
    let budget = MemoryBudget::default();
    let monitor = MemoryMonitor::new(budget);
    assert_eq!(monitor.tracked_bytes(), 0);
    Ok(())
}

#[test]
fn memory_monitor_track_and_release() -> Result<(), Box<dyn std::error::Error>> {
    let budget = MemoryBudget::default();
    let monitor = MemoryMonitor::new(budget);
    monitor.record_alloc(1024);
    assert_eq!(monitor.tracked_bytes(), 1024);
    monitor.record_alloc(512);
    assert_eq!(monitor.tracked_bytes(), 1536);
    monitor.record_free(512);
    assert_eq!(monitor.tracked_bytes(), 1024);
    Ok(())
}

#[test]
fn memory_monitor_free_saturates_at_zero() -> Result<(), Box<dyn std::error::Error>> {
    let budget = MemoryBudget::default();
    let monitor = MemoryMonitor::new(budget);
    monitor.record_free(9999);
    assert_eq!(monitor.tracked_bytes(), 0);
    Ok(())
}

#[test]
fn memory_monitor_pressure_normal_below_warning() -> Result<(), Box<dyn std::error::Error>> {
    let budget = MemoryBudget {
        warning_threshold_bytes: 1000,
        critical_threshold_bytes: 2000,
        ast_cache_max_bytes: 500,
    };
    let monitor = MemoryMonitor::new(budget);
    monitor.record_alloc(100);
    assert_eq!(monitor.pressure(), MemoryPressure::Normal);
    Ok(())
}

#[test]
fn memory_monitor_pressure_warning_at_threshold() -> Result<(), Box<dyn std::error::Error>> {
    let budget = MemoryBudget {
        warning_threshold_bytes: 1000,
        critical_threshold_bytes: 2000,
        ast_cache_max_bytes: 500,
    };
    let monitor = MemoryMonitor::new(budget);
    monitor.record_alloc(1000);
    assert_eq!(monitor.pressure(), MemoryPressure::Warning);
    Ok(())
}

#[test]
fn memory_monitor_pressure_critical_at_threshold() -> Result<(), Box<dyn std::error::Error>> {
    let budget = MemoryBudget {
        warning_threshold_bytes: 1000,
        critical_threshold_bytes: 2000,
        ast_cache_max_bytes: 500,
    };
    let monitor = MemoryMonitor::new(budget);
    monitor.record_alloc(2000);
    assert_eq!(monitor.pressure(), MemoryPressure::Critical);
    Ok(())
}

#[test]
fn memory_monitor_pressure_critical_above_threshold() -> Result<(), Box<dyn std::error::Error>> {
    let budget = MemoryBudget {
        warning_threshold_bytes: 1000,
        critical_threshold_bytes: 2000,
        ast_cache_max_bytes: 500,
    };
    let monitor = MemoryMonitor::new(budget);
    monitor.record_alloc(5000);
    assert_eq!(monitor.pressure(), MemoryPressure::Critical);
    Ok(())
}

#[test]
fn memory_monitor_ast_cache_within_budget() -> Result<(), Box<dyn std::error::Error>> {
    let budget = MemoryBudget {
        warning_threshold_bytes: 10_000,
        critical_threshold_bytes: 20_000,
        ast_cache_max_bytes: 1000,
    };
    let monitor = MemoryMonitor::new(budget);
    assert!(monitor.ast_cache_has_budget(500));
    assert!(monitor.ast_cache_has_budget(999));
    assert!(!monitor.ast_cache_has_budget(1001));
    Ok(())
}

#[test]
fn memory_monitor_ast_cache_exact_limit() -> Result<(), Box<dyn std::error::Error>> {
    let budget = MemoryBudget {
        warning_threshold_bytes: 10_000,
        critical_threshold_bytes: 20_000,
        ast_cache_max_bytes: 1000,
    };
    let monitor = MemoryMonitor::new(budget);
    assert!(monitor.ast_cache_has_budget(1000));
    assert!(!monitor.ast_cache_has_budget(1001));
    Ok(())
}

#[test]
fn memory_monitor_log_message_on_warning() -> Result<(), Box<dyn std::error::Error>> {
    let budget = MemoryBudget {
        warning_threshold_bytes: 500,
        critical_threshold_bytes: 1000,
        ast_cache_max_bytes: 250,
    };
    let monitor = MemoryMonitor::new(budget);
    monitor.record_alloc(600);
    let msg = monitor.pressure_log_message();
    assert!(msg.is_some());
    let msg = msg.unwrap();
    assert!(msg.contains("warning") || msg.contains("Warning") || msg.contains("WARNING"));
    Ok(())
}

#[test]
fn memory_monitor_log_message_on_critical() -> Result<(), Box<dyn std::error::Error>> {
    let budget = MemoryBudget {
        warning_threshold_bytes: 500,
        critical_threshold_bytes: 1000,
        ast_cache_max_bytes: 250,
    };
    let monitor = MemoryMonitor::new(budget);
    monitor.record_alloc(1100);
    let msg = monitor.pressure_log_message();
    assert!(msg.is_some());
    let msg = msg.unwrap();
    assert!(msg.contains("critical") || msg.contains("Critical") || msg.contains("CRITICAL"));
    Ok(())
}

#[test]
fn memory_monitor_no_log_message_when_normal() -> Result<(), Box<dyn std::error::Error>> {
    let budget = MemoryBudget {
        warning_threshold_bytes: 1000,
        critical_threshold_bytes: 2000,
        ast_cache_max_bytes: 500,
    };
    let monitor = MemoryMonitor::new(budget);
    monitor.record_alloc(100);
    assert!(monitor.pressure_log_message().is_none());
    Ok(())
}

#[test]
fn lsp_limits_has_memory_budget() -> Result<(), Box<dyn std::error::Error>> {
    let limits = LspLimits::default();
    assert!(limits.memory_budget.warning_threshold_bytes > 0);
    assert!(limits.memory_budget.critical_threshold_bytes > 0);
    assert!(limits.memory_budget.ast_cache_max_bytes > 0);
    Ok(())
}

#[test]
fn lsp_limits_constrained_preset_has_smaller_memory_budget()
-> Result<(), Box<dyn std::error::Error>> {
    let default = LspLimits::default();
    let constrained = LspLimits::constrained();
    assert!(
        constrained.memory_budget.warning_threshold_bytes
            <= default.memory_budget.warning_threshold_bytes
    );
    Ok(())
}

#[test]
fn lsp_limits_large_workspace_has_bigger_memory_budget() -> Result<(), Box<dyn std::error::Error>> {
    let default = LspLimits::default();
    let large = LspLimits::large_workspace();
    assert!(
        large.memory_budget.warning_threshold_bytes
            >= default.memory_budget.warning_threshold_bytes
    );
    Ok(())
}

#[test]
fn lsp_limits_update_from_value_sets_memory_warning() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    let settings = serde_json::json!({
        "limits": {
            "memoryWarningThresholdBytes": 536_870_912u64
        }
    });
    limits.update_from_value(&settings);
    assert_eq!(limits.memory_budget.warning_threshold_bytes, 536_870_912);
    Ok(())
}

#[test]
fn lsp_limits_update_from_value_sets_memory_critical() -> Result<(), Box<dyn std::error::Error>> {
    let mut limits = LspLimits::default();
    let settings = serde_json::json!({
        "limits": {
            "memoryCriticalThresholdBytes": 1_073_741_824u64
        }
    });
    limits.update_from_value(&settings);
    assert_eq!(limits.memory_budget.critical_threshold_bytes, 1_073_741_824);
    Ok(())
}

#[test]
fn lsp_limits_update_from_value_sets_ast_cache_max_memory() -> Result<(), Box<dyn std::error::Error>>
{
    let mut limits = LspLimits::default();
    let settings = serde_json::json!({
        "limits": {
            "astCacheMaxMemoryBytes": 52_428_800u64
        }
    });
    limits.update_from_value(&settings);
    assert_eq!(limits.memory_budget.ast_cache_max_bytes, 52_428_800);
    Ok(())
}

#[test]
fn global_accessor_memory_warning_threshold() -> Result<(), Box<dyn std::error::Error>> {
    let threshold = memory_warning_threshold_bytes();
    assert!(threshold > 0);
    Ok(())
}

#[test]
fn global_accessor_memory_critical_threshold() -> Result<(), Box<dyn std::error::Error>> {
    let threshold = memory_critical_threshold_bytes();
    assert!(threshold > 0);
    assert!(threshold >= memory_warning_threshold_bytes());
    Ok(())
}

#[test]
fn global_accessor_ast_cache_max_memory() -> Result<(), Box<dyn std::error::Error>> {
    let max = ast_cache_max_memory_bytes();
    assert!(max > 0);
    Ok(())
}

#[test]
fn memory_monitor_concurrent_updates() -> Result<(), Box<dyn std::error::Error>> {
    use std::sync::Arc;
    use std::thread;

    let budget = MemoryBudget {
        warning_threshold_bytes: 100_000,
        critical_threshold_bytes: 200_000,
        ast_cache_max_bytes: 50_000,
    };
    let monitor = Arc::new(MemoryMonitor::new(budget));

    let mut handles = Vec::new();
    for _ in 0..4 {
        let m = Arc::clone(&monitor);
        handles.push(thread::spawn(move || {
            m.record_alloc(1000);
            m.record_free(500);
        }));
    }
    for h in handles {
        h.join().map_err(|_| "thread panicked")?;
    }

    // 4 threads each alloc 1000 and free 500 -> net 500 each -> total 2000
    assert_eq!(monitor.tracked_bytes(), 2000);

    Ok(())
}

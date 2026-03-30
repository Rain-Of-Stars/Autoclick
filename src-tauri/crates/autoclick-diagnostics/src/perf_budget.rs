use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PerfBudget {
    pub idle_memory_mb: f32,
    pub scanning_cpu_percent: f32,
    pub preview_memory_delta_mb: f32,
    pub startup_ms: u64,
    pub stop_ms: u64,
    pub recovery_ms: u64,
}

impl PerfBudget {
    pub fn windows_release_default() -> Self {
        Self {
            idle_memory_mb: 220.0,
            scanning_cpu_percent: 35.0,
            preview_memory_delta_mb: 80.0,
            startup_ms: 2_000,
            stop_ms: 1_200,
            recovery_ms: 2_500,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PerfSnapshot {
    pub idle_memory_mb: f32,
    pub scanning_cpu_percent: f32,
    pub preview_memory_delta_mb: f32,
    pub startup_ms: u64,
    pub stop_ms: u64,
    pub recovery_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PerfBudgetViolation {
    pub metric: String,
    pub actual: String,
    pub budget: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PerfBudgetReport {
    pub passed: bool,
    pub violations: Vec<PerfBudgetViolation>,
}

pub fn validate_perf_budget(snapshot: PerfSnapshot, budget: PerfBudget) -> PerfBudgetReport {
    let mut violations = Vec::new();
    push_f32_violation(
        &mut violations,
        "idle_memory_mb",
        snapshot.idle_memory_mb,
        budget.idle_memory_mb,
        "MB",
    );
    push_f32_violation(
        &mut violations,
        "scanning_cpu_percent",
        snapshot.scanning_cpu_percent,
        budget.scanning_cpu_percent,
        "%",
    );
    push_f32_violation(
        &mut violations,
        "preview_memory_delta_mb",
        snapshot.preview_memory_delta_mb,
        budget.preview_memory_delta_mb,
        "MB",
    );
    push_u64_violation(
        &mut violations,
        "startup_ms",
        snapshot.startup_ms,
        budget.startup_ms,
        "ms",
    );
    push_u64_violation(
        &mut violations,
        "stop_ms",
        snapshot.stop_ms,
        budget.stop_ms,
        "ms",
    );
    push_u64_violation(
        &mut violations,
        "recovery_ms",
        snapshot.recovery_ms,
        budget.recovery_ms,
        "ms",
    );

    PerfBudgetReport {
        passed: violations.is_empty(),
        violations,
    }
}

fn push_f32_violation(
    violations: &mut Vec<PerfBudgetViolation>,
    metric: &str,
    actual: f32,
    budget: f32,
    unit: &str,
) {
    if actual > budget {
        violations.push(PerfBudgetViolation {
            metric: metric.to_string(),
            actual: format!("{actual:.2}{unit}"),
            budget: format!("{budget:.2}{unit}"),
        });
    }
}

fn push_u64_violation(
    violations: &mut Vec<PerfBudgetViolation>,
    metric: &str,
    actual: u64,
    budget: u64,
    unit: &str,
) {
    if actual > budget {
        violations.push(PerfBudgetViolation {
            metric: metric.to_string(),
            actual: format!("{actual}{unit}"),
            budget: format!("{budget}{unit}"),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::{PerfBudget, PerfSnapshot, validate_perf_budget};

    #[test]
    fn perf_budget_accepts_values_within_limit() {
        let budget = PerfBudget::windows_release_default();
        let report = validate_perf_budget(
            PerfSnapshot {
                idle_memory_mb: 180.0,
                scanning_cpu_percent: 18.0,
                preview_memory_delta_mb: 42.0,
                startup_ms: 1_200,
                stop_ms: 420,
                recovery_ms: 1_100,
            },
            budget,
        );
        assert!(report.passed);
        assert!(report.violations.is_empty());
    }

    #[test]
    fn perf_budget_reports_over_budget_metrics() {
        let budget = PerfBudget::windows_release_default();
        let report = validate_perf_budget(
            PerfSnapshot {
                idle_memory_mb: 300.0,
                scanning_cpu_percent: 18.0,
                preview_memory_delta_mb: 90.0,
                startup_ms: 1_200,
                stop_ms: 420,
                recovery_ms: 3_000,
            },
            budget,
        );
        assert!(!report.passed);
        assert_eq!(report.violations.len(), 3);
    }
}

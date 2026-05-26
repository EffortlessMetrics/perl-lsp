//! BDD-style scenario helper for narrative test logs.
//!
//! Tests across the workspace use a small Given/When/Then helper to make
//! scenario-style integration and unit tests self-documenting. Each call
//! prints a labeled line to stderr so failing test output reads like a
//! scenario transcript.

/// Narrative scenario logger that prints `Scenario` / `Given` / `When` / `Then`
/// lines to stderr, prefixed with the scenario name.
///
/// # Example
///
/// ```
/// use perl_tdd_support::BddScenario;
///
/// let scenario = BddScenario::new("addition is commutative");
/// scenario.given("two numbers");
/// scenario.when("they are added in either order");
/// scenario.then("the sums are equal");
/// ```
pub struct BddScenario {
    name: &'static str,
}

impl BddScenario {
    /// Start a new scenario, printing a `Scenario: <name>` line to stderr.
    pub fn new(name: &'static str) -> Self {
        eprintln!("Scenario: {name}");
        Self { name }
    }

    /// Print a `Given` step prefixed with the scenario name.
    pub fn given(&self, message: &str) {
        eprintln!("[{}] Given {message}", self.name);
    }

    /// Print a `When` step prefixed with the scenario name.
    pub fn when(&self, message: &str) {
        eprintln!("[{}] When {message}", self.name);
    }

    /// Print a `Then` step prefixed with the scenario name.
    pub fn then(&self, message: &str) {
        eprintln!("[{}] Then {message}", self.name);
    }
}

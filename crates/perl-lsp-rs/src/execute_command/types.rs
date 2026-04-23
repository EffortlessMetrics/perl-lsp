use serde::{Deserialize, Serialize};

/// Commands supported by the Perl LSP server for test execution and code analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PerlCommand {
    /// Run all tests in a file.
    RunTests {
        /// Path to the Perl test file to execute.
        file_path: String,
    },
    /// Run a specific test subroutine.
    RunTestSub {
        /// Path to the Perl file containing the subroutine.
        file_path: String,
        /// Name of the subroutine to execute.
        sub_name: String,
    },
    /// Run a Perl file directly.
    RunFile {
        /// Path to the Perl file to execute.
        file_path: String,
    },
    /// Debug a test file.
    DebugTests {
        /// Path to the Perl file to debug.
        file_path: String,
    },
}

/// Result of executing a command with standardized structure.
#[derive(Debug, Serialize)]
pub struct CommandResult {
    /// Whether the command executed successfully.
    pub success: bool,
    /// Standard output from the command execution.
    pub output: String,
    /// Error message if the command failed.
    pub error: Option<String>,
}

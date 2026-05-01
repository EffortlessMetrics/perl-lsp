/// Output from a subprocess execution
#[derive(Debug, Clone)]
pub struct SubprocessOutput {
    /// Standard output bytes
    pub stdout: Vec<u8>,
    /// Standard error bytes
    pub stderr: Vec<u8>,
    /// Exit status code (0 typically indicates success)
    pub status_code: i32,
}

impl SubprocessOutput {
    /// Returns true if the subprocess exited successfully (status code 0)
    pub fn success(&self) -> bool {
        self.status_code == 0
    }

    /// Returns stdout as a UTF-8 string, lossy converting invalid bytes
    pub fn stdout_lossy(&self) -> String {
        String::from_utf8_lossy(&self.stdout).into_owned()
    }

    /// Returns stderr as a UTF-8 string, lossy converting invalid bytes
    pub fn stderr_lossy(&self) -> String {
        String::from_utf8_lossy(&self.stderr).into_owned()
    }
}

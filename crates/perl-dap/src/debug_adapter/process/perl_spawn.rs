//! Perl executable validation and spawn-error formatting for process launch.

use std::path::Path;

pub(super) fn is_valid_perl_interpreter(perl_interpreter: &str) -> bool {
    let trimmed = perl_interpreter.trim();
    if trimmed.is_empty() {
        return false;
    }

    let candidate = Path::new(trimmed)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(trimmed)
        .to_ascii_lowercase();

    let candidate = candidate.strip_suffix(".exe").unwrap_or(&candidate);
    candidate == "perl" || candidate.starts_with("perl")
}

pub(super) fn format_perl_spawn_error(perl_interpreter: &str, error: &std::io::Error) -> String {
    if error.kind() == std::io::ErrorKind::NotFound {
        #[cfg(windows)]
        {
            return format!(
                "Perl executable ('{perl_interpreter}') is not available on PATH. Install Perl from \
                    https://strawberryperl.com (or ActivePerl), then reload VS Code. \
                    You can also set launch.json `perlPath` to a full Perl path."
            );
        }
        #[cfg(not(windows))]
        {
            return format!(
                "Perl executable ('{perl_interpreter}') is not available on PATH. Install Perl with your package manager \
                    (for example `brew install perl`, `apt install perl`, or your distro equivalent), \
                    then reload VS Code. You can also set launch.json `perlPath` to a full Perl path."
            );
        }
    }

    format!(
        "Perl executable ('{perl_interpreter}') could not be started: {}. \
         Check file permissions, antivirus/AppLocker policy, and sandbox restrictions.",
        error
    )
}

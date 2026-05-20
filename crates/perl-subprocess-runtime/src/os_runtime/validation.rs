use crate::SubprocessError;

pub(super) fn validate_command_input(program: &str, args: &[&str]) -> Result<(), SubprocessError> {
    if program.trim().is_empty() {
        return Err(SubprocessError::new("program name must not be empty"));
    }
    if program.contains('\0') {
        return Err(SubprocessError::new("program name must not contain NUL bytes"));
    }
    if args.iter().any(|arg| arg.contains('\0')) {
        return Err(SubprocessError::new("arguments must not contain NUL bytes"));
    }
    Ok(())
}

# perl-dap-command-args

Small SRP crate that formats command-line arguments for platform shells used by `perl-dap`.

## API

- `format_command_args(args: &[String]) -> Vec<String>`

The formatter wraps arguments that contain spaces using platform-appropriate quoting rules.

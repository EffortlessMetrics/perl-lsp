/// Debug adapter operating mode
///
/// Controls whether the DAP server uses its native `perl -d` adapter
/// or proxies to Perl::LanguageServer's DAP implementation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum DapMode {
    /// Native adapter using `perl -d` directly
    #[default]
    Native,
    /// Bridge adapter proxying to Perl::LanguageServer
    Bridge,
}

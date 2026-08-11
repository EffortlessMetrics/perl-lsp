/// Debug adapter operating mode
///
/// Controls whether the DAP server uses its native `perl -d` adapter.
///
/// The bridge variant remains only for source compatibility with legacy
/// integrations and is not part of the shipped `perl-dap` CLI.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum DapMode {
    /// Native adapter using `perl -d` directly
    #[default]
    Native,
    /// Legacy bridge adapter proxying to Perl::LanguageServer.
    #[deprecated(note = "legacy Perl::LanguageServer compatibility; use DapMode::Native instead")]
    Bridge,
}

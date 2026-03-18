use perl_tdd_governance::ReportFormat as DirectReportFormat;
use perl_tdd_support::governance::ReportFormat as ReexportedReportFormat;

#[test]
fn governance_reexport_matches_microcrate_types() {
    let direct = DirectReportFormat::Json;
    let reexported = ReexportedReportFormat::Json;

    assert_eq!(format!("{direct:?}"), format!("{reexported:?}"));
}

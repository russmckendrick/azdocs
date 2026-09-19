mod common;

use azdocs::cli::{Cli, Command, ReportArgs, ReportFormat};
use azdocs::commands::report::run_selected_with_progress;
use azdocs::config::Config;
use azdocs::store::Store;
use clap::Parser;

#[test]
fn unit_cli_reference_is_opt_in() {
    let Command::Report(args) = Cli::parse_from(["azdocs", "report", "--format", "pdf"]).command
    else {
        panic!("expected report command");
    };
    assert!(!args.include_reference);
    let Command::Report(args) =
        Cli::parse_from(["azdocs", "report", "--include-reference"]).command
    else {
        panic!("expected report command");
    };
    assert!(args.include_reference);
}

#[test]
fn unit_reference_receipts_include_only_selected_print_formats() {
    let store = Store::open_in_memory().unwrap();
    let snapshot = common::seed_estate(&store);
    let recorded_runs = store.query_runs(&snapshot).unwrap();
    assert!(recorded_runs.iter().any(|run| run.provenance.is_some()));
    for include_reference in [false, true] {
        let out = tempfile::tempdir().unwrap();
        let args = ReportArgs {
            snapshot: snapshot.clone(),
            format: ReportFormat::Docx,
            include_reference,
            theme: None,
            out: Some(out.path().into()),
            subscription: None,
            resource_group: None,
            severity: None,
        };
        let mut started = Vec::new();
        let outputs = run_selected_with_progress(
            &Config::default(),
            None,
            &store,
            &args,
            &[ReportFormat::Docx, ReportFormat::Csv],
            |path| {
                assert!(!path.exists(), "progress must precede artifact generation");
                started.push(path.to_path_buf());
            },
        )
        .unwrap();
        assert_eq!(
            started, outputs,
            "every companion needs its own progress phase"
        );
        assert_eq!(outputs.len(), if include_reference { 4 } else { 3 });
        assert!(outputs.iter().all(|path| path.is_file()));
        assert_eq!(
            out.path().join("technical-reference.docx").exists(),
            include_reference
        );
        assert!(!out.path().join("technical-reference.pdf").exists());
        assert!(!out.path().join("technical-reference.csv").exists());
        assert!(!out.path().join("query-provenance.json").exists());
        assert_eq!(store.query_runs(&snapshot).unwrap(), recorded_runs);
    }
}

//! `dnacalc-conform` binary: `diff --corpus <dir> --out <dir>` and
//! `bench --corpus <dir> --out <dir>`.
//!
//! Exit codes: `0` the run completed and the artifact was written (RED
//! coverage or mismatches are results, not failures — floors are a later
//! bead); `1` the run could not complete (I/O, no workbooks, an internal
//! invariant, a bench subject that could not be measured); `2` the
//! arguments did not parse; `3` (`diff --fail-on-twin` only) the run
//! completed, the artifact was written, and the engine's reference twin
//! reported at least one mismatch (W012-1.5).

use std::process::ExitCode;

use dnacalc_conform::bench::{run_bench, write_bench_summary};
use dnacalc_conform::bench_report::BENCH_ARTIFACT_FILE_NAME;
use dnacalc_conform::cli::{Command, USAGE, parse};
use dnacalc_conform::diff::{exit_code, run_diff, write_summary};
use dnacalc_conform::report::ARTIFACT_FILE_NAME;

fn main() -> ExitCode {
    let command = match parse(std::env::args().skip(1)) {
        Ok(command) => command,
        Err(error) => {
            eprintln!("dnacalc-conform: {error}\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    let stdout = std::io::stdout();
    match command {
        Command::Help => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        Command::Diff {
            corpus,
            out,
            options,
        } => match run_diff(&corpus, &out, &options) {
            Ok(report) => {
                let mut handle = stdout.lock();
                if let Err(error) =
                    write_summary(&report, &out.join(ARTIFACT_FILE_NAME), &mut handle)
                {
                    eprintln!("dnacalc-conform: could not write the summary: {error}");
                    return ExitCode::from(1);
                }
                let code = exit_code(&report, &options);
                if code != 0 {
                    eprintln!(
                        "dnacalc-conform: --fail-on-twin: the reference twin reported {} mismatch(es) in {} workbook(s)",
                        report.totals.reference_twin_mismatches,
                        report.totals.reference_twin_workbooks
                    );
                }
                ExitCode::from(code)
            }
            Err(error) => {
                eprintln!("dnacalc-conform: {error}");
                ExitCode::from(1)
            }
        },
        Command::Bench {
            corpus,
            out,
            options,
        } => match run_bench(&corpus, &out, &options) {
            Ok(report) => {
                let mut handle = stdout.lock();
                if let Err(error) =
                    write_bench_summary(&report, &out.join(BENCH_ARTIFACT_FILE_NAME), &mut handle)
                {
                    eprintln!("dnacalc-conform: could not write the summary: {error}");
                    return ExitCode::from(1);
                }
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("dnacalc-conform: {error}");
                ExitCode::from(1)
            }
        },
    }
}

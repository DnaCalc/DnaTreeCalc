//! `dnacalc-conform` binary: `diff --corpus <dir> --out <dir>`.
//!
//! Exit codes: `0` the run completed and the artifact was written (RED
//! coverage or mismatches are results, not failures — floors are a later
//! bead); `1` the run could not complete (I/O, no workbooks, an internal
//! invariant); `2` the arguments did not parse.

use std::process::ExitCode;

use dnacalc_conform::cli::{Command, USAGE, parse};
use dnacalc_conform::diff::{run_diff, write_summary};
use dnacalc_conform::report::ARTIFACT_FILE_NAME;

fn main() -> ExitCode {
    let command = match parse(std::env::args().skip(1)) {
        Ok(command) => command,
        Err(error) => {
            eprintln!("dnacalc-conform: {error}\n{USAGE}");
            return ExitCode::from(2);
        }
    };
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
                let stdout = std::io::stdout();
                let mut handle = stdout.lock();
                if let Err(error) =
                    write_summary(&report, &out.join(ARTIFACT_FILE_NAME), &mut handle)
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

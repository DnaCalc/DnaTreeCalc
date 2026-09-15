//! The `dnacalc-conform` command line — a hand-rolled parser (the workspace
//! has no clap, and the surface is two subcommands with two paths each).

use std::path::PathBuf;

use crate::bench::BenchOptions;
use crate::diff::DiffOptions;

/// The usage text printed on a parse error or `--help`. The test-only
/// `--no-force-recalc` flag is deliberately absent: it exists so the
/// Manual-mode coverage trap can be proven by a test, not as a way to run
/// the instrument.
pub const USAGE: &str = "usage: dnacalc-conform diff  --corpus <dir> --out <dir>
       dnacalc-conform bench --corpus <dir> --out <dir> [bench knobs]

diff:
  Walks every *.xlsx under <dir> (recursively), opens each through the
  product's own path (dnacalc-host-core WorkbookSession::open_xlsx_bytes),
  forces a recalc, scores every formula cell against Excel's cached <v>
  with the provenance-first verdict ladder, and writes
  <out>/divergence.v1.json. Coverage (engine_calculated / formula_cells)
  is printed first; RED is a result, not a failure. Every session runs
  under GridEngineValidationMode::DualValidated, so the engine's own
  reference twin checks every recalc; its mismatches are reported per
  workbook (reference_twin) and per cell (mismatch:ReferenceTwin).
    --fail-on-twin             exit 3 when the reference twin reported any
                               mismatch (the artifact is still written)

bench:
  The grid-engine wall-clock baseline (dtc-5zet.13): the largest
  formula-cell corpus workbooks that open on the product path, plus an
  in-memory synthetic fill-down and a two-sheet cross-reference workbook
  authored through WorkbookSession::enter_grid_cell. Records open, cold
  full recalc (F9, with cells_evaluated), and the Automatic single-cell
  edit wall-clock per subject, and writes <out>/bench.v1.json. Run it
  with --release; a debug build is recorded as such in the artifact.
  Knobs (defaults are the committed baseline shape):
    --corpus-subjects <n>      largest-formula-cell workbooks to bench (3)
    --filldown-rows <n>        synthetic fill-down formula cells (2000;
                               10000 is the bead's long run, ~30 min)
    --cross-sheet-rows <n>     synthetic cross-sheet formula cells (500)
    --open-repeats <n>         timed opens per corpus workbook (5)
    --edit-repeats <n>         timed single-cell edits per subject (5)
    --full-recalc-repeats <n>  Manual-edit + F9 cycles per subject (3)";

/// A parsed invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Diff {
        corpus: PathBuf,
        out: PathBuf,
        options: DiffOptions,
    },
    Bench {
        corpus: PathBuf,
        out: PathBuf,
        options: BenchOptions,
    },
    Help,
}

/// Why the arguments did not parse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgError {
    MissingSubcommand,
    UnknownSubcommand(String),
    MissingValue(String),
    MissingRequired(&'static str),
    InvalidNumber { flag: String, value: String },
    Unknown(String),
}

impl std::fmt::Display for ArgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingSubcommand => write!(f, "missing subcommand (expected `diff` or `bench`)"),
            Self::UnknownSubcommand(name) => write!(f, "unknown subcommand `{name}`"),
            Self::MissingValue(flag) => write!(f, "`{flag}` needs a value"),
            Self::MissingRequired(flag) => write!(f, "`{flag}` is required"),
            Self::InvalidNumber { flag, value } => {
                write!(f, "`{flag}` needs a positive integer, got `{value}`")
            }
            Self::Unknown(arg) => write!(f, "unknown argument `{arg}`"),
        }
    }
}

/// Parse `args` (without the program name).
pub fn parse<I, S>(args: I) -> Result<Command, ArgError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let args: Vec<String> = args.into_iter().map(|a| a.as_ref().to_string()).collect();
    let Some(subcommand) = args.first() else {
        return Err(ArgError::MissingSubcommand);
    };
    match subcommand.as_str() {
        "diff" => parse_diff(&args[1..]),
        "bench" => parse_bench(&args[1..]),
        "--help" | "-h" | "help" => Ok(Command::Help),
        other => Err(ArgError::UnknownSubcommand(other.to_string())),
    }
}

/// The two path arguments both subcommands share; `Ok(None)` when `flag`
/// is not a path flag.
fn take_path(
    args: &[String],
    i: usize,
    corpus: &mut Option<PathBuf>,
    out: &mut Option<PathBuf>,
) -> Result<bool, ArgError> {
    let flag = args[i].as_str();
    if flag != "--corpus" && flag != "--out" {
        return Ok(false);
    }
    let Some(value) = args.get(i + 1) else {
        return Err(ArgError::MissingValue(flag.to_string()));
    };
    if flag == "--corpus" {
        *corpus = Some(PathBuf::from(value));
    } else {
        *out = Some(PathBuf::from(value));
    }
    Ok(true)
}

fn parse_diff(args: &[String]) -> Result<Command, ArgError> {
    let mut corpus = None;
    let mut out = None;
    let mut options = DiffOptions::default();
    let mut i = 0;
    while i < args.len() {
        if take_path(args, i, &mut corpus, &mut out)? {
            i += 2;
            continue;
        }
        match args[i].as_str() {
            "--no-force-recalc" => {
                options.force_recalc = false;
                i += 1;
            }
            "--fail-on-twin" => {
                options.fail_on_twin = true;
                i += 1;
            }
            "--help" | "-h" => return Ok(Command::Help),
            other => return Err(ArgError::Unknown(other.to_string())),
        }
    }
    Ok(Command::Diff {
        corpus: corpus.ok_or(ArgError::MissingRequired("--corpus"))?,
        out: out.ok_or(ArgError::MissingRequired("--out"))?,
        options,
    })
}

fn positive(flag: &str, value: Option<&String>) -> Result<u32, ArgError> {
    let Some(value) = value else {
        return Err(ArgError::MissingValue(flag.to_string()));
    };
    match value.parse::<u32>() {
        Ok(n) if n > 0 => Ok(n),
        _ => Err(ArgError::InvalidNumber {
            flag: flag.to_string(),
            value: value.clone(),
        }),
    }
}

fn parse_bench(args: &[String]) -> Result<Command, ArgError> {
    let mut corpus = None;
    let mut out = None;
    let mut options = BenchOptions::default();
    let mut i = 0;
    while i < args.len() {
        if take_path(args, i, &mut corpus, &mut out)? {
            i += 2;
            continue;
        }
        let flag = args[i].as_str();
        let value = args.get(i + 1);
        match flag {
            "--corpus-subjects" => options.corpus_subjects = positive(flag, value)? as usize,
            "--filldown-rows" => options.filldown_rows = positive(flag, value)?,
            "--cross-sheet-rows" => options.cross_sheet_rows = positive(flag, value)?,
            "--open-repeats" => options.open_repeats = positive(flag, value)?,
            "--edit-repeats" => options.edit_repeats = positive(flag, value)?,
            "--full-recalc-repeats" => options.full_recalc_repeats = positive(flag, value)?,
            "--help" | "-h" => return Ok(Command::Help),
            other => return Err(ArgError::Unknown(other.to_string())),
        }
        i += 2;
    }
    Ok(Command::Bench {
        corpus: corpus.ok_or(ArgError::MissingRequired("--corpus"))?,
        out: out.ok_or(ArgError::MissingRequired("--out"))?,
        options,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_diff_with_both_paths_and_forces_recalc_by_default() {
        let command = parse(["diff", "--corpus", "corpus", "--out", "target/out"]).unwrap();
        assert_eq!(
            command,
            Command::Diff {
                corpus: PathBuf::from("corpus"),
                out: PathBuf::from("target/out"),
                options: DiffOptions {
                    force_recalc: true,
                    fail_on_twin: false
                },
            }
        );
    }

    #[test]
    fn parses_fail_on_twin_and_documents_it() {
        let command = parse(["diff", "--corpus", "c", "--fail-on-twin", "--out", "o"]).unwrap();
        assert!(matches!(
            command,
            Command::Diff {
                options: DiffOptions {
                    force_recalc: true,
                    fail_on_twin: true
                },
                ..
            }
        ));
        assert!(USAGE.contains("--fail-on-twin"));
    }

    #[test]
    fn the_hidden_flag_disables_forcing_and_is_not_in_the_usage_text() {
        let command = parse(["diff", "--no-force-recalc", "--out", "o", "--corpus", "c"]).unwrap();
        assert!(matches!(
            command,
            Command::Diff {
                options: DiffOptions {
                    force_recalc: false,
                    fail_on_twin: false
                },
                ..
            }
        ));
        assert!(!USAGE.contains("no-force-recalc"));
    }

    #[test]
    fn parses_bench_with_defaults_and_every_knob() {
        let command = parse(["bench", "--corpus", "c", "--out", "o"]).unwrap();
        assert_eq!(
            command,
            Command::Bench {
                corpus: PathBuf::from("c"),
                out: PathBuf::from("o"),
                options: BenchOptions::default(),
            }
        );
        let command = parse([
            "bench",
            "--filldown-rows",
            "50",
            "--cross-sheet-rows",
            "7",
            "--corpus",
            "c",
            "--corpus-subjects",
            "1",
            "--open-repeats",
            "2",
            "--edit-repeats",
            "3",
            "--full-recalc-repeats",
            "4",
            "--out",
            "o",
        ])
        .unwrap();
        assert_eq!(
            command,
            Command::Bench {
                corpus: PathBuf::from("c"),
                out: PathBuf::from("o"),
                options: BenchOptions {
                    corpus_subjects: 1,
                    filldown_rows: 50,
                    cross_sheet_rows: 7,
                    open_repeats: 2,
                    edit_repeats: 3,
                    full_recalc_repeats: 4,
                },
            }
        );
        for knob in [
            "--corpus-subjects",
            "--filldown-rows",
            "--cross-sheet-rows",
            "--open-repeats",
            "--edit-repeats",
            "--full-recalc-repeats",
        ] {
            assert!(USAGE.contains(knob), "{knob} is documented");
        }
    }

    #[test]
    fn errors_name_what_is_missing_or_unknown() {
        assert_eq!(
            parse(Vec::<String>::new()),
            Err(ArgError::MissingSubcommand)
        );
        assert_eq!(
            parse(["census"]),
            Err(ArgError::UnknownSubcommand("census".to_string()))
        );
        assert_eq!(
            parse(["diff", "--corpus"]),
            Err(ArgError::MissingValue("--corpus".to_string()))
        );
        assert_eq!(
            parse(["diff", "--corpus", "c"]),
            Err(ArgError::MissingRequired("--out"))
        );
        assert_eq!(
            parse(["diff", "--out", "o"]),
            Err(ArgError::MissingRequired("--corpus"))
        );
        assert_eq!(
            parse(["diff", "--corpus", "c", "--out", "o", "--verbose"]),
            Err(ArgError::Unknown("--verbose".to_string()))
        );
        assert_eq!(
            parse(["bench", "--corpus", "c", "--out", "o", "--filldown-rows"]),
            Err(ArgError::MissingValue("--filldown-rows".to_string()))
        );
        assert_eq!(
            parse([
                "bench",
                "--corpus",
                "c",
                "--out",
                "o",
                "--filldown-rows",
                "0"
            ]),
            Err(ArgError::InvalidNumber {
                flag: "--filldown-rows".to_string(),
                value: "0".to_string()
            })
        );
        assert_eq!(
            parse([
                "bench",
                "--corpus",
                "c",
                "--out",
                "o",
                "--edit-repeats",
                "five"
            ]),
            Err(ArgError::InvalidNumber {
                flag: "--edit-repeats".to_string(),
                value: "five".to_string()
            })
        );
        assert_eq!(
            parse(["bench", "--out", "o"]),
            Err(ArgError::MissingRequired("--corpus"))
        );
        assert_eq!(parse(["--help"]), Ok(Command::Help));
        assert_eq!(parse(["diff", "-h"]), Ok(Command::Help));
        assert_eq!(parse(["bench", "-h"]), Ok(Command::Help));
    }
}

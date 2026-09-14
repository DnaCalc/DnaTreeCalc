//! The `dnacalc-conform` command line — a hand-rolled parser (the workspace
//! has no clap, and the surface is one subcommand with two paths).

use std::path::PathBuf;

use crate::diff::DiffOptions;

/// The usage text printed on a parse error or `--help`. The test-only
/// `--no-force-recalc` flag is deliberately absent: it exists so the
/// Manual-mode coverage trap can be proven by a test, not as a way to run
/// the instrument.
pub const USAGE: &str = "usage: dnacalc-conform diff --corpus <dir> --out <dir>

  Walks every *.xlsx under <dir> (recursively), opens each through the
  product's own path (dnacalc-host-core WorkbookSession::open_xlsx_bytes),
  forces a recalc, scores every formula cell against Excel's cached <v>
  with the provenance-first verdict ladder, and writes
  <out>/divergence.v1.json. Coverage (engine_calculated / formula_cells)
  is printed first; RED is a result, not a failure.";

/// A parsed invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Diff {
        corpus: PathBuf,
        out: PathBuf,
        options: DiffOptions,
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
    Unknown(String),
}

impl std::fmt::Display for ArgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingSubcommand => write!(f, "missing subcommand (expected `diff`)"),
            Self::UnknownSubcommand(name) => write!(f, "unknown subcommand `{name}`"),
            Self::MissingValue(flag) => write!(f, "`{flag}` needs a value"),
            Self::MissingRequired(flag) => write!(f, "`{flag}` is required"),
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
        "--help" | "-h" | "help" => Ok(Command::Help),
        other => Err(ArgError::UnknownSubcommand(other.to_string())),
    }
}

fn parse_diff(args: &[String]) -> Result<Command, ArgError> {
    let mut corpus = None;
    let mut out = None;
    let mut options = DiffOptions::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--corpus" | "--out" => {
                let flag = args[i].clone();
                let Some(value) = args.get(i + 1) else {
                    return Err(ArgError::MissingValue(flag));
                };
                if flag == "--corpus" {
                    corpus = Some(PathBuf::from(value));
                } else {
                    out = Some(PathBuf::from(value));
                }
                i += 2;
            }
            "--no-force-recalc" => {
                options.force_recalc = false;
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
                options: DiffOptions { force_recalc: true },
            }
        );
    }

    #[test]
    fn the_hidden_flag_disables_forcing_and_is_not_in_the_usage_text() {
        let command = parse(["diff", "--no-force-recalc", "--out", "o", "--corpus", "c"]).unwrap();
        assert!(matches!(
            command,
            Command::Diff {
                options: DiffOptions {
                    force_recalc: false
                },
                ..
            }
        ));
        assert!(!USAGE.contains("no-force-recalc"));
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
        assert_eq!(parse(["--help"]), Ok(Command::Help));
        assert_eq!(parse(["diff", "-h"]), Ok(Command::Help));
    }
}

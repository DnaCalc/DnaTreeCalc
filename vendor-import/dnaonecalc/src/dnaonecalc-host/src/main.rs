use std::path::PathBuf;
use std::process::ExitCode;

use dnaonecalc_host::services::formula_drill_audit::{
    audit_formula_drill_case, build_default_formula_drill_audit_report,
    render_formula_drill_audit_markdown, FormulaDrillAuditReport,
};
use dnaonecalc_host::services::programmatic_testing::{
    default_verification_config, load_verification_config_xml, ProgrammaticComparisonStatus,
};
use dnaonecalc_host::services::verification_bundle::{
    default_output_root, load_verification_batch_request, run_verification_batch,
    single_case_request_with_config, single_xml_case_request_with_config, VerificationBundleReport,
};

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("verify-formula") => run_verify_formula(args.collect()),
        Some("verify-xml-cell") => run_verify_xml_cell(args.collect()),
        Some("verify-batch") => run_verify_batch(args.collect()),
        Some("audit-formula-drill") => run_audit_formula_drill(args.collect()),
        Some("--help") | Some("-h") | Some("help") => {
            print_help();
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("unknown command: {other}");
            print_help();
            ExitCode::from(2)
        }
        None => {
            print_help();
            ExitCode::SUCCESS
        }
    }
}

fn run_audit_formula_drill(args: Vec<String>) -> ExitCode {
    let mut formulas = Vec::new();
    let mut format = "markdown".to_string();
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--formula" => {
                if let Some(formula) = args.get(index + 1) {
                    let case_id = format!("manual_{}", formulas.len() + 1);
                    formulas.push((case_id, formula.clone()));
                    index += 2;
                } else {
                    eprintln!("audit-formula-drill requires a value after --formula");
                    return ExitCode::from(2);
                }
            }
            "--format" => {
                if let Some(value) = args.get(index + 1) {
                    format = value.clone();
                    index += 2;
                } else {
                    eprintln!("audit-formula-drill requires a value after --format");
                    return ExitCode::from(2);
                }
            }
            other => {
                eprintln!("unexpected audit-formula-drill argument: {other}");
                print_help();
                return ExitCode::from(2);
            }
        }
    }

    let report = if formulas.is_empty() {
        build_default_formula_drill_audit_report()
    } else {
        FormulaDrillAuditReport {
            schema_id: "dnaonecalc.formula-drill-audit.v1",
            cases: formulas
                .iter()
                .map(|(case_id, formula)| audit_formula_drill_case(case_id, formula))
                .collect(),
        }
    };

    match format.as_str() {
        "json" => match serde_json::to_string_pretty(&report) {
            Ok(json) => {
                println!("{json}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("failed to serialise formula drill audit JSON: {error}");
                ExitCode::from(4)
            }
        },
        "markdown" | "md" => {
            print!("{}", render_formula_drill_audit_markdown(&report));
            ExitCode::SUCCESS
        }
        other => {
            eprintln!("unsupported audit-formula-drill --format {other}; use markdown or json");
            ExitCode::from(2)
        }
    }
}

fn run_verify_formula(args: Vec<String>) -> ExitCode {
    let mut case_id = None;
    let mut formula = None;
    let mut output_root = None;
    let mut config_xml = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--case-id" => {
                case_id = args.get(index + 1).cloned();
                index += 2;
            }
            "--formula" => {
                formula = args.get(index + 1).cloned();
                index += 2;
            }
            "--output-root" => {
                output_root = args.get(index + 1).map(PathBuf::from);
                index += 2;
            }
            "--config-xml" => {
                config_xml = args.get(index + 1).map(PathBuf::from);
                index += 2;
            }
            other => {
                eprintln!("unexpected verify-formula argument: {other}");
                print_help();
                return ExitCode::from(2);
            }
        }
    }

    let Some(case_id) = case_id else {
        eprintln!("verify-formula requires --case-id <id>");
        return ExitCode::from(2);
    };
    let Some(formula) = formula else {
        eprintln!("verify-formula requires --formula <text>");
        return ExitCode::from(2);
    };

    let config = match load_config_or_default(config_xml.as_ref()) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(4);
        }
    };
    let request = single_case_request_with_config(case_id, formula, &config);
    let output_root = match output_root {
        Some(path) => path,
        None => match default_output_root() {
            Ok(path) => path,
            Err(error) => {
                eprintln!("{error}");
                return ExitCode::from(4);
            }
        },
    };

    match run_verification_batch(&request, &output_root) {
        Ok(report) => {
            print_report(&report);
            exit_code_for_report(&report)
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(4)
        }
    }
}

fn run_verify_batch(args: Vec<String>) -> ExitCode {
    let mut input_path = None;
    let mut output_root = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--input" => {
                input_path = args.get(index + 1).map(PathBuf::from);
                index += 2;
            }
            "--output-root" => {
                output_root = args.get(index + 1).map(PathBuf::from);
                index += 2;
            }
            other => {
                eprintln!("unexpected verify-batch argument: {other}");
                print_help();
                return ExitCode::from(2);
            }
        }
    }

    let Some(input_path) = input_path else {
        eprintln!("verify-batch requires --input <path>");
        return ExitCode::from(2);
    };

    let request = match load_verification_batch_request(&input_path) {
        Ok(request) => request,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(4);
        }
    };
    let output_root = match output_root {
        Some(path) => path,
        None => match default_output_root() {
            Ok(path) => path,
            Err(error) => {
                eprintln!("{error}");
                return ExitCode::from(4);
            }
        },
    };

    match run_verification_batch(&request, &output_root) {
        Ok(report) => {
            print_report(&report);
            exit_code_for_report(&report)
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(4)
        }
    }
}

fn run_verify_xml_cell(args: Vec<String>) -> ExitCode {
    let mut case_id = None;
    let mut workbook_xml = None;
    let mut locator = None;
    let mut output_root = None;
    let mut config_xml = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--case-id" => {
                case_id = args.get(index + 1).cloned();
                index += 2;
            }
            "--workbook-xml" => {
                workbook_xml = args.get(index + 1).cloned();
                index += 2;
            }
            "--locator" => {
                locator = args.get(index + 1).cloned();
                index += 2;
            }
            "--output-root" => {
                output_root = args.get(index + 1).map(PathBuf::from);
                index += 2;
            }
            "--config-xml" => {
                config_xml = args.get(index + 1).map(PathBuf::from);
                index += 2;
            }
            other => {
                eprintln!("unexpected verify-xml-cell argument: {other}");
                print_help();
                return ExitCode::from(2);
            }
        }
    }

    let Some(case_id) = case_id else {
        eprintln!("verify-xml-cell requires --case-id <id>");
        return ExitCode::from(2);
    };
    let Some(workbook_xml) = workbook_xml else {
        eprintln!("verify-xml-cell requires --workbook-xml <path>");
        return ExitCode::from(2);
    };
    let Some(locator) = locator else {
        eprintln!("verify-xml-cell requires --locator <Sheet!Cell>");
        return ExitCode::from(2);
    };

    let config = match load_config_or_default(config_xml.as_ref()) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(4);
        }
    };
    let request = match single_xml_case_request_with_config(case_id, workbook_xml, locator, &config)
    {
        Ok(request) => request,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(4);
        }
    };
    let output_root = match output_root {
        Some(path) => path,
        None => match default_output_root() {
            Ok(path) => path,
            Err(error) => {
                eprintln!("{error}");
                return ExitCode::from(4);
            }
        },
    };

    match run_verification_batch(&request, &output_root) {
        Ok(report) => {
            print_report(&report);
            exit_code_for_report(&report)
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(4)
        }
    }
}

fn print_report(report: &VerificationBundleReport) {
    println!("verification bundle: {}", report.output_root);
    println!(
        "host profile: {} | excel observation available: {}",
        report.host_profile.profile_id, report.capabilities.excel_observation_available
    );
    println!("cases: {}", report.case_reports.len());
    for case in &report.case_reports {
        println!(
            "- {} | {:?} | OxFml={} | Excel={} | value_match={} | display_match={} | replay_equivalent={} | artifact={}",
            case.case_id,
            case.comparison_status,
            case.oxfml_summary
                .effective_display_summary
                .as_deref()
                .unwrap_or("<unavailable>"),
            case.excel_summary
                .as_ref()
                .and_then(|summary| summary.effective_display_text.as_deref())
                .unwrap_or("<unavailable>"),
            case.value_match
                .map(|value| value.to_string())
                .unwrap_or_else(|| "n/a".to_string()),
            case.display_match
                .map(|value| value.to_string())
                .unwrap_or_else(|| "n/a".to_string()),
            case.replay_equivalent
                .map(|value| value.to_string())
                .unwrap_or_else(|| "n/a".to_string()),
            case.artifact_catalog_entry.artifact_id
        );
        if let Some(discrepancy_summary) = &case.discrepancy_summary {
            println!("  discrepancy: {discrepancy_summary}");
        }
        if let Some(extraction) = &case.spreadsheet_xml_extraction {
            println!(
                "  source workbook: {} @ {}",
                extraction.workbook_path, extraction.locator
            );
        }
        println!("  case output: {}", case.case_output_dir);
    }
}

fn exit_code_for_report(report: &VerificationBundleReport) -> ExitCode {
    if report
        .case_reports
        .iter()
        .any(|case| case.comparison_status != ProgrammaticComparisonStatus::Matched)
    {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn print_help() {
    eprintln!(
        "dnaonecalc-host\n\
         \n\
         Commands:\n\
          verify-formula --case-id <id> --formula <text> [--config-xml <path>] [--output-root <path>]\n\
          verify-xml-cell --case-id <id> --workbook-xml <path> --locator <Sheet!Cell> [--config-xml <path>] [--output-root <path>]\n\
          verify-batch --input <json-path> [--output-root <path>]\n\
          audit-formula-drill [--formula <text>]... [--format markdown|json]\n\
         \n\
         Verification config XML shape:\n\
          <verification-config>\n\
            <host-profile profile-id=\"windows_excel_default\" requires-excel-observation=\"true\" />\n\
            <capabilities host-summary=\"windows_native_excel_default\" excel-observation-available=\"true\" />\n\
          </verification-config>\n\
         \n\
         Batch input JSON shape:\n\
           {{\n\
             \"host_profile\": {{ \"profile_id\": \"windows_excel_default\", \"requires_excel_observation\": true }},\n\
             \"capabilities\": {{ \"host_summary\": \"windows_native_excel_default\", \"excel_observation_available\": true }},\n\
             \"cases\": [\n\
               {{ \"case_id\": \"case-1\", \"entered_cell_text\": \"=SUM(1,2,3)\", \"spreadsheet_xml_source\": null }},\n\
               {{ \"case_id\": \"case-2\", \"entered_cell_text\": \"\", \"spreadsheet_xml_source\": {{ \"workbook_path\": \"C:/path/workbook.xml\", \"locator\": \"Input!A1\" }} }}\n\
             ]\n\
           }}"
    );
}

fn load_config_or_default(
    config_xml: Option<&PathBuf>,
) -> Result<dnaonecalc_host::services::programmatic_testing::ProgrammaticVerificationConfig, String>
{
    match config_xml {
        Some(path) => load_verification_config_xml(path),
        None => Ok(default_verification_config()),
    }
}

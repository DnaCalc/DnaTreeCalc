// DNA OneCalc Desktop Shell
//
// Thin Tauri v2 wrapper over the shared Leptos/WASM application core.
// This host owns startup, window chrome, and platform-specific wiring only.
// Product behavior lives in the shared `dnaonecalc-host` crate.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::Path;

use dnaonecalc_host::services::vba_host::{
    VbaHostRuntime, VbaProjectAssociation, VbaUdfAdmissionStatus,
};

#[derive(Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum DesktopVbaLoadStatus {
    Loaded,
    Failed,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopVbaModuleSourceSelection {
    display_name: String,
    source_path: String,
    source_kind: String,
    source_text: String,
    load_status: DesktopVbaLoadStatus,
    admitted_udfs: Vec<String>,
    rejected_candidates: Vec<String>,
    diagnostics: Vec<String>,
}

#[tauri::command]
fn select_vba_module_source() -> Result<Option<DesktopVbaModuleSourceSelection>, String> {
    let Some(path) = rfd::FileDialog::new()
        .set_title("Select VBA module")
        .add_filter("VBA module", &["bas"])
        .pick_file()
    else {
        return Ok(None);
    };

    Ok(Some(load_vba_module_source_selection(&path)))
}

fn load_vba_module_source_selection(path: &Path) -> DesktopVbaModuleSourceSelection {
    let source_path = path.display().to_string();
    let display_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&source_path)
        .to_string();

    let source_text = match std::fs::read_to_string(path) {
        Ok(source_text) => source_text,
        Err(error) => {
            return DesktopVbaModuleSourceSelection {
                display_name,
                source_path,
                source_kind: "module_source".to_string(),
                source_text: String::new(),
                load_status: DesktopVbaLoadStatus::Failed,
                admitted_udfs: Vec::new(),
                rejected_candidates: Vec::new(),
                diagnostics: vec![format!("failed to read VBA module source: {error}")],
            };
        }
    };

    let association =
        VbaProjectAssociation::workspace_source("desktop-vba-command-probe", &source_path);
    match VbaHostRuntime::load_project_path_for_evaluation(
        association,
        path,
        env!("CARGO_PKG_VERSION"),
    ) {
        Ok(runtime) => DesktopVbaModuleSourceSelection {
            display_name,
            source_path,
            source_kind: "module_source".to_string(),
            source_text,
            load_status: DesktopVbaLoadStatus::Loaded,
            admitted_udfs: runtime
                .registrations()
                .iter()
                .map(|registration| registration.formula_name.clone())
                .collect(),
            rejected_candidates: runtime
                .candidates()
                .iter()
                .filter(|candidate| candidate.admission_status != VbaUdfAdmissionStatus::Admitted)
                .map(|candidate| candidate.procedure_name.clone())
                .collect(),
            diagnostics: Vec::new(),
        },
        Err(diagnostic) => DesktopVbaModuleSourceSelection {
            display_name,
            source_path,
            source_kind: "module_source".to_string(),
            source_text,
            load_status: DesktopVbaLoadStatus::Failed,
            admitted_udfs: Vec::new(),
            rejected_candidates: Vec::new(),
            diagnostics: vec![diagnostic],
        },
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![select_vba_module_source])
        .run(tauri::generate_context!())
        .expect("failed to run DNA OneCalc desktop application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture_path(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("target")
            .join("dnaonecalc-desktop-tests")
            .join(name)
    }

    #[test]
    fn load_vba_module_source_selection_reports_real_path_and_udfs() {
        let path = fixture_path("SimpleVba.bas");
        std::fs::create_dir_all(path.parent().expect("fixture parent"))
            .expect("fixture dir should be created under target");
        std::fs::write(
            &path,
            concat!(
                "Public Function AddThem(a As Double, b As Double) As Double\n",
                "AddThem = a + b\n",
                "End Function\n"
            ),
        )
        .expect("fixture source should be written");

        let selection = load_vba_module_source_selection(&path);

        assert_eq!(selection.display_name, "SimpleVba.bas");
        assert_eq!(selection.source_path, path.display().to_string());
        assert_eq!(selection.source_kind, "module_source");
        assert_eq!(selection.load_status, DesktopVbaLoadStatus::Loaded);
        assert!(selection.admitted_udfs[0].eq_ignore_ascii_case("AddThem"));
        assert!(selection.diagnostics.is_empty());
    }

    #[test]
    fn load_vba_module_source_selection_reports_read_failure() {
        let path = fixture_path("Missing.bas");
        let _ = std::fs::remove_file(&path);

        let selection = load_vba_module_source_selection(&path);

        assert_eq!(selection.display_name, "Missing.bas");
        assert_eq!(selection.source_path, path.display().to_string());
        assert_eq!(selection.load_status, DesktopVbaLoadStatus::Failed);
        assert!(selection.source_text.is_empty());
        assert!(selection.diagnostics[0].contains("failed to read VBA module source"));
    }
}

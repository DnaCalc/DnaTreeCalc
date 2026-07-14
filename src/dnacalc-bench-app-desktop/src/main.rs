// DNA Bench desktop shell (dtc-tsc.9).
//
// Thin Tauri v2 wrapper: the window loads the committed `dnacalc-bench-app`
// WASM frontend (frontendDist in tauri.conf.json). Product behavior lives in
// the shared app/host crates; this host owns startup + window chrome only.
//
// Tauri GUI launch cannot be asserted headlessly in CI, so the S0 smoke is a
// `cargo build` of this binary plus a documented manual launch note (bead
// acceptance's honest fallback). The one keyboard-verb-dispatch smoke is proven
// in the browser suite against the same shared shell.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

/// The `workspace.json` the native open dialog resolved (bead dtc-lfz.9).
/// The WASM frontend reads `path`/`content` back off this via the
/// `tauri_file_io` bridge; the desktop side does the file IO because the
/// wasm frontend cannot `std::fs`. Only raw text crosses — deserialize/apply
/// stays in the shared host, so this shell keeps zero `dnacalc-*` edges.
#[derive(Debug, serde::Serialize)]
struct OpenedWorkspaceFile {
    path: String,
    content: String,
}

/// Show the native open dialog for a `workspace.json` and read it. `Ok(None)`
/// is a user-cancelled dialog (a real no-op, not an error); `Err` is a genuine
/// read failure. The frontend's `open_workspace_via_tauri_dialog` invokes this
/// and hands the content to `persistence::open_workspace_from_content`.
#[tauri::command]
fn open_workspace_file() -> Result<Option<OpenedWorkspaceFile>, String> {
    let Some(path) = rfd::FileDialog::new()
        .set_title("Open DNA Bench workspace")
        .add_filter("DNA Bench workspace", &["json"])
        .pick_file()
    else {
        return Ok(None);
    };

    let content = std::fs::read_to_string(&path)
        .map_err(|error| format!("failed to read workspace file: {error}"))?;

    Ok(Some(OpenedWorkspaceFile {
        path: path.display().to_string(),
        content,
    }))
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![open_workspace_file])
        .run(tauri::generate_context!())
        .expect("run DNA Bench desktop shell");
}

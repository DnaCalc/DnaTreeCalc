// DNA Calc desktop shell (dtc-tsc.9; W011 Wave 1.5 file dialogs, dtc-j7n8.10).
//
// Thin Tauri v2 wrapper: the window loads the committed `dnacalc-app` WASM
// frontend (frontendDist in tauri.conf.json). Product behavior lives in the
// shared app/host crates; this host owns startup + window chrome — and, since
// dtc-j7n8.10, the NATIVE FILE DIALOGS. The shell is the host's outer skin,
// so native file APIs are fine HERE (never in a skin-IR skin): only plain
// bytes cross the IPC seam. `open_xlsx_file` hands the picked file's bytes to
// the WASM frontend, which routes them through the host dispatcher as
// `HostCommand::OpenXlsxBytes`; `save_xlsx_file` takes the bytes
// `HostCommand::SaveActiveXlsx` produced and writes them where the user
// pointed the save dialog. This crate keeps zero `dnacalc-*` edges — OxDoc,
// the engine and the command surface all live behind the frontend, exactly
// as the sibling `dnacalc-bench-app-desktop` keeps its `open_workspace_file`
// content-only.
//
// Tauri GUI launch cannot be asserted headlessly in CI, so the automated
// floor is the file helpers below (unit-tested against the committed W011
// fixture and a scratch write) plus a `cargo build` of this binary; the
// open -> edit -> save -> reopen click-through is manual and recorded in the
// bead close reason (AGENTS.md: UX beads add a click-through).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::{Path, PathBuf};

/// The `.xlsx` the native open dialog resolved: `path` for the mast /
/// persistence projection, `name` (the file name) for the document name the
/// host reports on `Opened`, and the raw package `bytes` OxDoc opens.
///
/// `bytes` crosses Tauri's JSON IPC as a number array — fine for the W011
/// fixture class of file (a few KB); a large workbook would want Tauri's raw
/// `ipc::Response` body instead (a later seam, not a Wave 1.5 concern).
#[derive(Debug, PartialEq, Eq, serde::Serialize)]
struct OpenedXlsxFile {
    path: String,
    name: String,
    bytes: Vec<u8>,
}

/// Where the native save dialog wrote the package, and how much.
#[derive(Debug, PartialEq, Eq, serde::Serialize)]
struct SavedXlsxFile {
    path: String,
    name: String,
    bytes_written: usize,
}

/// The file name of `path` as the user sees it (the whole path when it has
/// no final component — never a fabricated name).
fn file_name_of(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

/// Read one `.xlsx` package from disk. `Err` is the typed read failure text
/// (a missing / unreadable file), never a panic on the open path.
fn read_xlsx_file(path: &Path) -> Result<OpenedXlsxFile, String> {
    let bytes = std::fs::read(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    Ok(OpenedXlsxFile {
        path: path.display().to_string(),
        name: file_name_of(path),
        bytes,
    })
}

/// Write the package bytes the host produced to `path`, verbatim.
fn write_xlsx_file(path: &Path, bytes: &[u8]) -> Result<SavedXlsxFile, String> {
    std::fs::write(path, bytes)
        .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
    Ok(SavedXlsxFile {
        path: path.display().to_string(),
        name: file_name_of(path),
        bytes_written: bytes.len(),
    })
}

/// Make sure the chosen save path ends in `.xlsx` (the dialog's filter does
/// not append it on every platform). An existing `.xlsx` (any case) is kept;
/// anything else gets `.xlsx` APPENDED — `book.v2` becomes `book.v2.xlsx`,
/// never `book.xlsx`, so no part of the user's name is silently dropped.
fn with_xlsx_extension(path: PathBuf) -> PathBuf {
    let has_xlsx = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("xlsx"));
    if has_xlsx {
        return path;
    }
    let mut with_extension = path.into_os_string();
    with_extension.push(".xlsx");
    PathBuf::from(with_extension)
}

/// Show the native open dialog for an `.xlsx` and read it. `Ok(None)` is a
/// user-cancelled dialog (a real no-op, not an error); `Err` is a genuine
/// read failure. The frontend's `shell_files::pick_xlsx_to_open` invokes this
/// and hands the bytes to the host's `OpenXlsxBytes` command.
#[tauri::command]
fn open_xlsx_file() -> Result<Option<OpenedXlsxFile>, String> {
    let Some(path) = rfd::FileDialog::new()
        .set_title("Open workbook")
        .add_filter("Excel workbook", &["xlsx"])
        .pick_file()
    else {
        return Ok(None);
    };
    read_xlsx_file(&path).map(Some)
}

/// Show the native save dialog (seeded with the document's own name and, when
/// known, the folder it was opened from) and write the bytes the host's
/// `SaveActiveXlsx` produced. `Ok(None)` is a cancelled dialog; `Err` a real
/// write failure. The frontend's `shell_files::pick_path_and_save_xlsx`
/// invokes this.
#[tauri::command(rename_all = "snake_case")]
fn save_xlsx_file(
    suggested_name: String,
    suggested_directory: Option<String>,
    bytes: Vec<u8>,
) -> Result<Option<SavedXlsxFile>, String> {
    let mut dialog = rfd::FileDialog::new()
        .set_title("Save workbook as")
        .add_filter("Excel workbook", &["xlsx"])
        .set_file_name(suggested_name);
    if let Some(directory) = suggested_directory.filter(|directory| !directory.is_empty()) {
        dialog = dialog.set_directory(directory);
    }
    let Some(path) = dialog.save_file() else {
        return Ok(None);
    };
    write_xlsx_file(&with_xlsx_extension(path), &bytes).map(Some)
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![open_xlsx_file, save_xlsx_file])
        .run(tauri::generate_context!())
        .expect("run DNA Calc desktop shell");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Repo-relative location of the committed W011 fixture binary, walked
    /// from this crate's manifest dir (`src/dnacalc-app-desktop`).
    const FIXTURE_XLSX_REL: &str = "../../fixtures/w011/a1_times_three.xlsx";

    fn fixture_path() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE_XLSX_REL)
    }

    /// A unique scratch path under the OS temp dir (each test writes its own
    /// file, so parallel tests never collide), removed by the caller.
    fn scratch_path(stem: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "dnacalc-app-desktop-{stem}-{}-{}.xlsx",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|elapsed| elapsed.as_nanos())
                .unwrap_or_default()
        ))
    }

    /// The open half of the dialog seam reads the committed fixture as a
    /// real zip package (`PK\x03\x04` local-file header) under its own file
    /// name — the exact bytes + name the frontend hands to `OpenXlsxBytes`.
    #[test]
    fn read_xlsx_file_reads_the_committed_fixture_bytes() {
        let opened = read_xlsx_file(&fixture_path()).expect("the committed fixture reads");
        assert_eq!(opened.name, "a1_times_three.xlsx");
        assert!(
            opened.bytes.starts_with(b"PK\x03\x04"),
            "the fixture is a zip package: {:?}",
            &opened.bytes[..4.min(opened.bytes.len())]
        );
        assert!(!opened.bytes.is_empty());
        assert!(
            opened.path.ends_with("a1_times_three.xlsx"),
            "{}",
            opened.path
        );
    }

    /// A missing file is a typed `Err` naming the path — never a panic on the
    /// open path.
    #[test]
    fn read_xlsx_file_reports_a_typed_error_for_a_missing_path() {
        let missing = scratch_path("missing");
        let error = read_xlsx_file(&missing).expect_err("a missing file is refused");
        assert!(
            error.contains(&missing.display().to_string()),
            "the error names the path: {error}"
        );
    }

    /// The save half writes the host's bytes verbatim: a scratch write of the
    /// fixture bytes reads back byte-for-byte, and the report carries the
    /// written length + file name.
    #[test]
    fn write_xlsx_file_round_trips_bytes_to_a_scratch_path() {
        let bytes = std::fs::read(fixture_path()).expect("fixture bytes");
        let target = scratch_path("saved");
        let saved = write_xlsx_file(&target, &bytes).expect("the scratch write succeeds");
        let read_back = std::fs::read(&target).expect("the written file reads back");
        let _ = std::fs::remove_file(&target);
        assert_eq!(read_back, bytes, "the bytes round-trip verbatim");
        assert_eq!(saved.bytes_written, bytes.len());
        assert_eq!(saved.name, file_name_of(&target));
        assert_eq!(saved.path, target.display().to_string());
    }

    /// `.xlsx` is appended when missing (keeping every part of the user's
    /// name) and kept when present in any case.
    #[test]
    fn with_xlsx_extension_appends_only_when_missing() {
        assert_eq!(
            with_xlsx_extension(PathBuf::from("book")),
            PathBuf::from("book.xlsx")
        );
        assert_eq!(
            with_xlsx_extension(PathBuf::from("book.v2")),
            PathBuf::from("book.v2.xlsx")
        );
        assert_eq!(
            with_xlsx_extension(PathBuf::from("book.xlsx")),
            PathBuf::from("book.xlsx")
        );
        assert_eq!(
            with_xlsx_extension(PathBuf::from("BOOK.XLSX")),
            PathBuf::from("BOOK.XLSX")
        );
    }
}

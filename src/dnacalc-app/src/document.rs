//! The Calc app's document controller (W011 Wave 1.5, dtc-j7n8.10): the app
//! root's bookkeeping around the host's open/save commands.
//!
//! The shell's chrome already knows how to ask for a document lifecycle — the
//! command deck's `shell.open` / `shell.save` entries and the Ctrl+O / Ctrl+S
//! verbs, enabled by the [`PersistenceProjection`] the product advertises and
//! answered over `on_shell_intent` / `on_shell_verb` (bead dtc-lfz.3). This
//! controller is what the Calc product plugs into that seam:
//!
//! - it advertises `can_open` only when the desktop shell's file bridge is
//!   reachable, `can_save` only when, in addition, an `.xlsx`-backed document
//!   is active (the in-memory demo has no package to round-trip — host-core's
//!   typed `NoBackingSource`), `dirty` after an accepted model mutation until
//!   the next save/open, and `current_path` as the file the document was
//!   opened from / last saved to;
//! - it runs the two commands of dtc-j7n8.8's adapter surface
//!   ([`open_xlsx_command`] / [`save_xlsx_command`]) against the app's
//!   `WorkbookHostDispatcher` and folds the typed [`OpenOutcome`] /
//!   [`SaveOutcome`] into the projection, the mast's document name, and one
//!   honest [`DocumentStatus`] line the click-through reads;
//! - it never touches a file: bytes come in from and go out to the shell's
//!   file bridge (`shell_files`, desktop only), which the app root drives.
//!
//! Target-independent, so the native suite proves the whole open -> edit ->
//! save -> reopen bookkeeping over the committed fixture bytes.

use std::sync::Arc;

use leptos::prelude::*;

use dnacalc_skin_ir::keychord::SkinVerb;
use dnacalc_skin_ir::protocol::PersistenceProjection;
use dnacalc_skin_ir::state::{SharedStateChange, SharedStateOrigin};
use dnacalc_skin_ir::workspace::WorkspaceState;
use dnacalc_skin_leptos::state_handles::SharedSkinStateHandle;
use dnatreecalc_host::app::WorkbookHostDispatcher;

use crate::adapter::{
    OpenOutcome, SaveOutcome, interpret_open_outcome, interpret_save_outcome, open_xlsx_command,
    save_xlsx_command,
};

/// The default name a save dialog is seeded with when the active document
/// has no file name of its own (the in-memory demo — whose save the host
/// refuses anyway, so this only ever labels a status line).
pub const UNTITLED_XLSX: &str = "workbook.xlsx";

/// One honest line about the last document-lifecycle step, rendered by the
/// app root (`data-testid="calc-document"`, `data-document-status=label`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentStatus {
    /// A short, stable label (the `data-document-status` a test reads).
    pub label: &'static str,
    /// The human-readable detail.
    pub detail: String,
}

impl DocumentStatus {
    fn new(label: &'static str, detail: impl Into<String>) -> Self {
        Self {
            label,
            detail: detail.into(),
        }
    }
}

/// Which lifecycle verb a shell-side note is about (the two the bridge
/// carries).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileVerb {
    Open,
    Save,
}

impl FileVerb {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            FileVerb::Open => "open",
            FileVerb::Save => "save",
        }
    }

    /// The bridge-carried verbs, read off the shell's universal grammar
    /// (Ctrl+O / Ctrl+S); every other verb the shell forwards is `None`.
    #[must_use]
    pub fn from_shell_verb(verb: SkinVerb) -> Option<Self> {
        match verb {
            SkinVerb::Open => Some(FileVerb::Open),
            SkinVerb::Save => Some(FileVerb::Save),
            _ => None,
        }
    }
}

/// The app root's handle on the active document's lifecycle. Cheap to clone
/// (an `Arc` plus `Copy` signals); `Send + Sync` so it rides inside the
/// shell's `Callback`s.
#[derive(Clone)]
pub struct DocumentController {
    workbook: Arc<WorkbookHostDispatcher>,
    workspace: ReadSignal<WorkspaceState>,
    shared: SharedSkinStateHandle,
    persistence: RwSignal<PersistenceProjection>,
    status: RwSignal<Option<DocumentStatus>>,
    /// The name of the active `.xlsx` document (`None` for the demo).
    document_name: RwSignal<Option<String>>,
    bridge_available: bool,
}

impl DocumentController {
    /// Wrap the app's workbook dispatcher. `bridge_available` is whether the
    /// shell can pick/write files for this runtime (the desktop shell's
    /// Tauri bridge); it gates `can_open` / `can_save` honestly.
    #[must_use]
    pub fn new(
        workbook: Arc<WorkbookHostDispatcher>,
        workspace: ReadSignal<WorkspaceState>,
        shared: SharedSkinStateHandle,
        bridge_available: bool,
    ) -> Self {
        let persistence = RwSignal::new(PersistenceProjection {
            can_save: false,
            can_open: bridge_available,
            dirty: false,
            current_path: None,
            recent_documents: Vec::new(),
        });
        Self {
            workbook,
            workspace,
            shared,
            persistence,
            status: RwSignal::new(None),
            document_name: RwSignal::new(None),
            bridge_available,
        }
    }

    /// The projection the shell's `host_persistence` prop reads.
    #[must_use]
    pub fn persistence(&self) -> ReadSignal<PersistenceProjection> {
        self.persistence.read_only()
    }

    /// The last lifecycle status line (`None` until the first step).
    #[must_use]
    pub fn status(&self) -> ReadSignal<Option<DocumentStatus>> {
        self.status.read_only()
    }

    /// The active `.xlsx` document's name (`None` for the demo).
    #[must_use]
    pub fn document_name(&self) -> ReadSignal<Option<String>> {
        self.document_name.read_only()
    }

    /// Whether the shell can pick/write files in this runtime.
    #[must_use]
    pub fn bridge_available(&self) -> bool {
        self.bridge_available
    }

    /// The file name a save dialog is seeded with: the active document's own
    /// name, or [`UNTITLED_XLSX`].
    #[must_use]
    pub fn suggested_save_name(&self) -> String {
        self.document_name
            .get_untracked()
            .unwrap_or_else(|| UNTITLED_XLSX.to_string())
    }

    /// The folder a save dialog opens in: the directory of `current_path`
    /// (either separator), `None` when the document has no path.
    #[must_use]
    pub fn suggested_directory(&self) -> Option<String> {
        self.persistence
            .with_untracked(|projection| projection.current_path.clone())
            .and_then(|path| parent_directory(&path))
    }

    /// Open `.xlsx` bytes the shell resolved as the active document
    /// (`HostCommand::OpenXlsxBytes` through the dispatcher). On `Opened` the
    /// dispatcher has already republished the snapshot and moved the caret;
    /// this folds the result into the projection (`can_save` when the bridge
    /// can write, not dirty, `current_path` = `path`), names the document in
    /// the mast, and records the status. A refusal leaves the previous
    /// document — and the projection — exactly as they were.
    pub fn open_bytes(&self, bytes: Vec<u8>, name: String, path: Option<String>) -> OpenOutcome {
        let outcome = interpret_open_outcome(
            self.workbook
                .execute_host_command(open_xlsx_command(bytes, Some(name.clone()))),
        );
        match &outcome {
            OpenOutcome::Opened {
                sheet_count,
                cells,
                formulas_bound,
                recalc_path,
                ..
            } => {
                self.persistence.update(|projection| {
                    projection.can_save = self.bridge_available;
                    projection.dirty = false;
                    projection.current_path = path.clone();
                });
                self.document_name.set(Some(name.clone()));
                self.name_active_workspace(&name);
                self.status.set(Some(DocumentStatus::new(
                    "opened",
                    format!(
                        "opened {name}: {sheet_count} sheet(s), {cells} literal cell(s), \
                         {formulas_bound} formula(s) bound, recalc {recalc_path:?}{}",
                        path.as_deref()
                            .map(|path| format!(" — {path}"))
                            .unwrap_or_default()
                    ),
                )));
            }
            OpenOutcome::Rejected(rejection) => {
                self.status.set(Some(DocumentStatus::new(
                    "rejected",
                    format!("open {name} refused ({}): {rejection:?}", rejection.label()),
                )));
            }
            OpenOutcome::Unexpected { outcome } => {
                self.status.set(Some(DocumentStatus::new(
                    "unexpected",
                    format!("open {name} answered with an unexpected outcome: {outcome}"),
                )));
            }
        }
        outcome
    }

    /// Ask the host for the active document's package bytes
    /// (`HostCommand::SaveActiveXlsx`). The bytes are the caller's to hand to
    /// the shell's save dialog; nothing in the projection changes until
    /// [`Self::mark_saved`] reports the write. A refusal (the demo's
    /// `NoBackingSource`, an OxDoc policy rejection) is recorded as the
    /// status and leaves the live model untouched.
    pub fn save_to_bytes(&self) -> SaveOutcome {
        let outcome =
            interpret_save_outcome(self.workbook.execute_host_command(save_xlsx_command()));
        match &outcome {
            SaveOutcome::Saved { .. } => {}
            SaveOutcome::Rejected(rejection) => {
                self.status.set(Some(DocumentStatus::new(
                    "rejected",
                    format!("save refused ({}): {rejection:?}", rejection.label()),
                )));
            }
            SaveOutcome::Unexpected { outcome } => {
                self.status.set(Some(DocumentStatus::new(
                    "unexpected",
                    format!("save answered with an unexpected outcome: {outcome}"),
                )));
            }
        }
        outcome
    }

    /// The shell wrote the saved bytes to `path` as `name`: the document is
    /// clean again, lives at `path`, and carries `name` in the mast.
    pub fn mark_saved(&self, path: String, name: String, bytes_written: usize) {
        self.persistence.update(|projection| {
            projection.dirty = false;
            projection.current_path = Some(path.clone());
        });
        self.document_name.set(Some(name.clone()));
        self.name_active_workspace(&name);
        self.status.set(Some(DocumentStatus::new(
            "saved",
            format!("saved {name}: {bytes_written} bytes — {path}"),
        )));
    }

    /// An accepted model mutation (the app's dispatcher wrapper reports it):
    /// the document has unsaved changes until the next save/open.
    pub fn mark_dirty(&self) {
        self.persistence
            .update(|projection| projection.dirty = true);
    }

    /// The user dismissed the shell's dialog — an honest no-op, recorded.
    pub fn note_cancelled(&self, verb: FileVerb) {
        self.status.set(Some(DocumentStatus::new(
            "cancelled",
            format!("{} cancelled: nothing changed", verb.label()),
        )));
    }

    /// The shell's bridge or its command failed — the typed message, never
    /// a silent no-op.
    pub fn note_bridge_error(&self, verb: FileVerb, message: &str) {
        self.status.set(Some(DocumentStatus::new(
            "bridge-error",
            format!("{} failed in the shell: {message}", verb.label()),
        )));
    }

    /// A lifecycle verb arrived where no bridge exists (a browser tab, the
    /// native test compile): say so, change nothing.
    pub fn note_bridge_unavailable(&self, verb: FileVerb) {
        self.status.set(Some(DocumentStatus::new(
            "unavailable",
            format!(
                "{} needs the desktop shell's file bridge; this runtime has none",
                verb.label()
            ),
        )));
    }

    /// Name the active workspace in the shared state so the mast shows the
    /// file name instead of the raw `workspace_id`.
    fn name_active_workspace(&self, name: &str) {
        let workspace_id = self
            .workspace
            .with_untracked(|state| state.workspace_id.clone());
        if workspace_id.is_empty() {
            return;
        }
        let mut names = self.shared.get_untracked().workspace_names;
        names.insert(workspace_id, name.to_string());
        self.shared.apply(
            SharedStateChange::SetWorkspaceNames(names),
            SharedStateOrigin::Host,
        );
    }
}

/// The directory part of a path string, split on either separator (the
/// shell reports Windows paths; the app never touches `std::path` itself so
/// the wasm build stays free of platform path semantics). `None` when the
/// path has no directory part.
#[must_use]
pub fn parent_directory(path: &str) -> Option<String> {
    let cut = path.rfind(['\\', '/'])?;
    let parent = &path[..cut];
    (!parent.is_empty()).then(|| parent.to_string())
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::adapter::{CellOutcome, CommandRejection, interpret_receipt, node_value_display};
    use dnacalc_skin_ir::identity::NodeKey;
    use dnacalc_skin_ir::intent::{Dispatcher, WorkspaceDelta, WorkspaceIntent};
    use dnacalc_skin_ir::selection::SelectionState;
    use dnacalc_skin_ir::state::SharedSkinState;
    use dnacalc_skin_ir::workspace::GridAuthoredKindProjection;
    use dnatreecalc_host::LoadRecalcPath;
    use std::path::{Path, PathBuf};

    /// Repo-relative location of the committed fixture binary, walked from
    /// this crate's manifest dir (`src/dnacalc-app`).
    const FIXTURE_XLSX_REL: &str = "../../fixtures/w011/a1_times_three.xlsx";

    fn fixture_bytes() -> Vec<u8> {
        let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE_XLSX_REL);
        std::fs::read(&path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
    }

    struct Harness {
        workspace: RwSignal<WorkspaceState>,
        shared: SharedSkinStateHandle,
        workbook: Arc<WorkbookHostDispatcher>,
        controller: DocumentController,
    }

    /// The app's own mount: the demo workbook under a dispatcher that owns
    /// the shared state, wrapped in a controller for a runtime with (or
    /// without) a file bridge.
    fn harness(bridge_available: bool) -> Harness {
        let workspace = RwSignal::new(WorkspaceState::default());
        let latest_delta = RwSignal::new(WorkspaceDelta::unchanged(0));
        let selection = RwSignal::new(SelectionState::default());
        let shared = SharedSkinStateHandle::new(SharedSkinState::default());
        let workbook = Arc::new(
            WorkbookHostDispatcher::new_demo(workspace, latest_delta, selection, Some(shared))
                .expect("the demo mounts as CalcApp does"),
        );
        let controller = DocumentController::new(
            workbook.clone(),
            workspace.read_only(),
            shared,
            bridge_available,
        );
        Harness {
            workspace,
            shared,
            workbook,
            controller,
        }
    }

    /// Sheet1's grid id plus the displayed `A1` / `B1` values and `B1`'s
    /// authored formula text, read from the PUBLISHED workspace signal.
    fn sheet1_a1_b1(
        state: &WorkspaceState,
    ) -> (
        dnacalc_skin_ir::identity::NodeId,
        String,
        String,
        Option<String>,
    ) {
        let grid_id = state.sheets[0].grid_node_id.clone();
        let grid = state
            .grids
            .get(&grid_id)
            .expect("Sheet1's grid is published");
        let cell = |row: u32, col: u32| {
            grid.cells
                .iter()
                .find(|cell| cell.row == row && cell.col == col)
                .unwrap_or_else(|| panic!("no published cell at ({row}, {col})"))
        };
        let b1 = cell(1, 2);
        let b1_authored = b1.authored.as_ref().expect("B1 carries authored metadata");
        assert_eq!(b1_authored.kind, GridAuthoredKindProjection::Formula);
        (
            grid_id,
            node_value_display(&cell(1, 1).value),
            node_value_display(&b1.value),
            b1_authored.source_text.clone(),
        )
    }

    /// The whole Wave 1.5 bookkeeping over the committed fixture, exactly as
    /// the desktop click-through drives it: the demo advertises Open only
    /// (no package to save) -> the fixture opens (A1 7, B1 21 with its
    /// formula text; Save enabled; clean; path + mast name set; stale shared
    /// selection cleared) -> an accepted edit marks dirty (B1 30) -> the
    /// save bytes come back and the shell's write marks clean at the new
    /// path -> reopening THOSE bytes shows A1 10 / B1 30 with the formula
    /// text preserved.
    #[test]
    fn controller_tracks_open_edit_save_reopen_over_the_fixture() {
        let _owner = Owner::new();
        let h = harness(true);

        // Demo: the bridge can pick files, but the in-memory demo has no
        // package to save — `can_save` is honest-false, no status yet.
        let initial = h.controller.persistence().get_untracked();
        assert!(initial.can_open, "a runtime with a bridge advertises Open");
        assert!(!initial.can_save, "the demo has no backing package to save");
        assert!(!initial.dirty);
        assert_eq!(initial.current_path, None);
        assert_eq!(h.controller.status().get_untracked(), None);
        assert_eq!(h.controller.suggested_save_name(), UNTITLED_XLSX);
        assert_eq!(h.controller.suggested_directory(), None);

        // Seed a stale multi-select on the demo so the swap has something to
        // clear (the dispatcher's Opened arm owns that).
        h.shared.apply(
            SharedStateChange::SetSelectionSet(vec![NodeKey::from("demo:stale")]),
            SharedStateOrigin::Host,
        );
        h.shared.apply(
            SharedStateChange::SetSelectionAnchor(Some(NodeKey::from("demo:stale"))),
            SharedStateOrigin::Host,
        );

        // Open the fixture as the shell would hand it over.
        let path = "C:\\scratch\\w011\\a1_times_three.xlsx".to_string();
        let outcome = h.controller.open_bytes(
            fixture_bytes(),
            "a1_times_three.xlsx".to_string(),
            Some(path.clone()),
        );
        assert_eq!(
            outcome,
            OpenOutcome::Opened {
                name: Some("a1_times_three.xlsx".to_string()),
                sheet_count: 1,
                cells: 1,
                formulas_bound: 1,
                recalc_path: LoadRecalcPath::Automatic,
            }
        );
        let opened = h.workspace.get_untracked();
        let (grid_id, a1, b1, b1_formula) = sheet1_a1_b1(&opened);
        assert_eq!(
            (a1.as_str(), b1.as_str()),
            ("7", "21"),
            "the fixture renders A1 7, B1 21"
        );
        assert_eq!(
            b1_formula.as_deref(),
            Some("=A1*3"),
            "B1 keeps its formula text"
        );
        let after_open = h.controller.persistence().get_untracked();
        assert!(after_open.can_open);
        assert!(after_open.can_save, "an xlsx-backed document can be saved");
        assert!(!after_open.dirty, "freshly opened is clean");
        assert_eq!(after_open.current_path.as_deref(), Some(path.as_str()));
        assert_eq!(
            h.controller.document_name().get_untracked().as_deref(),
            Some("a1_times_three.xlsx")
        );
        assert_eq!(h.controller.suggested_save_name(), "a1_times_three.xlsx");
        assert_eq!(
            h.controller.suggested_directory().as_deref(),
            Some("C:\\scratch\\w011")
        );
        let status = h
            .controller
            .status()
            .get_untracked()
            .expect("an open status");
        assert_eq!(status.label, "opened");
        assert!(
            status.detail.contains("a1_times_three.xlsx"),
            "{}",
            status.detail
        );
        assert!(
            status.detail.contains("1 formula(s) bound"),
            "{}",
            status.detail
        );
        let shared = h.shared.get_untracked();
        assert_eq!(
            shared
                .workspace_names
                .get(&opened.workspace_id)
                .map(String::as_str),
            Some("a1_times_three.xlsx"),
            "the mast shows the file name for the loaded workspace"
        );
        assert!(
            shared.selection_set.is_empty() && shared.selection_anchor.is_none(),
            "a document swap clears the previous document's shared selection: {:?}",
            shared.selection_set
        );

        // Edit A1 7 -> 10 through the intent path; the app's dispatcher
        // wrapper reports the accepted mutation.
        let receipt = h.workbook.dispatch(WorkspaceIntent::EnterGridCell {
            grid: grid_id,
            row: 1,
            col: 1,
            text: "10".to_string(),
        });
        assert_eq!(
            interpret_receipt(&receipt),
            CellOutcome::Literal {
                value: "10".to_string()
            }
        );
        h.controller.mark_dirty();
        assert!(h.controller.persistence().get_untracked().dirty);
        let (_, a1, b1, _) = sheet1_a1_b1(&h.workspace.get_untracked());
        assert_eq!((a1.as_str(), b1.as_str()), ("10", "30"), "B1 recalcs live");

        // Save: bytes come back; the projection waits for the shell's write.
        let SaveOutcome::Saved { bytes, .. } = h.controller.save_to_bytes() else {
            panic!("the xlsx-backed document saves to bytes");
        };
        assert!(!bytes.is_empty());
        assert!(
            h.controller.persistence().get_untracked().dirty,
            "still dirty until the shell reports the write"
        );
        let saved_path = "C:\\scratch\\w011\\a1_times_three_saved.xlsx".to_string();
        h.controller.mark_saved(
            saved_path.clone(),
            "a1_times_three_saved.xlsx".to_string(),
            bytes.len(),
        );
        let after_save = h.controller.persistence().get_untracked();
        assert!(!after_save.dirty, "the write makes the document clean");
        assert_eq!(
            after_save.current_path.as_deref(),
            Some(saved_path.as_str())
        );
        assert_eq!(
            h.controller.suggested_save_name(),
            "a1_times_three_saved.xlsx"
        );
        let status = h
            .controller
            .status()
            .get_untracked()
            .expect("a save status");
        assert_eq!(status.label, "saved");
        assert!(
            status.detail.contains(&format!("{} bytes", bytes.len())),
            "{}",
            status.detail
        );

        // Reopen the saved bytes: 30 persists, formula text preserved.
        let reopened = h.controller.open_bytes(
            bytes,
            "a1_times_three_saved.xlsx".to_string(),
            Some(saved_path),
        );
        assert!(
            matches!(reopened, OpenOutcome::Opened { .. }),
            "{reopened:?}"
        );
        let (_, a1, b1, b1_formula) = sheet1_a1_b1(&h.workspace.get_untracked());
        assert_eq!((a1.as_str(), b1.as_str()), ("10", "30"));
        assert_eq!(b1_formula.as_deref(), Some("=A1*3"));
        assert!(!h.controller.persistence().get_untracked().dirty);
    }

    /// Refusals stay typed and change nothing: a save on the demo is the
    /// host's `NoBackingSource`, junk bytes are OxDoc's rejection, and the
    /// projection / mast name are untouched either way.
    #[test]
    fn refusals_are_recorded_and_leave_the_projection_alone() {
        let _owner = Owner::new();
        let h = harness(true);
        let before = h.controller.persistence().get_untracked();

        let saved = h.controller.save_to_bytes();
        assert_eq!(
            saved,
            SaveOutcome::Rejected(CommandRejection::NoBackingSource)
        );
        let status = h
            .controller
            .status()
            .get_untracked()
            .expect("a refusal status");
        assert_eq!(status.label, "rejected");
        assert!(
            status.detail.contains("no-backing-source"),
            "{}",
            status.detail
        );

        let opened = h.controller.open_bytes(
            b"not a zip".to_vec(),
            "junk.xlsx".to_string(),
            Some("C:\\scratch\\junk.xlsx".to_string()),
        );
        assert!(
            matches!(opened, OpenOutcome::Rejected(CommandRejection::Xlsx { .. })),
            "{opened:?}"
        );
        let status = h
            .controller
            .status()
            .get_untracked()
            .expect("a refusal status");
        assert_eq!(status.label, "rejected");
        assert!(status.detail.contains("junk.xlsx"), "{}", status.detail);

        assert_eq!(h.controller.persistence().get_untracked(), before);
        assert_eq!(h.controller.document_name().get_untracked(), None);
        assert!(h.shared.get_untracked().workspace_names.is_empty());
        assert_eq!(
            h.workspace.get_untracked().sheets.len(),
            2,
            "the demo is still active"
        );
    }

    /// Without a bridge (a browser tab, this native compile) nothing is
    /// advertised and a forwarded verb is answered with an honest note.
    #[test]
    fn no_bridge_advertises_nothing_and_notes_the_verb() {
        let _owner = Owner::new();
        let h = harness(false);
        let projection = h.controller.persistence().get_untracked();
        assert!(!projection.can_open);
        assert!(!projection.can_save);
        assert!(!h.controller.bridge_available());

        h.controller.note_bridge_unavailable(FileVerb::Open);
        let status = h.controller.status().get_untracked().expect("a note");
        assert_eq!(status.label, "unavailable");
        assert!(status.detail.starts_with("open needs"), "{}", status.detail);

        // Even after an open, a bridge-less runtime cannot save.
        let _ = h
            .controller
            .open_bytes(fixture_bytes(), "a1_times_three.xlsx".to_string(), None);
        assert!(!h.controller.persistence().get_untracked().can_save);

        h.controller.note_cancelled(FileVerb::Save);
        assert_eq!(
            h.controller
                .status()
                .get_untracked()
                .map(|status| status.label),
            Some("cancelled")
        );
        h.controller
            .note_bridge_error(FileVerb::Save, "command missing");
        let status = h.controller.status().get_untracked().expect("a note");
        assert_eq!(status.label, "bridge-error");
        assert!(
            status.detail.contains("command missing"),
            "{}",
            status.detail
        );
    }

    #[test]
    fn file_verbs_and_parent_directories_are_read_honestly() {
        assert_eq!(
            FileVerb::from_shell_verb(SkinVerb::Open),
            Some(FileVerb::Open)
        );
        assert_eq!(
            FileVerb::from_shell_verb(SkinVerb::Save),
            Some(FileVerb::Save)
        );
        assert_eq!(FileVerb::from_shell_verb(SkinVerb::Undo), None);
        assert_eq!(
            parent_directory("C:\\a\\b\\c.xlsx").as_deref(),
            Some("C:\\a\\b")
        );
        assert_eq!(parent_directory("/tmp/c.xlsx").as_deref(), Some("/tmp"));
        assert_eq!(parent_directory("c.xlsx"), None);
        assert_eq!(parent_directory("/c.xlsx"), None);
    }
}

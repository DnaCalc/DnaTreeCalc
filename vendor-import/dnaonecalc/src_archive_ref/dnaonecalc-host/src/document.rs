use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentViewStateRecord {
    pub active_surface: String,
    pub cursor_offset: usize,
    pub selection_anchor: usize,
    pub selection_focus: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentArtifactIndexEntry {
    pub artifact_kind: String,
    pub logical_id: String,
    pub path_hint: String,
    pub content_hash: Option<String>,
    pub embedded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OneCalcDocumentRecord {
    pub document_id: String,
    pub document_title: String,
    pub document_scope: String,
    pub persistence_format_id: String,
    pub worksheet_name: String,
    pub saved_at_unix_ms: u64,
    pub host_profile_id: String,
    pub scenario_slug: String,
    pub formula_stable_id: String,
    pub formula_text: String,
    pub formula_channel_kind: String,
    pub formula_text_version: u64,
    pub structure_context_version: String,
    pub host_session_id: String,
    pub host_recalc_sequence: u64,
    pub host_driving_packet_kind: String,
    pub host_driving_block: String,
    pub recalc_trigger_kind: String,
    pub display_context: String,
    pub effective_display_status: String,
    pub function_surface_policy_id: String,
    pub library_context_snapshot_ref: Option<String>,
    pub governing_capability_snapshot_id: Option<String>,
    pub view_state: DocumentViewStateRecord,
    pub artifact_index: Vec<DocumentArtifactIndexEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedOneCalcDocument {
    pub document: OneCalcDocumentRecord,
    pub document_path: PathBuf,
}

pub fn write_spreadsheetml_document(
    path: impl AsRef<Path>,
    document: &OneCalcDocumentRecord,
) -> Result<PersistedOneCalcDocument, String> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    let mut writer = Writer::new_with_indent(Vec::new(), b' ', 2);
    writer
        .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
        .map_err(|error| error.to_string())?;

    let mut workbook = BytesStart::new("Workbook");
    workbook.push_attribute(("xmlns", "urn:schemas-microsoft-com:office:spreadsheet"));
    workbook.push_attribute(("xmlns:ss", "urn:schemas-microsoft-com:office:spreadsheet"));
    workbook.push_attribute(("xmlns:x", "urn:schemas-microsoft-com:office:excel"));
    workbook.push_attribute(("xmlns:onecalc", "urn:dna-kode:onecalc"));
    writer
        .write_event(Event::Start(workbook))
        .map_err(|error| error.to_string())?;

    let mut worksheet = BytesStart::new("Worksheet");
    worksheet.push_attribute(("ss:Name", document.worksheet_name.as_str()));
    writer
        .write_event(Event::Start(worksheet))
        .map_err(|error| error.to_string())?;
    writer
        .write_event(Event::Start(BytesStart::new("Table")))
        .map_err(|error| error.to_string())?;

    for (key, value) in flatten_document(document) {
        write_row(&mut writer, &key, &value)?;
    }

    writer
        .write_event(Event::End(BytesEnd::new("Table")))
        .map_err(|error| error.to_string())?;
    writer
        .write_event(Event::End(BytesEnd::new("Worksheet")))
        .map_err(|error| error.to_string())?;
    writer
        .write_event(Event::End(BytesEnd::new("Workbook")))
        .map_err(|error| error.to_string())?;

    fs::write(path, writer.into_inner()).map_err(|error| error.to_string())?;

    Ok(PersistedOneCalcDocument {
        document: document.clone(),
        document_path: path.to_path_buf(),
    })
}

pub fn read_spreadsheetml_document(
    path: impl AsRef<Path>,
) -> Result<OneCalcDocumentRecord, String> {
    let body = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let mut reader = Reader::from_str(&body);
    reader.config_mut().trim_text(true);

    let mut current_row: Vec<String> = Vec::new();
    let mut current_cell_text = String::new();
    let mut rows = BTreeMap::new();
    let mut inside_row = false;
    let mut inside_data = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) => match event.local_name().as_ref() {
                b"Row" => {
                    inside_row = true;
                    current_row.clear();
                }
                b"Data" => {
                    if inside_row {
                        inside_data = true;
                        current_cell_text.clear();
                    }
                }
                _ => {}
            },
            Ok(Event::End(event)) => match event.local_name().as_ref() {
                b"Row" => {
                    if current_row.len() >= 2 {
                        rows.insert(current_row[0].clone(), current_row[1].clone());
                    }
                    current_row.clear();
                    inside_row = false;
                    inside_data = false;
                }
                b"Data" => {
                    if inside_row && inside_data {
                        current_row.push(current_cell_text.clone());
                        current_cell_text.clear();
                    }
                    inside_data = false;
                }
                _ => {}
            },
            Ok(Event::Text(text)) => {
                if inside_row && inside_data {
                    current_cell_text.push_str(
                        &text
                            .decode()
                            .map_err(|error| error.to_string())?
                            .into_owned(),
                    );
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(error) => return Err(error.to_string()),
        }
    }

    inflate_document(&rows)
}

fn flatten_document(document: &OneCalcDocumentRecord) -> Vec<(String, String)> {
    let mut rows = vec![
        (
            "onecalc.document_scope".to_string(),
            document.document_scope.clone(),
        ),
        (
            "onecalc.persistence_format_id".to_string(),
            document.persistence_format_id.clone(),
        ),
        ("document.id".to_string(), document.document_id.clone()),
        (
            "document.title".to_string(),
            document.document_title.clone(),
        ),
        (
            "document.saved_at_unix_ms".to_string(),
            document.saved_at_unix_ms.to_string(),
        ),
        (
            "document.worksheet_name".to_string(),
            document.worksheet_name.clone(),
        ),
        (
            "host.profile_id".to_string(),
            document.host_profile_id.clone(),
        ),
        ("scenario.slug".to_string(), document.scenario_slug.clone()),
        (
            "scenario.formula_stable_id".to_string(),
            document.formula_stable_id.clone(),
        ),
        (
            "scenario.formula_text".to_string(),
            document.formula_text.clone(),
        ),
        (
            "scenario.formula_channel_kind".to_string(),
            document.formula_channel_kind.clone(),
        ),
        (
            "scenario.formula_text_version".to_string(),
            document.formula_text_version.to_string(),
        ),
        (
            "scenario.structure_context_version".to_string(),
            document.structure_context_version.clone(),
        ),
        (
            "scenario.host_session_id".to_string(),
            document.host_session_id.clone(),
        ),
        (
            "scenario.host_recalc_sequence".to_string(),
            document.host_recalc_sequence.to_string(),
        ),
        (
            "scenario.host_driving_packet_kind".to_string(),
            document.host_driving_packet_kind.clone(),
        ),
        (
            "scenario.host_driving_block".to_string(),
            document.host_driving_block.clone(),
        ),
        (
            "scenario.recalc_trigger_kind".to_string(),
            document.recalc_trigger_kind.clone(),
        ),
        (
            "scenario.display_context".to_string(),
            document.display_context.clone(),
        ),
        (
            "scenario.effective_display_status".to_string(),
            document.effective_display_status.clone(),
        ),
        (
            "scenario.function_surface_policy_id".to_string(),
            document.function_surface_policy_id.clone(),
        ),
        (
            "scenario.library_context_snapshot_ref".to_string(),
            document
                .library_context_snapshot_ref
                .clone()
                .unwrap_or_default(),
        ),
        (
            "scenario.governing_capability_snapshot_id".to_string(),
            document
                .governing_capability_snapshot_id
                .clone()
                .unwrap_or_default(),
        ),
        (
            "view.active_surface".to_string(),
            document.view_state.active_surface.clone(),
        ),
        (
            "view.cursor_offset".to_string(),
            document.view_state.cursor_offset.to_string(),
        ),
        (
            "view.selection_anchor".to_string(),
            document.view_state.selection_anchor.to_string(),
        ),
        (
            "view.selection_focus".to_string(),
            document.view_state.selection_focus.to_string(),
        ),
        (
            "artifact_index.count".to_string(),
            document.artifact_index.len().to_string(),
        ),
    ];

    for (index, artifact) in document.artifact_index.iter().enumerate() {
        let prefix = format!("artifact_index.{index}");
        rows.push((
            format!("{prefix}.artifact_kind"),
            artifact.artifact_kind.clone(),
        ));
        rows.push((format!("{prefix}.logical_id"), artifact.logical_id.clone()));
        rows.push((format!("{prefix}.path_hint"), artifact.path_hint.clone()));
        rows.push((
            format!("{prefix}.content_hash"),
            artifact.content_hash.clone().unwrap_or_default(),
        ));
        rows.push((format!("{prefix}.embedded"), artifact.embedded.to_string()));
    }

    rows
}

fn inflate_document(rows: &BTreeMap<String, String>) -> Result<OneCalcDocumentRecord, String> {
    let artifact_count = parse_usize(rows, "artifact_index.count")?;
    let mut artifact_index = Vec::with_capacity(artifact_count);
    for index in 0..artifact_count {
        let prefix = format!("artifact_index.{index}");
        artifact_index.push(DocumentArtifactIndexEntry {
            artifact_kind: required(rows, &format!("{prefix}.artifact_kind"))?,
            logical_id: required(rows, &format!("{prefix}.logical_id"))?,
            path_hint: required(rows, &format!("{prefix}.path_hint"))?,
            content_hash: optional(rows, &format!("{prefix}.content_hash")),
            embedded: parse_bool(rows, &format!("{prefix}.embedded"))?,
        });
    }

    Ok(OneCalcDocumentRecord {
        document_id: required(rows, "document.id")?,
        document_title: required(rows, "document.title")?,
        document_scope: required(rows, "onecalc.document_scope")?,
        persistence_format_id: required(rows, "onecalc.persistence_format_id")?,
        worksheet_name: required(rows, "document.worksheet_name")?,
        saved_at_unix_ms: parse_u64(rows, "document.saved_at_unix_ms")?,
        host_profile_id: required(rows, "host.profile_id")?,
        scenario_slug: required(rows, "scenario.slug")?,
        formula_stable_id: required(rows, "scenario.formula_stable_id")?,
        formula_text: required(rows, "scenario.formula_text")?,
        formula_channel_kind: required(rows, "scenario.formula_channel_kind")?,
        formula_text_version: parse_u64(rows, "scenario.formula_text_version")?,
        structure_context_version: required(rows, "scenario.structure_context_version")?,
        host_session_id: required(rows, "scenario.host_session_id")?,
        host_recalc_sequence: parse_u64(rows, "scenario.host_recalc_sequence")?,
        host_driving_packet_kind: required(rows, "scenario.host_driving_packet_kind")?,
        host_driving_block: required(rows, "scenario.host_driving_block")?,
        recalc_trigger_kind: required(rows, "scenario.recalc_trigger_kind")?,
        display_context: required(rows, "scenario.display_context")?,
        effective_display_status: required(rows, "scenario.effective_display_status")?,
        function_surface_policy_id: required(rows, "scenario.function_surface_policy_id")?,
        library_context_snapshot_ref: optional(rows, "scenario.library_context_snapshot_ref"),
        governing_capability_snapshot_id: optional(
            rows,
            "scenario.governing_capability_snapshot_id",
        ),
        view_state: DocumentViewStateRecord {
            active_surface: required(rows, "view.active_surface")?,
            cursor_offset: parse_usize(rows, "view.cursor_offset")?,
            selection_anchor: parse_usize(rows, "view.selection_anchor")?,
            selection_focus: parse_usize(rows, "view.selection_focus")?,
        },
        artifact_index,
    })
}

fn write_row(writer: &mut Writer<Vec<u8>>, key: &str, value: &str) -> Result<(), String> {
    writer
        .write_event(Event::Start(BytesStart::new("Row")))
        .map_err(|error| error.to_string())?;
    write_cell(writer, key)?;
    write_cell(writer, value)?;
    writer
        .write_event(Event::End(BytesEnd::new("Row")))
        .map_err(|error| error.to_string())
}

fn write_cell(writer: &mut Writer<Vec<u8>>, value: &str) -> Result<(), String> {
    writer
        .write_event(Event::Start(BytesStart::new("Cell")))
        .map_err(|error| error.to_string())?;
    let mut data = BytesStart::new("Data");
    data.push_attribute(("ss:Type", "String"));
    writer
        .write_event(Event::Start(data))
        .map_err(|error| error.to_string())?;
    writer
        .write_event(Event::Text(BytesText::new(value)))
        .map_err(|error| error.to_string())?;
    writer
        .write_event(Event::End(BytesEnd::new("Data")))
        .map_err(|error| error.to_string())?;
    writer
        .write_event(Event::End(BytesEnd::new("Cell")))
        .map_err(|error| error.to_string())
}

fn required(rows: &BTreeMap<String, String>, key: &str) -> Result<String, String> {
    rows.get(key)
        .cloned()
        .ok_or_else(|| format!("missing document field: {key}"))
}

fn optional(rows: &BTreeMap<String, String>, key: &str) -> Option<String> {
    rows.get(key).cloned().filter(|value| !value.is_empty())
}

fn parse_u64(rows: &BTreeMap<String, String>, key: &str) -> Result<u64, String> {
    required(rows, key)?
        .parse::<u64>()
        .map_err(|error| format!("invalid u64 for {key}: {error}"))
}

fn parse_usize(rows: &BTreeMap<String, String>, key: &str) -> Result<usize, String> {
    required(rows, key)?
        .parse::<usize>()
        .map_err(|error| format!("invalid usize for {key}: {error}"))
}

fn parse_bool(rows: &BTreeMap<String, String>, key: &str) -> Result<bool, String> {
    required(rows, key)?
        .parse::<bool>()
        .map_err(|error| format!("invalid bool for {key}: {error}"))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn spreadsheetml_round_trip_preserves_empty_optional_fields() {
        let root = std::env::temp_dir().join(format!(
            "dnaonecalc-document-empty-optionals-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let path = root.join("empty-optionals.xml");
        let document = OneCalcDocumentRecord {
            document_id: "doc-001".to_string(),
            document_title: "Document".to_string(),
            document_scope: "isolated_single_formula_instance".to_string(),
            persistence_format_id: "spreadsheetml2003.onecalc.single_instance.v1".to_string(),
            worksheet_name: "Formula".to_string(),
            saved_at_unix_ms: 46_000,
            host_profile_id: "OC-H1".to_string(),
            scenario_slug: "sum".to_string(),
            formula_stable_id: "formula-001".to_string(),
            formula_text: "=SUM(1,2,3)".to_string(),
            formula_channel_kind: "worksheet_a1".to_string(),
            formula_text_version: 1,
            structure_context_version: "onecalc:single_formula:h1".to_string(),
            host_session_id: "session-001".to_string(),
            host_recalc_sequence: 1,
            host_driving_packet_kind: "edit_accept_recalc".to_string(),
            host_driving_block: "single_formula".to_string(),
            recalc_trigger_kind: "edit_accept".to_string(),
            display_context: "single_formula_result".to_string(),
            effective_display_status: "none".to_string(),
            function_surface_policy_id: "policy-001".to_string(),
            library_context_snapshot_ref: None,
            governing_capability_snapshot_id: Some("cap-001".to_string()),
            view_state: DocumentViewStateRecord {
                active_surface: "formula".to_string(),
                cursor_offset: 4,
                selection_anchor: 4,
                selection_focus: 4,
            },
            artifact_index: vec![DocumentArtifactIndexEntry {
                artifact_kind: "scenario_run".to_string(),
                logical_id: "run-001".to_string(),
                path_hint: "scenario-runs/run-001.json".to_string(),
                content_hash: None,
                embedded: false,
            }],
        };

        let persisted =
            write_spreadsheetml_document(&path, &document).expect("document should persist");
        let reopened =
            read_spreadsheetml_document(&persisted.document_path).expect("document should reopen");

        assert_eq!(reopened, document);

        let _ = fs::remove_dir_all(&root);
    }
}

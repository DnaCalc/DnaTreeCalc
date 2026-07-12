use dnaonecalc_host::test_support::{make_editor_syntax_snapshot, make_editor_token};
use dnaonecalc_host::ui::editor::render_projection::syntax_runs_from_snapshot;

#[test]
fn ex_01_projection_uses_editor_syntax_snapshot_as_render_source() {
    let snapshot = make_editor_syntax_snapshot(
        "formula-1",
        "green-42",
        vec![
            make_editor_token("=LET(", 0),
            make_editor_token("values", 5),
        ],
    );

    let runs = syntax_runs_from_snapshot(&snapshot);
    assert_eq!(runs.len(), 2);
    assert_eq!(runs[0].text, "=LET(");
    assert_eq!(runs[1].span_start, 5);
}

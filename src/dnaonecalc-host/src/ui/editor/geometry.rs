//! Editor overlay geometry — pure-Rust caret / span / popup-anchor
//! computation.
//!
//! ## Why a separate module
//!
//! The previous WS-13 attempt at popup positioning (since retired) mixed
//! browser measurement and pixel computation in one place. That tangled
//! the layers and made edge cases (line wraps, multi-line, scroll) hard
//! to test without a real browser. This module owns ONLY the pure
//! computation: given a text, a caret offset, and a `TextareaMeasurementMetrics`
//! snapshot (char_width, line_height, scroll position), produce a pixel
//! anchor. Browser measurement lives in a sibling adapter
//! (`ui::editor::caret_box_measurement`, introduced in bead dno-xcq.22)
//! that reads DOM dimensions and feeds them in.
//!
//! ## Popup-anchor entry point
//!
//! The completion popup, signature help, hover tooltip, and any future
//! caret-anchored surface should consume [`caret_box_for_offset`] for a
//! focused single-caret anchor, or [`derive_overlay_snapshot_with_metrics`]
//! when the surface needs multiple anchors at once (caret + selection +
//! popup-target span).
//!
//! ## Caveats
//!
//! * The functions count Rust [`char`]s. JavaScript `textarea.selectionStart`
//!   is in UTF-16 code units; non-BMP characters (e.g. emoji) occupy two
//!   code units in JS but one [`char`] in Rust. The caller is responsible
//!   for converting offsets at the boundary. For all-BMP formulas (the
//!   overwhelming common case) the two are identical.
//! * `\r\n` line endings: only `\n` triggers a row break. A bare `\r`
//!   advances column like any other character. Browsers normalise
//!   textarea contents to `\n` so this is rarely observed in practice.
//! * The pixel coordinates returned are relative to the textarea's
//!   content-box origin (i.e. inside any padding). The browser adapter
//!   composes padding separately when positioning a popup absolutely
//!   inside the editor frame.

use crate::adapters::oxfml::FormulaTextSpan;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorOverlayMeasurementSource {
    DerivedGrid,
    DomMeasured,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorLineColumn {
    pub line_index: usize,
    pub column_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorOverlayBox {
    pub start: EditorLineColumn,
    pub end: EditorLineColumn,
    pub top_px: usize,
    pub left_px: usize,
    pub width_px: usize,
    pub height_px: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorOverlayMeasurement {
    pub source: EditorOverlayMeasurementSource,
    pub char_width_px: usize,
    pub line_height_px: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorMeasuredOverlayBox {
    pub top_px: usize,
    pub left_px: usize,
    pub width_px: usize,
    pub height_px: usize,
    pub line_index: usize,
    pub column_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EditorOverlayGeometrySnapshot {
    pub caret_box: Option<EditorMeasuredOverlayBox>,
    pub selection_box: Option<EditorMeasuredOverlayBox>,
    pub completion_anchor_box: Option<EditorMeasuredOverlayBox>,
    pub signature_help_anchor_box: Option<EditorMeasuredOverlayBox>,
    pub completion_popup_box: Option<EditorMeasuredOverlayBox>,
    pub signature_help_popup_box: Option<EditorMeasuredOverlayBox>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorOverlayMeasurementEvent {
    pub snapshot: EditorOverlayGeometrySnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextareaMeasurementMetrics {
    pub char_width_px: usize,
    pub line_height_px: usize,
    pub scroll_top_px: usize,
    pub scroll_left_px: usize,
}

impl EditorOverlayMeasurement {
    pub fn derived_grid() -> Self {
        Self {
            source: EditorOverlayMeasurementSource::DerivedGrid,
            char_width_px: 8,
            line_height_px: 22,
        }
    }

    pub fn offset_box(&self, text: &str, offset: usize) -> EditorOverlayBox {
        let start = offset_to_line_column(text, offset);
        EditorOverlayBox {
            start,
            end: start,
            top_px: start.line_index * self.line_height_px,
            left_px: start.column_index * self.char_width_px,
            width_px: self.char_width_px.max(1),
            height_px: self.line_height_px,
        }
    }

    pub fn span_box(&self, text: &str, span: FormulaTextSpan) -> EditorOverlayBox {
        let start = offset_to_line_column(text, span.start);
        let end = offset_to_line_column(text, span.start + span.len);
        let same_line = start.line_index == end.line_index;
        let column_span = if same_line {
            end.column_index.saturating_sub(start.column_index).max(1)
        } else {
            1
        };
        let line_span = end.line_index.saturating_sub(start.line_index) + 1;

        EditorOverlayBox {
            start,
            end,
            top_px: start.line_index * self.line_height_px,
            left_px: start.column_index * self.char_width_px,
            width_px: column_span * self.char_width_px,
            height_px: line_span * self.line_height_px,
        }
    }
}

pub fn offset_to_line_column(text: &str, offset: usize) -> EditorLineColumn {
    let mut line_index = 0;
    let mut column_index = 0;

    for (current_offset, ch) in text.chars().enumerate() {
        if current_offset == offset {
            return EditorLineColumn {
                line_index,
                column_index,
            };
        }

        if ch == '\n' {
            line_index += 1;
            column_index = 0;
        } else {
            column_index += 1;
        }
    }

    EditorLineColumn {
        line_index,
        column_index,
    }
}

pub fn resolve_overlay_box(
    measured_box: Option<EditorMeasuredOverlayBox>,
    derived_box: EditorOverlayBox,
) -> (EditorOverlayMeasurementSource, EditorOverlayBox) {
    match measured_box {
        Some(measured_box) => (
            EditorOverlayMeasurementSource::DomMeasured,
            EditorOverlayBox {
                start: EditorLineColumn {
                    line_index: measured_box.line_index,
                    column_index: measured_box.column_index,
                },
                end: EditorLineColumn {
                    line_index: measured_box.line_index,
                    column_index: measured_box.column_index,
                },
                top_px: measured_box.top_px,
                left_px: measured_box.left_px,
                width_px: measured_box.width_px,
                height_px: measured_box.height_px,
            },
        ),
        None => (EditorOverlayMeasurementSource::DerivedGrid, derived_box),
    }
}

/// Single-caret pixel anchor for a popup or signature-help target.
///
/// Returns the box covering one character cell at `caret_offset`, with
/// pixel positions relative to the textarea's content-box origin and
/// adjusted for the textarea's current scroll position. This is the
/// focused entry point the popup view-model integration uses; for
/// composite surfaces that need caret + selection + popup-target spans
/// at the same time, prefer [`derive_overlay_snapshot_with_metrics`].
///
/// `caret_offset` is a Rust [`char`] index (see module docs for the JS
/// UTF-16 caveat). Past-end offsets clamp to the end of the text.
pub fn caret_box_for_offset(
    text: &str,
    caret_offset: usize,
    metrics: TextareaMeasurementMetrics,
) -> EditorMeasuredOverlayBox {
    let measurement = EditorOverlayMeasurement {
        source: EditorOverlayMeasurementSource::DomMeasured,
        char_width_px: metrics.char_width_px.max(1),
        line_height_px: metrics.line_height_px.max(1),
    };
    let raw_box = measurement.offset_box(text, caret_offset);
    measured_box_from_overlay_box(adjust_for_scroll(raw_box, metrics))
}

pub fn derive_overlay_snapshot(
    text: &str,
    caret_offset: usize,
    selection_span: FormulaTextSpan,
    completion_anchor_span: Option<FormulaTextSpan>,
    signature_help_span: Option<FormulaTextSpan>,
) -> EditorOverlayGeometrySnapshot {
    let measurement = EditorOverlayMeasurement::derived_grid();

    EditorOverlayGeometrySnapshot {
        caret_box: Some(measured_box_from_overlay_box(
            measurement.offset_box(text, caret_offset),
        )),
        selection_box: Some(measured_box_from_overlay_box(
            measurement.span_box(text, selection_span),
        )),
        completion_anchor_box: completion_anchor_span
            .map(|span| measured_box_from_overlay_box(measurement.span_box(text, span))),
        signature_help_anchor_box: signature_help_span
            .map(|span| measured_box_from_overlay_box(measurement.span_box(text, span))),
        completion_popup_box: completion_anchor_span
            .map(|span| measured_box_from_overlay_box(measurement.span_box(text, span))),
        signature_help_popup_box: signature_help_span
            .map(|span| measured_box_from_overlay_box(measurement.span_box(text, span))),
    }
}

pub fn derive_overlay_snapshot_with_metrics(
    text: &str,
    caret_offset: usize,
    selection_span: FormulaTextSpan,
    completion_anchor_span: Option<FormulaTextSpan>,
    signature_help_span: Option<FormulaTextSpan>,
    metrics: TextareaMeasurementMetrics,
) -> EditorOverlayGeometrySnapshot {
    let measurement = EditorOverlayMeasurement {
        source: EditorOverlayMeasurementSource::DomMeasured,
        char_width_px: metrics.char_width_px.max(1),
        line_height_px: metrics.line_height_px.max(1),
    };

    EditorOverlayGeometrySnapshot {
        caret_box: Some(measured_box_from_overlay_box(adjust_for_scroll(
            measurement.offset_box(text, caret_offset),
            metrics,
        ))),
        selection_box: Some(measured_box_from_overlay_box(adjust_for_scroll(
            measurement.span_box(text, selection_span),
            metrics,
        ))),
        completion_anchor_box: completion_anchor_span.map(|span| {
            measured_box_from_overlay_box(adjust_for_scroll(
                measurement.span_box(text, span),
                metrics,
            ))
        }),
        signature_help_anchor_box: signature_help_span.map(|span| {
            measured_box_from_overlay_box(adjust_for_scroll(
                measurement.span_box(text, span),
                metrics,
            ))
        }),
        completion_popup_box: completion_anchor_span.map(|span| {
            measured_box_from_overlay_box(adjust_for_scroll(
                measurement.span_box(text, span),
                metrics,
            ))
        }),
        signature_help_popup_box: signature_help_span.map(|span| {
            measured_box_from_overlay_box(adjust_for_scroll(
                measurement.span_box(text, span),
                metrics,
            ))
        }),
    }
}

fn adjust_for_scroll(
    mut box_geometry: EditorOverlayBox,
    metrics: TextareaMeasurementMetrics,
) -> EditorOverlayBox {
    box_geometry.top_px = box_geometry.top_px.saturating_sub(metrics.scroll_top_px);
    box_geometry.left_px = box_geometry.left_px.saturating_sub(metrics.scroll_left_px);
    box_geometry
}

fn measured_box_from_overlay_box(box_geometry: EditorOverlayBox) -> EditorMeasuredOverlayBox {
    EditorMeasuredOverlayBox {
        top_px: box_geometry.top_px,
        left_px: box_geometry.left_px,
        width_px: box_geometry.width_px,
        height_px: box_geometry.height_px,
        line_index: box_geometry.start.line_index,
        column_index: box_geometry.start.column_index,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --------------------------------------------------------------
    // Edge-case coverage for popup-anchor positioning.
    //
    // Each test below pins a specific situation the caret-anchor
    // computation must handle without panicking and without drifting
    // off-screen. The cases mirror the failure modes WS-13 hit, plus a
    // few new ones (non-ASCII, very large offset).
    // --------------------------------------------------------------

    /// Empty text + caret at offset 0: the only valid position is
    /// (line 0, col 0); no panic, no past-end fallthrough.
    #[test]
    fn offset_to_line_column_handles_empty_text_at_offset_zero() {
        assert_eq!(
            offset_to_line_column("", 0),
            EditorLineColumn {
                line_index: 0,
                column_index: 0
            }
        );
    }

    /// Past-end offset on empty text clamps to (0, 0) — never panics.
    #[test]
    fn offset_to_line_column_handles_past_end_on_empty_text() {
        assert_eq!(
            offset_to_line_column("", 10),
            EditorLineColumn {
                line_index: 0,
                column_index: 0
            }
        );
    }

    /// Offset exactly on a `\n` boundary: caret sits at end of the
    /// previous line, NOT at column 0 of the next line. Important
    /// for multi-line formula popups so the popup anchors at the
    /// trailing edge of the current line.
    #[test]
    fn offset_to_line_column_anchors_at_end_of_line_when_offset_is_on_newline() {
        // text: "abc\ndef", `\n` is at offset 3
        assert_eq!(
            offset_to_line_column("abc\ndef", 3),
            EditorLineColumn {
                line_index: 0,
                column_index: 3
            },
            "offset on the newline char itself = end of line 0",
        );
        // offset 4 is the first char of line 1
        assert_eq!(
            offset_to_line_column("abc\ndef", 4),
            EditorLineColumn {
                line_index: 1,
                column_index: 0
            },
        );
    }

    /// `\r` in the middle of a line is treated as an ordinary column
    /// (not a line break). Browsers normalise `\r\n` to `\n` in
    /// textarea contents so this is rarely observed; the test pins the
    /// documented behaviour for unusual inputs (e.g. paste from
    /// non-normalising sources).
    #[test]
    fn offset_to_line_column_treats_bare_cr_as_a_column_advance() {
        // "ab\rcd" — the `\r` is at offset 2, treated as a column.
        assert_eq!(
            offset_to_line_column("ab\rcd", 2),
            EditorLineColumn {
                line_index: 0,
                column_index: 2
            }
        );
        assert_eq!(
            offset_to_line_column("ab\rcd", 4),
            EditorLineColumn {
                line_index: 0,
                column_index: 4
            }
        );
    }

    /// `\r\n` line endings: the `\r` advances column, the `\n`
    /// advances line. Document the layered behavior: caller must
    /// normalise if a `\r\n` source needs to be treated as one line
    /// break.
    #[test]
    fn offset_to_line_column_treats_crlf_as_two_glyphs_with_lf_breaking() {
        // "ab\r\ncd" — `\r` at offset 2, `\n` at offset 3, `c` at offset 4.
        assert_eq!(
            offset_to_line_column("ab\r\ncd", 3),
            EditorLineColumn {
                line_index: 0,
                column_index: 3
            },
            "offset on `\\n` = end of line 0 (column index 3 includes the `\\r`)",
        );
        assert_eq!(
            offset_to_line_column("ab\r\ncd", 4),
            EditorLineColumn {
                line_index: 1,
                column_index: 0
            },
            "offset 4 = first char of line 1",
        );
    }

    /// Past-end offset on non-empty text clamps to the post-loop
    /// (line, column) pair — i.e. the position one past the last
    /// character. Caller can detect "caret past end" by seeing the
    /// returned column == line length.
    #[test]
    fn offset_to_line_column_clamps_past_end_offset_to_last_position() {
        assert_eq!(
            offset_to_line_column("abc", 3),
            EditorLineColumn {
                line_index: 0,
                column_index: 3
            }
        );
        assert_eq!(
            offset_to_line_column("abc", 100),
            EditorLineColumn {
                line_index: 0,
                column_index: 3
            }
        );
        // Multi-line: end of line 1 after `\n` is line 1, col 3
        assert_eq!(
            offset_to_line_column("abc\ndef", 7),
            EditorLineColumn {
                line_index: 1,
                column_index: 3
            }
        );
        assert_eq!(
            offset_to_line_column("abc\ndef", usize::MAX / 2),
            EditorLineColumn {
                line_index: 1,
                column_index: 3
            }
        );
    }

    /// Non-ASCII BMP characters (e.g. `é`, `中`) count as one [`char`]
    /// each. Caller is responsible for any UTF-16-vs-Rust-char
    /// conversion at the JS boundary; in Rust the offsets line up
    /// 1:1 with `chars().enumerate()`.
    #[test]
    fn offset_to_line_column_counts_non_ascii_bmp_as_one_char() {
        // "café" is 4 chars (c, a, f, é) — `é` is U+00E9, one BMP
        // code point, one Rust char.
        assert_eq!(
            offset_to_line_column("café", 4),
            EditorLineColumn {
                line_index: 0,
                column_index: 4
            }
        );
        // Mixed scripts: "中a" — 中 is one BMP char.
        assert_eq!(
            offset_to_line_column("中a", 1),
            EditorLineColumn {
                line_index: 0,
                column_index: 1
            }
        );
        assert_eq!(
            offset_to_line_column("中a", 2),
            EditorLineColumn {
                line_index: 0,
                column_index: 2
            }
        );
    }

    // --------------------------------------------------------------
    // Caret-box pixel positioning (the popup-anchor entry point).
    // --------------------------------------------------------------

    /// `caret_box_for_offset` produces the same coordinates as
    /// `derive_overlay_snapshot_with_metrics` puts in `caret_box`,
    /// minus the additional surfaces. This invariant lets the
    /// popup view-model adopt the focused helper without behavior
    /// drift.
    #[test]
    fn caret_box_for_offset_matches_full_snapshot_caret() {
        let metrics = TextareaMeasurementMetrics {
            char_width_px: 9,
            line_height_px: 22,
            scroll_top_px: 0,
            scroll_left_px: 0,
        };
        let text = "=SUM(1,2,3)";
        let focused = caret_box_for_offset(text, 5, metrics);
        let snapshot = derive_overlay_snapshot_with_metrics(
            text,
            5,
            FormulaTextSpan { start: 5, len: 0 },
            None,
            None,
            metrics,
        );
        assert_eq!(Some(focused), snapshot.caret_box);
    }

    /// Caret on a `\n` boundary produces a pixel position at the
    /// trailing edge of the previous line — vital for popups that
    /// anchor at the caret on multi-line formulas.
    #[test]
    fn caret_box_for_offset_anchors_at_end_of_line_when_caret_is_on_newline() {
        let metrics = TextareaMeasurementMetrics {
            char_width_px: 9,
            line_height_px: 22,
            scroll_top_px: 0,
            scroll_left_px: 0,
        };
        // "=SUM(\n  1)" — offset 5 is the `\n` at end of line 0
        let caret = caret_box_for_offset("=SUM(\n  1)", 5, metrics);
        assert_eq!(caret.line_index, 0);
        assert_eq!(caret.column_index, 5);
        assert_eq!(caret.top_px, 0);
        assert_eq!(caret.left_px, 5 * 9);
    }

    /// Scroll-adjusted caret: a caret on line 4 with the textarea
    /// scrolled to line 2 reports top_px = 2 * line_height (caret is
    /// two lines below the top of the visible viewport).
    #[test]
    fn caret_box_for_offset_subtracts_scroll_offset() {
        let metrics = TextareaMeasurementMetrics {
            char_width_px: 9,
            line_height_px: 22,
            scroll_top_px: 44,  // 2 lines scrolled
            scroll_left_px: 18, // 2 cols scrolled
        };
        // 5 lines of "abc": offset on line 4 col 0 = char index 16
        let text = "abc\nabc\nabc\nabc\nabc";
        let caret = caret_box_for_offset(text, 16, metrics);
        // raw position would be (4 * 22, 0) = (88, 0)
        // adjusted: (88 - 44, 0 - 18) clamped at 0 = (44, 0)
        assert_eq!(caret.top_px, 44, "should be 2 lines below viewport top");
        assert_eq!(caret.left_px, 0, "scroll_left underflow clamps to 0");
    }

    /// Past-end offset still produces a valid pixel position — the
    /// popup must not throw or render off-screen on an out-of-range
    /// offset (defensive against caller errors).
    #[test]
    fn caret_box_for_offset_clamps_past_end_offsets() {
        let metrics = TextareaMeasurementMetrics {
            char_width_px: 9,
            line_height_px: 22,
            scroll_top_px: 0,
            scroll_left_px: 0,
        };
        let caret = caret_box_for_offset("abc", 100, metrics);
        assert_eq!(caret.line_index, 0);
        assert_eq!(caret.column_index, 3);
        assert_eq!(caret.left_px, 27);
    }

    /// Zero-valued metrics get clamped to a 1px floor — protects
    /// against early-mount races where the browser hasn't laid out
    /// the textarea yet and reports 0 for char_width / line_height.
    #[test]
    fn caret_box_for_offset_clamps_zero_metrics_to_one_pixel_floor() {
        let metrics = TextareaMeasurementMetrics {
            char_width_px: 0,
            line_height_px: 0,
            scroll_top_px: 0,
            scroll_left_px: 0,
        };
        let caret = caret_box_for_offset("abc", 2, metrics);
        // char_width/line_height clamped to 1
        assert_eq!(caret.left_px, 2);
        assert_eq!(caret.height_px, 1);
        assert_eq!(caret.width_px, 1);
    }

    #[test]
    fn offset_to_line_column_tracks_multiline_positions() {
        let text = "=LET(\n  x, 1,\n  x)";
        assert_eq!(
            offset_to_line_column(text, 0),
            EditorLineColumn {
                line_index: 0,
                column_index: 0,
            }
        );
        assert_eq!(
            offset_to_line_column(text, 6),
            EditorLineColumn {
                line_index: 1,
                column_index: 0,
            }
        );
        assert_eq!(
            offset_to_line_column(text, text.chars().count()),
            EditorLineColumn {
                line_index: 2,
                column_index: 4,
            }
        );
    }

    #[test]
    fn derived_grid_span_box_produces_line_and_pixel_geometry() {
        let measurement = EditorOverlayMeasurement::derived_grid();
        let text = "=SUM(\n  1,\n  2)";
        let span_box = measurement.span_box(text, FormulaTextSpan { start: 6, len: 2 });

        assert_eq!(span_box.start.line_index, 1);
        assert_eq!(span_box.start.column_index, 0);
        assert_eq!(span_box.top_px, 22);
        assert_eq!(span_box.left_px, 0);
        assert_eq!(span_box.height_px, 22);
        assert_eq!(span_box.width_px, 16);
    }

    #[test]
    fn measured_box_is_preferred_over_derived_geometry() {
        let derived_box = EditorOverlayMeasurement::derived_grid().offset_box("=SUM(1,2)", 4);
        let (source, resolved_box) = resolve_overlay_box(
            Some(EditorMeasuredOverlayBox {
                top_px: 120,
                left_px: 48,
                width_px: 14,
                height_px: 20,
                line_index: 3,
                column_index: 6,
            }),
            derived_box,
        );

        assert_eq!(source, EditorOverlayMeasurementSource::DomMeasured);
        assert_eq!(resolved_box.top_px, 120);
        assert_eq!(resolved_box.left_px, 48);
        assert_eq!(resolved_box.start.line_index, 3);
        assert_eq!(resolved_box.start.column_index, 6);
    }

    #[test]
    fn derived_overlay_snapshot_captures_caret_selection_and_assist_boxes() {
        let snapshot = derive_overlay_snapshot(
            "=SUM(1,2)",
            4,
            FormulaTextSpan { start: 1, len: 3 },
            Some(FormulaTextSpan { start: 1, len: 3 }),
            Some(FormulaTextSpan { start: 0, len: 9 }),
        );

        assert_eq!(
            snapshot
                .caret_box
                .as_ref()
                .map(|box_geometry| box_geometry.left_px),
            Some(32)
        );
        assert_eq!(
            snapshot
                .selection_box
                .as_ref()
                .map(|box_geometry| box_geometry.width_px),
            Some(24)
        );
        assert_eq!(
            snapshot
                .completion_anchor_box
                .as_ref()
                .map(|box_geometry| box_geometry.column_index),
            Some(1)
        );
        assert_eq!(
            snapshot
                .signature_help_anchor_box
                .as_ref()
                .map(|box_geometry| box_geometry.width_px),
            Some(72)
        );
        assert_eq!(
            snapshot
                .completion_popup_box
                .as_ref()
                .map(|box_geometry| box_geometry.column_index),
            Some(1)
        );
        assert_eq!(
            snapshot
                .signature_help_popup_box
                .as_ref()
                .map(|box_geometry| box_geometry.width_px),
            Some(72)
        );
    }

    #[test]
    fn dom_metric_overlay_snapshot_accounts_for_scroll_and_multiline_offsets() {
        let snapshot = derive_overlay_snapshot_with_metrics(
            "=LET(\n  alpha,\n  beta,\n  alpha)",
            15,
            FormulaTextSpan { start: 6, len: 7 },
            Some(FormulaTextSpan { start: 6, len: 5 }),
            Some(FormulaTextSpan { start: 0, len: 31 }),
            TextareaMeasurementMetrics {
                char_width_px: 9,
                line_height_px: 20,
                scroll_top_px: 20,
                scroll_left_px: 9,
            },
        );

        assert_eq!(
            snapshot
                .caret_box
                .as_ref()
                .map(|box_geometry| box_geometry.top_px),
            Some(20)
        );
        assert_eq!(
            snapshot
                .caret_box
                .as_ref()
                .map(|box_geometry| box_geometry.left_px),
            Some(0)
        );
        assert_eq!(
            snapshot
                .completion_anchor_box
                .as_ref()
                .map(|box_geometry| box_geometry.top_px),
            Some(0)
        );
        assert_eq!(
            snapshot
                .signature_help_anchor_box
                .as_ref()
                .map(|box_geometry| box_geometry.height_px),
            Some(80)
        );
        assert_eq!(
            snapshot
                .completion_popup_box
                .as_ref()
                .map(|box_geometry| box_geometry.top_px),
            Some(0)
        );
        assert_eq!(
            snapshot
                .signature_help_popup_box
                .as_ref()
                .map(|box_geometry| box_geometry.height_px),
            Some(80)
        );
    }
}

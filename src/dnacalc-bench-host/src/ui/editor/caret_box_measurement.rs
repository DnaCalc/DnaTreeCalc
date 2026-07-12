//! Browser-side caret-box measurement adapter.
//!
//! Reads DOM dimensions off the textarea and its computed style, returns
//! a [`TextareaMeasurementMetrics`] snapshot the pure-Rust geometry layer
//! can convert into pixel anchors. This is the ONLY browser-coupled
//! piece of the caret-popup stack — every popup-anchored surface
//! (completion popup, signature help, hover tooltip, drill-down
//! hover-dim) consumes the metrics this module produces.
//!
//! ## Why not the previous WS-13 adapter
//!
//! The retired `browser_measurement.rs` (deleted in commit b00bcec)
//! depended on `ExploreEditorClusterViewModel` and computed metrics
//! from `textarea.rows` / `textarea.cols`, which is unreliable on
//! browsers that round box dimensions or apply user zoom. The new
//! adapter uses a hidden character-mirror span (a known-good
//! technique used by Monaco, CodeMirror, and the GitHub web editor)
//! and reads computed style for line-height + padding directly.
//!
//! ## Char-mirror approach
//!
//! `getComputedStyle(textarea).font` gives us the font shorthand. We
//! create a hidden `<span>` somewhere visible to the browser layout
//! engine, set the same font and a known string of monospace
//! characters (`MMMMMMMMMM` = 10 M's), and read its
//! `getBoundingClientRect().width / 10` for an accurate per-character
//! width. This avoids the `client_width / cols` fragility because we
//! never reason about the textarea's box dimensions — only the font
//! the browser is actually using to render it.
//!
//! ## Non-wasm32 stub
//!
//! On non-wasm32 targets (host-side SSR / unit tests) the function
//! returns `None`. State-side reducer tests exercise the
//! `apply_editor_box_metrics_to_active_formula_space` code path with
//! synthetic metrics; only the browser corpus exercises the real
//! adapter.

#![allow(dead_code)]

use crate::ui::editor::geometry::TextareaMeasurementMetrics;

#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::*;
    use wasm_bindgen::JsCast;
    use web_sys::{Document, Element, HtmlTextAreaElement};

    /// CSS `id` of the hidden character-mirror span. Created lazily by
    /// `ensure_char_mirror_span`, reused across measurement calls.
    const CHAR_MIRROR_ID: &str = "onecalc-home-shell__char-mirror";

    /// Sample string for the char-width calculation. `M` is the widest
    /// ASCII letter under most monospace fonts; for true monospace the
    /// width of any character is identical, but using `M` defends
    /// against fallback proportional fonts that may slip through.
    const SAMPLE_STRING: &str = "MMMMMMMMMM";
    const SAMPLE_LEN: f64 = 10.0;

    /// Read the textarea's geometry and return a metrics snapshot.
    /// Returns `None` if any DOM read fails — the caller treats this
    /// as "not yet measured" and suppresses caret-anchored surfaces.
    pub fn measure_textarea_box(
        textarea: &HtmlTextAreaElement,
        document: &Document,
    ) -> Option<TextareaMeasurementMetrics> {
        let window = web_sys::window()?;
        let style = window.get_computed_style(textarea).ok().flatten()?;

        let font = style
            .get_property_value("font")
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| compose_font_shorthand(&style));
        let line_height_px = read_line_height_px(&style)?;

        let mirror = ensure_char_mirror_span(document, font.as_deref())?;
        let rect = mirror.get_bounding_client_rect();
        let mirror_width = rect.width();
        if mirror_width <= 0.0 {
            return None;
        }
        let char_width_px = (mirror_width / SAMPLE_LEN).round().max(1.0) as usize;

        let scroll_top_px = textarea.scroll_top().max(0) as usize;
        let scroll_left_px = textarea.scroll_left().max(0) as usize;

        Some(TextareaMeasurementMetrics {
            char_width_px,
            line_height_px: line_height_px.max(1),
            scroll_top_px,
            scroll_left_px,
        })
    }

    /// Resolve the textarea's effective `line-height` to pixels. Falls
    /// back to `font-size * 1.2` when `line-height: normal` (the
    /// browser default for monospace).
    fn read_line_height_px(style: &web_sys::CssStyleDeclaration) -> Option<usize> {
        let raw = style.get_property_value("line-height").ok()?;
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return None;
        }

        if let Some(stripped) = trimmed.strip_suffix("px") {
            if let Ok(value) = stripped.trim().parse::<f64>() {
                return Some(value.round().max(1.0) as usize);
            }
        }

        // "normal" or unitless number: derive from font-size.
        let font_size_raw = style.get_property_value("font-size").ok()?;
        let font_size_px = font_size_raw
            .strip_suffix("px")
            .and_then(|s| s.trim().parse::<f64>().ok())?;

        if trimmed == "normal" {
            return Some((font_size_px * 1.2).round().max(1.0) as usize);
        }
        if let Ok(multiplier) = trimmed.parse::<f64>() {
            return Some((font_size_px * multiplier).round().max(1.0) as usize);
        }
        Some((font_size_px * 1.2).round().max(1.0) as usize)
    }

    /// Compose a `font` shorthand string from the constituent computed
    /// style properties when the browser doesn't expose `font` directly
    /// (Firefox computed style omits the shorthand).
    fn compose_font_shorthand(style: &web_sys::CssStyleDeclaration) -> Option<String> {
        let family = style
            .get_property_value("font-family")
            .ok()
            .filter(|s| !s.is_empty())?;
        let size = style
            .get_property_value("font-size")
            .ok()
            .filter(|s| !s.is_empty())?;
        let weight = style
            .get_property_value("font-weight")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "normal".to_string());
        let style_kw = style
            .get_property_value("font-style")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "normal".to_string());
        Some(format!("{style_kw} {weight} {size} / 1 {family}"))
    }

    /// Locate the hidden char-mirror span in the document, creating it
    /// the first time. The span is positioned off-screen but visible to
    /// layout (`position: absolute; visibility: hidden; top: -9999px`)
    /// so its `getBoundingClientRect` reports an honest width.
    fn ensure_char_mirror_span(document: &Document, font: Option<&str>) -> Option<Element> {
        let element = match document.get_element_by_id(CHAR_MIRROR_ID) {
            Some(existing) => existing,
            None => {
                let span = document.create_element("span").ok()?;
                span.set_id(CHAR_MIRROR_ID);
                span.set_attribute("aria-hidden", "true").ok()?;
                span.set_text_content(Some(SAMPLE_STRING));

                let html_span = span.dyn_ref::<web_sys::HtmlElement>()?;
                let css_text = "position: absolute; visibility: hidden; \
                                top: -9999px; left: -9999px; \
                                white-space: pre; pointer-events: none;";
                html_span.style().set_css_text(css_text);

                document.body()?.append_child(&span).ok()?;
                span
            }
        };

        if let Some(font) = font {
            if let Some(html) = element.dyn_ref::<web_sys::HtmlElement>() {
                let _ = html.style().set_property("font", font);
            }
        }
        Some(element)
    }
}

#[cfg(target_arch = "wasm32")]
pub use wasm::measure_textarea_box;

/// Non-wasm stub so call sites compile cleanly under `cargo test`
/// (host) and `cargo build` (SSR). The host-side test surface
/// exercises the reducer entry point with synthetic metrics; only the
/// browser corpus exercises the real adapter. The signature matches
/// the wasm version so the caller in `home_shell.rs` (which compiles
/// for both targets) doesn't need a `cfg` of its own.
#[cfg(not(target_arch = "wasm32"))]
pub fn measure_textarea_box(
    _textarea: &web_sys::HtmlTextAreaElement,
    _document: &web_sys::Document,
) -> Option<TextareaMeasurementMetrics> {
    None
}

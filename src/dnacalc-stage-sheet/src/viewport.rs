//! VIEWPORT — the Sheet stage's pure scroll/zoom logic (S3.9 + S3.10).
//!
//! A DOM-free, Leptos-free, fully unit-testable layer the crate-root wiring
//! ([`crate`]) consumes but never re-implements, so the reactive viewport/zoom
//! plumbing in `SheetStage::mount` stays a thin threading of these functions:
//!
//! 1. **The RAF interest coalescer** — [`ScrollCoalescer`] accumulates the wheel
//!    deltas that arrive within one animation frame into a single pending scroll
//!    delta, which the frame boundary drains once ([`ScrollCoalescer::note_delta`]
//!    / [`ScrollCoalescer::take_pending`]). No DOM/RAF dependency — the frame
//!    boundary is owned by the caller (one `take_pending` per
//!    `requestAnimationFrame` tick in the live stage), so the "N deltas in a
//!    frame → 1 applied" invariant is a pure property of the calls. This mirrors
//!    the estate's `GridInterestCoalescer` (route-map §C.2 K1b) — the trunk
//!    coalesces to one `SetGridInterest`, this stage coalesces to one viewport
//!    repaint, and G4 makes the same frame boundary emit real interest.
//!
//! 2. **Scroll clamping** — [`clamp_scroll`] bounds a scroll offset to
//!    `[0, used-range + margin]` so a wheel storm can never park the viewport
//!    before the origin or past the grid's real content.
//!
//! 3. **Semantic zoom** — [`zoom_tier`] classifies a zoom factor into the three
//!    SHEET_SPEC §3 tiers, [`clamp_zoom`] holds the factor to a sane range, and
//!    [`legible_factor`] is the legibility floor: the effective metrics factor
//!    never drops below [`LEGIBILITY_FLOOR_FACTOR`], so cell text (drawn at a
//!    fixed legible size by [`crate::canvas`]) never shrinks below readable —
//!    below the floor the Structure/District tiers HONEST-DEGRADE to a
//!    disabled-with-reason note rather than half-build the labeled-block/district
//!    renderers.
//!
//! 4. **The Ctrl+arrow edge-jump degrade** — [`edge_dir_from_key`] /
//!    [`window_edge_jump`] land the active cell on the current window's edge, the
//!    honest bounded-scale stand-in for the data-aware edge-jump that needs the
//!    G4 model query ([`EDGE_JUMP_DEGRADE_REASON`]).
//!
//! Every honest-degrade reason string is a greppable `const` here (each citing
//! the gap it defers to — `G4` for interest / edge-jump, the tier name for zoom)
//! so the wiring and the notes it renders read identically.

/// The zoom step each `+`/`−` press multiplies / divides the factor by.
pub const ZOOM_STEP: f64 = 1.25;
/// The minimum zoom factor the control clamps to (10%).
pub const ZOOM_MIN: f64 = 0.1;
/// The maximum zoom factor the control clamps to (400%).
pub const ZOOM_MAX: f64 = 4.0;

/// The Detail-tier lower bound (SHEET_SPEC §3: Detail ≥ 60%). Also the
/// legibility floor: the effective metrics factor never drops below this, so
/// cell text (fixed size) never renders below the readable size.
pub const DETAIL_MIN_FACTOR: f64 = 0.6;
/// The Structure-tier lower bound (SHEET_SPEC §3: Structure 15–60%).
pub const STRUCTURE_MIN_FACTOR: f64 = 0.15;
/// The legibility floor: the smallest factor the Detail grid is ever drawn at,
/// so text stays above the readable size even in the Structure/District tiers
/// (which clamp the drawn grid here and add the honest degrade note on top).
pub const LEGIBILITY_FLOOR_FACTOR: f64 = DETAIL_MIN_FACTOR;

/// How many extra cells of overscan [`clamp_scroll`] allows past the used range,
/// so the last row/column is not jammed against the viewport edge.
const SCROLL_MARGIN_CELLS: f64 = 2.0;

/// The three semantic-zoom tiers (SHEET_SPEC §3). Only [`Tier::Detail`] is built
/// (full cells/gridlines/text); [`Tier::Structure`] and [`Tier::District`] are
/// honest-degraded to a disabled-with-reason note in v1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    /// ≥ 60%: full cell text, gridlines, provenance typography (the built tier).
    Detail,
    /// 15–60%: values fade to labeled Strand blocks (named ranges/tables/spills)
    /// — NOT built in v1 (honest degrade note).
    Structure,
    /// < 15%: the sheet as a used-range map, visually continuous with Atlas —
    /// NOT built in v1 (honest degrade note).
    District,
}

/// Classify a zoom `factor` into its [`Tier`]: Detail ≥ 0.6, Structure in
/// `[0.15, 0.6)`, District `< 0.15` (SHEET_SPEC §3). A non-finite factor is a
/// guard case that resolves to [`Tier::Detail`] (the safe, always-legible tier).
#[must_use]
pub fn zoom_tier(factor: f64) -> Tier {
    if !factor.is_finite() {
        return Tier::Detail;
    }
    if factor >= DETAIL_MIN_FACTOR {
        Tier::Detail
    } else if factor >= STRUCTURE_MIN_FACTOR {
        Tier::Structure
    } else {
        Tier::District
    }
}

/// Clamp a zoom factor to the control's `[ZOOM_MIN, ZOOM_MAX]` range; a
/// non-finite value resets to `1.0` (100%). Pure and total, so a `+`/`−`/reset
/// press can never drive the zoom out of range or to `NaN`.
#[must_use]
pub fn clamp_zoom(factor: f64) -> f64 {
    if !factor.is_finite() {
        return 1.0;
    }
    factor.clamp(ZOOM_MIN, ZOOM_MAX)
}

/// The effective metrics factor for a zoom `factor`, floored at the legibility
/// floor ([`LEGIBILITY_FLOOR_FACTOR`]). At Detail zoom the factor passes through
/// unchanged; in the Structure/District tiers it clamps up to the floor so the
/// still-drawn Detail grid keeps its text above the readable size. A non-finite
/// value resets to `1.0`.
#[must_use]
pub fn legible_factor(factor: f64) -> f64 {
    if !factor.is_finite() {
        return 1.0;
    }
    factor.max(LEGIBILITY_FLOOR_FACTOR)
}

/// The honest degrade reason for a non-Detail [`Tier`], or `None` for Detail
/// (which renders the full grid with no note). The Structure/District tiers are
/// deliberately not built in v1 — the note says so and cites the tier by name.
#[must_use]
pub const fn tier_degrade_reason(tier: Tier) -> Option<&'static str> {
    match tier {
        Tier::Detail => None,
        Tier::Structure => Some(STRUCTURE_TIER_DEFERRED_REASON),
        Tier::District => Some(DISTRICT_TIER_DEFERRED_REASON),
    }
}

/// The Structure-tier honest-degrade note: the labeled-block renderer is not
/// built, so the Detail grid is held at the legibility floor and this note
/// explains the semantic tier is absent.
pub const STRUCTURE_TIER_DEFERRED_REASON: &str =
    "Structure tier (named ranges, tables and spills as labeled blocks) isn't built yet — \
     showing the Detail grid held at the legibility floor.";

/// The District-tier honest-degrade note: the used-range map renderer is not
/// built, so the Detail grid is held at the legibility floor and this note
/// explains the semantic tier is absent.
pub const DISTRICT_TIER_DEFERRED_REASON: &str =
    "District tier (the used-range map, continuous with Atlas) isn't built yet — \
     showing the Detail grid held at the legibility floor.";

/// The honest bounded-scale interest note (S3.9): the workbook dispatcher treats
/// `SetGridInterest` as a no-op today (G4 unbuilt), so the stage renders the
/// host's already-windowed cells as-is with no client-side virtualization, and
/// windowing / prefetch / multi-rect interest are deferred to G4. Cites `G4`.
pub const BOUNDED_SCALE_INTEREST_NOTE: &str =
    "Scrolling renders the host's full bounded window as-is; viewport windowing, prefetch \
     and multi-rect interest arrive with G4.";

/// The honest degrade reason for a Ctrl+arrow edge-jump: the data-aware jump
/// (stop at the next data boundary) needs the G4 model query, so the stage
/// degrades to a window-local jump to the current window's edge. Cites `G4`.
pub const EDGE_JUMP_DEGRADE_REASON: &str =
    "Ctrl+arrow jumps to the window edge — data-aware edge-jump needs the G4 model query.";

/// Clamp a scroll offset to `[0, max]` where `max` is the used range past the
/// visible data area plus a small overscan margin.
///
/// `extent × band_size` is the grid's content length along this axis;
/// `viewport_len − header` is the visible data area; the furthest legal scroll is
/// `content − data_area` (so the last band aligns with the viewport's far edge)
/// plus [`SCROLL_MARGIN_CELLS`] of overscan. The result is never negative (a
/// wheel-up past the origin parks at `0`) and never past `max` (a wheel-down past
/// the content parks at the used range + margin). A non-finite `scroll` resets to
/// `0`, and a degenerate `band_size` falls back to a non-negative clamp, so the
/// function is total and never produces `NaN`/negative scroll.
#[must_use]
pub fn clamp_scroll(scroll: f64, extent: u32, band_size: f64, header: f64, viewport_len: f64) -> f64 {
    if !scroll.is_finite() {
        return 0.0;
    }
    if !band_size.is_finite() || band_size <= 0.0 {
        return scroll.max(0.0);
    }
    let content = f64::from(extent) * band_size;
    let data_area = if viewport_len.is_finite() && header.is_finite() {
        (viewport_len - header).max(0.0)
    } else {
        0.0
    };
    let margin = band_size * SCROLL_MARGIN_CELLS;
    let max_scroll = (content - data_area + margin).max(0.0);
    scroll.clamp(0.0, max_scroll)
}

/// A per-frame scroll coalescer: it accumulates the wheel deltas noted within one
/// animation frame into a single pending `(dx, dy)`, drained once at the frame
/// boundary.
///
/// The whole point is coalescing: any number of [`note_delta`](Self::note_delta)
/// calls between two [`take_pending`](Self::take_pending) calls collapse into one
/// summed delta the frame applies once, so a scroll storm produces at most one
/// viewport update per frame. No DOM/RAF here — the caller owns the frame
/// boundary (a `requestAnimationFrame` callback calls `take_pending`), so this is
/// unit-testable as a pure function of the call sequence.
#[derive(Debug, Default)]
pub struct ScrollCoalescer {
    pending: Option<(f64, f64)>,
}

impl ScrollCoalescer {
    #[must_use]
    pub fn new() -> Self {
        Self { pending: None }
    }

    /// Add one wheel event's `(dx, dy)` to this frame's pending scroll delta.
    /// Non-finite deltas are ignored, so wheel noise never poisons the sum with
    /// `NaN`.
    pub fn note_delta(&mut self, dx: f64, dy: f64) {
        if !dx.is_finite() || !dy.is_finite() {
            return;
        }
        let (px, py) = self.pending.unwrap_or((0.0, 0.0));
        self.pending = Some((px + dx, py + dy));
    }

    /// Take this frame's accumulated delta, clearing it. Returns `None` when no
    /// delta was noted since the last drain (so the frame applies nothing).
    pub fn take_pending(&mut self) -> Option<(f64, f64)> {
        self.pending.take()
    }
}

/// A cardinal direction for a Ctrl+arrow edge-jump.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeDir {
    Up,
    Down,
    Left,
    Right,
}

/// The [`EdgeDir`] a raw arrow key names, or `None` for any other key (so the
/// wiring only treats the four arrows as edge-jumps and lets everything else
/// bubble).
#[must_use]
pub fn edge_dir_from_key(key: &str) -> Option<EdgeDir> {
    match key {
        "ArrowUp" => Some(EdgeDir::Up),
        "ArrowDown" => Some(EdgeDir::Down),
        "ArrowLeft" => Some(EdgeDir::Left),
        "ArrowRight" => Some(EdgeDir::Right),
        _ => None,
    }
}

/// The window-local edge-jump target: the current window's edge in `dir`, the
/// honest bounded-scale degrade for a Ctrl+arrow (the data-aware jump needs the
/// G4 model query, see [`EDGE_JUMP_DEGRADE_REASON`]). Since the host hands the
/// whole bounded sheet as one window, the window edge is the grid extent edge.
/// Pure and total — the result is always a legal in-extent cell.
#[must_use]
pub fn window_edge_jump(
    current: (u32, u32),
    dir: EdgeDir,
    extent_rows: u32,
    extent_cols: u32,
) -> (u32, u32) {
    let (row, col) = current;
    let rows = extent_rows.max(1);
    let cols = extent_cols.max(1);
    let (r, c) = match dir {
        EdgeDir::Up => (1, col),
        EdgeDir::Down => (rows, col),
        EdgeDir::Left => (row, 1),
        EdgeDir::Right => (row, cols),
    };
    (r.clamp(1, rows), c.clamp(1, cols))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `zoom_tier` splits at the SHEET_SPEC §3 boundaries (Detail ≥ 0.6,
    /// Structure `[0.15, 0.6)`, District `< 0.15`), with the boundary value owned
    /// by the higher tier (half-open from below), and a non-finite factor guarded
    /// to Detail.
    #[test]
    fn zoom_tier_splits_at_the_spec_boundaries() {
        assert_eq!(zoom_tier(4.0), Tier::Detail);
        assert_eq!(zoom_tier(1.0), Tier::Detail);
        assert_eq!(zoom_tier(0.6), Tier::Detail, "0.6 is the Detail floor");
        assert_eq!(zoom_tier(0.5999), Tier::Structure, "just below 0.6 is Structure");
        assert_eq!(zoom_tier(0.3), Tier::Structure);
        assert_eq!(zoom_tier(0.15), Tier::Structure, "0.15 is the Structure floor");
        assert_eq!(zoom_tier(0.1499), Tier::District, "just below 0.15 is District");
        assert_eq!(zoom_tier(0.05), Tier::District);
        // Guard: a non-finite factor resolves to the safe, legible tier.
        assert_eq!(zoom_tier(f64::NAN), Tier::Detail);
        assert_eq!(zoom_tier(f64::INFINITY), Tier::Detail);
    }

    /// `clamp_zoom` holds the factor to `[ZOOM_MIN, ZOOM_MAX]` and resets a
    /// non-finite value to 1.0.
    #[test]
    fn clamp_zoom_bounds_the_factor() {
        assert!((clamp_zoom(1.0) - 1.0).abs() < 1e-9);
        assert!((clamp_zoom(0.05) - ZOOM_MIN).abs() < 1e-9, "below min clamps up");
        assert!((clamp_zoom(10.0) - ZOOM_MAX).abs() < 1e-9, "above max clamps down");
        assert!((clamp_zoom(f64::NAN) - 1.0).abs() < 1e-9, "NaN resets to 1.0");
    }

    /// `legible_factor` is the legibility floor: Detail zoom passes through, and
    /// any sub-floor factor (Structure/District) clamps up to
    /// [`LEGIBILITY_FLOOR_FACTOR`] so the drawn grid's text never shrinks below
    /// readable.
    #[test]
    fn legible_factor_never_drops_below_the_floor() {
        assert!((legible_factor(2.0) - 2.0).abs() < 1e-9, "zoom-in passes through");
        assert!((legible_factor(1.0) - 1.0).abs() < 1e-9);
        assert!(
            (legible_factor(0.6) - 0.6).abs() < 1e-9,
            "the floor value passes through"
        );
        assert!(
            (legible_factor(0.3) - LEGIBILITY_FLOOR_FACTOR).abs() < 1e-9,
            "Structure clamps up to the floor"
        );
        assert!(
            (legible_factor(0.02) - LEGIBILITY_FLOOR_FACTOR).abs() < 1e-9,
            "District clamps up to the floor"
        );
        assert!((legible_factor(f64::NAN) - 1.0).abs() < 1e-9);
        // The floor factor keeps a data row above the fixed 12px cell font:
        // 0.6 × 22px default row = 13.2px ≥ 12px — a compile-time invariant.
        const { assert!(LEGIBILITY_FLOOR_FACTOR * 22.0 >= 12.0) };
    }

    /// `tier_degrade_reason` is `None` only for Detail; the Structure/District
    /// reasons name their tier and mark it as not-built.
    #[test]
    fn tier_degrade_reason_is_none_only_for_detail() {
        assert_eq!(tier_degrade_reason(Tier::Detail), None);
        let structure = tier_degrade_reason(Tier::Structure).expect("Structure degrades");
        assert!(structure.contains("Structure") && structure.contains("isn't built"));
        let district = tier_degrade_reason(Tier::District).expect("District degrades");
        assert!(district.contains("District") && district.contains("isn't built"));
    }

    /// Both bounded-scale S3.9 notes cite the gap they defer to (`G4`), the
    /// greppable honesty marker the interest note and the edge-jump note share.
    #[test]
    fn bounded_scale_notes_cite_g4() {
        assert!(BOUNDED_SCALE_INTEREST_NOTE.contains("G4"));
        assert!(EDGE_JUMP_DEGRADE_REASON.contains("G4"));
    }

    /// `clamp_scroll` never returns a negative offset and never exceeds the used
    /// range + margin: a wheel-up past the origin parks at 0, a mid value passes
    /// through, and a wheel-down past the content parks at the computed maximum.
    #[test]
    fn clamp_scroll_bounds_to_zero_and_the_used_range() {
        // 100 rows × 22px = 2200px content; 200 − 22 = 178px data area; margin
        // = 44px → max = 2200 − 178 + 44 = 2066px.
        let (extent, band, header, vp) = (100u32, 22.0, 22.0, 200.0);
        let max = 2200.0 - 178.0 + 44.0;

        assert_eq!(clamp_scroll(-50.0, extent, band, header, vp), 0.0, "never negative");
        assert_eq!(clamp_scroll(0.0, extent, band, header, vp), 0.0);
        assert_eq!(clamp_scroll(500.0, extent, band, header, vp), 500.0, "mid passes through");
        assert!(
            (clamp_scroll(1.0e9, extent, band, header, vp) - max).abs() < 1e-9,
            "never past the used range + margin"
        );
        assert_eq!(clamp_scroll(f64::NAN, extent, band, header, vp), 0.0, "NaN resets to 0");
    }

    /// A grid that already fits the viewport clamps every scroll to 0 (there is
    /// nothing to scroll to), and a degenerate band size falls back to a
    /// non-negative clamp rather than dividing by zero or fabricating NaN.
    #[test]
    fn clamp_scroll_handles_fits_and_degenerate_metrics() {
        // 3 rows × 22px = 66px content, but a 178px data area → nothing to scroll.
        assert_eq!(clamp_scroll(500.0, 3, 22.0, 22.0, 200.0), 0.0);
        // Degenerate band size: fall back to a non-negative clamp.
        assert_eq!(clamp_scroll(-10.0, 100, 0.0, 22.0, 200.0), 0.0);
        assert_eq!(clamp_scroll(30.0, 100, 0.0, 22.0, 200.0), 30.0);
    }

    /// THE coalescer invariant: N deltas noted within one frame collapse to ONE
    /// summed delta, drained once — the next drain returns `None` (the frame
    /// applies nothing).
    #[test]
    fn scroll_coalescer_collapses_n_deltas_to_one_summed_apply() {
        let mut coalescer = ScrollCoalescer::new();
        assert_eq!(coalescer.take_pending(), None, "nothing pending before any note");

        coalescer.note_delta(10.0, 0.0);
        coalescer.note_delta(5.0, -3.0);
        coalescer.note_delta(0.0, 7.0);
        // One summed delta for the whole frame, not three.
        assert_eq!(coalescer.take_pending(), Some((15.0, 4.0)));
        // Drained: the next frame boundary applies nothing.
        assert_eq!(coalescer.take_pending(), None);

        // A fresh frame accumulates independently.
        coalescer.note_delta(2.0, 2.0);
        assert_eq!(coalescer.take_pending(), Some((2.0, 2.0)));
    }

    /// Non-finite wheel deltas are ignored, so they never poison the accumulated
    /// sum with `NaN`.
    #[test]
    fn scroll_coalescer_ignores_non_finite_deltas() {
        let mut coalescer = ScrollCoalescer::new();
        coalescer.note_delta(f64::NAN, 5.0);
        coalescer.note_delta(10.0, f64::INFINITY);
        assert_eq!(coalescer.take_pending(), None, "only non-finite deltas → nothing pending");

        coalescer.note_delta(4.0, 4.0);
        coalescer.note_delta(f64::NAN, f64::NAN);
        assert_eq!(coalescer.take_pending(), Some((4.0, 4.0)), "the finite delta survives");
    }

    /// `edge_dir_from_key` recognizes exactly the four arrow keys and nothing
    /// else, and `window_edge_jump` lands on the window (extent) edge in that
    /// direction, keeping the orthogonal axis and clamping into the extent.
    #[test]
    fn edge_jump_maps_arrows_and_lands_on_the_window_edge() {
        assert_eq!(edge_dir_from_key("ArrowUp"), Some(EdgeDir::Up));
        assert_eq!(edge_dir_from_key("ArrowDown"), Some(EdgeDir::Down));
        assert_eq!(edge_dir_from_key("ArrowLeft"), Some(EdgeDir::Left));
        assert_eq!(edge_dir_from_key("ArrowRight"), Some(EdgeDir::Right));
        assert_eq!(edge_dir_from_key("Home"), None);
        assert_eq!(edge_dir_from_key("a"), None);

        // From (3, 2) over a 10×5 window: each jump parks on the matching edge.
        assert_eq!(window_edge_jump((3, 2), EdgeDir::Up, 10, 5), (1, 2));
        assert_eq!(window_edge_jump((3, 2), EdgeDir::Down, 10, 5), (10, 2));
        assert_eq!(window_edge_jump((3, 2), EdgeDir::Left, 10, 5), (3, 1));
        assert_eq!(window_edge_jump((3, 2), EdgeDir::Right, 10, 5), (3, 5));

        // A degenerate extent floors to one legal cell (never an out-of-range
        // address).
        assert_eq!(window_edge_jump((9, 9), EdgeDir::Down, 0, 0), (1, 1));
    }
}

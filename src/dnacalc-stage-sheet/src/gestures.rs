//! Touch-gesture classification for the Sheet canvas (SHELL_SPEC §1.1).
//!
//! The canvas owns its viewport (`touch-action: none`), so touch scrolling,
//! tapping and pinching are handled here as explicit gestures. The geometry-
//! free decision logic lives in this module as pure state machines pinned by
//! native tests; [`crate`]'s event handlers only translate web-sys pointer
//! events into these calls and apply the results to the reactive viewport.
//!
//! Semantics (matching the mouse grammar):
//! - A tap SELECTS — selection happens on `pointerdown`, exactly where the
//!   old mouse `mousedown` handler did, so feedback is immediate.
//! - A quick second tap near the first enters EDIT — the touch equivalent of
//!   the mouse `dblclick`. Detected on `pointerup`; the tracker never fires
//!   it once the gesture turned into a pan or a pinch.
//! - One-finger drag PANS (content follows the finger); two-finger pinch
//!   ZOOMS around the pinch's starting zoom factor.

/// Maximum gap between the two taps of a double-tap (ms).
pub const DOUBLE_TAP_GAP_MS: f64 = 350.0;
/// Maximum duration of a single tap for it to count as a tap (ms).
pub const MAX_TAP_MS: f64 = 400.0;
/// A tap that moved further than this from its `pointerdown` is a pan, not a
/// tap (CSS px, total from the origin).
pub const TAP_SLOP_PX: f64 = 10.0;
/// Finger travel beyond this starts a pan (CSS px, total from the origin).
pub const PAN_SLOP_PX: f64 = 6.0;
/// Two taps within this distance (between the tap POINTS) are a double-tap.
pub const DOUBLE_TAP_RADIUS_PX: f64 = 24.0;

/// Euclidean distance between two points (the pinch span metric).
#[must_use]
pub fn point_distance(x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    (x2 - x1).hypot(y2 - y1)
}

/// The pinch zoom multiplier for a span grown/shrunk from `start_dist` to
/// `cur_dist` (pure ratio — the caller applies `clamp_zoom`). A degenerate
/// start span yields 1.0 (never scale).
#[must_use]
pub fn pinch_scale(start_dist: f64, cur_dist: f64) -> f64 {
    if start_dist <= 0.0 {
        1.0
    } else {
        cur_dist / start_dist
    }
}

/// Classifies one finger's down/move/up stream into pans and taps, and
/// recognizes the double-tap across consecutive taps.
///
/// `pointer_down` arms the tracker; `pointer_move` returns scroll-space
/// deltas (finger-inverted) once panning starts; `pointer_up` reports whether
/// the completed gesture was the SECOND tap of a double-tap.
#[derive(Debug, Default, Clone)]
pub struct PanTapTracker {
    armed: bool,
    down_ms: f64,
    origin_x: f64,
    origin_y: f64,
    last_x: f64,
    last_y: f64,
    panning: bool,
    last_tap: Option<TapStamp>,
}

#[derive(Debug, Default, Clone, Copy)]
struct TapStamp {
    up_ms: f64,
    x: f64,
    y: f64,
}

impl PanTapTracker {
    /// Arm tracking for a fresh finger at `(x, y)`.
    pub fn pointer_down(&mut self, now_ms: f64, x: f64, y: f64) {
        self.armed = true;
        self.panning = false;
        self.down_ms = now_ms;
        self.origin_x = x;
        self.origin_y = y;
        self.last_x = x;
        self.last_y = y;
    }

    /// Feed a move; returns `(dx, dy)` SCROLL-SPACE deltas to apply once the
    /// finger traveled past [`PAN_SLOP_PX`] (content follows the finger, so
    /// the deltas are finger-inverted), or `None` while the finger is still a
    /// candidate tap. Disarmed trackers (a pinch took over) never pan.
    pub fn pointer_move(&mut self, x: f64, y: f64) -> Option<(f64, f64)> {
        if !self.armed {
            return None;
        }
        if !self.panning && point_distance(self.origin_x, self.origin_y, x, y) > PAN_SLOP_PX {
            self.panning = true;
        }
        if !self.panning {
            return None;
        }
        let dx = self.last_x - x;
        let dy = self.last_y - y;
        self.last_x = x;
        self.last_y = y;
        Some((dx, dy))
    }

    /// Complete the gesture. `true` only when it was a genuine tap (fast,
    /// within [`TAP_SLOP_PX`], never panned) AND the previous genuine tap was
    /// recent ([`DOUBLE_TAP_GAP_MS`]) and near ([`DOUBLE_TAP_RADIUS_PX`]) —
    /// i.e. the EDIT-entering second tap. Any completed tap (first or second)
    /// becomes the new double-tap reference.
    #[must_use]
    pub fn pointer_up(&mut self, now_ms: f64) -> bool {
        if !self.armed {
            return false;
        }
        let is_tap = !self.panning
            && (now_ms - self.down_ms) <= MAX_TAP_MS
            && point_distance(self.origin_x, self.origin_y, self.last_x, self.last_y)
                <= TAP_SLOP_PX;
        self.armed = false;
        if !is_tap {
            self.last_tap = None;
            return false;
        }
        let stamp = TapStamp {
            up_ms: now_ms,
            x: self.last_x,
            y: self.last_y,
        };
        let is_double = self.last_tap.is_some_and(|previous| {
            (stamp.up_ms - previous.up_ms) <= DOUBLE_TAP_GAP_MS
                && point_distance(previous.x, previous.y, stamp.x, stamp.y) <= DOUBLE_TAP_RADIUS_PX
        });
        self.last_tap = Some(stamp);
        is_double
    }

    /// True while an armed finger has become a pan (used by the owner to skip
    /// pinch bookkeeping mid-drag).
    #[must_use]
    pub fn is_panning(&self) -> bool {
        self.armed && self.panning
    }
    /// Disarm without a verdict (a pinch took over the gesture).
    pub fn disarm(&mut self) {
        self.armed = false;
    }

    /// The completed tap's position (`pointer_up` must have returned true for
    /// it to be meaningful).
    #[must_use]
    pub fn tap_position(&self) -> (f64, f64) {
        (self.last_x, self.last_y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinch_scale_is_the_span_ratio_with_a_safe_degenerate() {
        assert!((pinch_scale(100.0, 200.0) - 2.0).abs() < 1e-9);
        assert!((pinch_scale(200.0, 100.0) - 0.5).abs() < 1e-9);
        assert_eq!(pinch_scale(0.0, 100.0), 1.0, "degenerate span never scales");
    }

    #[test]
    fn distance_is_plain_euclidean() {
        assert!((point_distance(0.0, 0.0, 3.0, 4.0) - 5.0).abs() < 1e-9);
    }

    #[test]
    fn quick_second_nearby_tap_is_a_double() {
        let mut t = PanTapTracker::default();
        t.pointer_down(0.0, 50.0, 50.0);
        assert!(!t.pointer_up(80.0), "first tap is never a double");
        t.pointer_down(150.0, 52.0, 51.0);
        assert!(t.pointer_up(180.0), "quick nearby second tap is a double");
    }

    #[test]
    fn slow_second_tap_is_single() {
        let mut t = PanTapTracker::default();
        t.pointer_down(0.0, 50.0, 50.0);
        assert!(!t.pointer_up(80.0));
        t.pointer_down(150.0 + DOUBLE_TAP_GAP_MS + 1.0, 50.0, 50.0);
        assert!(
            !t.pointer_up(150.0 + DOUBLE_TAP_GAP_MS + 101.0),
            "gap beyond DOUBLE_TAP_GAP_MS resets the pair"
        );
    }

    #[test]
    fn distant_second_tap_is_not_a_double() {
        let mut t = PanTapTracker::default();
        t.pointer_down(0.0, 50.0, 50.0);
        assert!(!t.pointer_up(80.0));
        t.pointer_down(150.0, 50.0 + DOUBLE_TAP_RADIUS_PX + 5.0, 50.0);
        assert!(!t.pointer_up(180.0));
    }

    #[test]
    fn long_press_is_not_a_tap() {
        let mut t = PanTapTracker::default();
        t.pointer_down(0.0, 50.0, 50.0);
        assert!(!t.pointer_up(MAX_TAP_MS + 1.0));
    }

    #[test]
    fn pan_swallows_the_tap_and_reports_inverted_deltas() {
        let mut t = PanTapTracker::default();
        t.pointer_down(0.0, 50.0, 50.0);
        // Sub-slop wiggle: still a tap candidate, no deltas.
        assert_eq!(t.pointer_move(53.0, 51.0), None);
        // Past the slop: panning starts; deltas are finger-inverted and
        // INCREMENTAL (relative to the previous position, not the origin).
        assert_eq!(t.pointer_move(70.0, 65.0), Some((-20.0, -15.0)));
        assert_eq!(t.pointer_move(60.0, 60.0), Some((10.0, 5.0)));
        assert!(t.is_panning());
        assert!(!t.pointer_up(120.0), "a pan is never a tap");
    }

    #[test]
    fn disarmed_tracker_is_inert() {
        let mut t = PanTapTracker::default();
        assert_eq!(t.pointer_move(1.0, 1.0), None);
        assert!(!t.pointer_up(10.0), "up without down is not a tap");
    }
}

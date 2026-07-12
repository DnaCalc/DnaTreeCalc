//! Cross-platform wall-clock helper. Returns a millisecond
//! timestamp suitable for computing **deltas** (not for absolute
//! time-of-day reasoning — see `live_bridge::current_excel_serial`
//! for that).
//!
//! On wasm we read `js_sys::Date::now()` (UTC milliseconds since
//! the Unix epoch — fine for deltas inside a single browser tab).
//! On native (host tests, perf probes) we use a process-relative
//! `Instant::now()` baseline and report `(now - baseline).as_secs_f64()
//! * 1000.0`. The two epochs differ by ~50 years, but the absolute
//! value never leaves the host — only differences are consumed
//! (see `editor_session::handle_formula_edit_intent`).

#[cfg(target_arch = "wasm32")]
pub fn wall_clock_now_ms() -> f64 {
    js_sys::Date::now()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn wall_clock_now_ms() -> f64 {
    use std::sync::OnceLock;
    use std::time::Instant;

    static EPOCH: OnceLock<Instant> = OnceLock::new();
    let epoch = EPOCH.get_or_init(Instant::now);
    epoch.elapsed().as_secs_f64() * 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wall_clock_now_ms_returns_a_finite_number() {
        let now = wall_clock_now_ms();
        assert!(now.is_finite());
        assert!(now >= 0.0);
    }

    #[test]
    fn wall_clock_now_ms_advances_monotonically_within_a_thread() {
        let a = wall_clock_now_ms();
        // Spin briefly to force a measurable delta even on very
        // fast machines without sleeping (the test is otherwise
        // wall-clock-dependent).
        let mut spin = 0u64;
        for i in 0..1_000_000 {
            spin = spin.wrapping_add(i);
        }
        std::hint::black_box(spin);
        let b = wall_clock_now_ms();
        assert!(b >= a, "wall clock went backwards: {a} -> {b}");
    }
}

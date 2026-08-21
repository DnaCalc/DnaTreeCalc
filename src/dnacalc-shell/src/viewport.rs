//! Narrow-viewport detection for the shell's responsive provisions
//! (SHELL_SPEC.md §1.1).
//!
//! The cockpit's region contract (§1) is desktop-first: fixed px rails that
//! squeeze the stage. Below the narrow breakpoint the rails compose as
//! overlay panels over the stage instead of shrinking it, and they start
//! collapsed so a phone sees the stage, not chrome.
//!
//! The breakpoint DECISION is a pure function pinned by native tests; only
//! the browser wiring (reading `window.innerWidth`, listening to `resize`)
//! is wasm-only. Native builds and tests get the desktop layout.

/// Viewports at or below this CSS-px width are NARROW.
///
/// 900px sits just under where Calc's full chrome (registry 232px + stage +
/// inspector 268px) stops leaving a usable stage; Bench's narrower
/// composition gets the same treatment for one consistent rule.
pub const NARROW_MAX_WIDTH_PX: f64 = 900.0;

/// The narrow-viewport decision. At or below [`NARROW_MAX_WIDTH_PX`] is
/// narrow; above it is the desktop layout.
#[must_use]
pub fn is_narrow_width(width_px: f64) -> bool {
    width_px <= NARROW_MAX_WIDTH_PX
}

/// The initial narrow decision from the live window. A missing window or an
/// unreadable width defaults to WIDE (the desktop contract) rather than
/// guessing narrow.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn initial_is_narrow() -> bool {
    leptos::web_sys::window()
        .and_then(|window| window.inner_width().ok())
        .and_then(|value| value.as_f64())
        .map(is_narrow_width)
        .unwrap_or(false)
}

/// Install the `resize` watcher that keeps a narrow signal live for the
/// page's lifetime. Must run inside a reactive owner ([`Shell`]'s body) —
/// Leptos disposes the listener with that owner. The signal starts from
/// [`initial_is_narrow`] truth set by the caller before this runs.
#[cfg(target_arch = "wasm32")]
pub fn install_narrow_watcher(narrow: leptos::prelude::RwSignal<bool>) {
    use leptos::prelude::Update;

    let handler = leptos::prelude::window_event_listener(leptos::ev::resize, move |_| {
        let Some(window) = leptos::web_sys::window() else {
            return;
        };
        if let Ok(width) = window.inner_width()
            && let Some(width) = width.as_f64()
        {
            narrow.update(|value| *value = is_narrow_width(width));
        }
    });
    // The listener lives for the owner's lifetime; hold it so it is not
    // dropped (and thus never disconnected) before then.
    std::mem::forget(handler);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breakpoint_boundary_is_inclusive() {
        assert!(is_narrow_width(900.0), "at the breakpoint is narrow");
        assert!(!is_narrow_width(900.01), "just above is wide");
        assert!(is_narrow_width(375.0));
        assert!(is_narrow_width(768.0));
        assert!(!is_narrow_width(1280.0));
    }
}

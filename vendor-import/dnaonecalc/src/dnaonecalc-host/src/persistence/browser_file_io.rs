//! Browser-host file IO for `.dnafml`. Save uses a download anchor;
//! Open uses a hidden `<input type="file">` that the user selects
//! through the native browser file picker.
//!
//! These functions are wasm32-only — Tauri host gets a parallel
//! `tauri_file_io.rs` (deferred to a later slice; `Save as…` and
//! `Open…` actions stub through `SEAM-ONECALC-SCENARIO-PERSIST` on
//! the desktop host until then).
//!
//! Why download / file-input instead of the Save/Open File Picker
//! APIs (`window.showSaveFilePicker` / `window.showOpenFilePicker`):
//! those are Chromium-only. The download/file-input fallback works
//! everywhere browsers run today (Edge, Chrome, Firefox, Safari).
//! When we move to Tauri the platform-native dialogs replace this
//! shim entirely.

#![cfg(target_arch = "wasm32")]

use std::cell::RefCell;
use std::rc::Rc;

use js_sys::{Array, Uint8Array};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    Blob, BlobPropertyBag, Document, Event, File, FileReader, HtmlAnchorElement, HtmlInputElement,
    Url, Window,
};

/// Suggest a filename for the save dialog given a scenario. The
/// caller picks the extension (`.dnafml` or `.xml`) per §8 of the
/// persistence plan; this helper just provides a sensible stem.
pub fn suggested_filename_stem(scenario_name: &str, scenario_id: &str) -> String {
    let raw = if scenario_name.is_empty() {
        scenario_id
    } else {
        scenario_name
    };
    sanitise_filename_stem(raw)
}

fn sanitise_filename_stem(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return "formula".to_string();
    }
    let mut out = String::with_capacity(trimmed.len());
    for ch in trimmed.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
            out.push(ch);
        } else if ch == ' ' {
            out.push('-');
        }
        // Drop everything else (control chars, slashes, colons,
        // quotes, etc.) — they would be rejected or rewritten by
        // the OS anyway.
    }
    if out.is_empty() {
        "formula".to_string()
    } else {
        out
    }
}

/// Trigger a download of the given XML payload. Browser host:
/// constructs a Blob, builds an Object URL, programmatically clicks
/// a temporary `<a download>` anchor. The browser shows its native
/// download UI (location and confirmation). On success returns
/// `Ok(())`; on host-side failure returns the JS error message.
pub fn save_xml_via_download(filename: &str, xml: &str) -> Result<(), String> {
    let document = current_document()?;

    let blob = build_xml_blob(xml)?;
    let object_url = Url::create_object_url_with_blob(&blob)
        .map_err(|error| format!("URL.createObjectURL failed: {}", js_error_message(&error)))?;

    let anchor: HtmlAnchorElement = document
        .create_element("a")
        .map_err(|error| {
            format!(
                "document.createElement(`a`) failed: {}",
                js_error_message(&error),
            )
        })?
        .dyn_into()
        .map_err(|_| "createElement(`a`) did not return HtmlAnchorElement".to_string())?;
    anchor.set_href(&object_url);
    anchor.set_download(filename);
    // Hide the anchor; we never attach it visibly.
    let html_element: web_sys::HtmlElement = anchor.clone().unchecked_into();
    html_element
        .style()
        .set_property("display", "none")
        .map_err(|error| format!("anchor style.display failed: {}", js_error_message(&error)))?;

    if let Some(body) = document.body() {
        body.append_child(&anchor)
            .map_err(|error| format!("body.appendChild failed: {}", js_error_message(&error)))?;
    }

    html_element.click();

    // Best-effort cleanup; ignore the result. Browsers also revoke
    // object URLs automatically when the document unloads.
    if let Some(body) = document.body() {
        let _ = body.remove_child(&anchor);
    }
    let _ = Url::revoke_object_url(&object_url);

    Ok(())
}

fn build_xml_blob(xml: &str) -> Result<Blob, String> {
    let bytes = xml.as_bytes();
    let buffer = Uint8Array::new_with_length(bytes.len() as u32);
    buffer.copy_from(bytes);
    let parts = Array::new();
    parts.push(&buffer.buffer());

    let property_bag = BlobPropertyBag::new();
    property_bag.set_type("application/vnd.dnaonecalc.formula+xml");

    Blob::new_with_buffer_source_sequence_and_options(&parts, &property_bag)
        .map_err(|error| format!("Blob construction failed: {}", js_error_message(&error),))
}

/// Opened-file payload returned by the open dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenedFormulaFile {
    pub filename: String,
    pub xml: String,
}

/// Show a file-picker dialog and resolve to the user's selection.
/// `None` means the user dismissed the dialog without picking; an
/// `Err` is reserved for actual host-side failure.
///
/// Implementation: builds a hidden `<input type="file">` with the
/// `.dnafml,.xml` accept filter, programmatically clicks it, then
/// awaits the change event via a oneshot Closure-driven future.
/// FileReader pulls the contents as text. The `.xml` filter is
/// permissive (per §8 of the plan, both extensions are first-class),
/// not a guarantee — content-detection happens in
/// `read_formula_xml`.
pub async fn open_xml_via_file_input() -> Result<Option<OpenedFormulaFile>, String> {
    let document = current_document()?;

    let input: HtmlInputElement = document
        .create_element("input")
        .map_err(|error| {
            format!(
                "document.createElement(`input`) failed: {}",
                js_error_message(&error),
            )
        })?
        .dyn_into()
        .map_err(|_| "createElement(`input`) did not return HtmlInputElement".to_string())?;
    input.set_type("file");
    input.set_accept(".dnafml,.xml,application/xml,text/xml");
    let html_element: web_sys::HtmlElement = input.clone().unchecked_into();
    html_element
        .style()
        .set_property("display", "none")
        .map_err(|error| format!("input style.display failed: {}", js_error_message(&error)))?;

    // The change event must outlive this call until the user picks
    // (or dismisses) — but JsFuture awaits a Promise, not an Event.
    // Build a small Promise/resolver shim by stashing a oneshot
    // signal that the change handler resolves.
    let (sender, receiver) = oneshot_pair::<Option<File>>();
    let sender_for_change = sender.clone();
    let on_change = Closure::wrap(Box::new(move |event: Event| {
        let target = event
            .target()
            .and_then(|t| t.dyn_into::<HtmlInputElement>().ok());
        let file = target
            .and_then(|input| input.files())
            .and_then(|list| list.item(0));
        sender_for_change.send(file);
    }) as Box<dyn FnMut(Event)>);
    input
        .add_event_listener_with_callback("change", on_change.as_ref().unchecked_ref())
        .map_err(|error| {
            format!(
                "input.addEventListener(`change`) failed: {}",
                js_error_message(&error),
            )
        })?;
    on_change.forget();

    if let Some(body) = document.body() {
        body.append_child(&input)
            .map_err(|error| format!("body.appendChild failed: {}", js_error_message(&error)))?;
    }
    html_element.click();

    let file = receiver.recv().await;

    // Cleanup the input element regardless of outcome.
    if let Some(body) = document.body() {
        let _ = body.remove_child(&input);
    }

    let Some(file) = file else {
        return Ok(None);
    };
    let filename = file.name();
    let xml = read_file_as_text(file).await?;
    Ok(Some(OpenedFormulaFile { filename, xml }))
}

/// Read a `.bas` file's text content via the browser FileReader API.
/// Called from the VBA host panel's file-input change handler.
pub async fn read_bas_file_as_text(file: File) -> Result<String, String> {
    read_file_as_text(file).await
}

async fn read_file_as_text(file: File) -> Result<String, String> {
    let reader = FileReader::new().map_err(|error| {
        format!(
            "FileReader construction failed: {}",
            js_error_message(&error)
        )
    })?;
    let promise = js_sys::Promise::new(&mut |resolve, reject| {
        let reader_clone = reader.clone();
        let resolve_clone = resolve.clone();
        let on_load = Closure::once_into_js(move || {
            let result = reader_clone.result().unwrap_or(JsValue::NULL);
            let _ = resolve_clone.call1(&JsValue::NULL, &result);
        });
        let _ = reader.add_event_listener_with_callback("load", on_load.as_ref().unchecked_ref());

        let on_error = Closure::once_into_js(move || {
            let _ = reject.call1(&JsValue::NULL, &JsValue::from_str("FileReader error"));
        });
        let _ = reader.add_event_listener_with_callback("error", on_error.as_ref().unchecked_ref());
    });

    reader
        .read_as_text(&file)
        .map_err(|error| format!("FileReader.readAsText failed: {}", js_error_message(&error)))?;

    let result = JsFuture::from(promise)
        .await
        .map_err(|error| format!("FileReader load failed: {}", js_error_message(&error)))?;
    result
        .as_string()
        .ok_or_else(|| "FileReader.result was not a string".to_string())
}

fn current_document() -> Result<Document, String> {
    let window: Window = web_sys::window().ok_or_else(|| "window unavailable".to_string())?;
    window
        .document()
        .ok_or_else(|| "document unavailable".to_string())
}

fn js_error_message(error: &JsValue) -> String {
    error
        .as_string()
        .or_else(|| {
            js_sys::Reflect::get(error, &JsValue::from_str("message"))
                .ok()
                .and_then(|message| message.as_string())
        })
        .unwrap_or_else(|| format!("{:?}", error))
}

// ---------------------------------------------------------------------------
// Tiny oneshot signal — the file-input change handler needs to push
// the picked file across an async boundary. js_sys::Promise gives us
// that, but for the change-event handler the cleanest shape is a
// Rc<RefCell<Option<T>>> guarded oneshot. Keeps the dependency graph
// small (no `oneshot::channel` crate); we just need send + recv where
// send is non-async and recv is an async polling loop.
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct OneshotSender<T> {
    state: Rc<RefCell<OneshotState<T>>>,
}

struct OneshotReceiver<T> {
    state: Rc<RefCell<OneshotState<T>>>,
}

struct OneshotState<T> {
    value: Option<T>,
    delivered: bool,
}

fn oneshot_pair<T>() -> (OneshotSender<T>, OneshotReceiver<T>) {
    let state = Rc::new(RefCell::new(OneshotState {
        value: None,
        delivered: false,
    }));
    (
        OneshotSender {
            state: Rc::clone(&state),
        },
        OneshotReceiver { state },
    )
}

impl<T> OneshotSender<T> {
    fn send(&self, value: T) {
        let mut state = self.state.borrow_mut();
        if !state.delivered {
            state.value = Some(value);
            state.delivered = true;
        }
    }
}

impl<T> OneshotReceiver<T> {
    async fn recv(self) -> T
    where
        T: Default,
    {
        // Poll the cell once per microtask until the sender deposits a
        // value. wasm-bindgen-futures' `next_microtask` would be
        // cleaner but isn't a dep; this manual Promise.resolve loop is
        // equivalent.
        loop {
            {
                let mut state = self.state.borrow_mut();
                if state.delivered {
                    return state.value.take().unwrap_or_default();
                }
            }
            let promise = js_sys::Promise::resolve(&JsValue::UNDEFINED);
            let _ = JsFuture::from(promise).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitise_filename_keeps_alphanumerics_dashes_underscores() {
        assert_eq!(sanitise_filename_stem("invoice-eu-tax"), "invoice-eu-tax");
        assert_eq!(sanitise_filename_stem("My Formula 1"), "My-Formula-1");
        assert_eq!(sanitise_filename_stem(""), "formula");
        assert_eq!(sanitise_filename_stem("   "), "formula");
        // Drop OS-special chars: slashes, colons, quotes.
        assert_eq!(sanitise_filename_stem("a/b:c\"d"), "abcd");
    }

    #[test]
    fn suggested_filename_stem_prefers_name_then_id_then_default() {
        assert_eq!(suggested_filename_stem("my-name", "untitled-1"), "my-name",);
        assert_eq!(suggested_filename_stem("", "untitled-1"), "untitled-1",);
        assert_eq!(suggested_filename_stem("", ""), "formula");
    }
}

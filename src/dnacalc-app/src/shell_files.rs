//! The desktop shell's file bridge (W011 Wave 1.5, dtc-j7n8.10) — the WASM
//! frontend's one door to the native file dialogs `dnacalc-app-desktop`
//! registers as Tauri commands (`open_xlsx_file` / `save_xlsx_file`).
//!
//! Only bytes cross this seam: the shell picks a file and hands its bytes in,
//! or takes the bytes the host's `SaveActiveXlsx` produced and writes them
//! where the user pointed the save dialog. Everything that understands those
//! bytes (OxDoc, the engine, the command surface) stays behind the host
//! dispatcher, reached through [`crate::document::DocumentController`]; no
//! skin ever sees a file API. Mirrors the `tauri_file_io` bridge the Bench
//! product uses (`window.__TAURI__.core.invoke`, present only when
//! `withGlobalTauri` is on inside the Tauri webview — a plain browser tab has
//! no bridge, and [`bridge_available`] says so honestly).
//!
//! wasm32 only: the native `rlib` compile (tests) has no window to reach a
//! bridge through; the app advertises `can_open`/`can_save` false there.

use js_sys::{Array, Function, Object, Promise, Reflect, Uint8Array};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

/// The `.xlsx` the native open dialog resolved (see the desktop crate's
/// `OpenedXlsxFile`): the real filesystem `path`, the file `name` the host
/// reports on `Opened`, and the package `bytes`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenedXlsxFile {
    pub path: String,
    pub name: String,
    pub bytes: Vec<u8>,
}

/// Where the native save dialog wrote the package, and how much.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedXlsxFile {
    pub path: String,
    pub name: String,
    pub bytes_written: usize,
}

/// Whether the desktop shell's command bridge is reachable from this page
/// (`window.__TAURI__.core.invoke` exists). False in a plain browser tab.
#[must_use]
pub fn bridge_available() -> bool {
    tauri_core_invoke().is_ok()
}

/// Invoke the desktop shell's `open_xlsx_file` command: the native open
/// dialog plus the read. `Ok(None)` is a cancelled dialog (nothing opened);
/// `Err` is a real bridge/host failure (the bridge or command missing, an IO
/// error), never a silent no-op.
pub async fn pick_xlsx_to_open() -> Result<Option<OpenedXlsxFile>, String> {
    let picked = invoke("open_xlsx_file", &Object::new()).await?;
    if picked.is_null() || picked.is_undefined() {
        return Ok(None);
    }
    Ok(Some(OpenedXlsxFile {
        path: reflect_string(&picked, "path")?,
        name: reflect_string(&picked, "name")?,
        bytes: reflect_bytes(&picked, "bytes")?,
    }))
}

/// Invoke the desktop shell's `save_xlsx_file` command with the package bytes
/// the host produced: the native save dialog (seeded with `suggested_name`
/// in `suggested_directory` when known) plus the write. `Ok(None)` is a
/// cancelled dialog (nothing written); `Err` a real bridge/host failure.
pub async fn pick_path_and_save_xlsx(
    suggested_name: &str,
    suggested_directory: Option<&str>,
    bytes: &[u8],
) -> Result<Option<SavedXlsxFile>, String> {
    let args = Object::new();
    set_arg(&args, "suggested_name", &JsValue::from_str(suggested_name))?;
    set_arg(
        &args,
        "suggested_directory",
        &suggested_directory.map_or(JsValue::NULL, JsValue::from_str),
    )?;
    // A plain `Array` of numbers: Tauri's JSON IPC serializes that as the
    // `Vec<u8>` the command takes, whereas a `Uint8Array` would stringify as
    // an object keyed by index.
    let typed = Uint8Array::from(bytes);
    set_arg(&args, "bytes", &Array::from(typed.as_ref()))?;
    let saved = invoke("save_xlsx_file", &args).await?;
    if saved.is_null() || saved.is_undefined() {
        return Ok(None);
    }
    let bytes_written = Reflect::get(&saved, &JsValue::from_str("bytes_written"))
        .map_err(|error| {
            format!(
                "Tauri response field `bytes_written` read failed: {}",
                js_error_message(&error)
            )
        })?
        .as_f64()
        .ok_or_else(|| "Tauri response field `bytes_written` is not a number".to_string())?;
    Ok(Some(SavedXlsxFile {
        path: reflect_string(&saved, "path")?,
        name: reflect_string(&saved, "name")?,
        // The command reports the length it wrote as a `usize`; a JSON number
        // round-trips through `f64` exactly for any real file size.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        bytes_written: bytes_written as usize,
    }))
}

async fn invoke(command: &str, args: &Object) -> Result<JsValue, String> {
    let (core, invoke) = tauri_core_invoke()?;
    let promise_value = invoke
        .call2(&core, &JsValue::from_str(command), args)
        .map_err(|error| format!("Tauri invoke failed: {}", js_error_message(&error)))?;
    JsFuture::from(Promise::from(promise_value))
        .await
        .map_err(|error| format!("Tauri command rejected: {}", js_error_message(&error)))
}

fn set_arg(args: &Object, field: &str, value: &JsValue) -> Result<(), String> {
    Reflect::set(args, &JsValue::from_str(field), value)
        .map(|_| ())
        .map_err(|error| {
            format!(
                "Tauri argument `{field}` write failed: {}",
                js_error_message(&error)
            )
        })
}

fn tauri_core_invoke() -> Result<(JsValue, Function), String> {
    let window = web_sys::window().ok_or_else(|| "window unavailable".to_string())?;
    let tauri = Reflect::get(window.as_ref(), &JsValue::from_str("__TAURI__"))
        .map_err(|error| format!("window.__TAURI__ read failed: {}", js_error_message(&error)))?;
    if tauri.is_undefined() || tauri.is_null() {
        return Err("Tauri command bridge unavailable".to_string());
    }
    let core = Reflect::get(&tauri, &JsValue::from_str("core")).map_err(|error| {
        format!(
            "window.__TAURI__.core read failed: {}",
            js_error_message(&error)
        )
    })?;
    if core.is_undefined() || core.is_null() {
        return Err("Tauri core command bridge unavailable".to_string());
    }
    let invoke = Reflect::get(&core, &JsValue::from_str("invoke"))
        .map_err(|error| format!("Tauri invoke read failed: {}", js_error_message(&error)))?
        .dyn_into::<Function>()
        .map_err(|_| "window.__TAURI__.core.invoke is not a function".to_string())?;
    Ok((core, invoke))
}

fn reflect_string(value: &JsValue, field: &str) -> Result<String, String> {
    Reflect::get(value, &JsValue::from_str(field))
        .map_err(|error| {
            format!(
                "Tauri response field `{field}` read failed: {}",
                js_error_message(&error)
            )
        })?
        .as_string()
        .ok_or_else(|| format!("Tauri response field `{field}` is not a string"))
}

/// A `Vec<u8>` serialized by the command as a JSON number array: copy it
/// through a `Uint8Array` view (`new Uint8Array(array)` accepts any
/// array-like), so the frontend holds a plain byte buffer.
fn reflect_bytes(value: &JsValue, field: &str) -> Result<Vec<u8>, String> {
    let raw = Reflect::get(value, &JsValue::from_str(field)).map_err(|error| {
        format!(
            "Tauri response field `{field}` read failed: {}",
            js_error_message(&error)
        )
    })?;
    if raw.is_undefined() || raw.is_null() || !Array::is_array(&raw) {
        return Err(format!(
            "Tauri response field `{field}` is not a byte array"
        ));
    }
    Ok(Uint8Array::new(&raw).to_vec())
}

fn js_error_message(error: &JsValue) -> String {
    error
        .as_string()
        .or_else(|| {
            Reflect::get(error, &JsValue::from_str("message"))
                .ok()
                .and_then(|message| message.as_string())
        })
        .unwrap_or_else(|| format!("{error:?}"))
}

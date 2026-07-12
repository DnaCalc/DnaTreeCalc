//! Tauri command bridge for desktop-only file selection from the WASM UI.

use js_sys::{Function, Object, Promise, Reflect};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TauriVbaModuleSourceSelection {
    pub display_name: String,
    pub source_path: String,
    pub source_kind: String,
    pub source_text: String,
    pub load_status: String,
    pub admitted_udfs: Vec<String>,
    pub rejected_candidates: Vec<String>,
    pub diagnostics: Vec<String>,
}

pub fn tauri_command_bridge_available() -> bool {
    tauri_core_invoke().is_ok()
}

pub async fn select_vba_module_source() -> Result<Option<TauriVbaModuleSourceSelection>, String> {
    let (core, invoke) = tauri_core_invoke()?;
    let args = Object::new();
    let promise_value = invoke
        .call2(&core, &JsValue::from_str("select_vba_module_source"), &args)
        .map_err(|error| format!("Tauri invoke failed: {}", js_error_message(&error)))?;
    let selection = JsFuture::from(Promise::from(promise_value))
        .await
        .map_err(|error| format!("Tauri command rejected: {}", js_error_message(&error)))?;

    if selection.is_null() || selection.is_undefined() {
        return Ok(None);
    }

    Ok(Some(TauriVbaModuleSourceSelection {
        display_name: reflect_string(&selection, "displayName")?,
        source_path: reflect_string(&selection, "sourcePath")?,
        source_kind: reflect_string(&selection, "sourceKind")?,
        source_text: reflect_string(&selection, "sourceText")?,
        load_status: reflect_string(&selection, "loadStatus")?,
        admitted_udfs: reflect_string_array(&selection, "admittedUdfs")?,
        rejected_candidates: reflect_string_array(&selection, "rejectedCandidates")?,
        diagnostics: reflect_string_array(&selection, "diagnostics")?,
    }))
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

fn reflect_string_array(value: &JsValue, field: &str) -> Result<Vec<String>, String> {
    let raw = Reflect::get(value, &JsValue::from_str(field)).map_err(|error| {
        format!(
            "Tauri response field `{field}` read failed: {}",
            js_error_message(&error)
        )
    })?;
    if raw.is_undefined() || raw.is_null() {
        return Ok(Vec::new());
    }
    let array = js_sys::Array::from(&raw);
    let mut values = Vec::with_capacity(array.length() as usize);
    for item in array.iter() {
        if let Some(text) = item.as_string() {
            values.push(text);
        }
    }
    Ok(values)
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

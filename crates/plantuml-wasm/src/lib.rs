use wasm_bindgen::prelude::*;

/// Routes `log` output to the browser console. Safe to call more than once.
#[wasm_bindgen]
pub fn init_logging() {
    let _ = console_log::init_with_level(log::Level::Info);
}

/// Renders the first diagram in `source` to SVG.
#[wasm_bindgen]
pub fn render(source: &str) -> Result<String, JsError> {
    plantuml_core::render(source).map_err(|e| JsError::new(&e.to_string()))
}

use wasm_bindgen::prelude::*;

/// The crate version, for feature-detection on the JS side.
#[wasm_bindgen]
pub fn version() -> String {
    super::version()
}

/// Lex `source` and return its tokens as JSON (an array of `Token` objects).
#[wasm_bindgen]
pub fn tokenize_json(source: &str) -> Result<String, JsValue> {
    super::tokenize_json(source).map_err(|msg| js_sys::Error::new(&msg).into())
}

/// Parse `source` and return its typed AST as JSON.
#[wasm_bindgen]
pub fn parse_json(source: &str) -> Result<String, JsValue> {
    super::parse_json(source).map_err(|msg| js_sys::Error::new(&msg).into())
}

/// Typecheck `source` and return a `CheckReport` as JSON.
#[wasm_bindgen]
pub fn check_json(source: &str) -> Result<String, JsValue> {
    super::check_json(source).map_err(|msg| js_sys::Error::new(&msg).into())
}

//! WASM bindings for the Sakko compiler.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

pub mod dto;

#[cfg(target_arch = "wasm32")]
pub mod wasm;

use serde::Serialize;

/// The crate version, for feature-detection on the JS side.
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Lex `source` and return its tokens as JSON (an array of `Token` objects).
pub fn tokenize_json(source: &str) -> Result<String, String> {
    let tokens = sakko::syntax::lexer::tokenize(source).map_err(|e| e.to_string())?;
    serialize(&tokens)
}

/// Parse `source` and return its typed AST as JSON.
pub fn parse_json(source: &str) -> Result<String, String> {
    let ast = sakko::parse_sakko(source).map_err(|e| e.to_string())?;
    serialize(&ast)
}

/// Typecheck `source` and return a [`dto::CheckReport`] as JSON. A
/// document-level parse failure is returned as an error carrying a
/// [`dto::ErrorDto`] JSON payload.
pub fn check_json(source: &str) -> Result<String, String> {
    match sakko::typecheck::check_source(source) {
        Ok(report) => serialize(&dto::CheckReport::from(&report)),
        Err(e) => Err(serialize(&dto::ErrorDto::from(&e)).unwrap_or_else(|e2| e2)),
    }
}

fn serialize<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string(value).map_err(|e| e.to_string())
}

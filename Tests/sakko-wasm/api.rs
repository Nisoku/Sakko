use serde_json::Value;

#[test]
fn version_matches_package() {
    assert_eq!(sakko_wasm::version(), env!("CARGO_PKG_VERSION"));
}

#[test]
fn tokenize_json_produces_token_array() {
    let json = sakko_wasm::tokenize_json("<todo { div: \"x\" }>").expect("tokens should lex");
    let value: Value = serde_json::from_str(&json).expect("valid JSON");
    let tokens = value.as_array().expect("token list is an array");
    assert!(!tokens.is_empty());
    let first = tokens[0].as_object().expect("token is an object");
    assert_eq!(first["kind"], "LT");
    assert_eq!(first["value"], "<");
}

#[test]
fn tokenize_json_reports_lex_error() {
    assert!(sakko_wasm::tokenize_json("\"").is_err());
}

#[test]
fn parse_json_produces_typed_ast() {
    let json = sakko_wasm::parse_json("<app { div { span: \"y\" } }>").expect("should parse");
    let value: Value = serde_json::from_str(&json).expect("valid JSON");
    assert_eq!(value["name"], "app");
    assert_eq!(value["children"][0]["type"], "element");
    assert_eq!(value["children"][0]["name"], "div");
    assert_eq!(value["children"][0]["children"][0]["type"], "inline");
}

#[test]
fn parse_json_reports_document_error() {
    assert!(sakko_wasm::parse_json("<unclosed {").is_err());
}

#[test]
fn check_json_clean_source_reports_ok() {
    let json = sakko_wasm::check_json("<counter { @state { count = 0 } text: \"{count}\" }>")
        .expect("should check");
    let value: Value = serde_json::from_str(&json).expect("valid JSON");
    assert_eq!(value["ok"], true);
    assert_eq!(value["diagnostics"].as_array().map_or(0, Vec::len), 0);
}

#[test]
fn check_json_captures_diagnostics() {
    let json = sakko_wasm::check_json("<app { div @class={5}: \"x\" }>").expect("should check");
    let value: Value = serde_json::from_str(&json).expect("valid JSON");
    assert_eq!(value["ok"], false);
    let diags = value["diagnostics"]
        .as_array()
        .expect("diagnostics is an array");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0]["code"], "SKT015");
    assert_eq!(diags[0]["severity"], "error");
    assert!(
        diags[0]["rendered"]
            .as_str()
            .expect("rendered is a string")
            .contains("SKT015")
    );
}

#[test]
fn check_json_documents_js_escapes() {
    let json = sakko_wasm::check_json(
        "<app { @state { screen = js { return 1 } as number } text: \"{screen}\" }>",
    )
    .expect("should check");
    let value: Value = serde_json::from_str(&json).expect("valid JSON");
    assert_eq!(
        value["jsEscapes"].as_array().map_or(0, Vec::len),
        1,
        "js {{}} block should be recorded as an escape"
    );
}

#[test]
fn check_json_reports_document_error_as_error_dto() {
    let err = sakko_wasm::check_json("<unclosed {").expect_err("unclosed doc is a parse error");
    let value: Value = serde_json::from_str(&err).expect("error payload is JSON");
    assert!(value["message"].as_str().is_some());
}

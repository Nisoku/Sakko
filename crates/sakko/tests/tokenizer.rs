use sakko::token::TokenKind as K;
use sakko::token::{Token, TokenKind};
use sakko::tokenize;

fn sig(t: &Token) -> (TokenKind, String, u32, u32) {
    (t.kind, t.value.to_string(), t.line, t.col)
}

fn kind_of(t: &Token) -> TokenKind {
    t.kind
}

#[test]
fn tokenizes_basic_elements() {
    let tokens = tokenize("card { text: Hello }").unwrap();
    let actual: Vec<_> = tokens.iter().map(sig).collect();
    assert_eq!(
        actual,
        vec![
            (K::Ident, "card".into(), 1, 1),
            (K::Lbrace, "{".into(), 1, 6),
            (K::Ident, "text".into(), 1, 8),
            (K::Colon, ":".into(), 1, 12),
            (K::Ident, "Hello".into(), 1, 14),
            (K::Rbrace, "}".into(), 1, 20),
        ]
    );
}

#[test]
fn tokenizes_root_block() {
    let tokens = tokenize("<page { card { text: \"Hello World\" } }>").unwrap();
    assert_eq!(tokens[0].kind, K::Lt);
    assert_eq!((&tokens[1].kind, &*tokens[1].value), (&K::Ident, "page"));
    assert_eq!(
        (&tokens[7].kind, &*tokens[7].value),
        (&K::String, "Hello World")
    );
    assert_eq!(tokens[tokens.len() - 1].kind, K::Gt);
}

#[test]
fn tokenizes_modifiers() {
    let tokens = tokenize("button(accent large): Save").unwrap();
    assert_eq!((&tokens[0].kind, &*tokens[0].value), (&K::Ident, "button"));
    assert_eq!(tokens[1].kind, K::Lparen);
    assert_eq!((&tokens[2].kind, &*tokens[2].value), (&K::Ident, "accent"));
    assert_eq!((&tokens[3].kind, &*tokens[3].value), (&K::Ident, "large"));
    assert_eq!(tokens[4].kind, K::Rparen);
}

#[test]
fn tokenizes_key_value_modifiers() {
    let tokens = tokenize("grid(cols 3 gap medium): [ item ]").unwrap();
    assert_eq!((&tokens[2].kind, &*tokens[2].value), (&K::Ident, "cols"));
    assert_eq!((&tokens[3].kind, &*tokens[3].value), (&K::Ident, "3"));
    assert_eq!((&tokens[4].kind, &*tokens[4].value), (&K::Ident, "gap"));
    assert_eq!((&tokens[5].kind, &*tokens[5].value), (&K::Ident, "medium"));
}

#[test]
fn tokenizes_lists_and_semicolons() {
    let tokens = tokenize("controls { button: play; button: pause; button: stop }").unwrap();
    assert_eq!(tokens.iter().filter(|t| t.kind == K::Semi).count(), 2);
    assert_eq!(
        (&tokens[0].kind, &*tokens[0].value),
        (&K::Ident, "controls")
    );
}

#[test]
fn handles_comments() {
    let input = "\n      // This is a comment\n      card { \n        // Another comment\n        text: Hello \n      }\n    ";
    let tokens = tokenize(input).unwrap();
    let idents: Vec<&str> = tokens
        .iter()
        .filter(|t| t.kind == K::Ident)
        .map(|t| &*t.value)
        .collect();
    assert_eq!(idents, vec!["card", "text", "Hello"]);
}

#[test]
fn handles_identifiers_with_hyphens_and_underscores() {
    let tokens = tokenize("custom-button_name: \"Test value\"").unwrap();
    assert_eq!(
        (&tokens[0].kind, &*tokens[0].value),
        (&K::Ident, "custom-button_name")
    );
    assert_eq!(
        (&tokens[2].kind, &*tokens[2].value),
        (&K::String, "Test value")
    );
}

#[test]
fn throws_error_on_unterminated_string() {
    let err = tokenize("text: \"unclosed string").unwrap_err();
    assert!(
        err.message.contains("Unterminated string"),
        "{}",
        err.message
    );
}

#[test]
fn handles_at_token() {
    let tokens = tokenize("@state").unwrap();
    assert_eq!(tokens[0].kind, K::At);
    assert_eq!(tokens[1].kind, K::Ident);
}

#[test]
fn preserves_urls_inside_strings() {
    let tokens = tokenize("image: \"https://example.com/photo.jpg\"").unwrap();
    let str = tokens.iter().find(|t| t.kind == K::String).unwrap();
    assert_eq!(&*str.value, "https://example.com/photo.jpg");
}

#[test]
fn does_not_treat_slashes_inside_strings_as_comments() {
    let tokens = tokenize("coverart: \"https://placehold.co/80\"").unwrap();
    let str = tokens.iter().find(|t| t.kind == K::String).unwrap();
    assert_eq!(&*str.value, "https://placehold.co/80");
}

#[test]
fn tracks_line_numbers() {
    let tokens = tokenize("a\nb\nc").unwrap();
    assert_eq!(tokens[0].line, 1);
    assert_eq!(tokens[1].line, 2);
    assert_eq!(tokens[2].line, 3);
}

#[test]
fn handles_backslash_escape_sequences_in_strings() {
    // \n should now be decoded to an actual newline character
    let tokens = tokenize("text: \"hello\\nworld\"").unwrap();
    let str = tokens.iter().find(|t| t.kind == K::String).unwrap();
    assert_eq!(&*str.value, "hello\nworld");
}

#[test]
fn handles_empty_strings() {
    let tokens = tokenize("input: \"\"").unwrap();
    let str = tokens.iter().find(|t| t.kind == K::String).unwrap();
    assert_eq!(&*str.value, "");
}

#[test]
fn throws_on_unterminated_string() {
    let err = tokenize("text: \"hello").unwrap_err();
    assert!(
        err.message.contains("Unterminated string"),
        "{}",
        err.message
    );
}

#[test]
fn tokenizes_angle_brackets() {
    let tokens = tokenize("<page>").unwrap();
    assert_eq!((&tokens[0].kind, &*tokens[0].value), (&K::Lt, "<"));
    assert_eq!((&tokens[2].kind, &*tokens[2].value), (&K::Gt, ">"));
}

#[test]
fn tokenizes_commas() {
    let tokens = tokenize("a, b, c").unwrap();
    let commas = tokens.iter().filter(|t| t.kind == K::Comma).count();
    assert_eq!(commas, 2);
}

#[test]
fn handles_multiline_strings_with_comments() {
    let tokens = tokenize("// comment\ntext: \"value\"").unwrap();
    let ident = tokens
        .iter()
        .find(|t| t.kind == K::Ident && &*t.value == "text")
        .unwrap();
    assert_eq!(ident.line, 2);
}

#[test]
fn handles_string_with_colon_inside() {
    let tokens = tokenize("text: \"key: value\"").unwrap();
    let str = tokens.iter().find(|t| t.kind == K::String).unwrap();
    assert_eq!(&*str.value, "key: value");
}

#[test]
fn tokenizes_interpolation_inside_strings() {
    let tokens = tokenize("text: \"{ hello }\"").unwrap();
    assert!(tokens.iter().any(|t| t.kind == K::InterpStart));
    let expr = tokens.iter().find(|t| t.kind == K::Expr).unwrap();
    assert_eq!(&*expr.value, "hello");
    assert!(tokens.iter().any(|t| matches!(kind_of(t), K::InterpEnd)));
}

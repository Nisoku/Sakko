use sakko::syntax::token::TokenKind as K;
use sakko::tokenize;

#[test]
fn tokenizes_at_token() {
    let tokens = tokenize("@state").unwrap();
    let sigs: Vec<_> = tokens
        .iter()
        .map(|t| (t.kind, t.value.to_string(), t.line, t.col))
        .collect();
    assert_eq!(
        sigs,
        vec![(K::At, "@".into(), 1, 1), (K::Ident, "state".into(), 1, 2),]
    );
}

#[test]
fn tokenizes_equals_token() {
    let tokens = tokenize("count = 0").unwrap();
    let sigs: Vec<_> = tokens
        .iter()
        .map(|t| (t.kind, t.value.to_string(), t.line, t.col))
        .collect();
    assert_eq!(
        sigs,
        vec![
            (K::Ident, "count".into(), 1, 1),
            (K::Equals, "=".into(), 1, 7),
            (K::Ident, "0".into(), 1, 9),
        ]
    );
}

#[test]
fn tokenizes_interpolation_in_string() {
    let tokens = tokenize("text: \"Count: {count}\"").unwrap();
    let types: Vec<K> = tokens.iter().map(|t| t.kind).collect();
    assert!(types.contains(&K::InterpStart));
    assert!(types.contains(&K::Expr));
    assert!(types.contains(&K::InterpEnd));

    let expr = tokens.iter().find(|t| t.kind == K::Expr).unwrap();
    assert_eq!(&*expr.value, "count");
}

#[test]
fn tokenizes_multiple_interpolations() {
    let tokens = tokenize("text: \"Hello {name}, you have {count} items\"").unwrap();
    let exprs: Vec<_> = tokens.iter().filter(|t| t.kind == K::Expr).collect();
    assert_eq!(exprs.len(), 2);
    assert_eq!(&*exprs[0].value, "name");
    assert_eq!(&*exprs[1].value, "count");
}

#[test]
fn tokenizes_nested_braces_in_interpolation() {
    let tokens = tokenize("text: \"{items.map(x => x.name)}\"").unwrap();
    let expr = tokens.iter().find(|t| t.kind == K::Expr).unwrap();
    assert_eq!(&*expr.value, "items.map(x => x.name)");
}

#[test]
fn handles_standalone_at() {
    let tokens = tokenize("@").unwrap();
    assert_eq!(tokens[0].kind, K::At);
}

// Edge cases

#[test]
fn unterminated_interpolated_string_throws() {
    // Missing closing quote after the interpolation
    assert!(tokenize("\"Hello {name").is_err());
}

#[test]
fn escaped_brace_does_not_produce_interpolation_tokens() {
    // \{ is an unknown escape -> stored as \{ in the text, no INTERP_START emitted
    let tokens = tokenize(r#""Hello \{not interpolation\}""#).unwrap();
    let types: Vec<K> = tokens.iter().map(|t| t.kind).collect();
    assert!(!types.contains(&K::InterpStart));
    assert!(!types.contains(&K::Expr));
    assert!(!types.contains(&K::InterpEnd));
    let str = tokens.iter().find(|t| t.kind == K::String).unwrap();
    // The value should contain the literal brace characters (backslash preserved)
    assert!(str.value.contains('{'));
}

#[test]
fn empty_interpolation_produces_empty_expr_token() {
    let tokens = tokenize(r#""{}""#).unwrap();
    let expr = tokens.iter().find(|t| t.kind == K::Expr).unwrap();
    assert_eq!(&*expr.value, "");
}

#[test]
fn adjacent_interpolations_produce_two_expr_tokens() {
    let tokens = tokenize(r#""{a}{b}""#).unwrap();
    let exprs: Vec<_> = tokens.iter().filter(|t| t.kind == K::Expr).collect();
    assert_eq!(exprs.len(), 2);
    assert_eq!(&*exprs[0].value, "a");
    assert_eq!(&*exprs[1].value, "b");
}

// backtick strings
#[test]
fn tokenizes_backtick_string_as_backtick_string() {
    let tokens = tokenize("`hello world`").unwrap();
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].kind, K::BacktickString);
    assert_eq!(&*tokens[0].value, "hello world");
}

#[test]
fn backtick_string_with_dollar_braces_has_no_interpolation() {
    let tokens = tokenize("`Count: ${count}`").unwrap();
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].kind, K::BacktickString);
    assert_eq!(&*tokens[0].value, "Count: ${count}");
}

#[test]
fn backtick_string_with_nested_template_expressions() {
    let tokens = tokenize("`${a} + ${b} = ${a + b}`").unwrap();
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].kind, K::BacktickString);
}

#[test]
fn mixed_double_quoted_and_backtick_strings() {
    let tokens = tokenize(r#""normal" `template` "also normal""#).unwrap();
    let types: Vec<K> = tokens.iter().map(|t| t.kind).collect();
    assert_eq!(types, vec![K::String, K::BacktickString, K::String]);
}

#[test]
fn backtick_inside_effect_body_preserves_template_literal() {
    let tokens = tokenize("console.log(`Count: ${count}`)").unwrap();
    let backtick = tokens.iter().find(|t| t.kind == K::BacktickString).unwrap();
    assert_eq!(&*backtick.value, "Count: ${count}");
}

#[test]
fn unterminated_backtick_string_throws() {
    assert!(tokenize("`unclosed").is_err());
}

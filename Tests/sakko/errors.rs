use sakko::{AstNode, InlineValue, TokenKind as K, parse_sakko, tokenize};

#[test]
fn tokenizer_throws_on_unterminated_string() {
    let err = tokenize("text: \"hello").unwrap_err();
    assert!(
        err.message.contains("Unterminated string"),
        "{}",
        err.message
    );
}

#[test]
fn tokenizer_throws_on_unterminated_string_at_end_of_input() {
    let err = tokenize("\"").unwrap_err();
    assert!(
        err.message.contains("Unterminated string"),
        "{}",
        err.message
    );
}

#[test]
fn tokenizer_throws_on_unterminated_string_with_content_after() {
    let err = tokenize("\"hello world").unwrap_err();
    assert!(
        err.message.contains("Unterminated string"),
        "{}",
        err.message
    );
}

#[test]
fn tokenizer_throws_on_unexpected_character_hash() {
    let err = tokenize("#heading").unwrap_err();
    assert!(
        err.message.contains("Unexpected character: #"),
        "{}",
        err.message
    );
}

#[test]
fn tokenizer_throws_on_unexpected_character_dollar() {
    let err = tokenize("price: $5").unwrap_err();
    assert!(
        err.message.contains("Unexpected character: $"),
        "{}",
        err.message
    );
}

#[test]
fn tokenizer_bang_is_valid_operator() {
    let tokens = tokenize("!important").unwrap();
    assert_eq!((&tokens[0].kind, &*tokens[0].value), (&K::Bang, "!"));
    assert_eq!(
        (&tokens[1].kind, &*tokens[1].value),
        (&K::Ident, "important")
    );
}

#[test]
fn tokenizer_string_with_only_whitespace_content() {
    let tokens = tokenize("text: \"   \"").unwrap();
    assert_eq!((&tokens[2].kind, &*tokens[2].value), (&K::String, "   "));
}

#[test]
fn tokenizer_bracket_characters_inside_string_as_interpolation() {
    let tokens = tokenize("text: \"{[(<>)]}\"").unwrap();
    assert_eq!((&tokens[2].kind, &*tokens[2].value), (&K::InterpStart, "{"));
    assert_eq!((&tokens[3].kind, &*tokens[3].value), (&K::Expr, "[(<>)]"));
}

#[test]
fn tokenizer_single_character_identifiers() {
    let tokens = tokenize("a").unwrap();
    assert_eq!((&tokens[0].kind, &*tokens[0].value), (&K::Ident, "a"));
}

#[test]
fn tokenizer_identifiers_with_hyphens_and_underscores() {
    let tokens = tokenize("icon-btn my_var data-id").unwrap();
    assert_eq!(
        (&tokens[0].kind, &*tokens[0].value),
        (&K::Ident, "icon-btn")
    );
    assert_eq!((&tokens[1].kind, &*tokens[1].value), (&K::Ident, "my_var"));
    assert_eq!((&tokens[2].kind, &*tokens[2].value), (&K::Ident, "data-id"));
}

#[test]
fn tokenizer_consecutive_strings() {
    let tokens = tokenize("\"hello\" \"world\"").unwrap();
    assert_eq!((&tokens[0].kind, &*tokens[0].value), (&K::String, "hello"));
    assert_eq!((&tokens[1].kind, &*tokens[1].value), (&K::String, "world"));
}

#[test]
fn tokenizer_semicolons_as_tokens() {
    let tokens = tokenize("a; b; c").unwrap();
    assert_eq!(tokens.iter().filter(|t| t.kind == K::Semi).count(), 2);
}

#[test]
fn tokenizer_strips_comments_before_strings_on_same_line() {
    let tokens = tokenize("text: Hello // \"this is not a string\"").unwrap();
    assert_eq!(tokens.iter().filter(|t| t.kind == K::String).count(), 0);
}

#[test]
fn tokenizer_tab_characters_as_whitespace() {
    let tokens = tokenize("a\tb\tc").unwrap();
    assert_eq!((&tokens[0].kind, &*tokens[0].value), (&K::Ident, "a"));
    assert_eq!((&tokens[1].kind, &*tokens[1].value), (&K::Ident, "b"));
    assert_eq!((&tokens[2].kind, &*tokens[2].value), (&K::Ident, "c"));
}

// Parser - Error handling

#[test]
fn parser_throws_on_completely_empty_input() {
    assert!(parse_sakko("").is_err());
}

#[test]
fn parser_throws_on_whitespace_only_input() {
    assert!(parse_sakko("   \n\n   ").is_err());
}

#[test]
fn parser_handles_comment_only_input_gracefully() {
    let result = parse_sakko("// just a comment").unwrap();
    // TS checks result.type === 'root'; RootNode is the root type itself.
    assert_eq!(&*result.name, "__sakko_wrapper__");
}

#[test]
fn parser_auto_wraps_input_missing_opening_lt() {
    let result = parse_sakko("page { text: Hello }").unwrap();
    assert_eq!(&*result.name, "__sakko_wrapper__");
}

#[test]
fn parser_throws_when_missing_closing_gt() {
    let err = parse_sakko("<page { text: Hello }").unwrap_err();
    assert!(err.to_string().contains("Expected '>'"), "{}", err);
}

#[test]
fn parser_throws_when_missing_opening_lbrace() {
    let err = parse_sakko("<page text: Hello }>").unwrap_err();
    assert!(err.to_string().contains("Expected '{'"), "{}", err);
}

#[test]
fn parser_throws_when_missing_closing_rbrace() {
    assert!(parse_sakko("<page { text: Hello >").is_err());
}

#[test]
fn parser_throws_when_root_name_is_missing() {
    let err = parse_sakko("< { text: Hello }>").unwrap_err();
    assert!(
        err.to_string().contains("Expected identifier after '<'"),
        "{}",
        err
    );
}

#[test]
fn parser_throws_on_non_identifier_after_lt() {
    let err = parse_sakko("<{ text: Hello }>").unwrap_err();
    assert!(
        err.to_string().contains("Expected identifier after '<'"),
        "{}",
        err
    );
}

#[test]
fn parser_throws_on_value_missing_after_colon() {
    assert!(parse_sakko("<page { text: }>").is_err());
}

#[test]
fn parser_throws_on_colon_followed_by_closing_brace() {
    assert!(parse_sakko("<page { name: }>").is_err());
}

#[test]
fn parser_throws_on_nested_unclosed_block() {
    assert!(parse_sakko("<page { card { text: Hello }>").is_err());
}

#[test]
fn parser_throws_on_deeply_nested_unclosed_block() {
    assert!(parse_sakko("<page { a { b { c: d }>").is_err());
}

#[test]
fn parser_throws_on_unclosed_modifier_parenthesis() {
    assert!(parse_sakko("<page { button(accent : Click }>").is_err());
}

#[test]
fn parser_throws_on_empty_modifiers_with_unclosed_paren() {
    assert!(parse_sakko("<page { button( }>").is_err());
}

#[test]
fn parser_throws_on_unclosed_list_bracket() {
    assert!(parse_sakko("<page { row: [a: 1, b: 2 }>").is_err());
}

#[test]
fn parser_throws_on_list_missing_comma_between_items() {
    let err = parse_sakko("<page { row: [a: 1 b: 2] }>").unwrap_err();
    assert!(
        err.to_string().contains("Expected \",\" or \"]\""),
        "{}",
        err
    );
}

#[test]
fn parser_throws_on_element_name_that_is_not_an_identifier() {
    let err = parse_sakko("<page { : value }>").unwrap_err();
    assert!(err.to_string().contains("Expected identifier"), "{}", err);
}

#[test]
fn parser_throws_on_non_identifier_inside_modifiers() {
    let err = parse_sakko("<page { button(: value): Click }>").unwrap_err();
    assert!(
        err.to_string().contains("Expected identifier in modifiers"),
        "{}",
        err
    );
}

#[test]
fn parser_parses_void_elements_no_body_colon_or_list() {
    let ast = parse_sakko("<page { card button: Click }>").unwrap();
    assert_eq!(ast.children.len(), 2);
    match &ast.children[0] {
        AstNode::Inline(n) => {
            assert_eq!(&*n.name, "card");
            assert!(n.modifiers.is_empty());
            assert_eq!(n.value, InlineValue::Plain("".into()));
        }
        other => panic!("expected inline node, got {:?}", other),
    }
    match &ast.children[1] {
        AstNode::Inline(n) => {
            assert_eq!(&*n.name, "button");
            assert_eq!(n.value, InlineValue::Plain("Click".into()));
        }
        other => panic!("expected inline node, got {:?}", other),
    }
}

#[test]
fn parser_throws_on_just_angle_brackets() {
    assert!(parse_sakko("<>").is_err());
}

#[test]
fn parser_throws_on_just_lt_with_name() {
    assert!(parse_sakko("<page").is_err());
}

#[test]
fn parser_duplicate_closing_gt_is_fine() {
    let ast = parse_sakko("<page { }>").unwrap();
    assert_eq!(&*ast.name, "page");
}

// Malformed but parseable edge cases

#[test]
fn parses_empty_block_element() {
    let ast = parse_sakko("<page { card {} }>").unwrap();
    assert_eq!(ast.children.len(), 1);
    match &ast.children[0] {
        AstNode::Element(n) => assert!(n.children.is_empty()),
        other => panic!("expected element node, got {:?}", other),
    }
}

#[test]
fn parses_element_with_empty_modifiers() {
    let ast = parse_sakko("<page { button(): Click }>").unwrap();
    match &ast.children[0] {
        AstNode::Inline(n) => {
            assert!(n.modifiers.is_empty());
            assert_eq!(n.value, InlineValue::Plain("Click".into()));
        }
        other => panic!("expected inline node, got {:?}", other),
    }
}

#[test]
fn parses_empty_list() {
    let ast = parse_sakko("<page { row: [] }>").unwrap();
    match &ast.children[0] {
        AstNode::Element(n) => match &n.children[0] {
            AstNode::List(list) => assert!(list.items.is_empty()),
            other => panic!("expected list, got {:?}", other),
        },
        other => panic!("expected element node, got {:?}", other),
    }
}

#[test]
fn parses_list_with_trailing_comma() {
    let ast = parse_sakko("<page { row: [a: 1, b: 2,] }>").unwrap();
    match &ast.children[0] {
        AstNode::Element(n) => match &n.children[0] {
            AstNode::List(list) => assert_eq!(list.items.len(), 2),
            other => panic!("expected list, got {:?}", other),
        },
        other => panic!("expected element node, got {:?}", other),
    }
}

#[test]
fn parses_root_with_no_children() {
    let ast = parse_sakko("<page {}>").unwrap();
    assert_eq!(&*ast.name, "page");
    assert!(ast.children.is_empty());
}

#[test]
fn parses_trailing_semicolons() {
    let ast = parse_sakko("<page { text: A; text: B; }>").unwrap();
    assert_eq!(ast.children.len(), 2);
}

#[test]
fn throws_on_multiple_semicolons_between_items() {
    assert!(parse_sakko("<page { text: A;; text: B }>").is_err());
}

#[test]
fn list_with_trailing_comma_parses_successfully() {
    let ast = parse_sakko("<page { row: [a: 1,] }>").unwrap();
    assert!(matches!(&ast.children[0], AstNode::Element(_)));
}

#[test]
fn parses_single_inline_child() {
    let ast = parse_sakko("<page { text: Hello }>").unwrap();
    assert_eq!(ast.children.len(), 1);
    assert!(matches!(&ast.children[0], AstNode::Inline(_)));
}

#[test]
fn parses_string_value_with_spaces() {
    let ast = parse_sakko("<page { text: \"Hello World\" }>").unwrap();
    match &ast.children[0] {
        AstNode::Inline(n) => assert_eq!(n.value, InlineValue::Plain("Hello World".into())),
        other => panic!("expected inline node, got {:?}", other),
    }
}

#[test]
fn parses_bare_identifier_as_value() {
    let ast = parse_sakko("<page { icon: play }>").unwrap();
    match &ast.children[0] {
        AstNode::Inline(n) => assert_eq!(n.value, InlineValue::Plain("play".into())),
        other => panic!("expected inline node, got {:?}", other),
    }
}

#[test]
fn parses_known_key_at_end_of_modifiers_as_flag() {
    let ast = parse_sakko("<page { row(gap): [] }>").unwrap();
    match &ast.children[0] {
        AstNode::Element(n) => {
            assert_eq!(n.modifiers.len(), 1);
            assert!(matches!(&n.modifiers[0], sakko::Modifier::Flag { value } if value == "gap"));
        }
        other => panic!("expected element node, got {:?}", other),
    }
}

#[test]
fn parses_element_with_no_children() {
    let ast = parse_sakko("<page { card { } }>").unwrap();
    match &ast.children[0] {
        AstNode::Element(n) => assert!(n.children.is_empty()),
        other => panic!("expected element node, got {:?}", other),
    }
}

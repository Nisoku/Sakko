use sakko::{
    AstNode, AtcodeBody, AtcodeDeclaration, BlockSnippet, ExprSnippet, InlineValue,
    InterpolatedText, InterpolatedTextPart, Modifier, parse_sakko,
};

fn expect_inline<'a>(node: &'a AstNode<'a>) -> (&'a str, &'a [Modifier<'a>], &'a InlineValue<'a>) {
    match node {
        AstNode::Inline(n) => (&n.name, &n.modifiers, &n.value),
        other => panic!("expected inline node, got {:?}", other),
    }
}

fn interpolated(parts: Vec<InterpolatedTextPart>) -> InlineValue {
    InlineValue::Interpolated(InterpolatedText::new(parts))
}

#[test]
fn parses_state_declaration() {
    let input = "<counter {\n      @state {\n        count = 0\n        step = 1\n      }\n      text: \"Count\"\n    }>";
    let ast = parse_sakko(input).unwrap();

    assert_eq!(ast.declarations.len(), 1);
    match &ast.declarations[0] {
        AtcodeDeclaration::State { declarations, .. } => {
            assert_eq!(declarations.len(), 2);
            assert_eq!(declarations[0].name.as_ref(), "count");
            assert_eq!(declarations[0].value.raw.as_ref(), "0");
            assert!(declarations[0].value.parsed.is_some());
            assert_eq!(declarations[1].name.as_ref(), "step");
            assert_eq!(declarations[1].value.raw.as_ref(), "1");
            assert!(declarations[1].value.parsed.is_some());
        }
        other => panic!("expected state declaration, got {:?}", other),
    }
}

#[test]
fn parses_effect_declaration() {
    let input = "<app {\n      @state {\n        count = 0\n      }\n      \n      @effect {\n        console.log(\"Count:\", count)\n      }\n      \n      text: \"App\"\n    }>";

    let ast = parse_sakko(input).unwrap();

    assert_eq!(ast.declarations.len(), 2);
    match &ast.declarations[1] {
        AtcodeDeclaration::Effect { body, .. } => {
            assert_eq!(body.raw.as_ref(), "console.log(\"Count:\",count)");
            assert!(body.parsed.is_some());
        }
        other => panic!("expected effect declaration, got {:?}", other),
    }
}

#[test]
fn parses_derived_declaration() {
    let input = "<app {\n      @state {\n        items = []\n      }\n      \n      @derived {\n        count = items.length\n      }\n      \n      text: \"App\"\n    }>";

    let ast = parse_sakko(input).unwrap();

    assert_eq!(ast.declarations.len(), 2);
    match &ast.declarations[1] {
        AtcodeDeclaration::Derived { declarations, .. } => {
            assert_eq!(declarations.len(), 1);
            assert_eq!(declarations[0].name.as_ref(), "count");
            assert_eq!(declarations[0].expr.raw.as_ref(), "items.length");
            assert!(declarations[0].expr.parsed.is_some());
        }
        other => panic!("expected derived declaration, got {:?}", other),
    }
}

#[test]
fn derived_rejects_malformed_declarations() {
    // A bare identifier or a missing `=` must be diagnosed rather than
    // silently ending the declaration list.
    for (body, needle) in [
        ("count", "Expected variable declaration"),
        ("= 5", "Expected variable declaration"),
        ("const", "Expected identifier after 'const'"),
    ] {
        let input = format!("<app {{\n @derived {{\n  {body}\n }}\n text: \"A\"\n}}>");
        let err = parse_sakko(&input).expect_err(&format!("{body} should be rejected"));
        assert!(
            err.message.contains(needle),
            "unexpected message for {body:?}: {}",
            err.message
        );
    }

    // Well-formed forms keep working: empty, bare (unbraced), multi, and
    // comma-separated.
    for input in [
        "<app {\n @derived {\n }\n text: \"A\"\n}>",
        "<app {\n @derived count = a.length\n text: \"A\"\n}>",
        "<app {\n @derived {\n  count = a.length\n }\n text: \"A\"\n}>",
        "<app {\n @derived {\n  count = a.length,\n  other = b.length,\n }\n text: \"A\"\n}>",
    ] {
        parse_sakko(input).unwrap_or_else(|e| panic!("{input:?} should parse: {e}"));
    }
}

#[test]
fn parses_on_event_modifier() {
    let input = "<app {\n      @state {\n        count = 0\n      }\n      \n      button @on:click {\n        count++\n      }: \"Increment\"\n    }>";

    let ast = parse_sakko(input).unwrap();
    let (_, modifiers, _) = expect_inline(&ast.children[0]);

    assert!(modifiers.contains(&Modifier::Event {
        event: "click".into(),
        handler: BlockSnippet::parse("count++".into()),
    }));
}

#[test]
fn parses_bind_modifier() {
    let input = "<app {\n      input @bind=\"username\": \"\"\n    }>";

    let ast = parse_sakko(input).unwrap();
    let (_, modifiers, _) = expect_inline(&ast.children[0]);

    assert!(modifiers.contains(&Modifier::Atcode {
        name: "bind".into(),
        body: AtcodeBody::Text("username".into()),
    }));
}

#[test]
fn parses_interpolated_string() {
    let input = "<app {\n      @state {\n        name = \"Alice\"\n      }\n      \n      text: \"Hello, {name}!\"\n    }>";

    let ast = parse_sakko(input).unwrap();
    let (_, _, value) = expect_inline(&ast.children[0]);

    assert_eq!(
        value,
        &interpolated(vec![
            InterpolatedTextPart::Text {
                value: "Hello, ".into()
            },
            InterpolatedTextPart::Expr {
                value: ExprSnippet::parse("name".into())
            },
            InterpolatedTextPart::Text { value: "!".into() },
        ])
    );
}

#[test]
fn parses_mixed_text_and_interpolation() {
    let input = "<app {\n      @state {\n        a = 1\n        b = 2\n      }\n      \n      text: \"{a} + {b} = {a + b}\"\n    }>";

    let ast = parse_sakko(input).unwrap();
    let (_, _, value) = expect_inline(&ast.children[0]);

    assert_eq!(
        value,
        &interpolated(vec![
            InterpolatedTextPart::Expr {
                value: ExprSnippet::parse("a".into())
            },
            InterpolatedTextPart::Text {
                value: " + ".into()
            },
            InterpolatedTextPart::Expr {
                value: ExprSnippet::parse("b".into())
            },
            InterpolatedTextPart::Text {
                value: " = ".into()
            },
            InterpolatedTextPart::Expr {
                value: ExprSnippet::parse("a + b".into())
            },
        ])
    );
}

#[test]
fn throws_on_unknown_atcode() {
    let input = "<app {\n      @unknown {\n        foo = bar\n      }\n    }>";

    let err = parse_sakko(input).unwrap_err();
    assert!(
        err.to_string().contains("Unknown atcode '@unknown'"),
        "{}",
        err
    );
}

#[test]
fn throws_on_on_without_block() {
    let input = "<app {\n      button @on:click: \"Click\"\n    }>";

    let err = parse_sakko(input).unwrap_err();
    assert!(
        err.to_string()
            .contains("Event handlers must use block syntax"),
        "{}",
        err
    );
}

#[test]
fn throws_on_malformed_state_declaration() {
    let input = "<app {\n      @state {\n        invalid_no_equals\n      }\n    }>";

    let err = parse_sakko(input).unwrap_err();
    assert!(
        err.to_string().contains("Expected variable declaration"),
        "{}",
        err
    );
}

#[test]
fn parses_effect_with_backtick_template_literal() {
    let input = "<app {\n      @state { count = 0 }\n      @effect {\n        document.title = `Count: ${count}`\n      }\n      text: \"App\"\n    }>";

    let ast = parse_sakko(input).unwrap();
    let effect = ast
        .declarations
        .iter()
        .find(|d| matches!(d, AtcodeDeclaration::Effect { .. }))
        .expect("effect declaration expected");
    if let AtcodeDeclaration::Effect { body, .. } = effect {
        assert!(body.raw.contains("`Count:"), "{}", body.raw);
        assert!(body.raw.contains("${count}"), "{}", body.raw);
    }
}

#[test]
fn parses_style_modifier_as_atcode() {
    let input = "<page { button(@style \"color: red\"): \"Click\" }>";
    let ast = parse_sakko(input).unwrap();
    let (_, modifiers, _) = expect_inline(&ast.children[0]);
    assert!(modifiers.contains(&Modifier::Atcode {
        name: "style".into(),
        body: AtcodeBody::Text("color: red".into()),
    }));
}

#[test]
fn parses_if_modifier_as_atcode() {
    let input = "<page { button(@if=\"isVisible\"): \"Click\" }>";
    let ast = parse_sakko(input).unwrap();
    let (_, modifiers, _) = expect_inline(&ast.children[0]);
    assert_eq!(modifiers.len(), 1);
    assert!(matches!(
        &modifiers[0],
        Modifier::Atcode {
            name,
            body: AtcodeBody::Expr(ExprSnippet { raw, .. }),
        } if name == "if" && raw.as_ref() == "isVisible"
    ));
}

#[test]
fn parses_if_with_identifier_no_quotes() {
    let input = "<page { button(@if=isVisible): \"Click\" }>";
    let ast = parse_sakko(input).unwrap();
    let (_, modifiers, _) = expect_inline(&ast.children[0]);
    assert_eq!(modifiers.len(), 1);
    assert!(matches!(
        &modifiers[0],
        Modifier::Atcode {
            name,
            body: AtcodeBody::Expr(ExprSnippet { raw, .. }),
        } if name == "if" && raw.as_ref() == "isVisible"
    ));
}

#[test]
fn each_rejects_non_identifier_binding_in_string_form() {
    // The token form requires a single `Ident`; the quoted form must not
    // accept a name like "row item".
    let bad = "<app {\n  @state { xs = [] }\n  li @each=\"row item in xs\": \"x\"\n}>";
    let err = parse_sakko(bad).expect_err("should reject a multi-word binding");
    assert!(
        err.message.contains("Expected a single identifier"),
        "unexpected message: {}",
        err.message
    );

    for good in [
        "<app {\n  @state { xs = [] }\n  li @each=\"item in xs\": \"x\"\n}>",
        "<app {\n  @state { xs = [] }\n  li @each=\"i$1 in xs\": \"x\"\n}>",
    ] {
        parse_sakko(good).unwrap_or_else(|e| panic!("{good:?} should parse: {e}"));
    }
}

#[test]
fn parses_style_inside_parenthesized_modifiers() {
    // `button(@style="color: red")` must accept the optional `=`, matching the
    // unparenthesized `button @style="color: red"` form.
    let ast = parse_sakko("<app {\n  button(@style=\"color: red\"): \"x\"\n}>")
        .expect("parenthesized @style should parse");

    let child = &ast.children[0];
    let mods = expect_inline(child).1;
    assert!(
        mods.iter()
            .any(|m| matches!(m, Modifier::Atcode { name, .. } if name == "style")),
        "expected a @style modifier, got {:?}",
        mods
    );
}

#[test]
fn parses_attribute_pairs_with_equals_inside_parens() {
    // `key="value"` inside a parenthesized modifier list used to be routed into
    // the class-expression path, which refused to start at a lone `=` and so
    // consumed nothing -- spinning forever. It must parse as a Pair.
    let ast = parse_sakko("<app {\n  div(placeholder=\"What needs doing?\"): \"\"\n}>")
        .expect("known-key pair should parse");

    let child = &ast.children[0];
    let mods = expect_inline(child).1;
    assert!(
        mods.iter()
            .any(|m| matches!(m, Modifier::Pair { key, .. } if key == "placeholder")),
        "expected a placeholder Pair, got {:?}",
        mods
    );
}

#[test]
fn keeps_bare_flags_as_flags() {
    // Pairing is decided by what follows the ident, never by key membership
    // alone: `gap` is both a flag and a known key.
    let ast = parse_sakko("<app {\n  row(gap small): \"\"\n}>").expect("flags should parse");

    let child = &ast.children[0];
    let mods = expect_inline(child).1;
    assert!(
        matches!(mods.first(), Some(Modifier::Pair { key, value }) if key == "gap" && value == "small"),
        "expected gap=small to stay a bare-token pair, got {:?}",
        mods
    );

    // A lone ident with nothing after it remains a flag.
    let ast = parse_sakko("<app {\n  div(accent): \"\"\n}>").expect("flag should parse");
    let mods = expect_inline(&ast.children[0]).1;
    assert!(
        mods.iter().all(|m| matches!(m, Modifier::Flag { .. })),
        "expected a flag, got {:?}",
        mods
    );
}

#[test]
fn unmatched_closer_does_not_run_the_expression_scan_to_eof() {
    // `paren/brace/bracket` depth used `saturating_sub`, which floors at
    // `i32::MIN`, not 0. A stray closer went negative, so the depth-0 guard
    // never held again and the scan consumed everything to end of input.
    let input = r#"<app { @state { xs = ["a"] } text @class={a]}: "after" }>"#;
    let ast = parse_sakko(input)
        .unwrap_or_else(|e| panic!("should stop at the closing brace: {}", e.message));
    assert_eq!(
        ast.children.len(),
        1,
        "the trailing element after the class expression must still be parsed"
    );
}

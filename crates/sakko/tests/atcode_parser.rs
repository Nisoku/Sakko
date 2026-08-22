use sakko::{
    AstNode, AtcodeDeclaration, InlineValue, InterpolatedText, InterpolatedTextPart, Modifier,
    parse_sakko,
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
            assert_eq!(declarations[0].value.as_ref(), "0");
            assert_eq!(declarations[1].name.as_ref(), "step");
            assert_eq!(declarations[1].value.as_ref(), "1");
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
            assert_eq!(&**body, "console.log(\"Count:\",count)");
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
            assert_eq!(declarations[0].expr.as_ref(), "items.length");
        }
        other => panic!("expected derived declaration, got {:?}", other),
    }
}

#[test]
fn parses_on_event_modifier() {
    let input = "<app {\n      @state {\n        count = 0\n      }\n      \n      button @on:click {\n        count++\n      }: \"Increment\"\n    }>";

    let ast = parse_sakko(input).unwrap();
    let (_, modifiers, _) = expect_inline(&ast.children[0]);

    assert!(modifiers.contains(&Modifier::Event {
        event: "click".into(),
        handler: "count++".into(),
    }));
}

#[test]
fn parses_bind_modifier() {
    let input = "<app {\n      input @bind=\"username\": \"\"\n    }>";

    let ast = parse_sakko(input).unwrap();
    let (_, modifiers, _) = expect_inline(&ast.children[0]);

    assert!(modifiers.contains(&Modifier::Atcode {
        name: "bind".into(),
        body: "username".into(),
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
                value: "name".into()
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
            InterpolatedTextPart::Expr { value: "a".into() },
            InterpolatedTextPart::Text {
                value: " + ".into()
            },
            InterpolatedTextPart::Expr { value: "b".into() },
            InterpolatedTextPart::Text {
                value: " = ".into()
            },
            InterpolatedTextPart::Expr {
                value: "a + b".into()
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
        assert!(body.contains("`Count:"), "{}", body);
        assert!(body.contains("${count}"), "{}", body);
    }
}

#[test]
fn parses_style_modifier_as_atcode() {
    let input = "<page { button(@style \"color: red\"): \"Click\" }>";
    let ast = parse_sakko(input).unwrap();
    let (_, modifiers, _) = expect_inline(&ast.children[0]);
    assert!(modifiers.contains(&Modifier::Atcode {
        name: "style".into(),
        body: "color: red".into(),
    }));
}

#[test]
fn parses_if_modifier_as_atcode() {
    let input = "<page { button(@if=\"isVisible\"): \"Click\" }>";
    let ast = parse_sakko(input).unwrap();
    let (_, modifiers, _) = expect_inline(&ast.children[0]);
    assert!(modifiers.contains(&Modifier::Atcode {
        name: "if".into(),
        body: "isVisible".into(),
    }));
}

#[test]
fn parses_if_with_identifier_no_quotes() {
    let input = "<page { button(@if=isVisible): \"Click\" }>";
    let ast = parse_sakko(input).unwrap();
    let (_, modifiers, _) = expect_inline(&ast.children[0]);
    assert!(modifiers.contains(&Modifier::Atcode {
        name: "if".into(),
        body: "isVisible".into(),
    }));
}

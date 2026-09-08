use sakko::{AstNode, InlineValue, ListNode, Modifier, parse_sakko};

fn as_inline_name<'a>(node: &'a AstNode<'a>) -> &'a str {
    match node {
        AstNode::Inline(n) => &n.name,
        other => panic!("expected inline node, got {:?}", other),
    }
}

fn expect_inline<'a>(node: &'a AstNode<'a>) -> (&'a str, &'a [Modifier<'a>], &'a InlineValue<'a>) {
    match node {
        AstNode::Inline(n) => (&n.name, &n.modifiers, &n.value),
        other => panic!("expected inline node, got {:?}", other),
    }
}

fn expect_element<'a>(node: &'a AstNode<'a>) -> (&'a str, &'a [Modifier<'a>], &'a [AstNode<'a>]) {
    match node {
        AstNode::Element(n) => (&n.name, &n.modifiers, &n.children),
        other => panic!("expected element node, got {:?}", other),
    }
}

fn plain(s: &str) -> InlineValue<'_> {
    InlineValue::Plain(s.to_string().into())
}

#[test]
fn parses_simple_root_block() {
    let ast = parse_sakko("<page { text: Hello }>").unwrap();
    assert_eq!(&*ast.name, "page");
    assert_eq!(ast.children.len(), 1);
    let (name, modifiers, value) = expect_inline(&ast.children[0]);
    assert_eq!(name, "text");
    assert!(modifiers.is_empty());
    assert_eq!(value, &plain("Hello"));
}

#[test]
fn parses_nested_elements() {
    let ast = parse_sakko("<page { card { text: Hello; button: Click } }>").unwrap();
    assert_eq!(ast.children.len(), 1);
    let (_, _, children) = expect_element(&ast.children[0]);
    assert_eq!(children.len(), 2);
}

#[test]
fn parses_modifiers() {
    let ast = parse_sakko("<page { button(accent large): Save }>").unwrap();
    let (_, modifiers, value) = expect_inline(&ast.children[0]);
    assert_eq!(
        modifiers,
        &vec![
            Modifier::Flag {
                value: "accent".into()
            },
            Modifier::Flag {
                value: "large".into()
            },
        ]
    );
    assert_eq!(value, &plain("Save"));
}

#[test]
fn parses_key_value_modifiers() {
    let ast = parse_sakko(
        "<page { grid(cols 3 gap medium): [ card { text: One }, card { text: Two } ] }>",
    )
    .unwrap();
    let (_, modifiers, _) = expect_element(&ast.children[0]);
    assert_eq!(
        modifiers,
        &vec![
            Modifier::Pair {
                key: "cols".into(),
                value: "3".into()
            },
            Modifier::Pair {
                key: "gap".into(),
                value: "medium".into()
            },
        ]
    );
}

#[test]
fn handles_mixed_flag_and_key_value_modifiers() {
    let ast = parse_sakko("<page { text(bold dim): \"Label\" }>").unwrap();
    let (_, modifiers, _) = expect_inline(&ast.children[0]);
    assert_eq!(
        modifiers,
        &vec![
            Modifier::Flag {
                value: "bold".into()
            },
            Modifier::Flag {
                value: "dim".into()
            },
        ]
    );
}

#[test]
fn parses_complex_nested_structure() {
    let ast = parse_sakko(
        r#"
      <player {
        card(row medium center curved) {
          coverart(round): "album.jpg"
          details {
            text(bold): "Midnight City"
            text(dim small): "M83"
          }
          controls {
            icon-btn: play;
            icon-btn: skip;
            badge(accent): LIVE
          }
        }
      }>
    "#,
    )
    .unwrap();

    assert_eq!(&*ast.name, "player");
    assert_eq!(ast.children.len(), 1);
    let (name, _, children) = expect_element(&ast.children[0]);
    assert_eq!(name, "card");
    assert_eq!(children.len(), 3);
}

#[test]
fn parses_list_with_modifiers() {
    let ast =
        parse_sakko("<page { row(center): [ button: One, button: Two, button: Three ] }>").unwrap();
    let (_, modifiers, children) = expect_element(&ast.children[0]);
    assert_eq!(
        modifiers,
        &vec![Modifier::Flag {
            value: "center".into()
        }]
    );
    match &children[0] {
        AstNode::List(ListNode { items }) => assert_eq!(items.len(), 3),
        other => panic!("expected list, got {:?}", other),
    }
}

#[test]
fn handles_malformed_input_gracefully() {
    // These get auto-wrapped and parse successfully
    assert!(parse_sakko("page { text: Hello }").is_ok());
    // These are truly malformed and will throw
    assert!(parse_sakko("<page text: Hello }").is_err());
    assert!(parse_sakko("<page { text: Hello ").is_err());
}

#[test]
fn handles_quoted_strings_correctly() {
    let ast = parse_sakko("<page { text: \"Hello World with spaces\" }>").unwrap();
    let (_, _, value) = expect_inline(&ast.children[0]);
    assert_eq!(value, &plain("Hello World with spaces"));
}

#[test]
fn parses_empty_block() {
    let ast = parse_sakko("<page { card {} }>").unwrap();
    assert_eq!(ast.children.len(), 1);
    let (_, _, children) = expect_element(&ast.children[0]);
    assert!(children.is_empty());
}

#[test]
fn error_messages_include_line_and_column_info() {
    let err = parse_sakko("<page {\n  text:\n}>").unwrap_err();
    assert!(err.to_string().contains("line"), "{}", err);
}

#[test]
fn error_messages_include_source_snippet() {
    let err = parse_sakko("<page {\n  text:\n}>").unwrap_err();
    // TS asserts on e.message, which embeds position + snippet; the Rust
    // equivalent of that full string is Display.
    let rendered = err.to_string();
    assert!(
        rendered.contains("Expected value after ':'"),
        "{}",
        rendered
    );
    assert!(rendered.contains("line 3"), "{}", rendered);
}

#[test]
fn handles_url_strings_without_treating_slashes_as_comments() {
    let ast = parse_sakko("<page { image: \"https://example.com/img.png\" }>").unwrap();
    assert_eq!(ast.children.len(), 1);
    let (_, _, value) = expect_inline(&ast.children[0]);
    assert_eq!(value, &plain("https://example.com/img.png"));
}

#[test]
fn supports_string_values_in_modifiers() {
    let ast = parse_sakko("<page { input(placeholder \"Enter your name\"): \"\" }>").unwrap();
    let (_, modifiers, _) = expect_inline(&ast.children[0]);
    assert_eq!(
        modifiers,
        &vec![Modifier::Pair {
            key: "placeholder".into(),
            value: "Enter your name".into(),
        }]
    );
}

#[test]
fn supports_mixed_string_and_ident_modifier_values() {
    let ast = parse_sakko("<page { input(placeholder \"Email\" type email): \"\" }>").unwrap();
    let (_, modifiers, _) = expect_inline(&ast.children[0]);
    assert_eq!(
        modifiers,
        &vec![
            Modifier::Pair {
                key: "placeholder".into(),
                value: "Email".into()
            },
            Modifier::Pair {
                key: "type".into(),
                value: "email".into()
            },
        ]
    );
}

#[test]
fn handles_deeply_nested_structures() {
    let ast = parse_sakko(
        "<page {\n      card {\n        row {\n          column {\n            text: \"Deep\"\n          }\n        }\n      }\n    }>",
    )
    .unwrap();
    assert_eq!(ast.children.len(), 1);
    let (_, _, card_children) = expect_element(&ast.children[0]);
    assert_eq!(card_children.len(), 1);
    let (_, _, row_children) = expect_element(&card_children[0]);
    assert_eq!(row_children.len(), 1);
}

#[test]
fn handles_multiple_semicolons_on_same_line() {
    let ast = parse_sakko("<page { text: A; text: B; text: C }>").unwrap();
    assert_eq!(ast.children.len(), 3);
}

#[test]
fn handles_list_inside_block_with_trailing_comma() {
    let ast = parse_sakko("<page { row: [text: A, text: B] }>").unwrap();
    let (_, _, row_children) = expect_element(&ast.children[0]);
    assert_eq!(row_children.len(), 1);
    match &row_children[0] {
        AstNode::List(list) => assert_eq!(list.items.len(), 2),
        other => panic!("expected list, got {:?}", other),
    }
}

#[test]
fn handles_empty_root() {
    let ast = parse_sakko("<page {}>").unwrap();
    assert_eq!(&*ast.name, "page");
    assert!(ast.children.is_empty());
}

#[test]
fn handles_element_with_only_modifiers_and_no_children() {
    let ast = parse_sakko("<page { badge(accent): \"NEW\" }>").unwrap();
    let (name, modifiers, value) = expect_inline(&ast.children[0]);
    assert_eq!(value, &plain("NEW"));
    assert_eq!(
        modifiers,
        &vec![Modifier::Flag {
            value: "accent".into()
        }]
    );
    assert_eq!(name, "badge");
}

#[test]
fn handles_many_modifier_flags() {
    let ast = parse_sakko("<page { card(row medium center curved disabled) {} }>").unwrap();
    let (_, modifiers, _) = expect_element(&ast.children[0]);
    assert_eq!(modifiers.len(), 5);
}

#[test]
fn throws_on_double_semicolons() {
    assert!(parse_sakko("<page { text: A;; text: B }>").is_err());
}

#[test]
fn throws_on_missing_closing_angle_bracket() {
    assert!(parse_sakko("<page { text: A }").is_err());
}

#[test]
fn throws_on_missing_opening_angle_bracket() {
    // Auto-wrap still leaves the stray '>' unconsumable -> throws
    assert!(parse_sakko("page { text: A }>").is_err());
}

#[test]
fn parses_root_with_modifiers() {
    let ast = parse_sakko("<stack(gap medium) { text: A }>").unwrap();
    assert_eq!(&*ast.name, "stack");
    assert_eq!(ast.modifiers.len(), 1);
    assert_eq!(
        ast.modifiers[0],
        Modifier::Pair {
            key: "gap".into(),
            value: "medium".into()
        }
    );
}

#[test]
fn parses_root_with_flag_modifiers() {
    let ast = parse_sakko("<card(row center curved) { text: Hello }>").unwrap();
    assert_eq!(ast.modifiers.len(), 3);
    assert_eq!(
        ast.modifiers[0],
        Modifier::Flag {
            value: "row".into()
        }
    );
    assert_eq!(
        ast.modifiers[1],
        Modifier::Flag {
            value: "center".into()
        }
    );
    assert_eq!(
        ast.modifiers[2],
        Modifier::Flag {
            value: "curved".into()
        }
    );
}

#[test]
fn parses_void_elements() {
    let ast = parse_sakko("<page { divider }>").unwrap();
    assert_eq!(ast.children.len(), 1);
    let (name, modifiers, value) = expect_inline(&ast.children[0]);
    assert_eq!(name, "divider");
    assert!(modifiers.is_empty());
    assert_eq!(value, &plain(""));
}

#[test]
fn parses_void_elements_with_modifiers() {
    let ast = parse_sakko("<page { spacer(large) }>").unwrap();
    assert_eq!(ast.children.len(), 1);
    let (name, modifiers, _) = expect_inline(&ast.children[0]);
    assert_eq!(name, "spacer");
    assert_eq!(modifiers.len(), 1);
    assert_eq!(
        modifiers[0],
        Modifier::Flag {
            value: "large".into()
        }
    );
}

#[test]
fn allows_commas_as_separators_in_braces() {
    let ast = parse_sakko("<page { card { text: A }, card { text: B } }>").unwrap();
    assert_eq!(ast.children.len(), 2);
}

#[test]
fn allows_mixed_semicolons_and_commas() {
    let ast = parse_sakko("<page { text: A; text: B, text: C }>").unwrap();
    assert_eq!(ast.children.len(), 3);
}

#[test]
fn parses_multiple_void_elements_in_sequence() {
    let ast = parse_sakko("<page { divider spacer divider }>").unwrap();
    assert_eq!(ast.children.len(), 3);
    let names: Vec<&str> = ast.children.iter().map(|c| as_inline_name(c)).collect();
    assert_eq!(names, vec!["divider", "spacer", "divider"]);
}

#[test]
fn parses_string_modifier_with_special_characters() {
    let ast = parse_sakko("<page { input(placeholder \"Hello, World! (test)\"): \"\" }>").unwrap();
    let (_, modifiers, _) = expect_inline(&ast.children[0]);
    assert_eq!(modifiers.len(), 1);
    assert_eq!(
        modifiers[0],
        Modifier::Pair {
            key: "placeholder".into(),
            value: "Hello, World! (test)".into(),
        }
    );
}

#[test]
fn parses_url_in_string_value() {
    let ast = parse_sakko("<page { image(src \"https://example.com/img.jpg\"): \"\" }>").unwrap();
    let (_, modifiers, _) = expect_inline(&ast.children[0]);
    if let Modifier::Pair { value, .. } = &modifiers[0] {
        assert_eq!(value.as_ref(), "https://example.com/img.jpg");
    } else {
        panic!("expected pair, got {:?}", modifiers[0]);
    }
}

#[test]
fn parses_deeply_nested_void_elements() {
    let ast = parse_sakko("<page { card { text: Title; divider; text: Body } }>").unwrap();
    let (_, _, children) = expect_element(&ast.children[0]);
    assert_eq!(children.len(), 3);
    let (name, _, value) = expect_inline(&children[1]);
    assert_eq!(name, "divider");
    assert_eq!(value, &plain(""));
}

#[test]
fn parses_min_max_step_modifiers_for_slider() {
    let ast = parse_sakko("<page { slider(value 50 min 0 max 100 step 1): \"\" }>").unwrap();
    let (_, modifiers, _) = expect_inline(&ast.children[0]);
    assert_eq!(
        modifiers,
        &vec![
            Modifier::Pair {
                key: "value".into(),
                value: "50".into()
            },
            Modifier::Pair {
                key: "min".into(),
                value: "0".into()
            },
            Modifier::Pair {
                key: "max".into(),
                value: "100".into()
            },
            Modifier::Pair {
                key: "step".into(),
                value: "1".into()
            },
        ]
    );
}

#[test]
fn parses_name_modifier_for_radio() {
    let ast = parse_sakko("<page { radio(name \"r1\" value \"a\"): \"Option A\" }>").unwrap();
    let (_, modifiers, _) = expect_inline(&ast.children[0]);
    assert_eq!(
        modifiers,
        &vec![
            Modifier::Pair {
                key: "name".into(),
                value: "r1".into()
            },
            Modifier::Pair {
                key: "value".into(),
                value: "a".into()
            },
        ]
    );
}

#[test]
fn parses_heading_modifier_for_accordion() {
    let ast = parse_sakko("<page { div(heading \"Section 1\"): \"Content\" }>").unwrap();
    let (_, modifiers, _) = expect_inline(&ast.children[0]);
    assert_eq!(
        modifiers,
        &vec![Modifier::Pair {
            key: "heading".into(),
            value: "Section 1".into(),
        }]
    );
}

#[test]
fn parses_slot_modifier() {
    let ast = parse_sakko("<page { div(slot \"panel\"): \"Content\" }>").unwrap();
    let (_, modifiers, _) = expect_inline(&ast.children[0]);
    assert_eq!(
        modifiers,
        &vec![Modifier::Pair {
            key: "slot".into(),
            value: "panel".into()
        }]
    );
}

#[test]
fn parses_active_modifier_for_tabs() {
    let ast = parse_sakko("<page { tabs(active 0): \"\" }>").unwrap();
    let (_, modifiers, _) = expect_inline(&ast.children[0]);
    assert_eq!(
        modifiers,
        &vec![Modifier::Pair {
            key: "active".into(),
            value: "0".into()
        }]
    );
}

#[test]
fn parses_open_modifier_for_accordion() {
    let ast = parse_sakko("<page { div(heading \"Section\" open): \"Content\" }>").unwrap();
    let (_, modifiers, _) = expect_inline(&ast.children[0]);
    assert_eq!(
        modifiers,
        &vec![
            Modifier::Pair {
                key: "heading".into(),
                value: "Section".into()
            },
            Modifier::Flag {
                value: "open".into()
            },
        ]
    );
}

#[test]
fn parses_message_modifier_for_toast() {
    let ast = parse_sakko("<page { toast(message \"Hello\" variant \"success\"): \"\" }>").unwrap();
    let (_, modifiers, _) = expect_inline(&ast.children[0]);
    assert_eq!(
        modifiers,
        &vec![
            Modifier::Pair {
                key: "message".into(),
                value: "Hello".into()
            },
            Modifier::Pair {
                key: "variant".into(),
                value: "success".into()
            },
        ]
    );
}

#[test]
fn parses_title_modifier_for_modal() {
    let ast = parse_sakko("<page { modal(title \"Hello\"): \"\" }>").unwrap();
    let (_, modifiers, _) = expect_inline(&ast.children[0]);
    assert_eq!(
        modifiers,
        &vec![Modifier::Pair {
            key: "title".into(),
            value: "Hello".into()
        }]
    );
}

#[test]
fn parses_id_as_pair_key() {
    let ast = parse_sakko("<page { input(id \"myInput\"): \"\" }>").unwrap();
    let (_, modifiers, _) = expect_inline(&ast.children[0]);
    assert!(modifiers.contains(&Modifier::Pair {
        key: "id".into(),
        value: "myInput".into(),
    }));
}

#[test]
fn parses_class_as_pair_key() {
    let ast = parse_sakko("<page { div(class \"container\"): \"\" }>").unwrap();
    let (_, modifiers, _) = expect_inline(&ast.children[0]);
    assert!(modifiers.contains(&Modifier::Pair {
        key: "class".into(),
        value: "container".into(),
    }));
}

#[test]
fn parses_data_star_as_pair_key() {
    let ast = parse_sakko("<page { div(data-tile \"3\"): \"\" }>").unwrap();
    let (_, modifiers, _) = expect_inline(&ast.children[0]);
    assert!(modifiers.contains(&Modifier::Pair {
        key: "data-tile".into(),
        value: "3".into(),
    }));
}

#[test]
fn parses_position_modifiers_as_pair_keys() {
    let ast = parse_sakko("<page { div(position absolute top 10 left 20): \"\" }>").unwrap();
    let (_, modifiers, _) = expect_inline(&ast.children[0]);
    assert!(modifiers.contains(&Modifier::Pair {
        key: "position".into(),
        value: "absolute".into()
    }));
    assert!(modifiers.contains(&Modifier::Pair {
        key: "top".into(),
        value: "10".into()
    }));
    assert!(modifiers.contains(&Modifier::Pair {
        key: "left".into(),
        value: "20".into()
    }));
}

#[test]
fn parses_layout_modifiers_as_pair_keys() {
    let ast = parse_sakko("<page { div(display flex width \"100%\"): \"\" }>").unwrap();
    let (_, modifiers, _) = expect_inline(&ast.children[0]);
    assert!(modifiers.contains(&Modifier::Pair {
        key: "display".into(),
        value: "flex".into()
    }));
    assert!(modifiers.contains(&Modifier::Pair {
        key: "width".into(),
        value: "100%".into()
    }));
}

#[test]
fn parses_z_index_and_opacity_as_pair_keys() {
    let ast = parse_sakko("<page { div(z-index 10 opacity \"0.5\"): \"\" }>").unwrap();
    let (_, modifiers, _) = expect_inline(&ast.children[0]);
    assert!(modifiers.contains(&Modifier::Pair {
        key: "z-index".into(),
        value: "10".into()
    }));
    assert!(modifiers.contains(&Modifier::Pair {
        key: "opacity".into(),
        value: "0.5".into()
    }));
}

// Guard against silent KNOWN_KEYS ordering regressions (binary search):
// every key must resolve to a pair modifier.
// NOTE: "md:cols"/"lg:cols" are excluded because ':' can never appear inside an
// IDENT token, so those KNOWN_KEYS entries are unreachable in TS too.
#[test]
fn known_keys_are_sorted() {
    for key in [
        "active",
        "align-self",
        "alt",
        "bottom",
        "center-point",
        "class",
        "cols",
        "display",
        "flex",
        "float",
        "gap",
        "heading",
        "height",
        "hidden",
        "icon",
        "id",
        "inset",
        "justify-self",
        "label",
        "layout",
        "left",
        "margin",
        "max",
        "message",
        "min",
        "name",
        "opacity",
        "open",
        "order",
        "overflow",
        "padding",
        "placeholder",
        "position",
        "radius",
        "right",
        "size",
        "slot",
        "src",
        "step",
        "title",
        "top",
        "transform",
        "transition",
        "type",
        "value",
        "variant",
        "width",
        "z-index",
    ] {
        let src = format!("<page {{ div({k} v): \"\" }}>", k = key);
        let ast = parse_sakko(&src).unwrap_or_else(|e| panic!("key {:?}: {}", key, e));
        let (_, modifiers, _) = expect_inline(&ast.children[0]);
        assert!(
            matches!(&modifiers[0], Modifier::Pair { key: k, .. } if k == key),
            "key {:?} did not produce a pair: {:?}",
            key,
            modifiers
        );
    }
}

#[test]
fn root_shape_matches_ts_json_layout() {
    let ast = parse_sakko("<page {}>").unwrap();
    let json = serde_json::to_string(&AstNode::Root(ast)).unwrap();
    assert!(json.contains(r#""type":"root""#), "{}", json);
    assert!(json.contains(r#""name":"page""#), "{}", json);
}

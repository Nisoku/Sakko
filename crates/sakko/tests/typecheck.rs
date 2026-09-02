use std::path::Path;

fn snap(name: &str, src: &str) {
    let report = match sakko::typecheck::check_source(src) {
        Ok(r) => r,
        Err(e) => panic!("source did not parse: {e}"),
    };
    let actual = if report.diagnostics.is_empty() {
        String::from("ok\n")
    } else {
        report
            .diagnostics
            .iter()
            .map(|d| d.render())
            .collect::<String>()
    };

    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/snapshots/typecheck")
        .join(format!("{name}.snap"));

    if std::env::var("BLESS").is_ok() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &actual).unwrap();
    }

    let expected = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "missing snapshot {}; run tests with BLESS=1",
            path.display()
        )
    });

    assert_eq!(actual, expected, "snapshot mismatch for {name}");
}

#[test]
fn happy_counter() {
    snap(
        "happy_counter",
        r#"<counter {
  @state {
    count = 0
  }

  button @on:click { count++ }: "+"
  text: "Count: {count}"
}>"#,
    );
}

#[test]
fn happy_derived_and_nullish() {
    snap(
        "happy_derived_and_nullish",
        r#"<profile {
  @state {
    nickname = null
    items = []
  }

  @derived {
    total = items.length
    label = nickname ?? "anon"
  }

  text: "{label}: {total}"
  row: [text: A, text: B]
  input @bind="nickname": ""
}>"#,
    );
}

#[test]
fn happy_builtins() {
    snap(
        "happy_builtins",
        r#"<chart {
  @state {
    raw = "3.14"
    xs = [1, 2, 3]
  }

  @effect {
    const n = Number.parseFloat(raw)
    console.log(Math.floor(n), JSON.stringify(xs))
  }

  text: "{xs.map(i => i * 2).join(',')}"
}>"#,
    );
}

#[test]
fn unknown_identifier() {
    snap(
        "unknown_identifier",
        r#"<counter {
  @state { count = 0 }
  text: "{cont}"
}>"#,
    );
}

#[test]
fn unknown_property_typo() {
    snap(
        "unknown_property_typo",
        r#"<list {
  @state { items = [] }
  text: "{items.lenght}"
  text: "{items.length}"
}>"#,
    );
}

#[test]
fn not_callable() {
    snap(
        "not_callable",
        r#"<counter {
  @state { count = 0 }
  text: "{count()}"
}>"#,
    );
}

#[test]
fn assign_mismatch_in_handler() {
    snap(
        "assign_mismatch_in_handler",
        r#"<counter {
  @state { count = 0 }
  button @on:click { count = "many" }: "+"
}>"#,
    );
}

#[test]
fn bad_operand_mixed_add() {
    snap(
        "bad_operand_mixed_add",
        r#"<counter {
  @state { count = 0 }
  text: "{count + 'x'}"
}>"#,
    );
}

#[test]
fn bad_unary_operand() {
    snap(
        "bad_unary_operand",
        r#"<form {
  @state { name = "" }
  text: "{-name}"
}>"#,
    );
}

#[test]
fn duplicate_state_declarations() {
    snap(
        "duplicate_state_declarations",
        r#"<counter {
  @state { count = 0 }
  @state { count = 1 }
}>"#,
    );
}

#[test]
fn const_reassign_in_effect() {
    snap(
        "const_reassign_in_effect",
        r#"<app {
  @state { done = false }

  @effect {
    const flag = true
    flag = false
    done = flag
  }
}>"#,
    );
}

#[test]
fn bad_bind_target() {
    snap(
        "bad_bind_target",
        r#"<form {
  @state { username = "" }
  input @bind="usrname": ""
  input @bind="username": ""
}>"#,
    );
}

#[test]
fn snippet_parse_error_in_effect() {
    snap(
        "snippet_parse_error_in_effect",
        r#"<app {
  @effect {
    console.log(1 +)
  }
}>"#,
    );
}

#[test]
fn handler_unknown_variable() {
    snap(
        "handler_unknown_variable",
        r#"<counter {
  @state { count = 0 }
  button @on:click { cont++ }: "+"
}>"#,
    );
}

#[test]
fn interpolated_function_value() {
    snap(
        "interpolated_function_value",
        r#"<list {
  @state { items = [] }
  text: "{items.map}"
}>"#,
    );
}

#[test]
fn strict_eq_banned() {
    snap(
        "strict_eq_banned",
        r#"<list {
  @state {
    items = []
    isEmpty = items.length === 0
  }
}>"#,
    );
}

#[test]
fn js_and_assert_happy() {
    snap(
        "js_and_assert_happy",
        r#"<dash {
  @state {
    theme = js { return localStorage.getItem("theme") } ?? "dark"
    width = js { return window.innerWidth } as number
    doubled = width * 2
  }
  text: "{doubled}px"
  button @on:click {
    js { document.title = "Dash" }
  }: "focus"
}>"#,
    );
}

#[test]
fn unknown_consumption_gated() {
    snap(
        "unknown_consumption_gated",
        r#"<dash {
  @state {
    n = js { return Math.random() }
    big = n * 2
  }
  text: "{n + 1}"
}>"#,
    );
}

#[test]
fn impossible_cast() {
    snap(
        "impossible_cast",
        r#"<app {
  @state { n = 5 as string }
}>"#,
    );
}

#[test]
fn handler_event_param_cast() {
    snap(
        "handler_event_param_cast",
        r#"<form {
  @state { value = "" }
  input @on:input {
    value = e.target.value as string
  }: ""
}>"#,
    );
}

#[test]
fn js_escapes_are_recorded() {
    let src = r#"<dash {
  @state {
    w = js { return window.innerWidth } as number
  }
  button @on:click {
    js { document.title = "hi" }
  }: "go"
}>"#;
    let report = sakko::typecheck::check_source(src).unwrap();
    assert!(report.diagnostics.is_empty(), "expected no diagnostics");
    assert_eq!(report.js_escapes.len(), 2);
    assert_eq!(report.js_escapes[0].body, r#"return window.innerWidth"#);
    assert_eq!(report.js_escapes[1].kind_label, "@on:click");
}

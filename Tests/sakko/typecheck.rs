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
        .join("../../Tests/sakko/snapshots/typecheck")
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

    // Compare against LF regardless of how the snapshot was checked out
    let expected = expected.replace("\r\n", "\n");
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
fn dom_globals_rejected_outside_js() {
    snap(
        "dom_globals_rejected_outside_js",
        r#"<app {
  text: "{document.title}"
  text: "{window.innerWidth}"
  text: "{localStorage.getItem('k')}"
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

#[test]
fn happy_flexible_class_forms() {
    // @class supports all spellings: static name, `=` string/array, braced.
    snap(
        "happy_flexible_class_forms",
        r#"<app {
  @state {
    theme = "light"
    on = true
  }
  div @class:name: "x"
  div @class="theme dark": "x"
  button @class={theme}: "y"
  button @class=["a", "b"]: "y"
  button @class={on ? "on" : "off"}: "y"
  input @style="color: red": ""
}>"#,
    );
}

#[test]
fn class_rejects_non_string_values() {
    snap(
        "class_rejects_non_string_values",
        r#"<app {
  @state {
    n = 5
    on = true
  }
  div @class={n}: "x"
  div @class={ { active: on } }: "y"
}>"#,
    );
}

#[test]
fn method_call_return_types_flow() {
    // `infer_member` records the signature under the whole `obj.name` span so
    // `callee_ret` finds it; otherwise every method call degrades to `any` and
    // bogus members on the result go unreported.
    snap(
        "method_call_return_types_flow",
        r#"<app {
  @effect {
    const i = "hi".indexOf("h")
    console.log(i.bogusProp)
    const parts = "a,b".split(",")
    console.log(parts.bogusProp)
    const up = "hi".toUpperCase()
    console.log(up.length)
  }
  text: "A"
}>"#,
    );
}

#[test]
fn each_accepts_union_of_arrays() {
    // Method return types now flow through `@derived`, so a ternary of array
    // branches is a *union* of arrays. `@each` must still accept it, and the
    // bound item keeps the branches' common element type.
    snap(
        "each_accepts_union_of_arrays",
        r#"<app {
  @state {
    items = []
    mode = "all"
  }
  @derived {
    visible = mode == "active" ? items.filter(i => !i.done) : items
  }
  li @each="item in visible": item.text
}>"#,
    );
}

#[test]
fn each_rejects_non_iterable_union_member() {
    snap(
        "each_rejects_non_iterable_union_member",
        r#"<app {
  @state {
    items = []
    count = 0
  }
  @derived {
    visible = count > 0 ? items : count
  }
  li @each="item in visible": "x"
}>"#,
    );
}

#[test]
fn expr_snippet_reassembles_multiple_interpolations() {
    // The whole `String`/`InterpStart`/`Expr`/`InterpEnd` run must be
    // reassembled; stopping at the first `InterpEnd` truncated the source.
    snap(
        "expr_snippet_reassembles_multiple_interpolations",
        r#"<app {
  @state {
    a = 1
    b = 2
  }
  @derived {
    label = "x {a} y {b} z"
  }
  text(dim): label
}>"#,
    );
}

#[test]
fn if_branches_do_not_leak_declarations() {
    snap(
        "if_branches_do_not_leak_declarations",
        r#"<app {
  @effect {
    if true {
      const leaked = 1
    }
    console.log(leaked)
  }
  text: "A"
}>"#,
    );
}

#[test]
fn each_supports_member_and_call_sources() {
    snap(
        "each_supports_member_and_call_sources",
        r#"<app {
  @state {
    o = { list: [] }
    items = []
  }
  li @each="i in o.list": "x"
  li @each="i in items.filter(a => a > 1)": "x"
  li @each="i in nope": "x"
}>"#,
    );
}

#[test]
fn class_rejects_arrays_of_non_strings() {
    snap(
        "class_rejects_arrays_of_non_strings",
        r#"<app {
  @state {
    names = ["a", "b"]
    counts = [1, 2]
  }
  div @class={names}: "ok"
  div @class={counts}: "bad"
}>"#,
    );
}

#[test]
fn namespace_method_return_types() {
    // `ns_member` used to hand every method `Function` as its *return* type,
    // so `Math.round(...)` was itself typed `Function`. Each name now reports
    // its real result: number, string, boolean, or undefined.
    snap(
        "namespace_method_return_types",
        r#"<app {
  @effect {
    const a = Math.round(1.5)
    const b = JSON.stringify({})
    const c = Number.parseFloat("1.5")
    const d = Number.isNaN(1)
    const e = String.fromCharCode(65)
    const f = console.log("x")
    console.log(a + 1)
    console.log(b.length)
    console.log(c + 1)
    console.log(d ? "y" : "n")
    console.log(e.length)
    console.log(f)
  }
  text: "A"
}>"#,
    );
}

#[test]
fn each_item_type_ignores_null_union_members() {
    // `check_each` tolerates `null`/`undefined` union members, but they carry
    // no element, so they must not widen the bound item to `number | null`.
    snap(
        "each_item_type_ignores_null_union_members",
        r#"<app {
  @state {
    xs = [1]
    flag = true
  }
  @derived {
    source = flag ? xs : null
  }
  li @each="i in source": [button @on:click { i() }: "x"]
}>"#,
    );
}

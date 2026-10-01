use sakko::saho;

/// Assert `parse` succeeds and lowering reproduces the (trimmed) source.
fn roundtrip(src: &str) {
    let trimmed = src.trim();
    let node = saho::parse(trimmed)
        .unwrap_or_else(|e| panic!("parse failed for {trimmed:?}: {} {:?}", e.message, e.span));
    let out = saho::lower(&node, trimmed).unwrap_or_else(|| panic!("lower failed for {trimmed:?}"));
    assert_eq!(out, trimmed, "round-trip mismatch");
}

#[test]
fn literals_and_identifiers() {
    roundtrip("count");
    roundtrip("_private$var9");
    roundtrip("0");
    roundtrip("42");
    roundtrip("3.14");
    roundtrip("1.5e10");
    roundtrip("0xFF");
    roundtrip("0b1010");
    roundtrip("0o777");
    roundtrip("10n");
    roundtrip("\"hello\"");
    roundtrip("'world'");
    roundtrip("\"a\\n\\t\\\"esc\\u0041\"");
    roundtrip("`plain template`");
    roundtrip("true");
    roundtrip("false");
    roundtrip("null");
    roundtrip("undefined");
}

#[test]
fn templates_with_substitutions() {
    roundtrip("`Hello ${name}!`");
    roundtrip("`${a}${b}`");
    roundtrip("`${user.name} has ${items.length}`");
    // nested template inside substitution
    roundtrip("`${`inner ${x}`}`");
    // call inside substitution
    roundtrip("`${fn(a, b)}`");
    // arithmetic inside substitution
    roundtrip("`${a + b * 2}`");
}

#[test]
fn operators_and_precedence() {
    roundtrip("a + b - c");
    roundtrip("a * b / c % d");
    roundtrip("a ** b");
    roundtrip("-a + ~b ^ !c");
    roundtrip("a << 2 >> 1 >>> 3");
    roundtrip("a < b <= c > d >= e");
    roundtrip("a == b != c == d != e");
    roundtrip("a & b | c ^ d");
    roundtrip("a && b || c ?? d");
    roundtrip("a in b instanceof C");
    roundtrip("(i++, j--)");
    roundtrip("++counter");
    roundtrip("cursor.position--");
    roundtrip("+x -x");
}

#[test]
fn conditional_assignment_sequence() {
    roundtrip("a ? b : c");
    roundtrip("a ? b : c ? d : e");
    roundtrip("x = y");
    roundtrip("obj.field += 3");
    roundtrip("arr[i] ??= fallback");
    roundtrip("flag &&= other");
    roundtrip("n *= 2 ** k");
    roundtrip("a = 1, b = 2, c = 3");
}

#[test]
fn member_index_call_new() {
    roundtrip("user.name");
    roundtrip("user?.profile?.avatar");
    roundtrip("list[0]");
    roundtrip("matrix[i][j]");
    roundtrip("list?.[key]");
    roundtrip("fn()");
    roundtrip("fn(1, 'two', three)");
    roundtrip("fn?.(x)");
    roundtrip("new Map()");
    roundtrip("new Foo");
    roundtrip("new a.b(c)");
    roundtrip("tagged`x${y}z`");
}

#[test]
fn arrays_objects() {
    roundtrip("[]");
    roundtrip("[1, 2, 3]");
    roundtrip("[1, [2, [3]],]");
    roundtrip("[...rest, last]");
    roundtrip("{}");
    roundtrip("{ a: 1, b: 2 }");
    roundtrip("{ key, computed }");
    roundtrip("{ 'str-key': 1, 42: n }");
    roundtrip("{ [dyn]: v, ...spread }");
    roundtrip("{ deep: { deeper: [ { x } ] } }");
}

#[test]
fn arrows_functions_parens() {
    roundtrip("(x) => x * 2");
    roundtrip("(a, b) => a + b");
    roundtrip("x => x.id");
    roundtrip("() => 42");
    roundtrip("async x => x");
    roundtrip("async (a, b) => a.then(r => r)");
    roundtrip("(a = 1, ...rest) => rest");
    roundtrip("(x) => ({ y: x })");
    roundtrip("(function (a, b) { return a + b; })");
    roundtrip("(function named(x) { return x; })");
    // NOTE: `async function` expressions and `await` are v1 limitations
}

#[test]
fn control_flow_bodies() {
    // Single-expression bodies via parse_body
    let stmts = saho::parse_body("count + 1;").unwrap_or_else(|d| panic!("diags: {d:?}"));
    assert_eq!(stmts.len(), 1);

    let stmts = saho::parse_body("let total = 0; total += item.price;")
        .unwrap_or_else(|d| panic!("diags: {d:?}"));
    assert_eq!(stmts.len(), 2);

    let stmts = saho::parse_body("return found;").unwrap_or_else(|d| panic!("diags: {d:?}"));
    assert_eq!(stmts.len(), 1);

    // Multi-statement recovery: a bad statement produces a diagnostic but
    // parsing resumes on the next segment.
    let diags = saho::parse_body("ok = true; let ; done = yes;").expect_err("should have diags");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("let"));
    assert!(saho::parse_body("done = yes;").is_ok());

    // Bare declaration keywords are rejected as statements...
    for kw in ["let", "const", "var"] {
        assert!(
            saho::parse_body(kw).is_err(),
            "bare `{kw}` must not parse as an expression statement"
        );
    }
    // ...but remain valid inside larger expressions / pure-expr context.
    roundtrip("let.foo");
    roundtrip("let");
}

#[test]
fn comments_and_whitespace() {
    roundtrip("/* lead */ a /* mid */ + /* tail */ b");
    roundtrip("a\n  +\n\tb");
    roundtrip("f(\n  a,\n  b,\n)");
}

#[test]
fn complex_real_world_expressions() {
    roundtrip("items.filter(i => i.active).map(i => `${i.name}: ${i.qty}`).join(', ')");
    roundtrip("user.settings?.theme ?? 'light'");
    roundtrip("typeof value == 'string' ? JSON.parse(value) : value");
    roundtrip("new Date(entry.timestamp).toISOString().slice(0, 10)");
    roundtrip("Object.entries(map).reduce((acc, [k, v]) => acc + v, 0)");
    roundtrip("(a, b) => ({ sum: a + b, diff: a - b })[op] ?? 0");
}

#[test]
fn rejected_sources() {
    // Empty / bare punctuation / unbalanced constructs must produce diags,
    // never panic.
    for src in ["", "+", ")", "(", "[1, 2", "{ a: ", "a ??", "`unterminated"] {
        assert!(saho::parse(src).is_err(), "expected {src:?} to be rejected");
    }
}

#[test]
fn new_expression_shape() {
    // `new` callees may only be member/index chains: in `new a.b(c)` the
    // arguments belong to the constructor call, and `new Foo().bar` applies
    // `.bar` to the constructed object (i.e. `(new Foo()).bar`).
    let n = saho::parse("new Foo().bar").unwrap();
    match &n.kind {
        saho::EKind::Member {
            obj,
            name,
            optional: false,
        } => {
            assert_eq!(name, "bar");
            match &obj.kind {
                saho::EKind::New { callee, args } => {
                    assert!(matches!(&callee.kind, saho::EKind::Ident(_)));
                    // `new Foo()` carries its (empty) constructor-arg list.
                    assert!(args.as_ref().is_some_and(|a| a.is_empty()));
                }
                other => panic!("expected New callee, got {other:?}"),
            }
        }
        other => panic!("expected Member, got {other:?}"),
    }

    let n = saho::parse("new a.b(c)").unwrap();
    match &n.kind {
        saho::EKind::New { callee, args } => {
            match &callee.kind {
                saho::EKind::Member { obj, name, .. } => {
                    assert_eq!(name, "b");
                    assert!(matches!(&obj.kind, saho::EKind::Ident(_)));
                }
                other => panic!("expected Member callee, got {other:?}"),
            }
            assert_eq!(args.as_ref().map(|a| a.len()), Some(1));
        }
        other => panic!("expected New, got {other:?}"),
    }

    // Post-instantiation calls still chain off the constructed object.
    let n = saho::parse("new Date(entry.timestamp).toISOString()").unwrap();
    match &n.kind {
        saho::EKind::Call { callee, args, .. } => {
            assert!(args.is_empty());
            match &callee.kind {
                saho::EKind::Member { obj, name, .. } => {
                    assert_eq!(name, "toISOString");
                    assert!(matches!(&obj.kind, saho::EKind::New { .. }));
                }
                other => panic!("expected Member callee, got {other:?}"),
            }
        }
        other => panic!("expected Call, got {other:?}"),
    }
}

#[test]
fn as_assertions_roundtrip() {
    roundtrip("count as number");
    roundtrip("name as string");
    roundtrip("items as string[]");
    roundtrip("items as number[] | null");
    roundtrip("maybe as string | null");
    roundtrip("w as unknown");
}

#[test]
fn raw_js_blocks_roundtrip() {
    roundtrip("js { return window.innerWidth }");
    roundtrip("js { if (a) { b(); } }");
    roundtrip("js { const s = \"no { fake } brace\"; return s; }");
    let node = saho::parse("js { // comment }\nreturn 1; }").unwrap();
    assert!(matches!(node.kind, saho::EKind::RawJs));
}

#[test]
fn strict_equality_ops_are_banned() {
    for src in ["a === b", "a !== b"] {
        let err = saho::parse(src).expect_err("should reject strict equality");
        assert!(err.message.contains("'==' is already strict"));
    }
}

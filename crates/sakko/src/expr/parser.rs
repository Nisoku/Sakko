//! Chumsky grammar for Saho expressions.

use chumsky::{
    Parser, extra,
    input::BorrowInput,
    pratt::Operator,
    pratt::{infix, left, prefix, right},
    prelude::*,
    span::SimpleSpan,
};

use super::ast::{
    Arg, AssignOp, BinOp, Body, EKind, Key, Node, ObjPatProp, ObjProp, Pat, Stmt,
    TplPart as TplPartAst, TypeAst, UnaryOp, UpdateOp, VarKw,
};
use super::lexer::{ETok, Subst, TplPart};

type SP = SimpleSpan;
type Ex<'a> = extra::Err<Rich<'a, ETok, SP>>;

fn sp(s: SP) -> crate::span::Span {
    crate::span::Span::new(s.start, s.end)
}

fn node(kind: EKind, s: SP) -> Node {
    Node { kind, span: sp(s) }
}

/// Raw template parts straight from the lexer.
type RawTpl = Vec<TplPart>;

fn kw(s: &'static str) -> ETok {
    ETok::Ident(s.to_owned())
}

/// Resolve a raw template token into AST parts by recursively parsing each
/// substitution source.
fn resolve_template(parts: Vec<TplPart>) -> Result<Vec<TplPartAst>, String> {
    let mut out = Vec::with_capacity(parts.len());
    for part in parts {
        match part {
            TplPart::Quasi(text) => out.push(TplPartAst::Quasi(text)),
            TplPart::Subst(Subst { text, abs }) => {
                let inner = super::parse_substitution(&text).map_err(|e| {
                    format!(
                        "{} (at byte {} of enclosing source)",
                        e.message,
                        abs.start + e.span.start
                    )
                })?;
                out.push(TplPartAst::Expr(remap(inner, abs.start)));
            }
        }
    }
    Ok(out)
}

/// Shift every span in `node` by `delta` (substitution -> outer coordinates).
pub(crate) fn remap(node: Node, delta: u32) -> Node {
    fn inner(node: Node, delta: u32) -> Node {
        let kind = match node.kind {
            EKind::Template(parts) => EKind::Template(
                parts
                    .into_iter()
                    .map(|part| match part {
                        TplPartAst::Quasi(t) => TplPartAst::Quasi(t),
                        TplPartAst::Expr(child) => TplPartAst::Expr(inner(child, delta)),
                    })
                    .collect::<Vec<_>>(),
            ),
            EKind::Array(items) => EKind::Array(
                items
                    .into_iter()
                    .map(|item| item.map(|n| inner(n, delta)))
                    .collect::<Vec<_>>(),
            ),
            EKind::Object(props) => EKind::Object(
                props
                    .into_iter()
                    .map(|prop| match prop {
                        ObjProp::Kv { key, value } => ObjProp::Kv {
                            key: match key {
                                Key::Computed(n) => Key::Computed(inner(n, delta)),
                                other => other,
                            },
                            value: inner(value, delta),
                        },
                        ObjProp::Shorthand(n) => ObjProp::Shorthand(inner(n, delta)),
                        ObjProp::Spread(n) => ObjProp::Spread(inner(n, delta)),
                    })
                    .collect::<Vec<_>>(),
            ),
            EKind::Fn { params, body } => EKind::Fn {
                params,
                body: body_map(body, delta),
            },
            EKind::Arrow {
                params,
                body,
                is_async,
            } => EKind::Arrow {
                params,
                body: body_map(body, delta),
                is_async,
            },
            EKind::Call {
                callee,
                args,
                optional,
            } => EKind::Call {
                callee: Box::new(inner(*callee, delta)),
                args: args
                    .into_iter()
                    .map(|arg| arg_map(arg, delta))
                    .collect::<Vec<_>>(),
                optional,
            },
            EKind::New { callee, args } => EKind::New {
                callee: Box::new(inner(*callee, delta)),
                args: args.map(|xs| {
                    xs.into_iter()
                        .map(|arg| arg_map(arg, delta))
                        .collect::<Vec<_>>()
                }),
            },
            EKind::Member {
                obj,
                name,
                optional,
            } => EKind::Member {
                obj: Box::new(inner(*obj, delta)),
                name,
                optional,
            },
            EKind::Index {
                obj,
                index,
                optional,
            } => EKind::Index {
                obj: Box::new(inner(*obj, delta)),
                index: Box::new(inner(*index, delta)),
                optional,
            },
            EKind::TaggedTpl { tag, tpl } => EKind::TaggedTpl {
                tag: Box::new(inner(*tag, delta)),
                tpl: Box::new(inner(*tpl, delta)),
            },
            EKind::Assert { expr, ty } => EKind::Assert {
                expr: Box::new(inner(*expr, delta)),
                ty,
            },
            EKind::Unary { op, expr } => EKind::Unary {
                op,
                expr: Box::new(inner(*expr, delta)),
            },
            EKind::Update { op, prefix, target } => EKind::Update {
                op,
                prefix,
                target: Box::new(inner(*target, delta)),
            },
            EKind::Binary { op, lhs, rhs } => EKind::Binary {
                op,
                lhs: Box::new(inner(*lhs, delta)),
                rhs: Box::new(inner(*rhs, delta)),
            },
            EKind::Assign { op, target, value } => EKind::Assign {
                op,
                target: Box::new(inner(*target, delta)),
                value: Box::new(inner(*value, delta)),
            },
            EKind::Cond { test, cons, alt } => EKind::Cond {
                test: Box::new(inner(*test, delta)),
                cons: Box::new(inner(*cons, delta)),
                alt: Box::new(inner(*alt, delta)),
            },
            EKind::Seq(nodes) => EKind::Seq(
                nodes
                    .into_iter()
                    .map(|n| inner(n, delta))
                    .collect::<Vec<_>>(),
            ),
            EKind::Paren(x) => EKind::Paren(Box::new(inner(*x, delta))),
            EKind::Spread(x) => EKind::Spread(Box::new(inner(*x, delta))),
            leaf => leaf,
        };

        Node {
            kind,
            span: crate::span::Span {
                start: node.span.start + delta,
                end: node.span.end + delta,
            },
        }
    }

    inner(node, delta)
}

fn arg_map(arg: Arg, delta: u32) -> Arg {
    match arg {
        Arg::Plain(n) => Arg::Plain(remap(n, delta)),
        Arg::Spread(n) => Arg::Spread(remap(n, delta)),
    }
}

fn body_map(body: Body, delta: u32) -> Body {
    match body {
        Body::Expr(n) => Body::Expr(Box::new(remap(*n, delta))),
        Body::Block(stmts) => Body::Block(
            stmts
                .into_iter()
                .map(|stmt| match stmt {
                    Stmt::Expr(n) => Stmt::Expr(remap(n, delta)),
                    Stmt::VarDecl { kw, decls } => Stmt::VarDecl {
                        kw,
                        decls: decls
                            .into_iter()
                            .map(|(p, init)| (p, init.map(|n| remap(n, delta))))
                            .collect::<Vec<_>>(),
                    },
                    Stmt::Return(opt) => Stmt::Return(opt.map(|n| remap(n, delta))),
                })
                .collect::<Vec<_>>(),
        ),
    }
}

/// Convert one paren-group element into an arrow parameter pattern.
fn elem_to_pattern(elem: Node) -> Result<Pat, String> {
    fn conv(node: Node) -> Result<Pat, String> {
        match node.kind {
            EKind::Ident(name) => Ok(Pat::Ident(name)),
            EKind::Assign {
                op: AssignOp::Assign,
                target,
                value,
            } => Ok(Pat::Default {
                pat: Box::new(conv(*target)?),
                init: *value,
            }),
            EKind::Array(items) => Ok(Pat::Array(
                items
                    .into_iter()
                    .filter_map(|item| match item? {
                        Node {
                            kind: EKind::Spread(x),
                            ..
                        } => conv_plain(*x).ok().map(|p| Pat::Rest(Box::new(p))),
                        n => conv(n).ok(),
                    })
                    .collect(),
            )),
            EKind::Object(props) => Ok(Pat::Object(
                props
                    .into_iter()
                    .map(|prop| match prop {
                        ObjProp::Shorthand(n) => match n.kind {
                            EKind::Ident(name) => Ok(ObjPatProp::Shorthand(Pat::Ident(name))),
                            _ => Err("unsupported object shorthand in pattern".into()),
                        },
                        ObjProp::Kv { key, value } => Ok(ObjPatProp::Kv {
                            key,
                            pat: conv(value)?,
                        }),
                        ObjProp::Spread(n) => match n.kind {
                            EKind::Ident(name) => Ok(ObjPatProp::Rest(Pat::Ident(name))),
                            _ => Err("unsupported object rest in pattern".into()),
                        },
                    })
                    .collect::<Result<Vec<_>, String>>()?,
            )),
            _ => Err("expression cannot be used as a parameter".into()),
        }
    }

    match elem.kind {
        EKind::Spread(inner) => conv(*inner).map(|p| Pat::Rest(Box::new(p))),
        _ => conv(elem),
    }
}

fn conv_plain(node: Node) -> Result<Pat, String> {
    elem_to_pattern(node)
}

fn punct<'a, I>(sym: &'static str) -> impl Parser<'a, I, (), Ex<'a>> + Clone
where
    I: BorrowInput<'a, Token = ETok, Span = SP>,
{
    just(ETok::Punct(sym)).to(())
}

fn mk_pat<'a, I, E>(expr: E) -> impl Parser<'a, I, Pat, Ex<'a>> + Clone
where
    I: BorrowInput<'a, Token = ETok, Span = SP>,
    E: Parser<'a, I, Node, Ex<'a>> + Clone + 'a,
{
    recursive(move |pat| {
        let rest = punct("...")
            .ignore_then(pat.clone())
            .map(|p| Pat::Rest(Box::new(p)));
        let arr_pat = choice((rest.clone(), pat.clone()))
            .separated_by(punct(","))
            .allow_trailing()
            .collect::<Vec<_>>()
            .delimited_by(punct("["), punct("]"))
            .map(Pat::Array);
        let obj_prop = choice((
            rest.map(ObjPatProp::Rest),
            select_ref! { ETok::Ident(s) => s.clone() }
                .then_ignore(punct(":"))
                .then(pat.clone())
                .map(|(key, pat)| ObjPatProp::Kv {
                    key: Key::Ident(key),
                    pat,
                }),
            select_ref! { ETok::Str(s) => s.clone() }
                .then_ignore(punct(":"))
                .then(pat.clone())
                .map(|(key, pat)| ObjPatProp::Kv {
                    key: Key::Lit(key),
                    pat,
                }),
            select_ref! { ETok::Num(s) => s.clone() }
                .then_ignore(punct(":"))
                .then(pat.clone())
                .map(|(key, pat)| ObjPatProp::Kv {
                    key: Key::Lit(key),
                    pat,
                }),
            select_ref! { ETok::Ident(s) => ObjPatProp::Shorthand(Pat::Ident(s.clone())) },
        ));
        let obj_pat = obj_prop
            .separated_by(punct(","))
            .allow_trailing()
            .collect::<Vec<_>>()
            .delimited_by(punct("{"), punct("}"))
            .map(Pat::Object);

        choice((
            obj_pat,
            arr_pat,
            select_ref! { ETok::Ident(s) => Pat::Ident(s.clone()) },
        ))
        .then(punct("=").ignore_then(expr.clone()).or_not())
        .map(|(pat, init)| match init {
            Some(init) => Pat::Default {
                pat: Box::new(pat),
                init,
            },
            None => pat,
        })
    })
}

fn mk_stmt<'a, I, E>(expr: E) -> impl Parser<'a, I, Stmt, Ex<'a>> + Clone
where
    I: BorrowInput<'a, Token = ETok, Span = SP>,
    E: Parser<'a, I, Node, Ex<'a>> + Clone + 'a,
{
    let semi = punct(";").or_not();
    let var_decl = choice((
        just(kw("const")).to(VarKw::Const),
        just(kw("let")).to(VarKw::Let),
        just(kw("var")).to(VarKw::Var),
    ))
    .then(
        mk_pat(expr.clone())
            .then(punct("=").ignore_then(expr.clone()).or_not())
            .separated_by(punct(","))
            .at_least(1)
            .collect::<Vec<_>>(),
    )
    .then_ignore(semi.clone())
    .map(|(kw, decls)| Stmt::VarDecl { kw, decls });
    let ret = just(kw("return"))
        .ignore_then(expr.clone().or_not())
        .then_ignore(semi)
        .map(Stmt::Return);
    let expr_stmt = expr
        .then_ignore(punct(";").or(end()))
        .try_map(|n: Node, span| match &n.kind {
            // A bare declaration keyword is always a typo'd declaration,
            // never a legitimate expression statement.
            EKind::Ident(s) if s == "let" || s == "const" || s == "var" => Err(Rich::custom(
                span,
                format!("expected declaration after '{s}'"),
            )),
            _ => Ok(Stmt::Expr(n)),
        });

    choice((ret, var_decl, expr_stmt))
}

// Grammar

#[derive(Clone)]
enum PreOp {
    Unary(UnaryOp),
    Update(UpdateOp),
}

#[derive(Clone)]
enum PostOp {
    Member(String, bool),
    Index(Node, bool),
    Call(Vec<Arg>, bool),
    Tpl(Node),
    Update(UpdateOp),
    As(TypeAst),
}

/// Build the shared grammar: `(expression entry, statement-body entry)`.
pub(super) fn grammar<'a, I>() -> (
    impl Parser<'a, I, Node, Ex<'a>> + Clone,
    impl Parser<'a, I, Vec<Stmt>, Ex<'a>> + Clone,
)
where
    I: BorrowInput<'a, Token = ETok, Span = SP>,
{
    let expr_full = recursive(|expr| {
        let pat = mk_pat(expr.clone());
        let block_body = mk_stmt(expr.clone())
            .repeated()
            .collect::<Vec<_>>()
            .delimited_by(punct("{"), punct("}"));

        // ----- shared pieces -----
        let call_args = choice((
            punct("...").ignore_then(expr.clone()).map(Arg::Spread),
            expr.clone().map(Arg::Plain),
        ))
        .separated_by(punct(","))
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(punct("("), punct(")"));

        let tpl_tok = select_ref! { ETok::Template(parts) => parts.clone() };
        let tpl_node = tpl_tok.try_map(move |parts: RawTpl, span| {
            resolve_template(parts)
                .map(|resolved| Node {
                    kind: EKind::Template(resolved),
                    span: sp(span),
                })
                .map_err(|msg| Rich::custom(span, msg))
        });

        let chain = {
            let expr = expr.clone();
            recursive(move |chain| {
                let new_atom = just(kw("new"))
                    .ignore_then(chain.clone())
                    .then(call_args.clone().or_not())
                    .map_with(|(callee, args), e| {
                        node(
                            EKind::New {
                                callee: Box::new(callee),
                                args,
                            },
                            e.span(),
                        )
                    });

                let arrow_body = choice((
                    block_body.clone().map(Body::Block),
                    expr.clone().map(|e| Body::Expr(Box::new(e))),
                ));

                let group_elem = choice((
                    punct("...")
                        .ignore_then(expr.clone())
                        .map_with(|inner, e| Node {
                            kind: EKind::Spread(Box::new(inner)),
                            span: sp(e.span()),
                        }),
                    expr.clone(),
                ));
                let paren_group = group_elem
                    .separated_by(punct(","))
                    .allow_trailing()
                    .collect::<Vec<_>>()
                    .delimited_by(punct("("), punct(")"));

                let paren_arrow = paren_group
                    .clone()
                    .then(punct("=>").ignore_then(arrow_body.clone()))
                    .try_map(|(elems, body), span| {
                        let mut params = Vec::with_capacity(elems.len());
                        for el in elems {
                            params
                                .push(elem_to_pattern(el).map_err(|msg| Rich::custom(span, msg))?);
                        }
                        Ok(node(
                            EKind::Arrow {
                                params,
                                body,
                                is_async: false,
                            },
                            span,
                        ))
                    });
                let ident_arrow = select_ref! { ETok::Ident(s) => s.clone() }
                    .then(punct("=>").ignore_then(arrow_body.clone()))
                    .map_with(|(name, body), e| {
                        node(
                            EKind::Arrow {
                                params: vec![Pat::Ident(name)],
                                body,
                                is_async: false,
                            },
                            e.span(),
                        )
                    });
                let async_arrow = just(kw("async"))
                    .ignore_then(choice((paren_arrow.clone(), ident_arrow.clone())))
                    .map_with(|mut n, e| {
                        if let EKind::Arrow { is_async, .. } = &mut n.kind {
                            *is_async = true;
                        }
                        let sp: SP = e.span();
                        n.span.end = sp.end as u32;
                        n
                    });

                let fn_params = pat
                    .separated_by(punct(","))
                    .allow_trailing()
                    .collect::<Vec<_>>()
                    .delimited_by(punct("("), punct(")"));
                let fn_atom = just(kw("function"))
                    .ignore_then(select_ref! { ETok::Ident(s) => s.clone() }.or_not())
                    .ignore_then(fn_params)
                    .then(block_body.map(Body::Block))
                    .map_with(|(params, body), e| node(EKind::Fn { params, body }, e.span()));

                let obj_lit = choice((
                    punct("...").ignore_then(expr.clone()).map(ObjProp::Spread),
                    choice((
                        punct("[")
                            .ignore_then(expr.clone())
                            .then_ignore(punct("]"))
                            .map(Key::Computed),
                        select_ref! { ETok::Ident(s) => Key::Ident(s.clone()) },
                        select_ref! { ETok::Str(s) => Key::Lit(s.clone()) },
                        select_ref! { ETok::Num(s) => Key::Lit(s.clone()) },
                    ))
                    .then_ignore(punct(":"))
                    .then(expr.clone())
                    .map(|(key, value)| ObjProp::Kv { key, value }),
                    select_ref! { ETok::Ident(s) => s.clone() }
                        .map_with(|n, e| ObjProp::Shorthand(node(EKind::Ident(n), e.span()))),
                ))
                .separated_by(punct(","))
                .allow_trailing()
                .collect::<Vec<_>>()
                .delimited_by(punct("{"), punct("}"))
                .map_with(|props, e| node(EKind::Object(props), e.span()));

                let array_lit = choice((
                    punct("...")
                        .ignore_then(expr.clone())
                        .map_with(|inner, e| Node {
                            kind: EKind::Spread(Box::new(inner)),
                            span: sp(e.span()),
                        }),
                    expr.clone(),
                ))
                .separated_by(punct(","))
                .allow_trailing()
                .collect::<Vec<_>>()
                .delimited_by(punct("["), punct("]"))
                .map_with(|items, e| {
                    node(
                        EKind::Array(items.into_iter().map(Some).collect::<Vec<_>>()),
                        e.span(),
                    )
                });

                // `as` type annotations: primitives, `T[]`, `T | null`.
                let prim_ty = choice((
                    just(kw("number")).to(TypeAst::Number),
                    just(kw("string")).to(TypeAst::Str),
                    just(kw("boolean")).to(TypeAst::Bool),
                    just(kw("null")).to(TypeAst::Null),
                    just(kw("undefined")).to(TypeAst::Undefined),
                    just(kw("unknown")).to(TypeAst::Unknown),
                ));
                let as_type = prim_ty
                    .then(
                        punct("[")
                            .ignore_then(punct("]"))
                            .repeated()
                            .collect::<Vec<_>>(),
                    )
                    .map(|(base, arrs)| {
                        arrs.into_iter()
                            .fold(base, |t, ()| TypeAst::Array(Box::new(t)))
                    })
                    .then(
                        punct("|")
                            .ignore_then(choice((
                                just(kw("null")).to(()),
                                just(kw("undefined")).to(()),
                            )))
                            .or_not(),
                    )
                    .map(|(base, nullish)| match nullish {
                        Some(()) => TypeAst::Nullable(Box::new(base)),
                        None => base,
                    });

                let raw_js = select_ref! { ETok::RawJs(_) => () }
                    .map_with(|_, e| node(EKind::RawJs, e.span()));

                let primary = choice((
                    fn_atom,
                    async_arrow,
                    new_atom,
                    tpl_node,
                    raw_js,
                    select_ref! { ETok::Num(t) => t.clone() }
                        .map_with(|t, e| node(EKind::Num(t), e.span())),
                    select_ref! { ETok::Str(t) => t.clone() }
                        .map_with(|t, e| node(EKind::Str(t), e.span())),
                    just(kw("true")).map_with(|_, e| node(EKind::Bool(true), e.span())),
                    just(kw("false")).map_with(|_, e| node(EKind::Bool(false), e.span())),
                    just(kw("null")).map_with(|_, e| node(EKind::Null, e.span())),
                    just(kw("undefined")).map_with(|_, e| node(EKind::Undefined, e.span())),
                    just(kw("this")).map_with(|_, e| node(EKind::This, e.span())),
                    just(kw("super")).map_with(|_, e| node(EKind::Super, e.span())),
                    select_ref! { ETok::Ident(s) => s.clone() }
                        .then(punct("=>").ignore_then(arrow_body.clone()))
                        .map_with(|(name, body), e| {
                            node(
                                EKind::Arrow {
                                    params: vec![Pat::Ident(name)],
                                    body,
                                    is_async: false,
                                },
                                e.span(),
                            )
                        }),
                    paren_arrow.clone(),
                    obj_lit,
                    array_lit,
                    paren_group.try_map(|elems, span| match elems.len() {
                        0 => Err(Rich::custom(span, "empty parentheses")),
                        1 => Ok(node(
                            EKind::Paren(Box::new(elems.into_iter().next().unwrap())),
                            span,
                        )),
                        _ => Ok(node(EKind::Seq(elems), span)),
                    }),
                    select_ref! { ETok::Ident(s) => s.clone() }
                        .map_with(|n, e| node(EKind::Ident(n), e.span())),
                ));

                let member_name = select_ref! { ETok::Ident(s) => s.clone() };
                let post_op = choice((
                    punct(".")
                        .ignore_then(member_name)
                        .map(|n| PostOp::Member(n, false)),
                    punct("?.").ignore_then(choice((
                        member_name.map(|n| PostOp::Member(n, true)),
                        punct("[")
                            .ignore_then(expr.clone())
                            .then_ignore(punct("]"))
                            .map(|i| PostOp::Index(i, true)),
                        call_args.clone().map(|args| PostOp::Call(args, true)),
                    ))),
                    punct("[")
                        .ignore_then(expr.clone())
                        .then_ignore(punct("]"))
                        .map(|i| PostOp::Index(i, false)),
                    call_args.map(|args| PostOp::Call(args, false)),
                    tpl_tok
                        .try_map(move |parts: RawTpl, span| {
                            resolve_template(parts)
                                .map(|resolved| Node {
                                    kind: EKind::Template(resolved),
                                    span: sp(span),
                                })
                                .map_err(|msg| Rich::custom(span, msg))
                        })
                        .map(PostOp::Tpl),
                    punct("++").to(PostOp::Update(UpdateOp::Inc)),
                    punct("--").to(PostOp::Update(UpdateOp::Dec)),
                    just(kw("as")).ignore_then(as_type).map(PostOp::As),
                ));

                primary.foldl_with(post_op.repeated(), |lhs, op, e| {
                    let sp: SimpleSpan = e.span();
                    let span = crate::span::Span {
                        start: lhs.span.start,
                        end: sp.end as u32,
                    };
                    let kind = match op {
                        PostOp::Member(name, optional) => EKind::Member {
                            obj: Box::new(lhs),
                            name,
                            optional,
                        },
                        PostOp::Index(index, optional) => EKind::Index {
                            obj: Box::new(lhs),
                            index: Box::new(index),
                            optional,
                        },
                        PostOp::Call(args, optional) => EKind::Call {
                            callee: Box::new(lhs),
                            args,
                            optional,
                        },
                        PostOp::Tpl(tpl) => EKind::TaggedTpl {
                            tag: Box::new(lhs),
                            tpl: Box::new(tpl),
                        },
                        PostOp::Update(uop) => EKind::Update {
                            op: uop,
                            prefix: false,
                            target: Box::new(lhs),
                        },
                        PostOp::As(ty) => EKind::Assert {
                            expr: Box::new(lhs),
                            ty,
                        },
                    };
                    Node { kind, span }
                })
            })
        };

        let pre_op = choice((
            punct("!").to(PreOp::Unary(UnaryOp::Not)),
            punct("~").to(PreOp::Unary(UnaryOp::BitNot)),
            punct("-").to(PreOp::Unary(UnaryOp::Neg)),
            punct("+").to(PreOp::Unary(UnaryOp::Pos)),
            just(kw("typeof")).to(PreOp::Unary(UnaryOp::Typeof)),
            just(kw("void")).to(PreOp::Unary(UnaryOp::Void)),
            just(kw("delete")).to(PreOp::Unary(UnaryOp::Delete)),
            just(kw("await")).to(PreOp::Unary(UnaryOp::Await)),
            punct("++").to(PreOp::Update(UpdateOp::Inc)),
            punct("--").to(PreOp::Update(UpdateOp::Dec)),
        ));

        let bin_fold = |lhs, op, rhs, e: &mut chumsky::input::MapExtra<'a, '_, I, Ex<'a>>| {
            node(
                EKind::Binary {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
                e.span(),
            )
        };

        let pratted = chain.pratt(vec![
            prefix(17u16, pre_op, |op, operand, e| match op {
                PreOp::Unary(uop) => node(
                    EKind::Unary {
                        op: uop,
                        expr: Box::new(operand),
                    },
                    e.span(),
                ),
                PreOp::Update(uop) => node(
                    EKind::Update {
                        op: uop,
                        prefix: true,
                        target: Box::new(operand),
                    },
                    e.span(),
                ),
            })
            .boxed(),
            infix(right(15u16), punct("**").to(BinOp::Pow), bin_fold).boxed(),
            infix(
                left(14u16),
                choice((
                    punct("*").to(BinOp::Mul),
                    punct("/").to(BinOp::Div),
                    punct("%").to(BinOp::Rem),
                )),
                bin_fold,
            )
            .boxed(),
            infix(
                left(13u16),
                choice((punct("+").to(BinOp::Add), punct("-").to(BinOp::Sub))),
                bin_fold,
            )
            .boxed(),
            infix(
                left(12u16),
                choice((
                    punct("<<").to(BinOp::Shl),
                    punct(">>").to(BinOp::Shr),
                    punct(">>>").to(BinOp::UShr),
                )),
                bin_fold,
            )
            .boxed(),
            infix(
                left(11u16),
                choice((
                    punct("<").to(BinOp::Lt),
                    punct(">").to(BinOp::Gt),
                    punct("<=").to(BinOp::LtE),
                    punct(">=").to(BinOp::GtE),
                    just(kw("in")).to(BinOp::In),
                    just(kw("instanceof")).to(BinOp::Instanceof),
                )),
                bin_fold,
            )
            .boxed(),
            infix(
                left(10u16),
                choice((punct("==").to(BinOp::EqEq), punct("!=").to(BinOp::NotEq))),
                bin_fold,
            )
            .boxed(),
            infix(left(9u16), punct("&").to(BinOp::BitAnd), bin_fold).boxed(),
            infix(left(8u16), punct("^").to(BinOp::BitXor), bin_fold).boxed(),
            infix(left(7u16), punct("|").to(BinOp::BitOr), bin_fold).boxed(),
            infix(left(6u16), punct("&&").to(BinOp::And), bin_fold).boxed(),
            infix(left(5u16), punct("||").to(BinOp::Or), bin_fold).boxed(),
            infix(left(5u16), punct("??").to(BinOp::Nullish), bin_fold).boxed(),
        ]);

        let cond = pratted
            .then(
                punct("?")
                    .ignore_then(expr.clone())
                    .then_ignore(punct(":"))
                    .then(expr.clone())
                    .or_not(),
            )
            .map_with(|(test, rest), e| match rest {
                Some((cons, alt)) => node(
                    EKind::Cond {
                        test: Box::new(test),
                        cons: Box::new(cons),
                        alt: Box::new(alt),
                    },
                    e.span(),
                ),
                None => test,
            });

        // ----- assignment layer -----
        let assign_op = select_ref! { ETok::Punct(s) if AssignOp::from_punct(s).is_some() =>
            AssignOp::from_punct(s).unwrap_or(AssignOp::Assign)
        };
        cond.then(assign_op.then(expr).or_not())
            .map_with(|(target, rhs), e| match rhs {
                Some((op, value)) => node(
                    EKind::Assign {
                        op,
                        target: Box::new(target),
                        value: Box::new(value),
                    },
                    e.span(),
                ),
                None => target,
            })
            .labelled("expression")
    });

    let seq_tail = punct(",")
        .ignore_then(expr_full.clone())
        .repeated()
        .collect::<Vec<_>>();
    let expr_entry = expr_full
        .clone()
        .then(seq_tail)
        .map_with(|(first, rest), e| {
            if rest.is_empty() {
                first
            } else {
                let mut nodes = Vec::with_capacity(rest.len() + 1);
                nodes.push(first);
                nodes.extend(rest);
                node(EKind::Seq(nodes), e.span())
            }
        })
        .then_ignore(end());

    let body_entry = mk_stmt(expr_full)
        .repeated()
        .collect::<Vec<_>>()
        .then_ignore(end());

    (expr_entry, body_entry)
}

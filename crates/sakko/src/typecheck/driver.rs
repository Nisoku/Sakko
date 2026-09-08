//! Document walker: runs the checker over every snippet in a parsed tree
//! and collects a [`Report`].

use super::checker::{Checker, Resolved, Scopes};
use super::diag::Code;
use super::report::Report;
use super::types::Ty;
use crate::error::SakkoError;
use crate::saho as x;
use crate::span::Span;
use crate::syntax::ast::{
    AstNode, AtcodeBody, AtcodeDeclaration, BlockSnippet, ElementNode, ExprSnippet, InlineNode,
    InlineValue, Modifier, RootNode,
};
use std::collections::HashSet;

pub fn check_source(src: &str) -> Result<Report, SakkoError> {
    let ast = crate::parse_sakko(src)?;
    Ok(check_root(&ast))
}

pub fn check_ast(ast: &AstNode) -> Report {
    match ast {
        AstNode::Root(root) => check_root(root),
        _ => Report::default(),
    }
}

fn check_root(root: &RootNode) -> Report {
    let mut all = Report::default();
    let mut sc = Scopes::default();
    sc.push();
    let mut seen: HashSet<String> = HashSet::new();

    for decl in &root.declarations {
        match decl {
            AtcodeDeclaration::State {
                declarations,
                line,
                col,
            } => {
                for sv in declarations {
                    let loc = Some((*line, *col));
                    if !seen.insert(sv.name.to_string()) {
                        let mut c =
                            Checker::new(format!("@state '{}'", sv.name), loc, &sv.value.raw);
                        c.report(
                            Code::DuplicateDecl,
                            Span::whole(&sv.value.raw),
                            format!("duplicate declaration of '{}'", sv.name),
                        );
                        c.drain_into(&mut all);
                        continue;
                    }
                    let label = format!("@state '{}'", sv.name);
                    let ty = check_expr_snippet(&sv.value, label, loc, &mut sc, &mut all, false);
                    sc.declare(&sv.name, ty, true);
                }
            }
            AtcodeDeclaration::Derived {
                declarations,
                line,
                col,
            } => {
                for dv in declarations {
                    let loc = Some((*line, *col));
                    if seen.contains(dv.name.as_ref()) {
                        let mut c =
                            Checker::new(format!("@derived '{}'", dv.name), loc, &dv.expr.raw);
                        c.report(
                            Code::DuplicateDecl,
                            Span::whole(&dv.expr.raw),
                            format!("duplicate declaration of '{}'", dv.name),
                        );
                        c.drain_into(&mut all);
                        continue;
                    }
                    seen.insert(dv.name.to_string());
                    let label = format!("@derived '{}'", dv.name);
                    let ty = check_expr_snippet(&dv.expr, label, loc, &mut sc, &mut all, false);
                    sc.declare(&dv.name, ty, false);
                }
            }
            AtcodeDeclaration::Effect { body, line, col } => {
                check_block_snippet(
                    body,
                    "@effect".to_string(),
                    Some((*line, *col)),
                    &mut sc,
                    &mut all,
                    false,
                );
            }
        }
    }

    check_modifiers(&root.modifiers, &mut sc, &mut all);
    for child in &root.children {
        visit_node(child, &mut sc, &mut all);
    }

    all
}

fn visit_node(node: &AstNode, sc: &mut Scopes, out: &mut Report) {
    match node {
        AstNode::Root(r) => {
            check_modifiers(&r.modifiers, sc, out);
            r.children.iter().for_each(|c| visit_node(c, sc, out));
        }
        AstNode::Element(ElementNode {
            modifiers,
            children,
            ..
        }) => {
            // A `@each` binds an item into scope for this element's own
            // and its children's expressions (e.g. event handlers reading
            // `item.done`).
            let each_item = modifiers.iter().find_map(|m| match m {
                Modifier::Atcode {
                    name,
                    body: AtcodeBody::Each(each),
                } if name == "each" => Some(each),
                _ => None,
            });
            sc.push();
            if let Some(each) = each_item {
                let item_ty = each_item_type(each, sc);
                sc.declare(&each.item, item_ty, false);
            }
            check_modifiers(modifiers, sc, out);
            children.iter().for_each(|c| visit_node(c, sc, out));
            sc.pop();
        }
        AstNode::Inline(InlineNode {
            modifiers, value, ..
        }) => {
            check_modifiers(modifiers, sc, out);
            if let InlineValue::Interpolated(it) = value {
                for part in &it.parts {
                    if let crate::syntax::ast::InterpolatedTextPart::Expr { value } = part {
                        check_expr_snippet(
                            value,
                            "text interpolation".to_string(),
                            None,
                            sc,
                            out,
                            true,
                        );
                    }
                }
            }
        }
        AstNode::List(l) => l.items.iter().for_each(|c| visit_node(c, sc, out)),
    }
}

fn check_modifiers(modifiers: &[Modifier], sc: &mut Scopes, out: &mut Report) {
    for m in modifiers {
        match m {
            Modifier::Event { event, handler } => {
                let label = format!("@on:{event}");
                check_block_snippet(handler, label, None, sc, out, true);
            }
            Modifier::Atcode { name, body } => match (name.as_ref(), body) {
                ("bind", AtcodeBody::Text(signal)) => check_signal_target(signal, "@bind", sc, out),
                ("if", AtcodeBody::Expr(expr)) => {
                    check_expr_snippet(expr, "@if".to_string(), None, sc, out, false);
                }
                ("class", AtcodeBody::Expr(expr)) => {
                    check_class_expr(expr, sc, out);
                }
                ("each", AtcodeBody::Each(each)) => check_each(each, sc, out),
                _ => {}
            },
            _ => {}
        }
    }
}

fn check_expr_snippet(
    snip: &ExprSnippet,
    label: String,
    location: Option<(u32, u32)>,
    sc: &mut Scopes,
    out: &mut Report,
    is_interpolation: bool,
) -> Ty {
    let mut c = Checker::new(label.clone(), location, &snip.raw);
    match &snip.parsed {
        Some(node) => {
            let t = c.infer(node, sc);
            if is_interpolation && t == Ty::Function {
                c.report(
                    Code::RenderedFunction,
                    node.span,
                    "function values cannot be interpolated",
                );
            }
            c.drain_into(out);
            t
        }
        None => {
            for e in &snip.errors {
                c.report(Code::SnippetParse, e.span, e.message.clone());
            }
            c.drain_into(out);
            Ty::Any
        }
    }
}

fn check_class_expr(snip: &ExprSnippet, sc: &mut Scopes, out: &mut Report) {
    let mut c = Checker::new("@class".to_string(), None, &snip.raw).with_class_mode();
    match &snip.parsed {
        Some(node) => {
            let t = c.infer(node, sc);
            let valid = match &t {
                Ty::Str | Ty::Array(_) | Ty::Any | Ty::Unknown => true,
                Ty::Union(members) => members
                    .iter()
                    .all(|m| matches!(m, Ty::Str | Ty::Null | Ty::Undefined)),
                _ => false,
            };
            if !valid {
                c.report(
                    Code::BadClassType,
                    node.span,
                    format!(
                        "class expression must yield a string or an array of strings, got '{t}'"
                    ),
                );
            }
            c.drain_into(out);
        }
        None => {
            for e in &snip.errors {
                c.report(Code::SnippetParse, e.span, e.message.clone());
            }
            c.drain_into(out);
        }
    }
}

fn check_block_snippet(
    snip: &BlockSnippet,
    label: String,
    location: Option<(u32, u32)>,
    sc: &mut Scopes,
    out: &mut Report,
    with_event_param: bool,
) {
    let mut c = Checker::new(label, location, &snip.raw);
    sc.push();
    if with_event_param {
        sc.declare("e", Ty::Unknown, true);
    }
    match &snip.parsed {
        Some(stmts) => c.check_stmts(stmts, sc),
        None => {
            for e in &snip.errors {
                c.report(Code::SnippetParse, e.span, e.message.clone());
            }
        }
    }
    sc.pop();
    c.drain_into(out);
}

fn check_signal_target(name: &str, label: &str, sc: &Scopes, out: &mut Report) {
    if !matches!(sc.lookup(name), Some(Resolved::Var(_))) {
        let mut c = Checker::new(label.to_string(), None, name);
        c.report(
            Code::BadBindTarget,
            Span::whole(name),
            format!("'{name}' is not a declared state or derived variable"),
        );
        c.drain_into(out);
    }
}

fn each_item_type(each: &crate::syntax::ast::EachSpec, sc: &Scopes) -> Ty {
    let src = each
        .source
        .parsed
        .as_deref()
        .map(|node| match &node.kind {
            x::EKind::Ident(n) => n.as_str(),
            _ => each.source.raw.as_ref(),
        })
        .unwrap_or(each.source.raw.as_ref());
    match sc.lookup(src) {
        Some(Resolved::Var(b)) => match b.ty {
            Ty::Array(Some(inner)) => *inner,
            Ty::Array(None) | Ty::Str | Ty::Unknown => Ty::Any,
            other => other,
        },
        _ => Ty::Any,
    }
}

fn check_each(each: &crate::syntax::ast::EachSpec, sc: &Scopes, out: &mut Report) {
    let src = each
        .source
        .parsed
        .as_deref()
        .map(|node| match &node.kind {
            x::EKind::Ident(n) => n.as_str(),
            _ => each.source.raw.as_ref(),
        })
        .unwrap_or(each.source.raw.as_ref());
    let ty = sc.lookup(src).map(|r| match r {
        Resolved::Var(b) => b.ty,
        Resolved::Ns(_) => Ty::Function,
    });
    if !matches!(ty, Some(Ty::Array(_) | Ty::Str | Ty::Any)) {
        let (code, message) = match ty {
            Some(t) => (
                Code::BadEachSource,
                format!("cannot iterate over a value of type '{t}'"),
            ),
            None => (Code::UnknownIdent, format!("unknown identifier '{src}'")),
        };
        let mut c = Checker::new("@each".to_string(), None, &each.source.raw);
        c.report(code, Span::whole(src), message);
        c.drain_into(out);
    }
}

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
    AstNode, AtcodeDeclaration, ElementNode, InlineNode, InlineValue, Modifier, RootNode,
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
                        let mut c = Checker::new(format!("@state '{}'", sv.name), loc, &sv.value);
                        c.report(
                            Code::DuplicateDecl,
                            Span::whole(&sv.value),
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
                        let mut c = Checker::new(format!("@derived '{}'", dv.name), loc, &dv.expr);
                        c.report(
                            Code::DuplicateDecl,
                            Span::whole(&dv.expr),
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
            check_modifiers(modifiers, sc, out);
            children.iter().for_each(|c| visit_node(c, sc, out));
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
            Modifier::Atcode { name, body } => match name.as_ref() {
                "bind" => check_signal_target(body, "@bind", sc, out),
                "if" | "class" => check_defined(body, name.as_ref(), sc, out),
                "each" => check_each(body, sc, out),
                _ => {}
            },
            _ => {}
        }
    }
}

fn check_expr_snippet(
    src: &str,
    label: String,
    location: Option<(u32, u32)>,
    sc: &mut Scopes,
    out: &mut Report,
    is_interpolation: bool,
) -> Ty {
    let mut c = Checker::new(label.clone(), location, src);
    match x::parse(src) {
        Ok(node) => {
            let t = c.infer(&node, sc);
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
        Err(e) => {
            c.report(Code::SnippetParse, e.span, e.message);
            c.drain_into(out);
            Ty::Any
        }
    }
}

fn check_block_snippet(
    src: &str,
    label: String,
    location: Option<(u32, u32)>,
    sc: &mut Scopes,
    out: &mut Report,
    with_event_param: bool,
) {
    let mut c = Checker::new(label, location, src);
    sc.push();
    if with_event_param {
        sc.declare("e", Ty::Unknown, true);
    }
    match x::parse_body(src) {
        Ok(stmts) => c.check_stmts(&stmts, sc),
        Err(diags) => diags
            .into_iter()
            .for_each(|d| c.report(Code::SnippetParse, d.span, d.message)),
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

fn check_defined(name: &str, atname: &str, sc: &Scopes, out: &mut Report) {
    if sc.lookup(name).is_none() {
        let mut c = Checker::new(format!("@{atname}"), None, name);
        c.report(
            Code::UnknownIdent,
            Span::whole(name),
            format!("'{name}' is not defined"),
        );
        c.drain_into(out);
    }
}

fn check_each(body: &str, sc: &Scopes, out: &mut Report) {
    let parts: Vec<&str> = body.split_whitespace().collect();
    let [item, kw, source] = parts[..] else {
        let mut c = Checker::new("@each".to_string(), None, body);
        c.report(
            Code::SnippetParse,
            Span::whole(body),
            "malformed @each expression; expected 'item in source'",
        );
        c.drain_into(out);
        return;
    };
    debug_assert_eq!(kw, "in");
    let _ = item;
    match sc
        .lookup(source)
        .map(|r| match r {
            Resolved::Var(b) => b.ty,
            Resolved::Ns(_) => Ty::Function,
        })
        .filter(|t| matches!(t, Ty::Array(_) | Ty::Str | Ty::Any))
    {
        Some(_) => {}
        None => {
            let (code, message) = match sc.lookup(source).map(|r| match r {
                Resolved::Var(b) => b.ty,
                Resolved::Ns(_) => Ty::Function,
            }) {
                Some(ty) => (
                    Code::BadEachSource,
                    format!("cannot iterate over a value of type '{ty}'"),
                ),
                None => (Code::UnknownIdent, format!("unknown identifier '{source}'")),
            };
            let mut c = Checker::new("@each".to_string(), None, body);
            c.report(code, Span::whole(source), message);
            c.drain_into(out);
        }
    }
}

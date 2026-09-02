//! Saho sublanguage expression parser and AST.
//! The grammar is a subset of JS, with some extensions
//! (e.g. `@effect` blocks, `@import` expressions)

pub mod ast;
pub mod lexer;
pub(crate) mod parser;

pub use ast::{
    Arg, AssignOp, BinOp, Body, EKind, Key, Node, ObjPatProp, ObjProp, Pat, Stmt, TplPart, TypeAst,
    UnaryOp, UpdateOp, VarKw,
};
pub use lexer::{ETok, ExprDiag};

use std::cell::Cell;

use chumsky::{Parser, input::Input, span::SimpleSpan};

thread_local! {
    static TEMPLATE_DEPTH: Cell<usize> = const { Cell::new(0) };
}

const MAX_TEMPLATE_DEPTH: usize = 64;

/// Lower a node back to its exact source bytes.
pub fn lower<'a>(node: &Node, src: &'a str) -> &'a str {
    &src[node.span.start as usize..node.span.end as usize]
}

/// Convert lexed tokens into the `(token, chumsky-span)` pairs the grammar
/// consumes.
fn to_input_tokens(toks: Vec<(ETok, crate::span::Span)>) -> Vec<(ETok, SimpleSpan)> {
    toks.into_iter()
        .map(|(t, s)| (t, SimpleSpan::new(s.start as usize, s.end as usize)))
        .collect()
}

/// Saho equality is always strict
fn reject_banned_eq(toks: &[(ETok, SimpleSpan)]) -> Result<(), ExprDiag> {
    for (t, s) in toks {
        if let ETok::Punct(p @ ("===" | "!==")) = t {
            return Err(ExprDiag::new(
                crate::span::Span::new(s.start, s.end),
                format!("'{p}' does not exist in Saho; '==' is already strict"),
            ));
        }
    }
    Ok(())
}

/// Parse a single expression source; top-level comma sequences become
/// [`EKind::Seq`].
pub fn parse(src: &str) -> Result<Node, ExprDiag> {
    let raw = lexer::lex(src)?;
    let toks = to_input_tokens(raw);
    reject_banned_eq(&toks)?;
    let len = src.len();
    let eoi: SimpleSpan = (len..len).into();

    let (out, errs) = parser::grammar()
        .0
        .parse(toks.as_slice().map(eoi, |(t, s)| (t, s)))
        .into_output_errors();

    match out {
        Some(mut node) if errs.is_empty() => {
            // Widen to the full source so leading comments/whitespace are
            // preserved when lowering slices the input back out.
            node.span = crate::span::Span::new(0, src.len());
            Ok(node)
        }
        _ => Err(errs
            .into_iter()
            .next()
            .map(rich_to_diag)
            .unwrap_or_else(|| incomplete(src))),
    }
}

/// Parse a statement block (`@effect` bodies, function bodies).
pub fn parse_body(src: &str) -> Result<Vec<Stmt>, Vec<ExprDiag>> {
    let raw = lexer::lex(src).map_err(|e| vec![e])?;
    let toks = to_input_tokens(raw);
    if let Err(e) = reject_banned_eq(&toks) {
        return Err(vec![e]);
    }

    // Split at depth-0 semicolons and at depth-0 line breaks (Sakko treats
    // a newline between top-level tokens as a statement boundary).
    let mut bounds: Vec<(usize, bool)> = Vec::new();
    let mut depth = 0usize;
    let mut prev_end = 0usize;
    for (i, (t, s)) in toks.iter().enumerate() {
        match t {
            ETok::Punct("(") | ETok::Punct("[") | ETok::Punct("{") => depth += 1,
            ETok::Punct(")") | ETok::Punct("]") | ETok::Punct("}") => {
                depth = depth.saturating_sub(1);
            }
            _ => {}
        }

        let start = s.start;
        let end = s.end;
        if depth == 0 {
            let is_closer = matches!(t, ETok::Punct("}") | ETok::Punct(")") | ETok::Punct("]"));
            if matches!(t, ETok::Punct(";")) {
                bounds.push((i, true));
            } else if !is_closer && start > prev_end && src[prev_end..start].contains('\n') {
                // A depth-0 line break separates statements, but a closing
                // bracket can never *start* one
                bounds.push((i, false));
            }
        }
        prev_end = prev_end.max(end);
    }

    let mut out = Vec::new();
    let mut diags = Vec::new();
    let len = src.len();
    let eoi: SimpleSpan = (len..len).into();

    let mut start_idx = 0usize;
    for (end, drop_token) in bounds
        .into_iter()
        .chain(std::iter::once((toks.len(), false)))
    {
        let segment = &toks[start_idx..end];
        start_idx = if drop_token { end + 1 } else { end };
        if segment.is_empty() {
            continue;
        }
        let (stmt, errs) = parser::grammar()
            .1
            .parse(segment.map(eoi, |(t, s)| (t, s)))
            .into_output_errors();
        match stmt {
            Some(stmts) => out.extend(stmts),
            None => diags.extend(errs.into_iter().map(rich_to_diag)),
        }
    }

    if diags.is_empty() {
        Ok(out)
    } else {
        Err(diags)
    }
}

/// Parse a template substitution source; used recursively by the grammar.
pub(crate) fn parse_substitution(src: &str) -> Result<Node, ExprDiag> {
    TEMPLATE_DEPTH.with(|depth| {
        let current = depth.get();
        if current >= MAX_TEMPLATE_DEPTH {
            return Err(ExprDiag::new(
                crate::span::Span::new(0, src.len()),
                "template literal nested too deeply",
            ));
        }
        depth.set(current + 1);
        let result = parse(src);
        depth.set(current);
        result
    })
}

fn rich_to_diag(err: chumsky::error::Rich<'_, ETok, SimpleSpan>) -> ExprDiag {
    let span = err.span();
    ExprDiag::new(
        crate::span::Span::new(span.start, span.end),
        err.to_string(),
    )
}

fn incomplete(src: &str) -> ExprDiag {
    ExprDiag::new(
        crate::span::Span::new(0, src.len()),
        "expression parsed with no output",
    )
}

pub mod builtins;
pub mod diag;
pub mod types;

pub use diag::{Code, Diagnostic, Severity};
pub use types::Ty;

use crate::ast::{
    AstNode, AtcodeDeclaration, ElementNode, InlineNode, InlineValue, Modifier, RootNode,
};
use crate::error::SakkoError;
use crate::expr::{self as x, Node, Stmt};
use std::collections::{HashMap, HashSet};

/// One recorded `js { ... }` occurrence. Sakko never checks these bodies
/// semantically
#[derive(Debug, Clone)]
pub struct JsEscape {
    pub kind_label: String,
    pub location: Option<(u32, u32)>,
    /// Byte offsets of the whole block within [`Diagnostic::snippet`].
    pub span: crate::span::Span,
    /// Raw JavaScript source between the braces.
    pub body: String,
}

/// Everything a typecheck run observed.
#[derive(Debug, Clone, Default)]
pub struct Report {
    pub diagnostics: Vec<Diagnostic>,
    pub js_escapes: Vec<JsEscape>,
}

#[derive(Debug, Clone)]
struct Binding {
    ty: Ty,
    mutable: bool,
}

#[derive(Debug, Clone)]
enum Resolved {
    Var(Binding),
    Ns(builtins::Ns),
}

#[derive(Default)]
struct Scopes {
    stack: Vec<HashMap<String, Binding>>,
}

impl Scopes {
    fn push(&mut self) {
        self.stack.push(HashMap::new());
    }

    fn pop(&mut self) {
        self.stack.pop();
    }

    fn declare(&mut self, name: &str, ty: Ty, mutable: bool) {
        if let Some(top) = self.stack.last_mut() {
            top.insert(name.to_string(), Binding { ty, mutable });
        }
    }

    fn lookup(&self, name: &str) -> Option<Resolved> {
        for scope in self.stack.iter().rev() {
            if let Some(b) = scope.get(name) {
                return Some(Resolved::Var(b.clone()));
            }
        }
        match builtins::lookup_global(name) {
            Some(builtins::Global::Value(ty)) => {
                Some(Resolved::Var(Binding { ty, mutable: false }))
            }
            Some(builtins::Global::Namespace(ns)) => Some(Resolved::Ns(ns)),
            None => None,
        }
    }
}

struct Checker {
    diags: Vec<Diagnostic>,
    js_escapes: Vec<JsEscape>,
    kind_label: String,
    location: Option<(u32, u32)>,
    snippet: String,
    memos: HashMap<(u32, u32), Ty>,
    sigs: HashMap<(u32, u32), Ty>,
}

impl Checker {
    fn new(kind_label: impl Into<String>, location: Option<(u32, u32)>, snippet: &str) -> Self {
        Self {
            diags: Vec::new(),
            js_escapes: Vec::new(),
            kind_label: kind_label.into(),
            location,
            snippet: snippet.to_string(),
            memos: HashMap::new(),
            sigs: HashMap::new(),
        }
    }

    fn report(&mut self, code: Code, span: crate::span::Span, message: impl Into<String>) {
        self.diags.push(Diagnostic {
            severity: Severity::Error,
            code,
            message: message.into(),
            kind_label: self.kind_label.clone(),
            location: self.location,
            span,
            snippet: self.snippet.clone(),
        });
    }

    fn drain_into(self, out: &mut Report) {
        out.diagnostics.extend(self.diags);
        out.js_escapes.extend(self.js_escapes);
    }

    fn infer(&mut self, node: &Node, sc: &mut Scopes) -> Ty {
        let key = (node.span.start, node.span.end);
        if let Some(t) = self.memos.get(&key) {
            return t.clone();
        }
        let ty = self.infer_inner(node, sc);
        self.memos.insert(key, ty.clone());
        ty
    }

    fn infer_inner(&mut self, node: &Node, sc: &mut Scopes) -> Ty {
        use x::EKind::*;
        match &node.kind {
            Ident(name) => self.infer_ident(node, name, sc),
            Bool(_) => Ty::Bool,
            Null => Ty::Null,
            Undefined => Ty::Undefined,
            Num(_) => Ty::Number,
            Str(_) => Ty::Str,
            Template(parts) => {
                for part in parts {
                    if let x::TplPart::Expr(sub) = part {
                        self.infer(sub, sc);
                    }
                }
                Ty::Str
            }
            TaggedTpl { tag, tpl } => {
                self.check_call(tag, sc);
                self.infer(tpl, sc);
                Ty::Any
            }
            RawJs => {
                let raw = x::lower(node, &self.snippet);
                let start = raw.find('{').map_or(raw.len(), |i| i + 1);
                let body = &raw[start..raw.len().saturating_sub(1)];
                self.js_escapes.push(JsEscape {
                    kind_label: self.kind_label.clone(),
                    location: self.location,
                    span: node.span,
                    body: body.to_string(),
                });
                Ty::Unknown
            }
            Assert { expr, ty } => {
                let inner = self.infer(expr, sc);
                let target = ty.to_ty();
                if impossible_cast(&inner, &target) {
                    self.report(
                        Code::ImpossibleCast,
                        node.span,
                        format!("cannot cast '{inner}' to '{target}'"),
                    );
                    return Ty::Any;
                }
                target
            }
            This | Super | NewTarget => {
                let src = x::lower(node, &self.snippet);
                self.report(
                    Code::UnknownIdent,
                    node.span,
                    format!("'{src}' is not available in Sakko snippets"),
                );
                Ty::Any
            }
            Array(items) => {
                let mut elem: Option<Ty> = None;
                for item in items.iter().flatten() {
                    let t = self.infer(item, sc);
                    elem = Some(match elem {
                        Some(prev) => Ty::union(prev, t),
                        None => t,
                    });
                }
                Ty::Array(elem.map(Box::new))
            }
            Object(props) => {
                for p in props {
                    match p {
                        x::ObjProp::Kv { value, .. } => {
                            self.infer(value, sc);
                        }
                        x::ObjProp::Shorthand(v) | x::ObjProp::Spread(v) => {
                            self.infer(v, sc);
                        }
                    }
                }
                Ty::Object
            }
            Fn { params, body } | Arrow { params, body, .. } => {
                sc.push();
                self.bind_pats(params, None, true, sc);
                match body {
                    x::Body::Expr(e) => {
                        self.infer(e, sc);
                    }
                    x::Body::Block(stmts) => self.check_stmts(stmts, sc),
                }
                sc.pop();
                Ty::Function
            }
            Call { callee, args, .. } => {
                let ret = self.callee_ret(callee, sc);
                self.infer_args(args, sc);
                ret
            }
            New { callee, args } => {
                let ret = self.callee_ret(callee, sc);
                if let Some(args) = args {
                    self.infer_args(args, sc);
                }
                ret
            }
            Member {
                obj,
                name,
                optional,
            } => self.infer_member(obj, name, *optional, sc),
            Index {
                obj,
                index,
                optional,
            } => {
                let ot = self.infer(obj, sc);
                self.infer(index, sc);
                let base = match ot {
                    Ty::Array(Some(e)) => (*e).clone(),
                    Ty::Array(None) | Ty::Object | Ty::Any => Ty::Any,
                    Ty::Unknown => Ty::Unknown,
                    Ty::Str => Ty::Str,
                    other => {
                        self.report(
                            Code::UnknownProp,
                            node.span,
                            format!("cannot index into a value of type '{other}'"),
                        );
                        return Ty::Any;
                    }
                };
                if *optional {
                    Ty::union(base, Ty::Undefined)
                } else {
                    base
                }
            }
            Unary { op, expr } => {
                let t = self.infer(expr, sc);
                match op {
                    x::UnaryOp::Neg | x::UnaryOp::Pos | x::UnaryOp::BitNot => {
                        if !t.is_numeric_operand() {
                            self.report(
                                Code::BadUnaryOperand,
                                expr.span,
                                format!(
                                    "operator '{}' cannot be applied to type '{t}'",
                                    op.symbol()
                                ),
                            );
                        }
                        Ty::Number
                    }
                    x::UnaryOp::Not => Ty::Bool,
                    x::UnaryOp::Typeof => Ty::Str,
                    x::UnaryOp::Void => Ty::Undefined,
                    x::UnaryOp::Delete => Ty::Bool,
                    x::UnaryOp::Await => Ty::Any,
                }
            }
            Update { op, target, .. } => {
                self.require_lvalue(target);
                self.require_mutable(target, sc);
                let t = self.infer(target, sc);
                if !t.is_numeric_operand() {
                    self.report(
                        Code::AssignMismatch,
                        target.span,
                        format!(
                            "cannot apply '{}' to a value of type '{t}'",
                            update_symbol(*op)
                        ),
                    );
                }
                Ty::Number
            }
            Binary { op, lhs, rhs } => self.infer_binary(node, *op, lhs, rhs, sc),
            Assign { op, target, value } => self.infer_assign(*op, target, value, sc),
            Cond { test, cons, alt } => {
                self.infer(test, sc);
                let c = self.infer(cons, sc);
                let a = self.infer(alt, sc);
                Ty::union(c, a)
            }
            Seq(items) => {
                let mut last = Ty::Undefined;
                for item in items {
                    last = self.infer(item, sc);
                }
                last
            }
            Paren(inner) => self.infer(inner, sc),
            Spread(inner) => self.infer(inner, sc),
        }
    }

    fn infer_ident(&mut self, node: &Node, name: &str, sc: &Scopes) -> Ty {
        match sc.lookup(name) {
            Some(Resolved::Var(b)) => b.ty,
            Some(Resolved::Ns(_)) => Ty::Function,
            None => {
                self.report(
                    Code::UnknownIdent,
                    node.span,
                    format!("unknown identifier '{name}'"),
                );
                Ty::Any
            }
        }
    }

    fn check_call(&mut self, callee: &Node, sc: &mut Scopes) -> Ty {
        let ty = self.infer(callee, sc);
        match ty {
            // `unknown` values stay callable: opaque hosts (DOM nodes,
            // timers) are driven through methods by design. The result is
            // equally unknown.
            Ty::Function | Ty::Any | Ty::Unknown => ty,
            other => {
                self.report(
                    Code::NotCallable,
                    callee.span,
                    format!("value of type '{other}' is not callable"),
                );
                Ty::Any
            }
        }
    }

    fn callee_ret(&mut self, callee: &Node, sc: &mut Scopes) -> Ty {
        let ty = self.check_call(callee, sc);
        if ty == Ty::Unknown {
            return Ty::Unknown;
        }
        let key = (callee.span.start, callee.span.end);
        self.sigs.get(&key).cloned().unwrap_or(Ty::Any)
    }

    fn infer_args(&mut self, args: &[x::Arg], sc: &mut Scopes) {
        for arg in args {
            match arg {
                x::Arg::Plain(n) | x::Arg::Spread(n) => {
                    self.infer(n, sc);
                }
            }
        }
    }

    fn infer_member(&mut self, obj: &Node, name: &str, optional: bool, sc: &mut Scopes) -> Ty {
        let ot = self.infer(obj, sc);

        if let x::EKind::Ident(g) = &obj.kind
            && let Some(Resolved::Ns(ns)) = sc.lookup(g)
        {
            return match builtins::ns_member(ns, name) {
                Some(ty) => {
                    let key = (obj.span.start, obj.span.end);
                    self.sigs.insert(key, ty.clone());
                    if optional {
                        Ty::union(Ty::Function, Ty::Undefined)
                    } else {
                        Ty::Function
                    }
                }
                None => {
                    self.report(
                        Code::UnknownProp,
                        name_span(obj.span.end, name),
                        format!("'{g}' has no property '{name}'"),
                    );
                    Ty::Any
                }
            };
        }

        let resolved = match builtins::instance_member(&ot, name) {
            Some(builtins::Member::Prop(t)) => Some((t.clone(), None)),
            Some(builtins::Member::Method(t)) => Some((Ty::Function, Some(t))),
            None => match ot {
                Ty::Object | Ty::Any => Some((Ty::Any, None)),
                _ => None,
            },
        };

        match resolved {
            Some((access_ty, sig)) => {
                if let Some(sig) = sig {
                    let key = (obj.span.start, obj.span.end);
                    self.sigs.insert(key, sig);
                }
                if optional {
                    Ty::union(access_ty, Ty::Undefined)
                } else {
                    access_ty
                }
            }
            None => {
                self.report(
                    Code::UnknownProp,
                    name_span(obj.span.end, name),
                    format!("type '{ot}' has no property or method '{name}'"),
                );
                Ty::Any
            }
        }
    }

    fn infer_binary(
        &mut self,
        node: &Node,
        op: x::BinOp,
        lhs: &Node,
        rhs: &Node,
        sc: &mut Scopes,
    ) -> Ty {
        use x::BinOp::*;
        let lt = self.infer(lhs, sc);
        let rt = self.infer(rhs, sc);

        let numeric = |l: &Ty, r: &Ty| l.is_numeric_operand() && r.is_numeric_operand();
        let has_unknown = matches!(lt, Ty::Unknown) || matches!(rt, Ty::Unknown);

        match op {
            Add => {
                if numeric(&lt, &rt) {
                    Ty::Number
                } else if lt == Ty::Str && rt == Ty::Str {
                    Ty::Str
                } else if lt == Ty::Any || rt == Ty::Any {
                    Ty::Any
                } else if has_unknown {
                    let hint = if lt == Ty::Str || rt == Ty::Str {
                        "as string"
                    } else {
                        "as number"
                    };
                    self.report(
                        Code::UnknownUse,
                        node.span,
                        format!(
                            "operator '+' cannot be applied to '{lt}' and '{rt}'; assert with '{hint}' first"
                        ),
                    );
                    Ty::Any
                } else {
                    self.report(
                        Code::BadOperand,
                        node.span,
                        format!(
                            "operator '+' cannot be applied to '{lt}' and '{rt}'; use a template literal instead"
                        ),
                    );
                    Ty::Any
                }
            }
            Sub | Mul | Div | Rem | Pow | BitOr | BitXor | BitAnd | Shl | Shr | UShr => {
                if !numeric(&lt, &rt) {
                    if has_unknown {
                        self.report(
                            Code::UnknownUse,
                            node.span,
                            format!(
                                "operator '{}' cannot be applied to '{lt}' and '{rt}'; assert with 'as number' first",
                                bin_symbol(op)
                            ),
                        );
                    } else {
                        self.report(
                            Code::BadOperand,
                            node.span,
                            format!(
                                "operator '{}' cannot be applied to '{lt}' and '{rt}'",
                                bin_symbol(op)
                            ),
                        );
                    }
                }
                Ty::Number
            }
            Lt | Gt | LtE | GtE => {
                let ok = numeric(&lt, &rt) || (lt == Ty::Str && rt == Ty::Str);
                if !ok && !(lt == Ty::Any || rt == Ty::Any) {
                    if has_unknown {
                        self.report(
                            Code::UnknownUse,
                            node.span,
                            format!(
                                "operator '{}' cannot be applied to '{lt}' and '{rt}'; assert with 'as number' first",
                                bin_symbol(op)
                            ),
                        );
                    } else {
                        self.report(
                            Code::BadOperand,
                            node.span,
                            format!(
                                "operator '{}' cannot be applied to '{lt}' and '{rt}'",
                                bin_symbol(op)
                            ),
                        );
                    }
                }
                Ty::Bool
            }
            In | Instanceof | EqEq | NotEq => {
                if matches!(op, EqEq | NotEq) && disjoint_primitives(&lt, &rt) {
                    self.report(
                        Code::BadOperand,
                        node.span,
                        format!("comparing unrelated types '{lt}' and '{rt}' is always false"),
                    );
                }
                Ty::Bool
            }
            And => rt,
            Or | Nullish => match lt.without_nullish() {
                Some(stripped) => Ty::union(stripped, rt),
                None => rt,
            },
        }
    }

    fn infer_assign(
        &mut self,
        op: x::AssignOp,
        target: &Node,
        value: &Node,
        sc: &mut Scopes,
    ) -> Ty {
        use x::AssignOp::*;

        self.require_lvalue(target);
        let mut_ok = self.require_mutable(target, sc);
        let vt = self.infer(value, sc);

        let slot_ty = match &target.kind {
            x::EKind::Ident(name) => match sc.lookup(name) {
                Some(Resolved::Var(b)) => Some(b.ty),
                Some(Resolved::Ns(_)) => {
                    self.report(
                        Code::UnknownIdent,
                        target.span,
                        format!("cannot assign to global '{name}'"),
                    );
                    return vt;
                }
                None => {
                    self.report(
                        Code::UnknownIdent,
                        target.span,
                        format!("unknown identifier '{name}'"),
                    );
                    return vt;
                }
            },
            _ => None,
        };

        if let Some(expected) = &slot_ty
            && mut_ok
        {
            let ok = if op == Assign {
                vt.assigns_to(expected)
            } else {
                compound_ok(op, &vt, expected)
            };
            if !ok {
                let hint = if vt == Ty::Unknown && op == Assign {
                    format!("; assert with 'as {expected}' first")
                } else {
                    String::new()
                };
                self.report(
                    Code::AssignMismatch,
                    value.span,
                    format!("cannot assign a value of type '{vt}' to '{expected}'{hint}"),
                );
            }
        }
        vt
    }

    fn require_lvalue(&mut self, target: &Node) {
        if !matches!(
            target.kind,
            x::EKind::Ident(_) | x::EKind::Member { .. } | x::EKind::Index { .. }
        ) {
            self.report(
                Code::AssignMismatch,
                target.span,
                "invalid assignment target",
            );
        }
    }

    fn require_mutable(&mut self, target: &Node, sc: &Scopes) -> bool {
        if let x::EKind::Ident(name) = &target.kind
            && let Some(Resolved::Var(b)) = sc.lookup(name)
            && !b.mutable
        {
            self.report(
                Code::ConstReassign,
                target.span,
                format!("cannot reassign constant '{name}'"),
            );
            return false;
        }
        true
    }

    fn bind_pats(&mut self, pats: &[x::Pat], hint: Option<Ty>, mutable: bool, sc: &mut Scopes) {
        for p in pats {
            self.bind_pat(p, hint.clone(), mutable, sc);
        }
    }

    fn bind_pat(&mut self, pat: &x::Pat, hint: Option<Ty>, mutable: bool, sc: &mut Scopes) {
        match pat {
            x::Pat::Ident(name) => sc.declare(name, hint.unwrap_or(Ty::Any), mutable),
            x::Pat::Default { pat, init } => {
                let t = self.infer(init, sc);
                self.bind_pat(pat, Some(t), mutable, sc)
            }
            x::Pat::Rest(p) => self.bind_pat(p, None, mutable, sc),
            x::Pat::Array(_) | x::Pat::Object(_) => {
                collect_pat_names(pat, &mut |n| sc.declare(n, Ty::Any, mutable));
            }
        }
    }

    fn check_stmts(&mut self, stmts: &[Stmt], sc: &mut Scopes) {
        for stmt in stmts {
            self.check_stmt(stmt, sc);
        }
    }

    fn check_stmt(&mut self, stmt: &Stmt, sc: &mut Scopes) {
        match stmt {
            Stmt::Expr(e) => {
                self.infer(e, sc);
            }
            Stmt::VarDecl { kw, decls } => {
                let mutable = !matches!(kw, x::VarKw::Const);
                for (pat, init) in decls {
                    let hint = init.as_ref().map(|e| self.infer(e, sc));
                    self.bind_pat(pat, hint, mutable, sc);
                }
            }
            Stmt::Return(e) => {
                if let Some(e) = e {
                    self.infer(e, sc);
                }
            }
        }
    }
}

fn bin_symbol(op: x::BinOp) -> &'static str {
    use x::BinOp::*;
    match op {
        Nullish => "??",
        Or => "||",
        And => "&&",
        BitOr => "|",
        BitXor => "^",
        BitAnd => "&",
        EqEq => "==",
        NotEq => "!=",
        Lt => "<",
        Gt => ">",
        LtE => "<=",
        GtE => ">=",
        In => "in",
        Instanceof => "instanceof",
        Shl => "<<",
        Shr => ">>",
        UShr => ">>>",
        Add => "+",
        Sub => "-",
        Mul => "*",
        Div => "/",
        Rem => "%",
        Pow => "**",
    }
}

fn update_symbol(op: x::UpdateOp) -> &'static str {
    match op {
        x::UpdateOp::Inc => "++",
        x::UpdateOp::Dec => "--",
    }
}

fn compound_ok(op: x::AssignOp, value: &Ty, expected: &Ty) -> bool {
    use x::AssignOp::*;
    match op {
        Add => match expected {
            Ty::Number => value.is_numeric_operand(),
            Ty::Str => value == &Ty::Str,
            Ty::Any => true,
            _ => false,
        },
        Sub | Mul | Div | Rem | Pow | Shl | Shr | UShr | BitAnd | BitOr | BitXor => {
            matches!(expected, Ty::Number | Ty::Any)
        }
        And | Or | Nullish => value.assigns_to(expected),
        Assign => unreachable!(),
    }
}

fn disjoint_primitives(a: &Ty, b: &Ty) -> bool {
    let prim = |t: &Ty| matches!(t, Ty::Number | Ty::Str | Ty::Bool);
    prim(a) && prim(b) && a != b
}

/// A cast is impossible when a known primitive is asserted to a union of
/// unrelated primitives. Anything involving unknown/any/object/array stays
/// trusted: Sakko cannot prove it wrong.
fn impossible_cast(from: &Ty, to: &Ty) -> bool {
    let prim = |t: &Ty| matches!(t, Ty::Number | Ty::Str | Ty::Bool);
    if !prim(from) {
        return false;
    }
    match to {
        Ty::Union(ms) => ms.iter().all(|m| disjoint_primitives(from, m)),
        other => disjoint_primitives(from, other),
    }
}

fn name_span(after_dot: u32, name: &str) -> crate::span::Span {
    let start = (after_dot + 1) as usize;
    crate::span::Span::new(start, start + name.len())
}

fn collect_pat_names(pat: &x::Pat, f: &mut impl FnMut(&str)) {
    match pat {
        x::Pat::Ident(n) => f(n),
        x::Pat::Default { pat, .. } => collect_pat_names(pat, f),
        x::Pat::Rest(p) => collect_pat_names(p, f),
        x::Pat::Array(pats) => pats.iter().for_each(|p| collect_pat_names(p, f)),
        x::Pat::Object(props) => props.iter().for_each(|p| match p {
            x::ObjPatProp::Kv { pat, .. } | x::ObjPatProp::Shorthand(pat) => {
                collect_pat_names(pat, f)
            }
            x::ObjPatProp::Rest(pat) => collect_pat_names(pat, f),
        }),
    }
}

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
                            whole_span(&sv.value),
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
                            whole_span(&dv.expr),
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
                    if let crate::ast::InterpolatedTextPart::Expr { value } = part {
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
            whole_span(name),
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
            whole_span(name),
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
            whole_span(body),
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
            c.report(code, whole_span(source), message);
            c.drain_into(out);
        }
    }
}

fn whole_span(src: &str) -> crate::span::Span {
    crate::span::Span::new(0, src.len())
}

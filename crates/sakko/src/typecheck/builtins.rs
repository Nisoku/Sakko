use super::types::Ty;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ns {
    Math,
    Json,
    NumberCtor,
    StringCtor,
    Console,
}

pub enum Global {
    Value(Ty),
    Namespace(Ns),
}

pub fn lookup_global(name: &str) -> Option<Global> {
    match name {
        "Math" => Some(Global::Namespace(Ns::Math)),
        "JSON" => Some(Global::Namespace(Ns::Json)),
        "Number" => Some(Global::Namespace(Ns::NumberCtor)),
        "String" => Some(Global::Namespace(Ns::StringCtor)),
        "console" => Some(Global::Namespace(Ns::Console)),
        "NaN" | "Infinity" => Some(Global::Value(Ty::Number)),
        // Host-platform functions. Their return values are opaque to
        // Sakko; assert with `as` before typed use. DOM objects
        // (document, window, ...) are deliberately absent: reach them
        // through `js { ... }`.
        "fetch" | "setTimeout" | "clearTimeout" | "setInterval" | "clearInterval" => {
            Some(Global::Value(Ty::Function))
        }
        _ => None,
    }
}

fn method(ret: Ty) -> Option<Member> {
    Some(Member::Method(ret))
}

fn prop(ty: Ty) -> Option<Member> {
    Some(Member::Prop(ty))
}

pub fn ns_member(ns: Ns, name: &str) -> Option<Ty> {
    use Ty::*;
    let num_fn = || Function;
    match ns {
        Ns::Math => match name {
            "PI" | "E" | "LN2" | "LN10" => Some(Number),
            _ if MATH_FNS.contains(&name) => Some(num_fn()),
            _ => None,
        },
        Ns::Json => match name {
            "parse" | "stringify" => Some(Function),
            _ => None,
        },
        Ns::NumberCtor => match name {
            "parseFloat" | "parseInt" | "isNaN" | "isFinite" => Some(Function),
            "MAX_SAFE_INTEGER" | "MIN_SAFE_INTEGER" | "EPSILON" | "MAX_VALUE" | "MIN_VALUE" => {
                Some(Number)
            }
            _ => None,
        },
        Ns::StringCtor => match name {
            "fromCharCode" | "fromCodePoint" | "raw" => Some(Function),
            _ => None,
        },
        Ns::Console => match name {
            "log" | "warn" | "error" | "info" | "debug" | "table" | "trace" => Some(Function),
            _ => None,
        },
    }
}

const MATH_FNS: &[&str] = &[
    "floor", "ceil", "round", "trunc", "abs", "sqrt", "cbrt", "sign", "log", "log2", "log10",
    "exp", "sin", "cos", "tan", "atan", "asin", "acos", "atan2", "pow", "min", "max", "random",
    "hypot",
];

pub enum Member {
    Prop(Ty),
    Method(Ty),
}

pub fn instance_member(recv: &Ty, name: &str) -> Option<Member> {
    use Ty::*;
    match recv {
        Str => str_member(name),
        Number => match name {
            "toFixed" | "toPrecision" | "toString" | "toLocaleString" => method(Str),
            "valueOf" => method(Number),
            _ => None,
        },
        Bool => match name {
            "toString" => method(Str),
            _ => None,
        },
        Array(elem) => array_member(elem.as_deref(), name),
        Any | Unknown => Some(Member::Prop(recv.clone())),
        _ => None,
    }
}

fn str_member(name: &str) -> Option<Member> {
    use Ty::*;
    match name {
        "length" => prop(Number),
        "toUpperCase" | "toLowerCase" | "trim" | "trimStart" | "trimEnd" | "padStart"
        | "padEnd" | "charAt" | "concat" | "repeat" | "slice" | "substring" | "replace"
        | "replaceAll" | "toString" | "valueOf" => method(Str),
        "split" => method(Ty::array_of(Str)),
        "includes" | "startsWith" | "endsWith" => method(Bool),
        "indexOf" | "lastIndexOf" | "charCodeAt" | "codePointAt" => method(Number),
        _ => None,
    }
}

fn array_member(elem: Option<&Ty>, name: &str) -> Option<Member> {
    use Ty::*;
    let elem = elem.cloned().unwrap_or(Any);
    match name {
        "length" => prop(Number),
        "push" | "unshift" => method(Number),
        "pop" | "shift" => method(Ty::union(elem.clone(), Undefined)),
        "slice" | "concat" | "reverse" | "sort" => method(Ty::array_of(elem.clone())),
        "filter" => method(Ty::array_of(elem)),
        "map" => method(Array(None)),
        "join" => method(Str),
        "includes" | "some" | "every" => method(Bool),
        "indexOf" | "lastIndexOf" | "findIndex" => method(Number),
        "find" => method(Ty::union(elem, Undefined)),
        "forEach" => method(Undefined),
        "reduce" => method(Any),
        _ => None,
    }
}

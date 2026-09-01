use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ty {
    Number,
    Str,
    Bool,
    Null,
    Undefined,
    Array(Option<Box<Ty>>),
    Object,
    Function,
    /// A value whose shape Sakko cannot know (raw `js {}` output, the
    /// event parameter). Flows freely through navigation and calls; only
    /// typed consumption (arithmetic, comparison, assignment into a typed
    /// slot) requires an `as` assertion first.
    Unknown,
    Any,
    Union(Vec<Ty>),
}

impl Ty {
    pub fn array_of(elem: Ty) -> Self {
        Ty::Array(Some(Box::new(elem)))
    }

    pub fn union(a: Ty, b: Ty) -> Ty {
        if matches!(a, Ty::Any) || matches!(b, Ty::Any) || a == b {
            return if matches!(a, Ty::Any) { a } else { b };
        }
        let mut members = Vec::new();
        for t in [&a, &b] {
            match t {
                Ty::Union(ms) => members.extend(ms.iter().cloned()),
                other => members.push(other.clone()),
            }
        }
        let mut deduped: Vec<Ty> = Vec::new();
        for m in members {
            if !deduped.contains(&m) {
                deduped.push(m);
            }
        }
        deduped.sort_by_key(rank);
        if deduped.len() == 1 {
            deduped.remove(0)
        } else {
            Ty::Union(deduped)
        }
    }

    pub fn without_nullish(&self) -> Option<Ty> {
        let stripped = match self {
            Ty::Null | Ty::Undefined => None,
            Ty::Union(ms) => {
                let kept: Vec<Ty> = ms
                    .iter()
                    .filter(|m| !matches!(m, Ty::Null | Ty::Undefined))
                    .cloned()
                    .collect();
                if kept.is_empty() { None } else { Some(kept) }
            }
            other => Some(vec![other.clone()]),
        }?;
        let mut ty = stripped[0].clone();
        for m in &stripped[1..] {
            ty = Ty::union(ty, m.clone());
        }
        Some(ty)
    }

    pub fn assigns_to(&self, target: &Ty) -> bool {
        use Ty::*;
        if self == target {
            return true;
        }
        match (self, target) {
            (Any, _) | (_, Any) | (_, Unknown) => true,
            (Union(ms), _) => ms.iter().all(|m| m.assigns_to(target)),
            (_, Union(ts)) => ts.iter().any(|t| self.assigns_to(t)),
            (Array(Some(_)), Array(None)) => true,
            (Array(None), Array(Some(_))) => false,
            (Array(Some(a)), Array(Some(b))) => a.assigns_to(b),
            _ => false,
        }
    }

    pub fn is_numeric_operand(&self) -> bool {
        matches!(self, Ty::Number | Ty::Any)
    }
}

fn rank(t: &Ty) -> u8 {
    match t {
        Ty::Number => 0,
        Ty::Str => 1,
        Ty::Bool => 2,
        Ty::Null => 3,
        Ty::Undefined => 4,
        Ty::Array(_) => 5,
        Ty::Object => 6,
        Ty::Function => 7,
        Ty::Unknown => 8,
        Ty::Any => 9,
        Ty::Union(_) => 10,
    }
}

impl fmt::Display for Ty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Ty::Number => write!(f, "number"),
            Ty::Str => write!(f, "string"),
            Ty::Bool => write!(f, "boolean"),
            Ty::Null => write!(f, "null"),
            Ty::Undefined => write!(f, "undefined"),
            Ty::Array(Some(e)) => write!(f, "{e}[]"),
            Ty::Array(None) => write!(f, "unknown[]"),
            Ty::Object => write!(f, "object"),
            Ty::Function => write!(f, "function"),
            Ty::Unknown => write!(f, "unknown"),
            Ty::Any => write!(f, "any"),
            Ty::Union(ms) => {
                for (i, m) in ms.iter().enumerate() {
                    if i > 0 {
                        write!(f, " | ")?;
                    }
                    write!(f, "{m}")?;
                }
                Ok(())
            }
        }
    }
}

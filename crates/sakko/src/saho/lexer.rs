use crate::span::Span;

/// A lexical token inside an expression source.
#[derive(Debug, Clone, PartialEq)]
pub enum ETok {
    /// Identifier or contextual keyword (`count`, `true`, `new`, `async`...).
    Ident(String),
    /// Numeric literal, raw text (`42`, `1.5e3`, `0xff`, `10n`).
    Num(String),
    /// String literal, raw text including quotes (`"a\n"`).
    Str(String),
    /// Template literal, kept as alternating quasis and substitution
    /// sources (excluding backticks and `${`/`}` delimiters) so the parser
    /// can recursively parse each substitution against the same source.
    Template(Vec<TplPart>),
    Punct(&'static str),
    /// Raw JavaScript body of a `js { ... }` block. The span covers only
    /// the text between the braces; Sakko never tokenizes the contents.
    RawJs(Span),
}

/// One part of a template literal.
#[derive(Debug, Clone, PartialEq)]
pub enum TplPart {
    /// Raw text between substitutions.
    Quasi(String),
    /// One `${ ... }` substitution
    Subst(Subst),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Subst {
    pub text: String,
    /// Absolute `[start, end)` of `text` within the outer expression source.
    pub abs: Span,
}

/// Expression-layer diagnostic. Positions are byte offsets into the
/// expression source; conversion to file-level line/col happens at
/// integration time where the enclosing template is known.
#[derive(Debug, Clone, PartialEq)]
pub struct ExprDiag {
    pub span: Span,
    pub message: String,
}

impl ExprDiag {
    pub fn new(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
        }
    }
}

/// Punctuators, longest first so prefix matching is unambiguous.
const PUNCTS: &[&str] = &[
    ">>>=", "===", "!==", "**=", "<<=", ">>=", ">>>", "...", "&&=", "||=", "??=", "==", "!=", "<=",
    ">=", "&&", "||", "??", "?.", "=>", "++", "--", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=",
    "**", "<<", ">>", "+", "-", "*", "/", "%", "&", "|", "^", "~", "!", "<", ">", "=", "?", ":",
    ";", ",", ".", "(", ")", "[", "]", "{", "}", "#", "@",
];

fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_' || b == b'$'
}

fn is_ident_cont(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'$'
}

/// Lex an expression source into tokens. First error aborts; callers wanting
/// recovery split the source at statement boundaries themselves.
pub fn lex(src: &str) -> Result<Vec<(ETok, Span)>, ExprDiag> {
    let bytes = src.as_bytes();
    let len = bytes.len();
    let mut toks = Vec::new();
    let mut i = 0usize;

    while i < len {
        let b = bytes[i];
        match b {
            b' ' | b'\t' | b'\r' | b'\n' => {
                i += 1;
            }
            b'/' if i + 1 < len && bytes[i + 1] == b'/' => {
                i += 2;
                while i < len && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if i + 1 < len && bytes[i + 1] == b'*' => {
                let start = i;
                i += 2;
                loop {
                    if i + 1 >= len {
                        return Err(ExprDiag::new(
                            Span::new(start, len),
                            "unterminated block comment",
                        ));
                    }
                    if bytes[i] == b'*' && bytes[i + 1] == b'/' {
                        i += 2;
                        break;
                    }
                    i += 1;
                }
            }
            b'"' | b'\'' => {
                let start = i;
                let quote = b;
                i += 1;
                loop {
                    if i >= len {
                        return Err(ExprDiag::new(
                            Span::new(start, len),
                            "unterminated string literal",
                        ));
                    }
                    match bytes[i] {
                        b'\\' => i += 2,
                        b if b == quote => {
                            i += 1;
                            break;
                        }
                        _ => i += 1,
                    }
                }
                toks.push((ETok::Str(src[start..i].to_owned()), Span::new(start, i)));
            }
            b'`' => {
                let start = i;
                let parts = lex_template(src, &mut i)?;
                toks.push((ETok::Template(parts), Span::new(start, i)));
            }
            b'0'..=b'9' | b'.' if b != b'.' || next_is_digit(bytes, i + 1) => {
                let start = i;
                i = scan_number(bytes, i);
                // A lone `.` that only looked numeric (e.g. `.map`) must not
                // be consumed as a number; scan_number guarantees progress.
                debug_assert!(i > start);
                toks.push((ETok::Num(src[start..i].to_owned()), Span::new(start, i)));
            }
            _ if is_ident_start(b) => {
                let start = i;
                i += 1;
                while i < len && is_ident_cont(bytes[i]) {
                    i += 1;
                }
                // `js { ... }`: the `unsafe` of Saho. The
                // identifier `js` followed by a brace switches to a raw
                // scanner; every other use of `js` stays a plain ident.
                if &src[start..i] == "js" {
                    let mut j = i;
                    while j < len && (bytes[j] as char).is_ascii_whitespace() {
                        j += 1;
                    }
                    if j < len && bytes[j] == b'{' {
                        i = scan_raw_js(src, j)?;
                        let inner = Span::new(j + 1, i - 1);
                        toks.push((ETok::RawJs(inner), Span::new(start, i)));
                        continue;
                    }
                }
                toks.push((ETok::Ident(src[start..i].to_owned()), Span::new(start, i)));
            }
            _ => {
                let start = i;
                let matched = PUNCTS.iter().copied().find(|p| src[i..].starts_with(p));
                match matched {
                    Some(p) => {
                        // `?.5 : x` is a conditional on `.5`, not optional chaining.
                        if p == "?." && next_is_digit(bytes, i + 2) {
                            toks.push((ETok::Punct("?"), Span::new(i, i + 1)));
                            i += 1;
                        } else {
                            toks.push((ETok::Punct(p), Span::new(i, i + p.len())));
                            i += p.len();
                        }
                    }
                    None => {
                        let ch = src[start..].chars().next().unwrap_or('\u{fffd}');
                        return Err(ExprDiag::new(
                            Span::new(start, start + ch.len_utf8()),
                            format!("unexpected character `{ch}` in expression"),
                        ));
                    }
                }
            }
        }
    }

    Ok(toks)
}

/// Scan a `js { ... }` body starting at the opening `{`.
fn scan_raw_js(src: &str, open_brace: usize) -> Result<usize, ExprDiag> {
    let bytes = src.as_bytes();
    let len = bytes.len();
    let mut i = open_brace + 1;
    let mut depth = 1usize;

    while i < len {
        match bytes[i] {
            b'{' => {
                depth += 1;
                i += 1;
            }
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(i + 1);
                }
                i += 1;
            }
            b'"' | b'\'' | b'`' => skip_string(bytes, &mut i)?,
            b'/' if i + 1 < len && bytes[i + 1] == b'/' => {
                while i < len && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if i + 1 < len && bytes[i + 1] == b'*' => {
                i += 2;
                while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                i = (i + 2).min(len);
            }
            _ => i += 1,
        }
    }

    Err(ExprDiag::new(
        Span::new(open_brace, len),
        "unterminated `js {` block; missing closing '}'",
    ))
}

fn next_is_digit(bytes: &[u8], i: usize) -> bool {
    bytes.get(i).is_some_and(|b| b.is_ascii_digit())
}

fn scan_number(bytes: &[u8], mut i: usize) -> usize {
    let len = bytes.len();

    // Radix prefixes: 0x, 0o, 0b (+ BigInt suffix).
    if bytes[i] == b'0' && i + 1 < len && matches!(bytes[i + 1], b'x' | b'o' | b'b') {
        i += 2;
        while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
            i += 1;
        }
        if i < len && bytes[i] == b'n' {
            i += 1;
        }
        return i;
    }

    while i < len && (bytes[i].is_ascii_digit() || bytes[i] == b'_') {
        i += 1;
    }
    // Fraction (only when not followed by another ident char like `.map`).
    if i < len && bytes[i] == b'.' && !(i + 1 < len && is_ident_start(bytes[i + 1])) {
        i += 1;
        while i < len && (bytes[i].is_ascii_digit() || bytes[i] == b'_') {
            i += 1;
        }
    }
    // Exponent.
    if i < len && matches!(bytes[i], b'e' | b'E') {
        let mut j = i + 1;
        if j < len && matches!(bytes[j], b'+' | b'-') {
            j += 1;
        }
        if j < len && bytes[j].is_ascii_digit() {
            i = j;
            while i < len && (bytes[i].is_ascii_digit() || bytes[i] == b'_') {
                i += 1;
            }
        }
    }
    // BigInt suffix.
    if i < len && bytes[i] == b'n' {
        i += 1;
    }
    i
}

/// Scan a template literal starting at `` ` `` and advance `i` past the
/// closing backtick. Substitution sources are recorded verbatim; nested
/// templates inside substitutions recurse.
fn lex_template(src: &str, i: &mut usize) -> Result<Vec<TplPart>, ExprDiag> {
    let bytes = src.as_bytes();
    let len = bytes.len();
    let open = *i;
    *i += 1;

    let mut parts = Vec::new();
    let mut quasi_start = *i;

    loop {
        if *i >= len {
            return Err(ExprDiag::new(
                Span::new(open, len),
                "unterminated template literal",
            ));
        }
        match bytes[*i] {
            b'\\' => *i += 2,
            b'`' => {
                parts.push(TplPart::Quasi(src[quasi_start..*i].to_owned()));
                *i += 1;
                return Ok(parts);
            }
            b'$' if *i + 1 < len && bytes[*i + 1] == b'{' => {
                parts.push(TplPart::Quasi(src[quasi_start..*i].to_owned()));
                *i += 2;
                let subst_start = *i;
                let subst_src = scan_substitution(src, i)?;
                let abs = Span::new(subst_start, subst_start + subst_src.len());
                parts.push(TplPart::Subst(Subst {
                    text: subst_src.to_owned(),
                    abs,
                }));
                quasi_start = *i;
            }
            _ => *i += 1,
        }
    }
}

/// Starting just after `${`, scan to the matching `}` accounting for nested
/// braces, strings, comments, and templates. Returns the substitution source
/// and advances `i` past the closing brace.
fn scan_substitution<'a>(src: &'a str, i: &mut usize) -> Result<&'a str, ExprDiag> {
    let bytes = src.as_bytes();
    let len = bytes.len();
    let start = *i;
    let mut depth = 1usize;

    while *i < len {
        match bytes[*i] {
            // Plain grouping characters
            b'(' | b'[' | b')' | b']' => *i += 1,
            b'{' => {
                depth += 1;
                *i += 1;
            }
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    let s = &src[start..*i];
                    *i += 1;
                    return Ok(s);
                }
                *i += 1;
            }
            b'"' | b'\'' => skip_string(bytes, i)?,
            b'`' => {
                lex_template(src, i)?;
            }
            b'/' if *i + 1 < len && bytes[*i + 1] == b'/' => {
                while *i < len && bytes[*i] != b'\n' {
                    *i += 1;
                }
            }
            b'/' if *i + 1 < len && bytes[*i + 1] == b'*' => {
                *i += 2;
                while *i + 1 < len && !(bytes[*i] == b'*' && bytes[*i + 1] == b'/') {
                    *i += 1;
                }
                *i = (*i + 2).min(len);
            }
            _ => *i += 1,
        }
    }

    Err(ExprDiag::new(
        Span::new(start, len),
        "unterminated `${` substitution",
    ))
}

fn skip_string(bytes: &[u8], i: &mut usize) -> Result<(), ExprDiag> {
    let start = *i;
    let quote = bytes[*i];
    *i += 1;
    while *i < bytes.len() {
        match bytes[*i] {
            b'\\' => *i += 2,
            b if b == quote => {
                *i += 1;
                return Ok(());
            }
            _ => *i += 1,
        }
    }
    Err(ExprDiag::new(
        Span::new(start, bytes.len()),
        "unterminated string literal",
    ))
}

impl std::fmt::Display for ETok {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ETok::Ident(s) | ETok::Num(s) | ETok::Str(s) => write!(f, "{s}"),
            ETok::Template(_) => write!(f, "template literal"),
            ETok::RawJs(_) => write!(f, "`js {{` block"),
            ETok::Punct(s) => write!(f, "`{s}`"),
        }
    }
}

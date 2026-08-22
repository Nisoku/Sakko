use crate::ast::{
    AstNode, AtcodeDeclaration, DerivedVar, ElementNode, InlineNode, InlineValue, InterpolatedText,
    InterpolatedTextPart, ListNode, Modifier, RootNode, StateVar,
};
use crate::error::{Result, SakkoError};
use crate::lexer::tokenize;
use crate::token::{Token, TokenKind};
use std::borrow::Cow;

/// Attribute keys eligible for `pair` modifiers
/// Must remain sorted for `is_known_key`'s binary search.
const KNOWN_KEYS: &[&str] = &[
    "active",
    "align-self",
    "alt",
    "bottom",
    "center-point",
    "class",
    "cols",
    "display",
    "flex",
    "float",
    "gap",
    "heading",
    "height",
    "hidden",
    "icon",
    "id",
    "inset",
    "justify-self",
    "label",
    "layout",
    "left",
    "lg:cols",
    "margin",
    "max",
    "md:cols",
    "message",
    "min",
    "name",
    "opacity",
    "open",
    "order",
    "overflow",
    "padding",
    "placeholder",
    "position",
    "radius",
    "right",
    "size",
    "slot",
    "src",
    "step",
    "title",
    "top",
    "transform",
    "transition",
    "type",
    "value",
    "variant",
    "width",
    "z-index",
];

fn is_known_key(key: &str) -> bool {
    KNOWN_KEYS.binary_search(&key).is_ok()
}

/// Event names supporting `@click { ... }` shorthand, mirroring `EVENT_NAMES`.
const EVENT_NAMES: &[&str] = &[
    "blur",
    "change",
    "click",
    "dblclick",
    "drag",
    "drop",
    "focus",
    "input",
    "keydown",
    "keyup",
    "mousedown",
    "mouseenter",
    "mouseleave",
    "mouseup",
    "submit",
    "touchend",
    "touchstart",
];

fn is_event_name(name: &str) -> bool {
    EVENT_NAMES.binary_search(&name).is_ok()
}

pub struct Parser<'a> {
    pub tokens: Vec<Token<'a>>,
    pub position: usize,
    source_len: usize,
    lines: Vec<Cow<'a, str>>,
}

impl<'a> Parser<'a> {
    /// `lines` supplies the snippet text per line. The normal path passes
    /// borrowed lines of the source; the auto-wrap path passes a virtual
    /// wrapper layout (see `parse_sakko`).
    pub fn with_lines(tokens: Vec<Token<'a>>, lines: Vec<Cow<'a, str>>) -> Self {
        let source_len = lines
            .iter()
            .map(|l| l.len() + 1)
            .sum::<usize>()
            .saturating_sub(1);
        Self {
            tokens,
            position: 0,
            source_len,
            lines,
        }
    }

    pub fn new(tokens: Vec<Token<'a>>, source: &'a str) -> Self {
        let lines: Vec<Cow<'a, str>> = source.split('\n').map(Cow::Borrowed).collect();
        Self::with_lines(tokens, lines)
    }

    /// Build an error
    pub fn error_at(&self, msg: impl Into<String>, token: Option<&Token>) -> SakkoError {
        let msg = msg.into();
        let mut err = SakkoError::new(msg.clone());
        err.suggestion = self.get_suggestion(&msg);
        let token = match token {
            Some(t) => t,
            None => return err,
        };
        if self.source_len == 0 {
            return err;
        }
        let line_text = self
            .lines
            .get(token.line as usize - 1)
            .map(Cow::as_ref)
            .unwrap_or("");
        let pointer = format!("{}^", " ".repeat(token.col.saturating_sub(1) as usize));
        err.line = Some(token.line);
        err.col = Some(token.col);
        err.snippet = Some(format!(
            "\n  {}\n  {}",
            line_text.trim_end_matches('\r'),
            pointer
        ));
        err
    }

    fn get_suggestion(&self, msg: &str) -> Option<String> {
        if msg.contains("Unexpected end of input") {
            Some("Check for missing closing brackets".to_string())
        } else if msg.contains("Expected") {
            Some("Add the expected token".to_string())
        } else if msg.contains("Unexpected token") {
            Some("Remove or replace this token".to_string())
        } else {
            None
        }
    }

    fn last_token(&self) -> Option<&Token<'a>> {
        self.tokens.last()
    }

    pub fn peek(&self) -> Option<&Token<'a>> {
        self.tokens.get(self.position)
    }

    pub fn peek_ahead(&self, offset: usize) -> Option<&Token<'a>> {
        self.tokens.get(self.position + offset)
    }

    pub fn peek_ahead_is(&self, kind: TokenKind) -> bool {
        self.peek_ahead(1).is_some_and(|t| t.kind == kind)
    }

    pub fn consume(&mut self) -> Result<Token<'a>> {
        let token = self.peek().cloned();
        match token {
            Some(t) => {
                self.position += 1;
                Ok(t)
            }
            None => {
                let err = self.error_at("Unexpected end of input", self.last_token());
                Err(err)
            }
        }
    }

    pub fn check(&self, kind: TokenKind) -> bool {
        self.peek().is_some_and(|t| t.kind == kind)
    }

    pub fn expect(&mut self, kind: TokenKind, error_msg: Option<&str>) -> Result<Token<'a>> {
        let mismatch = match self.peek() {
            Some(t) if t.kind == kind => None,
            other => Some(match error_msg {
                Some(m) => m.to_string(),
                None => format!(
                    "Expected {} but got {}",
                    kind.ts_name(),
                    other.map(|t| t.kind.ts_name()).unwrap_or("end of input")
                ),
            }),
        };
        match mismatch {
            Some(msg) => Err(self.error_at(msg, self.peek())),
            None => self.consume(),
        }
    }

    pub fn parse_root(&mut self) -> Result<RootNode<'a>> {
        self.expect(TokenKind::Lt, Some("Expected '<'"))?;

        let name_token = self.peek();
        if name_token.map(|t| t.kind) != Some(TokenKind::Ident) {
            let err = self.error_at("Expected identifier after '<'", name_token);
            return Err(err);
        }
        let name = self.consume()?.value;

        let modifiers = if self.check(TokenKind::Lparen) {
            self.parse_modifiers()?
        } else {
            Vec::new()
        };

        self.expect(TokenKind::Lbrace, Some("Expected '{'"))?;

        let mut declarations = Vec::new();
        let mut children = Vec::new();

        while !self.check(TokenKind::Rbrace) {
            if self.peek().is_none() {
                let err = self.error_at("Unexpected end of input, expected '}'", self.last_token());
                return Err(err);
            }

            if self.check(TokenKind::At) {
                let at_token = self.consume()?;
                declarations.push(self.parse_atcode_declaration(&at_token)?);
            } else {
                children.push(self.parse_node()?);
            }

            if self.check(TokenKind::Semi) || self.check(TokenKind::Comma) {
                self.consume()?;
            }
        }

        self.expect(TokenKind::Rbrace, Some("Expected '}'"))?;
        self.expect(TokenKind::Gt, Some("Expected '>'"))?;

        Ok(RootNode {
            name,
            modifiers,
            declarations,
            children,
        })
    }

    fn should_insert_space(current: &str, next: &Token) -> bool {
        if current.is_empty() {
            return false;
        }
        let last_char = current.chars().last().unwrap();
        let next_char = match next.value.chars().next() {
            Some(c) => c,
            None => return false,
        };
        let is_word_end = is_js_word_char(last_char);
        let is_word_start = is_js_word_char(next_char);
        is_word_end && is_word_start
    }

    pub fn parse_block_body(&mut self) -> Result<Cow<'a, str>> {
        let mut body = String::new();
        let mut brace_depth = 0i32;
        let mut prev_line: Option<u32> = None;

        while let Some(token) = self.peek() {
            if token.kind == TokenKind::Rbrace && brace_depth == 0 {
                break;
            }

            if token.kind == TokenKind::Lbrace {
                brace_depth += 1;
            }
            if token.kind == TokenKind::Rbrace {
                brace_depth -= 1;
            }

            let on_new_line = matches!(prev_line, Some(pl) if pl < token.line);
            if on_new_line {
                body.push('\n');
            } else if Self::should_insert_space(&body, token) {
                body.push(' ');
            }

            match token.kind {
                TokenKind::String => {
                    body.push_str(&json_quote(&token.value));
                }
                TokenKind::BacktickString => {
                    body.push('`');
                    body.push_str(&token.value);
                    body.push('`');
                }
                _ => body.push_str(&token.value),
            }
            prev_line = Some(token.line);
            self.consume()?;
        }

        Ok(Cow::Owned(body.trim().to_string()))
    }

    pub fn parse_expression(&mut self) -> Result<Cow<'a, str>> {
        let mut expr = String::new();
        let mut paren_depth = 0i32;
        let mut brace_depth = 0i32;
        let mut bracket_depth = 0i32;

        while let Some(token) = self.peek() {
            if paren_depth == 0 && brace_depth == 0 && bracket_depth == 0 {
                if token.kind == TokenKind::Semi
                    || token.kind == TokenKind::Rbrace
                    || token.kind == TokenKind::Comma
                {
                    break;
                }
                if token.kind == TokenKind::Ident
                    && self.peek_ahead_is(TokenKind::Equals)
                    && self.peek_ahead(2).map(|t| t.kind) != Some(TokenKind::Equals)
                {
                    break;
                }
            }

            match token.kind {
                TokenKind::Lparen => paren_depth += 1,
                TokenKind::Rparen => paren_depth -= 1,
                TokenKind::Lbrace => brace_depth += 1,
                TokenKind::Rbrace => brace_depth -= 1,
                TokenKind::Lbracket => bracket_depth += 1,
                TokenKind::Rbracket => bracket_depth -= 1,
                _ => {}
            }

            if Self::should_insert_space(&expr, token) {
                expr.push(' ');
            }
            if token.kind == TokenKind::String {
                expr.push_str(&json_quote(&token.value));
            } else {
                expr.push_str(&token.value);
            }
            self.consume()?;
        }

        Ok(Cow::Owned(expr.trim().to_string()))
    }

    pub fn parse_node(&mut self) -> Result<AstNode<'a>> {
        let token_kind = self.peek().map(|t| t.kind);
        if token_kind != Some(TokenKind::Ident) {
            let desc = match self.peek() {
                Some(t) => t.kind.ts_name(),
                None => "end of input",
            };
            let err = self.error_at(format!("Expected identifier but got {}", desc), self.peek());
            return Err(err);
        }
        let name = self.consume()?.value;

        let mut modifiers: Vec<Modifier> = Vec::new();

        while self.check(TokenKind::Lparen) || self.check(TokenKind::At) {
            if self.check(TokenKind::Lparen) {
                modifiers.extend(self.parse_modifiers()?);
            }

            if self.check(TokenKind::At) {
                self.consume()?; // consume @
                modifiers.push(self.parse_inline_modifier()?);
            }
        }

        if self.check(TokenKind::Colon) {
            self.consume()?;

            if self.check(TokenKind::Lbracket) {
                let list = self.parse_list()?;
                return Ok(AstNode::Element(ElementNode {
                    name,
                    modifiers,
                    children: vec![AstNode::List(list)],
                }));
            }

            if self.peek().is_none() {
                let err = self.error_at(
                    "Expected value after ':' but got end of input",
                    self.last_token(),
                );
                return Err(err);
            }

            let val_kind = self.peek().unwrap().kind;
            if val_kind == TokenKind::String || val_kind == TokenKind::InterpStart {
                let value = self.parse_interpolated_value()?;
                return Ok(AstNode::Inline(InlineNode {
                    name,
                    modifiers,
                    value,
                }));
            }

            if val_kind == TokenKind::Ident {
                let value = self.consume()?.value;
                return Ok(AstNode::Inline(InlineNode {
                    name,
                    modifiers,
                    value: InlineValue::Plain(value),
                }));
            }

            let desc = self
                .peek()
                .map(|t| t.kind.ts_name())
                .unwrap_or("end of input");
            let err = self.error_at(
                format!("Expected value after ':' but got {}", desc),
                self.peek(),
            );
            return Err(err);
        }

        if self.check(TokenKind::Lbracket) {
            let list = self.parse_list()?;
            return Ok(AstNode::Element(ElementNode {
                name,
                modifiers,
                children: vec![AstNode::List(list)],
            }));
        }

        if self.check(TokenKind::Lbrace) {
            self.consume()?;
            let mut children = Vec::new();

            while !self.check(TokenKind::Rbrace) {
                if self.peek().is_none() {
                    let err =
                        self.error_at("Unexpected end of input, expected '}'", self.last_token());
                    return Err(err);
                }
                children.push(self.parse_node()?);
                if self.check(TokenKind::Semi) {
                    self.consume()?;
                }
                if self.check(TokenKind::Comma) {
                    self.consume()?;
                }
            }

            self.consume()?;
            return Ok(AstNode::Element(ElementNode {
                name,
                modifiers,
                children,
            }));
        }

        // Void element: no colon, braces, or brackets follows.
        Ok(AstNode::Inline(InlineNode {
            name,
            modifiers,
            value: InlineValue::Plain(Cow::Borrowed("")),
        }))
    }

    pub fn parse_modifiers(&mut self) -> Result<Vec<Modifier<'a>>> {
        self.consume()?; // consume (
        let mut modifiers: Vec<Modifier> = Vec::new();

        while !self.check(TokenKind::Rparen) {
            if self.peek().is_none() {
                let err = self.error_at("Unexpected end of input, expected ')'", self.last_token());
                return Err(err);
            }

            if self.check(TokenKind::At) {
                self.consume()?;
                let name_token = self.expect(TokenKind::Ident, None)?;
                let name = name_token.value.clone();

                if name == "on" {
                    self.expect(TokenKind::Colon, None)?;
                    let event_token = self.expect(TokenKind::Ident, None)?;
                    let event = event_token.value;

                    if !self.check(TokenKind::Lbrace) {
                        let err = self.error_at(
                            format!(
                                "Event handlers must use block syntax: @on:{} {{ ... }}",
                                event
                            ),
                            self.peek(),
                        );
                        return Err(err);
                    }
                    self.consume()?;
                    let handler = self.parse_block_body()?;
                    self.expect(TokenKind::Rbrace, None)?;

                    modifiers.push(Modifier::Event { event, handler });
                    continue;
                }

                if name == "bind" {
                    self.expect(TokenKind::Equals, None)?;
                    let signal = if self.check(TokenKind::String) {
                        self.consume()?.value
                    } else {
                        self.expect(TokenKind::Ident, None)?.value
                    };
                    modifiers.push(Modifier::Atcode { name, body: signal });
                    continue;
                }

                if name == "style" {
                    if self.check(TokenKind::String) {
                        let body = self.consume()?.value;
                        modifiers.push(Modifier::Atcode { name, body });
                        continue;
                    }
                    if self.check(TokenKind::Lbrace) {
                        self.consume()?;
                        let body = self.parse_block_body()?;
                        self.expect(TokenKind::Rbrace, None)?;
                        modifiers.push(Modifier::Atcode { name, body });
                        continue;
                    }
                    let err =
                        self.error_at("@style requires a string or object body", Some(&name_token));
                    return Err(err);
                }

                if name == "if" {
                    self.expect(TokenKind::Equals, None)?;
                    let signal = if self.check(TokenKind::String) {
                        self.consume()?.value
                    } else {
                        self.expect(TokenKind::Ident, None)?.value
                    };
                    modifiers.push(Modifier::Atcode { name, body: signal });
                    continue;
                }

                if name == "class" {
                    self.expect(TokenKind::Colon, None)?;
                    let class_token = self.expect(TokenKind::Ident, None)?;
                    modifiers.push(Modifier::Atcode {
                        name,
                        body: class_token.value,
                    });
                    continue;
                }

                if name == "each" {
                    let item = self.expect(TokenKind::Ident, None)?;
                    let in_token =
                        self.expect(TokenKind::Ident, Some("Expected 'in' in @each expression"))?;
                    if in_token.value != "in" {
                        let err = self.error_at(
                            format!(
                                "Expected 'in' in @each expression, got '{}'",
                                in_token.value
                            ),
                            Some(&in_token),
                        );
                        return Err(err);
                    }
                    let source = self.expect(TokenKind::Ident, None)?;
                    modifiers.push(Modifier::Atcode {
                        name,
                        body: Cow::Owned(format!("{} in {}", item.value, source.value)),
                    });
                    continue;
                }

                let err = self.error_at(
                    format!("Atcode @{} not yet supported in modifiers", name),
                    Some(&name_token),
                );
                return Err(err);
            }

            let token_kind = self.peek().map(|t| t.kind);
            if token_kind != Some(TokenKind::Ident) {
                let got = match self.peek() {
                    Some(t) => t.kind.ts_name(),
                    None => "end of input",
                };
                let common_keys: Vec<&str> = KNOWN_KEYS.iter().take(15).copied().collect();
                let suffix = if KNOWN_KEYS.len() > 15 { ", ..." } else { "" };
                let err = self.error_at(
                    format!(
                        "Expected identifier in modifiers but got {}. If you're setting an attribute, known keys include: {}{}",
                        got,
                        common_keys.join(", "),
                        suffix
                    ),
                    self.peek(),
                );
                return Err(err);
            }
            let token = self.consume()?;

            let next = self.peek();
            let next_qualifies = next.is_some_and(|t| {
                (t.kind == TokenKind::Ident || t.kind == TokenKind::String)
                    && !self.check(TokenKind::Rparen)
            });
            if next_qualifies && (is_known_key(&token.value) || token.value.starts_with("data-")) {
                let value = self.consume()?.value;
                modifiers.push(Modifier::Pair {
                    key: token.value,
                    value,
                });
            } else {
                modifiers.push(Modifier::Flag { value: token.value });
            }
        }

        self.consume()?;
        Ok(modifiers)
    }

    pub fn parse_list(&mut self) -> Result<ListNode<'a>> {
        self.consume()?; // consume [
        let mut items: Vec<AstNode> = Vec::new();

        while !self.check(TokenKind::Rbracket) {
            if self.peek().is_none() {
                let err = self.error_at("Unexpected end of input, expected ']'", self.last_token());
                return Err(err);
            }
            items.push(self.parse_node()?);
            if self.check(TokenKind::Comma) {
                self.consume()?;
            } else if !self.check(TokenKind::Rbracket) {
                let err = self.error_at("Expected \",\" or \"]\"", self.peek());
                return Err(err);
            }
        }

        self.consume()?;
        Ok(ListNode { items })
    }

    pub fn parse_interpolated_value(&mut self) -> Result<InlineValue<'a>> {
        let mut parts: Vec<InterpolatedTextPart> = Vec::new();

        while self.check(TokenKind::String) || self.check(TokenKind::InterpStart) {
            if self.check(TokenKind::String) {
                let text = self.consume()?.value;
                if !text.is_empty() {
                    parts.push(InterpolatedTextPart::Text { value: text });
                }
            }

            if self.check(TokenKind::InterpStart) {
                self.consume()?;
                let expr = self.expect(TokenKind::Expr, None)?.value;
                parts.push(InterpolatedTextPart::Expr { value: expr });
                self.expect(TokenKind::InterpEnd, None)?;
            }
        }

        if parts.is_empty() {
            return Ok(InlineValue::Plain(Cow::Borrowed("")));
        }

        if parts.len() == 1
            && let InterpolatedTextPart::Text { value } = &parts[0]
        {
            return Ok(InlineValue::Plain(value.clone()));
        }

        Ok(InlineValue::Interpolated(InterpolatedText::new(parts)))
    }

    // Atcode declarations
    fn parse_atcode_declaration(&mut self, at_token: &Token<'a>) -> Result<AtcodeDeclaration<'a>> {
        let name_token = self.peek();
        if name_token.map(|t| t.kind) != Some(TokenKind::Ident) {
            let err = self.error_at("Expected identifier after @", Some(at_token));
            return Err(err);
        }
        let name = self.consume()?.value;

        match name.as_ref() {
            "state" => self.parse_state_declaration(at_token),
            "effect" => self.parse_effect_declaration(at_token),
            "derived" => self.parse_derived_declaration(at_token),
            _ => Err(SakkoError::new(format!(
                "Unknown atcode '@{}' at line {}, col {}",
                name, at_token.line, at_token.col
            ))),
        }
    }

    fn parse_state_declaration(&mut self, at_token: &Token<'a>) -> Result<AtcodeDeclaration<'a>> {
        let has_braces = self.check(TokenKind::Lbrace);
        if has_braces {
            self.consume()?;
        }

        let mut declarations: Vec<StateVar> = Vec::new();

        loop {
            if self.peek().is_none() {
                break;
            }

            if has_braces && self.check(TokenKind::Rbrace) {
                self.consume()?;
                break;
            }

            if self.check(TokenKind::Ident) && self.peek().map(|t| &*t.value) == Some("const") {
                self.consume()?;
                if self.peek().map(|t| t.kind) != Some(TokenKind::Ident) {
                    let err = self.error_at("Expected identifier after 'const'", self.peek());
                    return Err(err);
                }
            }

            let var_token = self.peek();
            let is_var_decl = var_token.map(|t| t.kind) == Some(TokenKind::Ident)
                && self.peek_ahead_is(TokenKind::Equals);

            if !is_var_decl {
                if declarations.is_empty() {
                    let err = self.error_at("Expected variable declaration", var_token);
                    return Err(err);
                }
                break;
            }

            let var_name = self.consume()?.value; // Consume IDENT

            self.expect(TokenKind::Equals, None)?;
            let value_expr = self.parse_expression()?;
            declarations.push(StateVar {
                name: var_name,
                value: value_expr,
            });

            if self.check(TokenKind::Semi) || self.check(TokenKind::Comma) {
                self.consume()?;
            }
        }

        Ok(AtcodeDeclaration::State {
            declarations,
            line: at_token.line,
            col: at_token.col,
        })
    }

    fn parse_effect_declaration(&mut self, at_token: &Token<'a>) -> Result<AtcodeDeclaration<'a>> {
        if !self.check(TokenKind::Lbrace) {
            let err = self.error_at("@effect requires a braced block", Some(at_token));
            return Err(err);
        }
        self.consume()?;

        let body = self.parse_block_body()?;
        self.expect(TokenKind::Rbrace, None)?;

        Ok(AtcodeDeclaration::Effect {
            body,
            line: at_token.line,
            col: at_token.col,
        })
    }

    fn parse_derived_declaration(&mut self, at_token: &Token<'a>) -> Result<AtcodeDeclaration<'a>> {
        let has_braces = self.check(TokenKind::Lbrace);
        if has_braces {
            self.consume()?;
        }

        let mut declarations: Vec<DerivedVar> = Vec::new();

        loop {
            if self.peek().is_none() {
                break;
            }

            if has_braces && self.check(TokenKind::Rbrace) {
                self.consume()?;
                break;
            }

            if self.check(TokenKind::Ident) && self.peek().map(|t| &*t.value) == Some("const") {
                self.consume()?;
            }

            let var_token = self.peek();
            if var_token.map(|t| t.kind) != Some(TokenKind::Ident) {
                break;
            }

            let var_name = self.consume()?.value;

            if self.check(TokenKind::Equals) {
                self.consume()?;
                let expr = self.parse_expression()?;
                declarations.push(DerivedVar {
                    name: var_name,
                    expr,
                });

                if self.check(TokenKind::Semi) || self.check(TokenKind::Comma) {
                    self.consume()?;
                }
            } else {
                break;
            }
        }

        Ok(AtcodeDeclaration::Derived {
            declarations,
            line: at_token.line,
            col: at_token.col,
        })
    }

    // Inline modifiers
    fn parse_inline_modifier(&mut self) -> Result<Modifier<'a>> {
        let name_token = self.expect(TokenKind::Ident, None)?;
        let name = name_token.value.clone();

        // @on:event { ... }
        if name == "on" {
            self.expect(TokenKind::Colon, None)?;
            let event_token = self.expect(TokenKind::Ident, None)?;
            let event = event_token.value.clone();

            let handler = self.parse_event_handler(&event, Some(&event_token))?;

            return Ok(Modifier::Event { event, handler });
        }

        // @on:eventName (shorthand)
        if is_event_name(&name) {
            let handler = if self.check(TokenKind::Ident) {
                self.consume()?.value
            } else {
                self.parse_event_handler(&name, Some(&name_token))?
            };

            return Ok(Modifier::Event {
                event: name,
                handler,
            });
        }

        // @class:classname
        if name == "class" {
            self.expect(TokenKind::Colon, None)?;
            let class_token = self.expect(TokenKind::Ident, None)?;

            return Ok(Modifier::Atcode {
                name,
                body: class_token.value,
            });
        }

        // @bind="signal"
        if name == "bind" {
            self.expect(TokenKind::Equals, None)?;
            let signal = if self.check(TokenKind::String) {
                self.consume()?.value
            } else {
                self.expect(TokenKind::Ident, None)?.value
            };

            return Ok(Modifier::Atcode { name, body: signal });
        }

        // @style "css-string" | @style { ... }
        if name == "style" {
            if self.check(TokenKind::String) {
                let value = self.consume()?.value;
                return Ok(Modifier::Atcode { name, body: value });
            }
            if self.check(TokenKind::Lbrace) {
                self.consume()?;
                let body = self.parse_block_body()?;
                self.expect(TokenKind::Rbrace, None)?;
                return Ok(Modifier::Atcode { name, body });
            }
            let err = self.error_at("@style requires a string or object body", Some(&name_token));
            return Err(err);
        }

        // @if="signalName"
        if name == "if" {
            self.expect(TokenKind::Equals, None)?;
            let signal = if self.check(TokenKind::String) {
                self.consume()?.value
            } else {
                self.expect(TokenKind::Ident, None)?.value
            };

            return Ok(Modifier::Atcode { name, body: signal });
        }

        // @each item in source
        if name == "each" {
            let item = self.expect(TokenKind::Ident, None)?;
            let in_token =
                self.expect(TokenKind::Ident, Some("Expected 'in' in @each expression"))?;
            if in_token.value != "in" {
                let err = self.error_at(
                    format!(
                        "Expected 'in' in @each expression, got '{}'",
                        in_token.value
                    ),
                    Some(&in_token),
                );
                return Err(err);
            }
            let source = self.expect(TokenKind::Ident, None)?;
            return Ok(Modifier::Atcode {
                name,
                body: Cow::Owned(format!("{} in {}", item.value, source.value)),
            });
        }

        let err = self.error_at(format!("Unknown modifier: @{}", name), Some(&name_token));
        Err(err)
    }

    fn parse_event_handler(
        &mut self,
        event_name: &str,
        event_token: Option<&Token<'a>>,
    ) -> Result<Cow<'a, str>> {
        if self.check(TokenKind::Lbrace) {
            self.consume()?;
            let handler = self.parse_block_body()?;
            self.expect(TokenKind::Rbrace, None)?;
            return Ok(handler);
        }
        let err = self.error_at(
            format!(
                "Event handlers must use block syntax: @on:{} {{ ... }}",
                event_name
            ),
            event_token.or_else(|| self.peek()),
        );
        Err(err)
    }
}

fn is_js_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '$'
}

/// Reproduce `JSON.stringify(str)` output: quotes plus standard escapes.
fn json_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0C}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Parse a Sakko template. Input lacking an explicit `<root { ... }>` wrapper
/// is parsed as a component body using the reserved sentinel wrapper name.
pub fn parse_sakko<'a>(input: &'a str) -> Result<RootNode<'a>> {
    let trimmed = input.trim();

    if trimmed.is_empty() {
        return Err(SakkoError::new("Empty input").with_suggestion("Add some content to parse"));
    }

    let mut tokens = tokenize(input)?;

    let (lines, tokens) = if !trimmed.starts_with('<') {
        let start_byte = input.len() - input.trim_start().len();
        let prefix = &input[..start_byte];
        let leading_lines = prefix.matches('\n').count() as u32;
        // Whitespace chars between the last newline of the prefix and the
        // first non-whitespace char of the trimmed content.
        let first_line_ws = match prefix.rfind('\n') {
            Some(nl) => start_byte - nl - 1,
            None => start_byte,
        } as u32;
        let first_content_line = leading_lines + 1;

        for t in tokens.iter_mut() {
            if t.line == first_content_line {
                t.col = t.col.saturating_sub(first_line_ws).max(1);
            }
            // Drop the stripped leading lines, then add the wrapper header.
            t.line = t.line.saturating_sub(leading_lines).saturating_add(1);
        }

        // Virtual layout: header line 1, trimmed content from line 2.
        let mut lines: Vec<Cow<'a, str>> = Vec::with_capacity(trimmed.split('\n').count() + 2);
        lines.push(Cow::Borrowed("<__sakko_wrapper__ {"));
        lines.extend(trimmed.split('\n').map(Cow::Borrowed));
        lines.push(Cow::Borrowed("}>"));

        let last_line = tokens.last().map(|t| t.line).unwrap_or(2);
        let synthetic = |kind: TokenKind, value: &'static str, col: u32| Token {
            kind,
            value: Cow::Borrowed(value),
            span: Default::default(),
            line: last_line + 1,
            col,
        };
        let mut wrapped = Vec::with_capacity(tokens.len() + 5);
        wrapped.push(Token {
            kind: TokenKind::Lt,
            value: Cow::Borrowed("<"),
            span: Default::default(),
            line: 1,
            col: 1,
        });
        wrapped.push(Token {
            kind: TokenKind::Ident,
            value: Cow::Borrowed(crate::WRAPPER_NAME),
            span: Default::default(),
            line: 1,
            col: 2,
        });
        wrapped.push(Token {
            kind: TokenKind::Lbrace,
            value: Cow::Borrowed("{"),
            span: Default::default(),
            // "<" + "__sakko_wrapper__" + " " = 1 + 17 + 1 chars before '{'
            line: 1,
            col: 2 + crate::WRAPPER_NAME.len() as u32 + 1,
        });
        wrapped.append(&mut tokens);
        wrapped.push(synthetic(TokenKind::Rbrace, "}", 1));
        wrapped.push(synthetic(TokenKind::Gt, ">", 2));
        (lines, wrapped)
    } else {
        let lines: Vec<Cow<'a, str>> = input.split('\n').map(Cow::Borrowed).collect();
        (lines, tokens)
    };

    let mut parser = Parser::with_lines(tokens, lines);
    parser.parse_root()
}

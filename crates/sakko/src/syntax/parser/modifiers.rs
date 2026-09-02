//! Modifier parsing: `(key value ...)` pairs plus inline `@atcode`s.

use crate::error::Result;
use crate::syntax::ast::{AtcodeBody, BlockSnippet, EachSpec, ExprSnippet, Modifier};
use crate::syntax::token::{Token, TokenKind};
use std::borrow::Cow;

use super::Parser;

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

/// Event names supporting `@click { ... }` shorthand.
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

impl<'a> Parser<'a> {
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
                    let handler = self.parse_event_handler(&event, Some(&name_token))?;
                    modifiers.push(Modifier::Event { event, handler });
                    continue;
                }

                if name == "bind" {
                    let signal = self.parse_signal()?;
                    modifiers.push(Modifier::Atcode {
                        name,
                        body: AtcodeBody::Text(signal),
                    });
                    continue;
                }

                if name == "style" {
                    if self.check(TokenKind::String) {
                        let body = self.consume()?.value;
                        modifiers.push(Modifier::Atcode {
                            name,
                            body: AtcodeBody::Text(body),
                        });
                        continue;
                    }
                    if self.check(TokenKind::Lbrace) {
                        self.consume()?;
                        let body = self.parse_block_body()?;
                        self.expect(TokenKind::Rbrace, None)?;
                        modifiers.push(Modifier::Atcode {
                            name,
                            body: AtcodeBody::Text(body),
                        });
                        continue;
                    }
                    let err =
                        self.error_at("@style requires a string or object body", Some(&name_token));
                    return Err(err);
                }

                if name == "if" {
                    self.expect(TokenKind::Equals, None)?;
                    let expr = if self.check(TokenKind::String) {
                        ExprSnippet::parse(self.consume()?.value)
                    } else {
                        self.parse_expr_snippet()?
                    };
                    modifiers.push(Modifier::Atcode {
                        name,
                        body: AtcodeBody::Expr(expr),
                    });
                    continue;
                }

                if name == "class" {
                    let class = self.parse_class()?;
                    modifiers.push(class);
                    continue;
                }

                if name == "each" {
                    let (item, source) = self.parse_each_clause()?;
                    modifiers.push(Modifier::Atcode {
                        name,
                        body: AtcodeBody::Each(EachSpec { item, source }),
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

            // Reactive class expression: an arg that begins an expression
            // rather than a bare flag/pair, e.g.
            // `text(emailValid || !email ? " " : danger)` or
            // `badge(cond ? success : danger large)`. Parsed as a bounded
            // expression; trailing flags after the expression are parsed
            // normally on the next iterations.
            if self.modifier_arg_is_expression() {
                let expr = self.parse_class_expression()?;
                modifiers.push(Modifier::Atcode {
                    name: "class".into(),
                    body: AtcodeBody::Expr(expr),
                });
                continue;
            }

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

    pub(crate) fn parse_inline_modifier(&mut self) -> Result<Modifier<'a>> {
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
                BlockSnippet::parse(self.consume()?.value)
            } else {
                self.parse_event_handler(&name, Some(&name_token))?
            };
            return Ok(Modifier::Event {
                event: name,
                handler,
            });
        }

        // @class:classname | @class="..." | @class={...}
        if name == "class" {
            return self.parse_class();
        }

        // @bind="signal"
        if name == "bind" {
            let signal = self.parse_signal()?;
            return Ok(Modifier::Atcode {
                name,
                body: AtcodeBody::Text(signal),
            });
        }

        // @style "css-string" | @style { ... }
        if name == "style" {
            if self.check(TokenKind::String) {
                let value = self.consume()?.value;
                return Ok(Modifier::Atcode {
                    name,
                    body: AtcodeBody::Text(value),
                });
            }
            if self.check(TokenKind::Lbrace) {
                self.consume()?;
                let body = self.parse_block_body()?;
                self.expect(TokenKind::Rbrace, None)?;
                return Ok(Modifier::Atcode {
                    name,
                    body: AtcodeBody::Text(body),
                });
            }
            let err = self.error_at("@style requires a string or object body", Some(&name_token));
            return Err(err);
        }

        // @if="signalName" | @if=expr
        if name == "if" {
            self.expect(TokenKind::Equals, None)?;
            let expr = if self.check(TokenKind::String) {
                ExprSnippet::parse(self.consume()?.value)
            } else {
                self.parse_expr_snippet()?
            };
            return Ok(Modifier::Atcode {
                name,
                body: AtcodeBody::Expr(expr),
            });
        }

        // @each item in source
        if name == "each" {
            let (item, source) = self.parse_each_clause()?;
            return Ok(Modifier::Atcode {
                name,
                body: AtcodeBody::Each(EachSpec { item, source }),
            });
        }

        let err = self.error_at(format!("Unknown modifier: @{}", name), Some(&name_token));
        Err(err)
    }

    /// `@bind="signal"` (quoted) or `@bind=signal` (bare identifier).
    fn parse_signal(&mut self) -> Result<Cow<'a, str>> {
        self.expect(TokenKind::Equals, None)?;
        if self.check(TokenKind::String) {
            self.consume().map(|t| t.value)
        } else {
            self.expect(TokenKind::Ident, Some("Expected signal name after @bind="))
                .map(|t| t.value)
        }
    }

    /// Returns true when the next modifier arg is a reactive class expression
    /// rather than a bare flag or `key value` pair: an identifier directly
    /// followed by an operator/call token, or a literal/template.
    fn modifier_arg_is_expression(&self) -> bool {
        let Some(tok) = self.peek() else {
            return false;
        };
        match tok.kind {
            TokenKind::String | TokenKind::BacktickString | TokenKind::InterpStart => true,
            TokenKind::Ident => matches!(
                self.peek_ahead(1).map(|n| n.kind),
                Some(
                    TokenKind::Lt
                        | TokenKind::Gt
                        | TokenKind::Lparen
                        | TokenKind::Lbracket
                        | TokenKind::Dot
                        | TokenKind::Plus
                        | TokenKind::Minus
                        | TokenKind::Star
                        | TokenKind::Pipe
                        | TokenKind::Ampersand
                        | TokenKind::Bang
                        | TokenKind::Question
                        | TokenKind::Percent
                        | TokenKind::Equals
                )
            ),
            _ => false,
        }
    }

    /// Parses a reactive class expression, stopping at depth-0 identifiers
    /// that cannot continue it so trailing flags stay separate
    /// (`a ? b : c large` yields expression `a ? b : c` plus flag `large`).
    fn parse_class_expression(&mut self) -> Result<ExprSnippet<'a>> {
        let raw = self.parse_expression_until(|tok, prev| {
            tok.kind == TokenKind::Ident
                && prev.is_some_and(|k| {
                    matches!(
                        k,
                        TokenKind::Ident
                            | TokenKind::String
                            | TokenKind::BacktickString
                            | TokenKind::Rparen
                            | TokenKind::Rbracket
                            | TokenKind::Rbrace
                    )
                })
        })?;
        Ok(ExprSnippet::parse(raw))
    }

    /// `@class:name` or `@class="a b"` (static), `@class={expr}` (reactive).
    fn parse_class(&mut self) -> Result<Modifier<'a>> {
        if self.check(TokenKind::Colon) {
            self.consume()?;
            let name = self.expect(TokenKind::Ident, None)?;
            return Ok(Modifier::Atcode {
                name: "class".into(),
                body: AtcodeBody::Text(name.value),
            });
        }

        let expr = if self.check(TokenKind::String) {
            return Ok(Modifier::Atcode {
                name: "class".into(),
                body: AtcodeBody::Text(self.consume()?.value),
            });
        } else if self.check(TokenKind::Lbrace) {
            self.consume()?;
            let expr = self.parse_expr_snippet()?;
            self.expect(TokenKind::Rbrace, None)?;
            expr
        } else {
            let err = self.error_at(
                "@class requires ':name', a string, or an object body",
                self.peek(),
            );
            return Err(err);
        };

        Ok(Modifier::Atcode {
            name: "class".into(),
            body: AtcodeBody::Expr(expr),
        })
    }

    fn parse_event_handler(
        &mut self,
        event_name: &str,
        event_token: Option<&Token<'a>>,
    ) -> Result<BlockSnippet<'a>> {
        if self.check(TokenKind::Lbrace) {
            self.consume()?;
            let handler = self.parse_block_snippet()?;
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

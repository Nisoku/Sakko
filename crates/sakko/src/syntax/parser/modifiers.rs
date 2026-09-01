//! Modifier parsing: `(key value ...)` pairs plus inline `@atcode`s.

use crate::error::Result;
use crate::syntax::ast::Modifier;
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

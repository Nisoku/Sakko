//! Node parsing: elements, lists, and interpolated inline values.

use crate::error::Result;
use crate::syntax::ast::{
    AstNode, ElementNode, InlineNode, InlineValue, InterpolatedText, InterpolatedTextPart,
    ListNode, Modifier,
};
use crate::syntax::token::TokenKind;
use std::borrow::Cow;

use super::Parser;

impl<'a> Parser<'a> {
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

            let val_kind = match self.peek() {
                Some(t) => t.kind,
                None => return Err(self.error_at("Expected value after ':'", None)),
            };
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
}

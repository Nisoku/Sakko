//! Top-level atcode declarations: `@state`, `@effect`, `@derived`.

use crate::error::{Result, SakkoError};
use crate::syntax::ast::{AtcodeDeclaration, DerivedVar, StateVar};
use crate::syntax::token::{Token, TokenKind};

use super::Parser;

impl<'a> Parser<'a> {
    pub(crate) fn parse_atcode_declaration(
        &mut self,
        at_token: &Token<'a>,
    ) -> Result<AtcodeDeclaration<'a>> {
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
}

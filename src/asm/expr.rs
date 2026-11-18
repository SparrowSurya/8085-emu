//! Static integer expression parser and evaluator for `%define` and conditional directives.
//!
//! Evaluates static expressions composed of integer/char literals, previously defined
//! constants, parentheses, arithmetic operators, bitwise operators, and logical/relational
//! operators using standard C-style precedence.

use super::error::{AsmError, AsmErrorKind, Span};
use super::token::{Token, TokenKind};

/// Evaluates a static expression from a token slice.
///
/// # Arguments
/// * `tokens` - Token slice representing the expression.
/// * `lookup` - Closure to lookup defined constant identifiers (case-insensitive).
/// * `default_span` - Fallback span for errors if tokens are empty.
/// * `allow_undefined` - If true (used in `%if`), undefined identifiers evaluate to 0;
///   if false (used in `%define`), undefined identifiers return [`AsmErrorKind::UndefinedName`].
pub fn eval_expr<F>(
    tokens: &[Token],
    lookup: F,
    default_span: Span,
    allow_undefined: bool,
) -> Result<i64, AsmError>
where
    F: Fn(&str) -> Option<i64>,
{
    let clean: Vec<Token> = tokens
        .iter()
        .filter(|t| t.kind != TokenKind::Newline && t.kind != TokenKind::Eof)
        .cloned()
        .collect();

    if clean.is_empty() {
        return Err(AsmError::new(
            default_span,
            AsmErrorKind::InvalidExpression("empty expression".to_string()),
        ));
    }

    let mut parser = ExprParser::new(&clean, lookup, default_span, allow_undefined);
    let result = parser.parse_expr()?;
    if parser.pos < clean.len() {
        return Err(AsmError::new(
            clean[parser.pos].span,
            AsmErrorKind::InvalidExpression(format!("unexpected token '{:?}'", clean[parser.pos].kind)),
        ));
    }
    Ok(result)
}

struct ExprParser<'a, F> {
    tokens: &'a [Token],
    pos: usize,
    lookup: F,
    default_span: Span,
    allow_undefined: bool,
}

impl<'a, F> ExprParser<'a, F>
where
    F: Fn(&str) -> Option<i64>,
{
    fn new(tokens: &'a [Token], lookup: F, default_span: Span, allow_undefined: bool) -> Self {
        Self {
            tokens,
            pos: 0,
            lookup,
            default_span,
            allow_undefined,
        }
    }

    fn peek(&self) -> Option<&TokenKind> {
        self.tokens.get(self.pos).map(|t| &t.kind)
    }

    fn span(&self) -> Span {
        self.tokens
            .get(self.pos)
            .map(|t| t.span)
            .unwrap_or(self.default_span)
    }

    fn bump(&mut self) -> Option<&TokenKind> {
        let k = self.tokens.get(self.pos).map(|t| &t.kind);
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        k
    }

    fn parse_expr(&mut self) -> Result<i64, AsmError> {
        self.parse_logical_or()
    }

    // ── Precedence 1: Logical OR (||) ──────────────────────────────────────
    fn parse_logical_or(&mut self) -> Result<i64, AsmError> {
        let mut left = self.parse_logical_and()?;
        while let Some(TokenKind::PipePipe) = self.peek() {
            self.bump();
            let right = self.parse_logical_and()?;
            left = if left != 0 || right != 0 { 1 } else { 0 };
        }
        Ok(left)
    }

    // ── Precedence 2: Logical AND (&&) ─────────────────────────────────────
    fn parse_logical_and(&mut self) -> Result<i64, AsmError> {
        let mut left = self.parse_bitwise_or()?;
        while let Some(TokenKind::AmpAmp) = self.peek() {
            self.bump();
            let right = self.parse_bitwise_or()?;
            left = if left != 0 && right != 0 { 1 } else { 0 };
        }
        Ok(left)
    }

    // ── Precedence 3: Bitwise OR (|) ───────────────────────────────────────
    fn parse_bitwise_or(&mut self) -> Result<i64, AsmError> {
        let mut left = self.parse_bitwise_xor()?;
        while let Some(TokenKind::Pipe) = self.peek() {
            self.bump();
            let right = self.parse_bitwise_xor()?;
            left |= right;
        }
        Ok(left)
    }

    // ── Precedence 4: Bitwise XOR (^) ──────────────────────────────────────
    fn parse_bitwise_xor(&mut self) -> Result<i64, AsmError> {
        let mut left = self.parse_bitwise_and()?;
        while let Some(TokenKind::Caret) = self.peek() {
            self.bump();
            let right = self.parse_bitwise_and()?;
            left ^= right;
        }
        Ok(left)
    }

    // ── Precedence 5: Bitwise AND (&) ──────────────────────────────────────
    fn parse_bitwise_and(&mut self) -> Result<i64, AsmError> {
        let mut left = self.parse_equality()?;
        while let Some(TokenKind::Amp) = self.peek() {
            self.bump();
            let right = self.parse_equality()?;
            left &= right;
        }
        Ok(left)
    }

    // ── Precedence 6: Equality (==, !=) ────────────────────────────────────
    fn parse_equality(&mut self) -> Result<i64, AsmError> {
        let mut left = self.parse_relational()?;
        while let Some(k) = self.peek() {
            match k {
                TokenKind::DoubleEqual => {
                    self.bump();
                    let right = self.parse_relational()?;
                    left = if left == right { 1 } else { 0 };
                }
                TokenKind::NotEqual => {
                    self.bump();
                    let right = self.parse_relational()?;
                    left = if left != right { 1 } else { 0 };
                }
                _ => break,
            }
        }
        Ok(left)
    }

    // ── Precedence 7: Relational (<, <=, >, >=) ────────────────────────────
    fn parse_relational(&mut self) -> Result<i64, AsmError> {
        let mut left = self.parse_shift()?;
        while let Some(k) = self.peek() {
            match k {
                TokenKind::Less => {
                    self.bump();
                    let right = self.parse_shift()?;
                    left = if left < right { 1 } else { 0 };
                }
                TokenKind::LessEqual => {
                    self.bump();
                    let right = self.parse_shift()?;
                    left = if left <= right { 1 } else { 0 };
                }
                TokenKind::Greater => {
                    self.bump();
                    let right = self.parse_shift()?;
                    left = if left > right { 1 } else { 0 };
                }
                TokenKind::GreaterEqual => {
                    self.bump();
                    let right = self.parse_shift()?;
                    left = if left >= right { 1 } else { 0 };
                }
                _ => break,
            }
        }
        Ok(left)
    }

    // ── Precedence 8: Shift (<<, >>) ───────────────────────────────────────
    fn parse_shift(&mut self) -> Result<i64, AsmError> {
        let mut left = self.parse_additive()?;
        while let Some(k) = self.peek() {
            match k {
                TokenKind::Shl => {
                    self.bump();
                    let right = self.parse_additive()?;
                    let shift = (right as u64 & 63) as u32;
                    left = if shift >= 64 { 0 } else { left << shift };
                }
                TokenKind::Shr => {
                    self.bump();
                    let right = self.parse_additive()?;
                    let shift = (right as u64 & 63) as u32;
                    left = if shift >= 64 { 0 } else { ((left as u64) >> shift) as i64 };
                }
                _ => break,
            }
        }
        Ok(left)
    }

    // ── Precedence 9: Additive (+, -) ──────────────────────────────────────
    fn parse_additive(&mut self) -> Result<i64, AsmError> {
        let mut left = self.parse_multiplicative()?;
        while let Some(k) = self.peek() {
            match k {
                TokenKind::Plus => {
                    self.bump();
                    let right = self.parse_multiplicative()?;
                    left = left.wrapping_add(right);
                }
                TokenKind::Minus => {
                    self.bump();
                    let right = self.parse_multiplicative()?;
                    left = left.wrapping_sub(right);
                }
                _ => break,
            }
        }
        Ok(left)
    }

    // ── Precedence 10: Multiplicative (*, /, %) ────────────────────────────
    fn parse_multiplicative(&mut self) -> Result<i64, AsmError> {
        let mut left = self.parse_unary()?;
        while let Some(k) = self.peek() {
            match k {
                TokenKind::Star => {
                    self.bump();
                    let right = self.parse_unary()?;
                    left = left.wrapping_mul(right);
                }
                TokenKind::Slash => {
                    let op_span = self.span();
                    self.bump();
                    let right = self.parse_unary()?;
                    if right == 0 {
                        return Err(AsmError::new(op_span, AsmErrorKind::DivisionByZero));
                    }
                    left = match left.checked_div(right) {
                        Some(val) => val,
                        None => return Err(AsmError::new(op_span, AsmErrorKind::DivisionByZero)),
                    };
                }
                TokenKind::Percent => {
                    let op_span = self.span();
                    self.bump();
                    let right = self.parse_unary()?;
                    if right == 0 {
                        return Err(AsmError::new(op_span, AsmErrorKind::DivisionByZero));
                    }
                    left = match left.checked_rem(right) {
                        Some(val) => val,
                        None => return Err(AsmError::new(op_span, AsmErrorKind::DivisionByZero)),
                    };
                }
                _ => break,
            }
        }
        Ok(left)
    }

    // ── Precedence 11: Unary (+, -, ~, !) ──────────────────────────────────
    fn parse_unary(&mut self) -> Result<i64, AsmError> {
        match self.peek() {
            Some(TokenKind::Plus) => {
                self.bump();
                self.parse_unary()
            }
            Some(TokenKind::Minus) => {
                self.bump();
                let val = self.parse_unary()?;
                Ok(val.wrapping_neg())
            }
            Some(TokenKind::Tilde) => {
                self.bump();
                let val = self.parse_unary()?;
                Ok(!val)
            }
            Some(TokenKind::Exclamation) => {
                self.bump();
                let val = self.parse_unary()?;
                Ok(if val == 0 { 1 } else { 0 })
            }
            _ => self.parse_primary(),
        }
    }

    // ── Precedence 12: Primary (Literals, Parens, Identifiers) ─────────────
    fn parse_primary(&mut self) -> Result<i64, AsmError> {
        let sp = self.span();
        match self.peek() {
            Some(TokenKind::LParen) => {
                self.bump();
                let val = self.parse_expr()?;
                if let Some(TokenKind::RParen) = self.peek() {
                    self.bump();
                    Ok(val)
                } else {
                    Err(AsmError::new(
                        self.span(),
                        AsmErrorKind::InvalidExpression("missing closing ')'".to_string()),
                    ))
                }
            }
            Some(TokenKind::Number(n)) => {
                let v = *n as i64;
                self.bump();
                Ok(v)
            }
            Some(TokenKind::Char(c)) => {
                let v = *c as i64;
                self.bump();
                Ok(v)
            }
            Some(TokenKind::Str(s)) => {
                let v = if s.is_empty() { 0 } else { 1 };
                self.bump();
                Ok(v)
            }
            Some(TokenKind::Ident(s)) => {
                let ident = s.clone();
                self.bump();

                // Check for defined(NAME)
                if ident.eq_ignore_ascii_case("defined") && matches!(self.peek(), Some(TokenKind::LParen)) {
                    self.bump(); // consume '('
                    let name = match self.peek() {
                        Some(TokenKind::Ident(n)) => {
                            let n = n.clone();
                            self.bump();
                            n
                        }
                        _ => {
                            return Err(AsmError::new(
                                self.span(),
                                AsmErrorKind::InvalidExpression(
                                    "expected identifier inside defined(...)".to_string(),
                                ),
                            ));
                        }
                    };
                    if !matches!(self.peek(), Some(TokenKind::RParen)) {
                        return Err(AsmError::new(
                            self.span(),
                            AsmErrorKind::InvalidExpression(
                                "missing ')' after defined(NAME)".to_string(),
                            ),
                        ));
                    }
                    self.bump(); // consume ')'

                    let is_def = (self.lookup)(&name.to_ascii_lowercase()).is_some();
                    return Ok(if is_def { 1 } else { 0 });
                }

                // Normal identifier lookup
                if let Some(val) = (self.lookup)(&ident.to_ascii_lowercase()) {
                    Ok(val)
                } else if self.allow_undefined {
                    Ok(0)
                } else {
                    Err(AsmError::new(
                        sp,
                        AsmErrorKind::UndefinedName(ident),
                    ))
                }
            }
            other => Err(AsmError::new(
                sp,
                AsmErrorKind::InvalidExpression(format!("unexpected token '{other:?}'")),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asm::lexer::lex;
    use std::collections::HashMap;

    fn eval(expr_str: &str, defines: &HashMap<String, i64>) -> Result<i64, AsmError> {
        let tokens = lex(expr_str).unwrap();
        eval_expr(
            &tokens,
            |name| defines.get(&name.to_ascii_lowercase()).copied(),
            Span::new(1, 1),
            false,
        )
    }

    #[test]
    fn test_arithmetic_operations() {
        let defs = HashMap::new();
        assert_eq!(eval("10 + 20", &defs).unwrap(), 30);
        assert_eq!(eval("50 - 15", &defs).unwrap(), 35);
        assert_eq!(eval("10 * 12", &defs).unwrap(), 120);
        assert_eq!(eval("100 / 3", &defs).unwrap(), 33);
        assert_eq!(eval("10 % 3", &defs).unwrap(), 1);
    }

    #[test]
    fn test_bitwise_operations() {
        let defs = HashMap::new();
        assert_eq!(eval("1 << 4", &defs).unwrap(), 16);
        assert_eq!(eval("32 >> 2", &defs).unwrap(), 8);
        assert_eq!(eval("0x0F & 0x55", &defs).unwrap(), 0x05);
        assert_eq!(eval("0xF0 | 0x0F", &defs).unwrap(), 0xFF);
        assert_eq!(eval("0xFF ^ 0x0F", &defs).unwrap(), 0xF0);
        assert_eq!(eval("~0 & 0xFF", &defs).unwrap(), 0xFF);
    }

    #[test]
    fn test_precedence_and_parentheses() {
        let defs = HashMap::new();
        assert_eq!(eval("2 + 3 * 4", &defs).unwrap(), 14);
        assert_eq!(eval("(2 + 3) * 4", &defs).unwrap(), 20);
        assert_eq!(eval("1 << 2 + 1", &defs).unwrap(), 8);
        assert_eq!(eval("(1 << 2) + 1", &defs).unwrap(), 5);
        assert_eq!(eval("10 - 2 - 3", &defs).unwrap(), 5);
        assert_eq!(eval("100 / 5 / 2", &defs).unwrap(), 10);
    }

    #[test]
    fn test_identifier_references() {
        let mut defs = HashMap::new();
        defs.insert("rows".to_string(), 10);
        defs.insert("cols".to_string(), 12);
        assert_eq!(eval("ROWS * COLS", &defs).unwrap(), 120);
        assert_eq!(eval("(ROWS + 2) * COLS", &defs).unwrap(), 144);
    }

    #[test]
    fn test_division_by_zero() {
        let defs = HashMap::new();
        assert_eq!(eval("10 / 0", &defs).unwrap_err().kind, AsmErrorKind::DivisionByZero);
        assert_eq!(eval("10 % 0", &defs).unwrap_err().kind, AsmErrorKind::DivisionByZero);
    }

    #[test]
    fn test_undefined_identifier_error() {
        let defs = HashMap::new();
        assert!(matches!(
            eval("ROWS * COLS", &defs).unwrap_err().kind,
            AsmErrorKind::UndefinedName(name) if name == "ROWS"
        ));
    }
}

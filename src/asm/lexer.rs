//! The lexer: turns raw source text into a [`Token`] stream.
//!
//! It recognises the four number bases, double-quoted strings and single-quoted
//! characters (no escape sequences), the punctuation `% , : .`, line boundaries, and
//! identifiers. Whitespace and `;` comments are consumed silently. Reserved words are
//! left as identifiers here and classified during parsing.

use super::error::{AsmError, AsmErrorKind, Span};
use super::token::{Token, TokenKind};

/// Tokenize `src`, returning the full token stream terminated by [`TokenKind::Eof`].
pub fn lex(src: &str) -> Result<Vec<Token>, AsmError> {
    Lexer::new(src).run()
}

struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: u32,
    col: u32,
}

impl Lexer {
    fn new(src: &str) -> Self {
        Lexer {
            chars: src.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek2(&self) -> Option<char> {
        self.chars.get(self.pos + 1).copied()
    }

    fn span(&self) -> Span {
        Span::new(self.line, self.col)
    }

    /// Consume one character, advancing line/column tracking.
    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += 1;
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(c)
    }

    fn run(mut self) -> Result<Vec<Token>, AsmError> {
        let mut out = Vec::new();
        loop {
            match self.peek() {
                None => {
                    out.push(Token::new(TokenKind::Eof, self.span()));
                    return Ok(out);
                }
                Some(c) if c == ' ' || c == '\t' || c == '\r' => {
                    self.bump();
                }
                Some(';') => {
                    // Comment: skip to just before the newline (which stays a token).
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.bump();
                    }
                }
                Some(_) => {
                    let tok = self.next_token()?;
                    out.push(tok);
                }
            }
        }
    }

    fn next_token(&mut self) -> Result<Token, AsmError> {
        let start = self.span();
        let c = self.peek().expect("caller guaranteed a character");
        let kind = match c {
            '\n' => {
                self.bump();
                TokenKind::Newline
            }
            '%' => {
                self.bump();
                TokenKind::Percent
            }
            ',' => {
                self.bump();
                TokenKind::Comma
            }
            ':' => {
                self.bump();
                TokenKind::Colon
            }
            '.' => {
                self.bump();
                TokenKind::Period
            }
            '"' => self.lex_string(start)?,
            '\'' => self.lex_char(start)?,
            c if c.is_ascii_digit() => self.lex_number(start)?,
            c if c.is_ascii_alphabetic() || c == '_' => self.lex_ident(),
            other => {
                return Err(AsmError::new(start, AsmErrorKind::UnexpectedChar(other)));
            }
        };
        Ok(Token::new(kind, start))
    }

    fn lex_ident(&mut self) -> TokenKind {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == '_' {
                s.push(c);
                self.bump();
            } else {
                break;
            }
        }
        TokenKind::Ident(s)
    }

    fn lex_string(&mut self, start: Span) -> Result<TokenKind, AsmError> {
        self.bump(); // opening quote
        let mut s = String::new();
        loop {
            match self.peek() {
                None | Some('\n') => {
                    return Err(AsmError::new(start, AsmErrorKind::UnterminatedString));
                }
                Some('"') => {
                    self.bump(); // closing quote
                    return Ok(TokenKind::Str(s));
                }
                Some(c) => {
                    s.push(c);
                    self.bump();
                }
            }
        }
    }

    fn lex_char(&mut self, start: Span) -> Result<TokenKind, AsmError> {
        self.bump(); // opening quote
        let mut s = String::new();
        loop {
            match self.peek() {
                None | Some('\n') => {
                    return Err(AsmError::new(start, AsmErrorKind::UnterminatedString));
                }
                Some('\'') => {
                    self.bump(); // closing quote
                    if s.is_empty() {
                        return Err(AsmError::new(start, AsmErrorKind::EmptyCharLiteral));
                    } else if s.len() == 1 {
                        let c = s.chars().next().unwrap();
                        if (c as u32) <= 0xFF {
                            return Ok(TokenKind::Char(c as u8));
                        } else {
                            return Ok(TokenKind::Str(s));
                        }
                    } else {
                        // Multi-character single-quoted string (e.g. for %include 'file.e8085')
                        return Ok(TokenKind::Str(s));
                    }
                }
                Some(c) => {
                    s.push(c);
                    self.bump();
                }
            }
        }
    }

    fn lex_number(&mut self, start: Span) -> Result<TokenKind, AsmError> {
        let mut lexeme = String::new();
        // Detect a base prefix: 0x / 0b / 0o (case-insensitive).
        let (radix, prefixed) = if self.peek() == Some('0') {
            match self.peek2() {
                Some('x') | Some('X') => (16, true),
                Some('b') | Some('B') => (2, true),
                Some('o') | Some('O') => (8, true),
                _ => (10, false),
            }
        } else {
            (10, false)
        };

        if prefixed {
            self.bump(); // '0'
            let p = self.bump().unwrap(); // base letter
            lexeme.push('0');
            lexeme.push(p);
            let mut digits = String::new();
            while let Some(c) = self.peek() {
                if c.is_ascii_alphanumeric() {
                    digits.push(c);
                    lexeme.push(c);
                    self.bump();
                } else {
                    break;
                }
            }
            match u32::from_str_radix(&digits, radix) {
                Ok(v) if !digits.is_empty() => Ok(TokenKind::Number(v)),
                _ => Err(AsmError::new(start, AsmErrorKind::MalformedNumber(lexeme))),
            }
        } else {
            let mut digits = String::new();
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    digits.push(c);
                    self.bump();
                } else {
                    break;
                }
            }
            match u32::from_str_radix(&digits, 10) {
                Ok(v) => Ok(TokenKind::Number(v)),
                Err(_) => Err(AsmError::new(start, AsmErrorKind::MalformedNumber(digits))),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<TokenKind> {
        lex(src).unwrap().into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn numbers_in_all_bases() {
        use TokenKind::*;
        assert_eq!(
            kinds("42 0xFF 0b1010 0o77 0"),
            vec![
                Number(42),
                Number(255),
                Number(10),
                Number(63),
                Number(0),
                Eof
            ]
        );
    }

    #[test]
    fn strings_chars_and_punctuation() {
        use TokenKind::*;
        assert_eq!(
            kinds("mov A, 'x'  \"hi\" :"),
            vec![
                Ident("mov".into()),
                Ident("A".into()),
                Comma,
                Char(b'x'),
                Str("hi".into()),
                Colon,
                Eof,
            ]
        );
    }

    #[test]
    fn directive_and_segment_shapes() {
        use TokenKind::*;
        assert_eq!(
            kinds("%define X 1\nsegment .text\n"),
            vec![
                Percent,
                Ident("define".into()),
                Ident("X".into()),
                Number(1),
                Newline,
                Ident("segment".into()),
                Period,
                Ident("text".into()),
                Newline,
                Eof,
            ]
        );
    }

    #[test]
    fn comments_are_skipped_but_newline_survives() {
        use TokenKind::*;
        assert_eq!(
            kinds("hlt ; stop here\nnop"),
            vec![Ident("hlt".into()), Newline, Ident("nop".into()), Eof]
        );
    }

    #[test]
    fn lexer_errors() {
        assert_eq!(
            lex("'oops").unwrap_err().kind,
            AsmErrorKind::UnterminatedString
        );
        assert_eq!(lex("''").unwrap_err().kind, AsmErrorKind::EmptyCharLiteral);
        assert_eq!(
            lex("'ab'").unwrap()[0].kind,
            TokenKind::Str("ab".into())
        );
        assert!(matches!(
            lex("0x").unwrap_err().kind,
            AsmErrorKind::MalformedNumber(_)
        ));
        assert_eq!(
            lex("@").unwrap_err().kind,
            AsmErrorKind::UnexpectedChar('@')
        );
    }

    #[test]
    fn line_and_column_tracking() {
        let toks = lex("nop\n  hlt").unwrap();
        assert_eq!(toks[0].span, Span::new(1, 1)); // nop
        assert_eq!(toks[2].span, Span::new(2, 3)); // hlt after two spaces on line 2
    }
}

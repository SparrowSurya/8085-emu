//! The token stream produced by the lexer and consumed by the parser.

use super::error::Span;

/// A lexical token: its kind plus where it started in the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    /// What the token is.
    pub kind: TokenKind,
    /// Where the token begins (1-based line/col).
    pub span: Span,
}

impl Token {
    /// Bundle a kind with its position.
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Token { kind, span }
    }
}

/// The distinct kinds of token. Comments and inter-token whitespace are consumed by the
/// lexer and never appear here; statement boundaries surface as [`TokenKind::Newline`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    /// `%` — introduces a preprocessor directive.
    Percent,
    /// An identifier: mnemonic, register, size, segment name, label, or symbol. Reserved
    /// words are classified later, during parsing.
    Ident(String),
    /// A numeric literal, already parsed to its value (base is not retained).
    Number(u32),
    /// A double-quoted string literal (its bytes, without the quotes).
    Str(String),
    /// A single-quoted character literal (one byte).
    Char(u8),
    /// `,`
    Comma,
    /// `:`
    Colon,
    /// `.` — precedes a segment name.
    Period,
    /// End of a logical line.
    Newline,
    /// End of input.
    Eof,
}

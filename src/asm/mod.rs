//! Assembler front-end: turns 8085 assembly source text into a machine-code image the
//! [`Machine`](crate::machine::Machine) can load and run.
//!
//! The pipeline is a sequence of small, independently-testable stages. Implemented so
//! far: the [`lexer`] (source → tokens) and the [`encode`]r (a mnemonic plus resolved
//! operands → machine-code bytes). The parser, preprocessor, layout/symbol resolution,
//! and code generation build on these next.

pub mod assemble;
pub mod ast;
pub mod container;
pub mod encode;
pub mod error;
pub mod include;
pub mod inspect;
pub mod keyword;
pub mod lexer;
pub mod parser;
pub mod token;

pub use assemble::{
    ListingRow, LoadImage, assemble, assemble_and_link, assemble_listing, assemble_with_options,
    assemble_with_symbols, load,
};
pub use ast::{BssDecl, DataDef, Define, Instr, POperand, Program, Segment, Size, TextItem, Value};
pub use container::{BinaryContainer, ContainerHeader};
pub use encode::{AReg8, AReg16, Operand, encode};
pub use error::{AsmError, AsmErrorKind, Span};
pub use inspect::{
    ExtractedString, InspectOptions, SegmentRecord, extract_strings, format_header,
    format_segments, format_strings, format_symbols, get_segments, inspect_container,
};
pub use lexer::lex;
pub use parser::parse;
pub use token::{Token, TokenKind};

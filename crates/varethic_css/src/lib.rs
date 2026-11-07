//! varethic-css: a safe-Rust CSS tokenizer + basic selector/declaration parser.
//!
//! Early successor to Blink's CSS parser (`//third_party/blink/renderer/core/css`).
//! This crate intentionally covers only a subset of CSS Syntax Level 3. It is
//! not a rendering engine and is registered as `InProgress` until it reaches
//! behavioural parity.

pub mod parser;
pub mod tokenizer;

pub use parser::{
    parse_declaration_list, parse_stylesheet, Declaration, Rule, Selector, SimpleSelector,
    Stylesheet,
};
pub use tokenizer::{Token, Tokenizer};

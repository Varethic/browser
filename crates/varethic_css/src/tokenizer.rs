//! CSS Syntax Level 3 tokenizer (subset).
//!
//! Implements the token types a selector/declaration parser needs: idents,
//! functions, at-keywords, hashes, strings, urls, numbers, percentages,
//! dimensions, delimiters, the structural punctuation, whitespace and
//! comments. Not implemented (documented, not hidden): escapes (`\`),
//! unicode-range tokens, and `<!--`/`-->` CDO/CDC tokens.

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Ident(String),
    /// An ident immediately followed by `(`; the `(` is consumed.
    Function(String),
    AtKeyword(String),
    Hash(String),
    Str(String),
    Url(String),
    Number(f64),
    Percentage(f64),
    Dimension(f64, String),
    Delim(char),
    Colon,
    Semicolon,
    Comma,
    OpenParen,
    CloseParen,
    OpenBracket,
    CloseBracket,
    OpenBrace,
    CloseBrace,
    Whitespace,
    Comment(String),
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_' || c == '-' || !c.is_ascii()
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-' || !c.is_ascii()
}

fn is_ws(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\x0c')
}

pub struct Tokenizer {
    chars: Vec<char>,
    pos: usize,
}

impl Tokenizer {
    pub fn new(input: &str) -> Self {
        Self {
            chars: input.chars().collect(),
            pos: 0,
        }
    }

    /// Tokenize an entire input string.
    pub fn tokenize(input: &str) -> Vec<Token> {
        let mut t = Tokenizer::new(input);
        let mut out = Vec::new();
        while let Some(tok) = t.next_token() {
            out.push(tok);
        }
        out
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_at(&self, n: usize) -> Option<char> {
        self.chars.get(self.pos + n).copied()
    }

    fn starts_with(&self, n: usize, s: &[char]) -> bool {
        s.iter()
            .enumerate()
            .all(|(i, c)| self.chars.get(self.pos + n + i) == Some(c))
    }

    fn consume_ident(&mut self) -> String {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if is_ident_char(c) {
                s.push(c);
                self.pos += 1;
            } else {
                break;
            }
        }
        s
    }

    fn consume_while_digits(&mut self) -> String {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                s.push(c);
                self.pos += 1;
            } else {
                break;
            }
        }
        s
    }

    fn consume_number(&mut self) -> f64 {
        let mut s = String::new();
        if matches!(self.peek(), Some('+') | Some('-')) {
            s.push(self.peek().unwrap());
            self.pos += 1;
        }
        s.push_str(&self.consume_while_digits());
        if self.peek() == Some('.') && matches!(self.peek_at(1), Some(c) if c.is_ascii_digit()) {
            s.push('.');
            self.pos += 1;
            s.push_str(&self.consume_while_digits());
        }
        if matches!(self.peek(), Some('e') | Some('E')) {
            let save = self.pos;
            let mut exp = String::from("e");
            self.pos += 1;
            if matches!(self.peek(), Some('+') | Some('-')) {
                exp.push(self.peek().unwrap());
                self.pos += 1;
            }
            if matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                exp.push_str(&self.consume_while_digits());
                s.push_str(&exp);
            } else {
                self.pos = save; // not a valid exponent; leave as-is
            }
        }
        s.parse::<f64>().unwrap_or(0.0)
    }

    fn consume_string(&mut self, quote: char) -> String {
        let mut s = String::new();
        loop {
            match self.peek() {
                None => break,
                Some(c) if c == quote => {
                    self.pos += 1;
                    break;
                }
                Some('\n') => break, // unterminated; stop at newline
                Some(c) => {
                    s.push(c);
                    self.pos += 1;
                }
            }
        }
        s
    }

    fn consume_url(&mut self) -> String {
        // `url(` already consumed.
        while matches!(self.peek(), Some(c) if is_ws(c)) {
            self.pos += 1;
        }
        if matches!(self.peek(), Some('"') | Some('\'')) {
            let q = self.peek().unwrap();
            self.pos += 1;
            let s = self.consume_string(q);
            while matches!(self.peek(), Some(c) if is_ws(c)) {
                self.pos += 1;
            }
            if self.peek() == Some(')') {
                self.pos += 1;
            }
            return s;
        }
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c == ')' {
                self.pos += 1;
                break;
            }
            s.push(c);
            self.pos += 1;
        }
        s.trim().to_string()
    }

    pub fn next_token(&mut self) -> Option<Token> {
        let c = self.peek()?;

        if is_ws(c) {
            while matches!(self.peek(), Some(c) if is_ws(c)) {
                self.pos += 1;
            }
            return Some(Token::Whitespace);
        }

        // Comment.
        if c == '/' && self.peek_at(1) == Some('*') {
            self.pos += 2;
            let start = self.pos;
            while self.peek().is_some()
                && !(self.peek() == Some('*') && self.peek_at(1) == Some('/'))
            {
                self.pos += 1;
            }
            let text: String = self.chars[start..self.pos].iter().collect();
            if self.peek() == Some('*') {
                self.pos += 2;
            }
            return Some(Token::Comment(text));
        }

        match c {
            '"' | '\'' => {
                self.pos += 1;
                Some(Token::Str(self.consume_string(c)))
            }
            '#' => {
                if matches!(self.peek_at(1), Some(d) if is_ident_char(d)) {
                    self.pos += 1;
                    Some(Token::Hash(self.consume_ident()))
                } else {
                    self.pos += 1;
                    Some(Token::Delim('#'))
                }
            }
            '@' => {
                if matches!(self.peek_at(1), Some(d) if is_ident_start(d)) {
                    self.pos += 1;
                    Some(Token::AtKeyword(self.consume_ident()))
                } else {
                    self.pos += 1;
                    Some(Token::Delim('@'))
                }
            }
            '+' | '-' | '.' => {
                // Could start a number (e.g. `-3`, `.5`, `+2`).
                if matches!(self.peek_at(1), Some(d) if d.is_ascii_digit()) {
                    let n = self.consume_number();
                    return Some(self.finish_numeric(n));
                }
                // Negative ident (e.g. `-webkit-...`).
                if c == '-' && matches!(self.peek_at(1), Some(d) if is_ident_start(d)) {
                    let ident = self.consume_ident();
                    return self.finish_ident(ident);
                }
                self.pos += 1;
                Some(Token::Delim(c))
            }
            '0'..='9' => {
                let n = self.consume_number();
                Some(self.finish_numeric(n))
            }
            ':' => {
                self.pos += 1;
                Some(Token::Colon)
            }
            ';' => {
                self.pos += 1;
                Some(Token::Semicolon)
            }
            ',' => {
                self.pos += 1;
                Some(Token::Comma)
            }
            '(' => {
                self.pos += 1;
                Some(Token::OpenParen)
            }
            ')' => {
                self.pos += 1;
                Some(Token::CloseParen)
            }
            '[' => {
                self.pos += 1;
                Some(Token::OpenBracket)
            }
            ']' => {
                self.pos += 1;
                Some(Token::CloseBracket)
            }
            '{' => {
                self.pos += 1;
                Some(Token::OpenBrace)
            }
            '}' => {
                self.pos += 1;
                Some(Token::CloseBrace)
            }
            _ if is_ident_start(c) => {
                let ident = self.consume_ident();
                self.finish_ident(ident)
            }
            _ => {
                self.pos += 1;
                Some(Token::Delim(c))
            }
        }
    }

    fn finish_ident(&mut self, ident: String) -> Option<Token> {
        if self.peek() == Some('(') {
            self.pos += 1;
            if ident.eq_ignore_ascii_case("url") {
                return Some(Token::Url(self.consume_url()));
            }
            Some(Token::Function(ident))
        } else {
            Some(Token::Ident(ident))
        }
    }

    fn finish_numeric(&mut self, value: f64) -> Token {
        if self.peek() == Some('%') {
            self.pos += 1;
            return Token::Percentage(value);
        }
        if matches!(self.peek(), Some(c) if is_ident_start(c)) {
            let unit = self.consume_ident();
            return Token::Dimension(value, unit);
        }
        Token::Number(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idents_and_hashes() {
        assert_eq!(
            Tokenizer::tokenize("div#main.foo"),
            vec![
                Token::Ident("div".into()),
                Token::Hash("main".into()),
                Token::Delim('.'),
                Token::Ident("foo".into()),
            ]
        );
    }

    #[test]
    fn declaration_tokens() {
        assert_eq!(
            Tokenizer::tokenize("color: red;"),
            vec![
                Token::Ident("color".into()),
                Token::Colon,
                Token::Whitespace,
                Token::Ident("red".into()),
                Token::Semicolon,
            ]
        );
    }

    #[test]
    fn dimensional_and_percentage() {
        assert_eq!(
            Tokenizer::tokenize("12px 50% 1.5em"),
            vec![
                Token::Dimension(12.0, "px".into()),
                Token::Whitespace,
                Token::Percentage(50.0),
                Token::Whitespace,
                Token::Dimension(1.5, "em".into()),
            ]
        );
    }

    #[test]
    fn strings_and_urls() {
        assert_eq!(Tokenizer::tokenize("\"hi\""), vec![Token::Str("hi".into())]);
        assert_eq!(
            Tokenizer::tokenize("url(a/b.png)"),
            vec![Token::Url("a/b.png".into())]
        );
        assert_eq!(
            Tokenizer::tokenize("url('a b.png')"),
            vec![Token::Url("a b.png".into())]
        );
    }

    #[test]
    fn at_keyword_and_function() {
        assert_eq!(
            Tokenizer::tokenize("@media rgb(1,2,3)"),
            vec![
                Token::AtKeyword("media".into()),
                Token::Whitespace,
                Token::Function("rgb".into()),
                Token::Number(1.0),
                Token::Comma,
                Token::Number(2.0),
                Token::Comma,
                Token::Number(3.0),
                Token::CloseParen,
            ]
        );
    }

    #[test]
    fn comments_are_tokens() {
        assert_eq!(
            Tokenizer::tokenize("a /* c */ b"),
            vec![
                Token::Ident("a".into()),
                Token::Whitespace,
                Token::Comment(" c ".into()),
                Token::Whitespace,
                Token::Ident("b".into()),
            ]
        );
    }

    #[test]
    fn negative_values() {
        assert_eq!(
            Tokenizer::tokenize("-3px -webkit-box"),
            vec![
                Token::Dimension(-3.0, "px".into()),
                Token::Whitespace,
                Token::Ident("-webkit-box".into()),
            ]
        );
    }
}

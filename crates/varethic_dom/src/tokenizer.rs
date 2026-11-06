//! Minimal HTML5 tokenizer (WHATWG subset).
//!
//! Implements the states needed for a useful subset of real documents: data,
//! tag open/name, attributes (double/single/unquoted/boolean), self-closing,
//! comments, doctype, bogus comments, and RAWTEXT/RCDATA elements
//! (`script`, `style`, `title`, `textarea`, ...). Character references are
//! decoded via [`crate::entities`].
//!
//! Known simplifications (documented, not hidden): no `CDATA` section handling
//! (treated as a bogus comment), no script-data escaped states, no
//! `\0`-replacement rules, and named character references are a small subset.

use crate::entities::decode_entities;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Doctype {
        name: String,
        public_id: String,
        system_id: String,
    },
    StartTag {
        name: String,
        attrs: Vec<(String, String)>,
        self_closing: bool,
    },
    EndTag {
        name: String,
    },
    Comment(String),
    Text(String),
    Eof,
}

enum Step {
    Token(Token),
    /// Consumed input that produced no token (e.g. `</>`); keep going.
    Consumed,
    /// Not markup; emit a literal `<`.
    Literal,
}

fn is_ws(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\x0c')
}

/// RAWTEXT elements treat their content as raw text (no entity decoding).
/// RCDATA elements treat content as text with entity decoding.
fn raw_mode(name: &str) -> Option<bool> {
    // Some(bool) where bool = decode entities (RCDATA).
    match name {
        "script" | "style" | "xmp" | "iframe" | "noembed" | "noframes" | "plaintext" => Some(false),
        "title" | "textarea" => Some(true),
        _ => None,
    }
}

pub struct Tokenizer {
    chars: Vec<char>,
    pos: usize,
    raw: Option<(String, bool)>,
    done: bool,
}

impl Tokenizer {
    pub fn new(input: &str) -> Self {
        Self {
            chars: input.chars().collect(),
            pos: 0,
            raw: None,
            done: false,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_at(&self, n: usize) -> Option<char> {
        self.chars.get(self.pos + n).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek();
        if c.is_some() {
            self.pos += 1;
        }
        c
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(c) if is_ws(c)) {
            self.pos += 1;
        }
    }

    /// Case-insensitive comparison of `chars` starting at `idx` against `s`.
    fn matches_at(&self, idx: usize, s: &str, ci: bool) -> bool {
        let mut i = idx;
        for c in s.chars() {
            match self.chars.get(i) {
                Some(&d) if (ci && d.eq_ignore_ascii_case(&c)) || (!ci && d == c) => i += 1,
                _ => return false,
            }
        }
        true
    }

    pub fn next_token(&mut self) -> Token {
        loop {
            if self.done {
                return Token::Eof;
            }
            if let Some((tag, decode)) = self.raw.clone() {
                return self.raw_text_token(&tag, decode);
            }
            match self.peek() {
                None => {
                    self.done = true;
                    return Token::Eof;
                }
                Some('<') => match self.markup() {
                    Step::Token(t) => return t,
                    Step::Consumed => continue,
                    Step::Literal => {
                        self.bump();
                        return Token::Text("<".into());
                    }
                },
                Some(_) => return self.text_run(),
            }
        }
    }

    fn text_run(&mut self) -> Token {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c == '<' {
                break;
            }
            self.pos += 1;
        }
        let raw: String = self.chars[start..self.pos].iter().collect();
        Token::Text(decode_entities(&raw))
    }

    fn markup(&mut self) -> Step {
        // self.peek() == '<'
        match self.peek_at(1) {
            Some('!') => Step::Token(self.markup_declaration()),
            Some('/') => match self.peek_at(2) {
                Some(c) if c.is_ascii_alphabetic() => Step::Token(self.end_tag()),
                Some('>') => {
                    self.pos += 3; // </> is ignored
                    Step::Consumed
                }
                _ => Step::Token(self.bogus_comment(2)),
            },
            Some(c) if c.is_ascii_alphabetic() => Step::Token(self.start_tag()),
            _ => Step::Literal,
        }
    }

    fn markup_declaration(&mut self) -> Token {
        if self.matches_at(self.pos, "<!--", false) {
            return self.comment();
        }
        if self.matches_at(self.pos + 2, "doctype", true) {
            return self.doctype();
        }
        // <![CDATA[ ... ]]> and anything else: bogus comment up to '>'.
        self.bogus_comment(self.pos + 2)
    }

    fn comment(&mut self) -> Token {
        self.pos += 4; // past "<!--"
        let start = self.pos;
        loop {
            if self.pos + 2 < self.chars.len()
                && self.chars[self.pos] == '-'
                && self.chars[self.pos + 1] == '-'
            {
                if self.chars[self.pos + 2] == '>' {
                    let text: String = self.chars[start..self.pos].iter().collect();
                    self.pos += 3;
                    return Token::Comment(text);
                }
                if self.chars[self.pos + 2] == '!' && self.peek_at(3) == Some('>') {
                    let text: String = self.chars[start..self.pos].iter().collect();
                    self.pos += 4;
                    return Token::Comment(text);
                }
            }
            if self.pos >= self.chars.len() {
                let text: String = self.chars[start..].iter().collect();
                self.done = true;
                return Token::Comment(text);
            }
            self.pos += 1;
        }
    }

    fn doctype(&mut self) -> Token {
        // pos at '<', "DOCTYPE" (ci) follows '<!'.
        self.pos += 9; // "<!doctype" length
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c == '>' {
                break;
            }
            self.pos += 1;
        }
        let content: String = self.chars[start..self.pos].iter().collect();
        if self.peek() == Some('>') {
            self.pos += 1;
        }
        let name = content
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        Token::Doctype {
            name,
            public_id: String::new(),
            system_id: String::new(),
        }
    }

    fn bogus_comment(&mut self, content_start: usize) -> Token {
        self.pos = content_start;
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c == '>' {
                break;
            }
            self.pos += 1;
        }
        let text: String = self.chars[start..self.pos].iter().collect();
        if self.peek() == Some('>') {
            self.pos += 1;
        }
        Token::Comment(text)
    }

    fn read_tag_name(&mut self) -> String {
        let mut name = String::new();
        while let Some(c) = self.peek() {
            if is_ws(c) || c == '/' || c == '>' {
                break;
            }
            name.push(c.to_ascii_lowercase());
            self.pos += 1;
        }
        name
    }

    fn start_tag(&mut self) -> Token {
        self.pos += 1; // '<'
        let name = self.read_tag_name();
        let mut attrs: Vec<(String, String)> = Vec::new();
        let mut self_closing = false;
        loop {
            self.skip_ws();
            match self.peek() {
                None => break,
                Some('>') => {
                    self.pos += 1;
                    break;
                }
                Some('/') => {
                    if self.peek_at(1) == Some('>') {
                        self.pos += 2;
                        self_closing = true;
                        break;
                    }
                    self.pos += 1; // stray slash
                }
                Some(_) => {
                    let (an, av) = self.read_attribute();
                    if !an.is_empty() {
                        attrs.push((an, av));
                    }
                }
            }
        }
        if let Some(decode) = raw_mode(&name) {
            self.raw = Some((name.clone(), decode));
        }
        Token::StartTag {
            name,
            attrs,
            self_closing,
        }
    }

    fn read_attribute(&mut self) -> (String, String) {
        let mut name = String::new();
        while let Some(c) = self.peek() {
            if c == '=' || c == '/' || c == '>' || is_ws(c) {
                break;
            }
            name.push(c.to_ascii_lowercase());
            self.pos += 1;
        }
        self.skip_ws();
        let mut value = String::new();
        if self.peek() == Some('=') {
            self.pos += 1;
            self.skip_ws();
            match self.peek() {
                Some(q) if q == '"' || q == '\'' => {
                    self.pos += 1;
                    let start = self.pos;
                    while let Some(c) = self.peek() {
                        if c == q {
                            break;
                        }
                        self.pos += 1;
                    }
                    let raw: String = self.chars[start..self.pos].iter().collect();
                    value = decode_entities(&raw);
                    if self.peek() == Some(q) {
                        self.pos += 1;
                    }
                }
                _ => {
                    let start = self.pos;
                    while let Some(c) = self.peek() {
                        if is_ws(c) || c == '>' {
                            break;
                        }
                        self.pos += 1;
                    }
                    let raw: String = self.chars[start..self.pos].iter().collect();
                    value = decode_entities(&raw);
                }
            }
        }
        (name, value)
    }

    fn end_tag(&mut self) -> Token {
        self.pos += 2; // '</'
        let name = self.read_tag_name();
        while let Some(c) = self.peek() {
            self.pos += 1;
            if c == '>' {
                break;
            }
        }
        Token::EndTag { name }
    }

    /// Does an end tag for `tag` start at char index `idx` (just after `</`)?
    fn end_tag_matches(&self, idx: usize, tag: &str) -> bool {
        if !self.matches_at(idx, tag, true) {
            return false;
        }
        match self.chars.get(idx + tag.len()) {
            None => true,
            Some(&c) => is_ws(c) || c == '>' || c == '/',
        }
    }

    fn raw_text_token(&mut self, tag: &str, decode: bool) -> Token {
        let start = self.pos;
        let n = self.chars.len();
        let mut i = start;
        while i < n {
            if self.chars[i] == '<'
                && i + 1 < n
                && self.chars[i + 1] == '/'
                && self.end_tag_matches(i + 2, tag)
            {
                break;
            }
            i += 1;
        }
        if i == start {
            // The end tag is immediate: leave raw mode and tokenize normally.
            self.raw = None;
            return self.next_token();
        }
        let raw: String = self.chars[start..i].iter().collect();
        self.pos = i;
        if i >= n {
            self.raw = None;
        }
        Token::Text(if decode { decode_entities(&raw) } else { raw })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(input: &str) -> Vec<Token> {
        let mut t = Tokenizer::new(input);
        let mut out = Vec::new();
        loop {
            let tok = t.next_token();
            let end = tok == Token::Eof;
            out.push(tok);
            if end {
                break;
            }
        }
        out
    }

    #[test]
    fn simple_start_and_end_tag() {
        assert_eq!(
            toks("<p>hi</p>"),
            vec![
                Token::StartTag {
                    name: "p".into(),
                    attrs: vec![],
                    self_closing: false
                },
                Token::Text("hi".into()),
                Token::EndTag { name: "p".into() },
                Token::Eof,
            ]
        );
    }

    #[test]
    fn attributes_all_flavors() {
        let tk = toks("<a href=\"/x\" data-n=1 checked>");
        assert_eq!(
            tk[0],
            Token::StartTag {
                name: "a".into(),
                attrs: vec![
                    ("href".into(), "/x".into()),
                    ("data-n".into(), "1".into()),
                    ("checked".into(), String::new()),
                ],
                self_closing: false,
            }
        );
    }

    #[test]
    fn self_closing_and_case_folding() {
        let tk = toks("<BR/>");
        assert_eq!(
            tk[0],
            Token::StartTag {
                name: "br".into(),
                attrs: vec![],
                self_closing: true
            }
        );
    }

    #[test]
    fn comment_and_doctype() {
        assert_eq!(toks("<!-- hi -->")[0], Token::Comment(" hi ".into()));
        assert_eq!(
            toks("<!DOCTYPE html>")[0],
            Token::Doctype {
                name: "html".into(),
                public_id: String::new(),
                system_id: String::new()
            }
        );
    }

    #[test]
    fn raw_text_and_rcdata() {
        let tk = toks("<script>a < b</script>");
        assert_eq!(tk[1], Token::Text("a < b".into()));
        assert_eq!(
            tk[2],
            Token::EndTag {
                name: "script".into()
            }
        );

        let tk = toks("<title>x &amp; y</title>");
        assert_eq!(tk[1], Token::Text("x & y".into()));
    }

    #[test]
    fn character_references_in_text() {
        assert_eq!(toks("a &amp; b")[0], Token::Text("a & b".into()));
    }

    #[test]
    fn bare_less_than_is_text() {
        assert_eq!(toks("a < b")[0], Token::Text("a ".into()));
        assert_eq!(toks("a < b")[1], Token::Text("<".into()));
    }
}

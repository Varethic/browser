//! A basic CSS parser over [`crate::tokenizer::Token`]s: stylesheets into
//! qualified rules (selectors + declarations) and at-rules, plus declaration
//! lists (for `style="..."` attributes).
//!
//! Selectors are parsed into a structural form (type/universal/id/class/
//! attribute/pseudo + combinators). This is a *subset* of CSS: no `@media`
//! block recursion, no nesting, no `calc()` evaluation, no shorthand
//! expansion, no specificity computation, no error recovery beyond skipping to
//! the next `;`/`}`.

use crate::tokenizer::{Token, Tokenizer};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declaration {
    pub property: String,
    pub value: String,
    pub important: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimpleSelector {
    Type(String),
    Universal,
    Id(String),
    Class(String),
    /// Raw text inside `[...]`, e.g. `href^="https"`.
    Attr(String),
    /// Raw pseudo text including leading colons, e.g. `:hover`.
    Pseudo(String),
    /// Combinator: `' '` (descendant), `'>'`, `'+'`, `'~'`.
    Combinator(char),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selector {
    pub parts: Vec<SimpleSelector>,
}

impl Selector {
    /// Canonical text (ASCII), used for round-trip tests and summaries.
    pub fn to_css(&self) -> String {
        let mut out = String::new();
        for p in &self.parts {
            match p {
                SimpleSelector::Type(t) => out.push_str(t),
                SimpleSelector::Universal => out.push('*'),
                SimpleSelector::Id(i) => {
                    out.push('#');
                    out.push_str(i);
                }
                SimpleSelector::Class(c) => {
                    out.push('.');
                    out.push_str(c);
                }
                SimpleSelector::Attr(a) => {
                    out.push('[');
                    out.push_str(a);
                    out.push(']');
                }
                SimpleSelector::Pseudo(p) => out.push_str(p),
                SimpleSelector::Combinator(c) => {
                    if *c == ' ' {
                        out.push(' ');
                    } else {
                        out.push(' ');
                        out.push(*c);
                        out.push(' ');
                    }
                }
            }
        }
        out.trim().to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rule {
    Style {
        selectors: Vec<Selector>,
        declarations: Vec<Declaration>,
    },
    At {
        name: String,
        prelude: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Stylesheet {
    pub rules: Vec<Rule>,
}

struct Cursor {
    toks: Vec<Token>,
    i: usize,
}

impl Cursor {
    fn new(toks: Vec<Token>) -> Self {
        Self { toks, i: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.toks.get(self.i)
    }

    fn next(&mut self) -> Option<Token> {
        let t = self.toks.get(self.i).cloned();
        if t.is_some() {
            self.i += 1;
        }
        t
    }

    fn skip_trivia(&mut self) {
        while matches!(
            self.peek(),
            Some(Token::Whitespace) | Some(Token::Comment(_))
        ) {
            self.i += 1;
        }
    }

    /// Collect tokens until a top-level `{`, `;` or EOF. Returns the tokens and
    /// whether it stopped at `{` (true) or `;`/EOF (false).
    fn collect_until_boundary(&mut self) -> (Vec<Token>, bool) {
        let mut out = Vec::new();
        let mut depth = 0i32;
        while let Some(t) = self.peek() {
            match t {
                Token::OpenParen | Token::OpenBracket => depth += 1,
                Token::CloseParen | Token::CloseBracket => depth -= 1,
                Token::OpenBrace if depth == 0 => return (out, true),
                Token::Semicolon if depth == 0 => return (out, false),
                _ => {}
            }
            out.push(self.next().unwrap());
        }
        (out, false)
    }

    /// Skip a `{ ... }` block, honoring nesting.
    fn skip_block(&mut self) {
        debug_assert!(self.peek() == Some(&Token::OpenBrace));
        self.i += 1;
        let mut depth = 1i32;
        while let Some(t) = self.next() {
            match t {
                Token::OpenBrace => depth += 1,
                Token::CloseBrace => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
        }
    }
}

/// Parse a full stylesheet.
pub fn parse_stylesheet(input: &str) -> Stylesheet {
    let mut c = Cursor::new(Tokenizer::tokenize(input));
    let mut rules = Vec::new();
    loop {
        c.skip_trivia();
        match c.peek() {
            None => break,
            Some(Token::AtKeyword(_)) => rules.push(parse_at_rule(&mut c)),
            _ => {
                if let Some(rule) = parse_qualified_rule(&mut c) {
                    rules.push(rule);
                }
            }
        }
    }
    Stylesheet { rules }
}

/// Parse the contents of a declaration block (`color: red; margin: 0`).
pub fn parse_declaration_list(input: &str) -> Vec<Declaration> {
    let mut c = Cursor::new(Tokenizer::tokenize(input));
    parse_declarations(&mut c)
}

fn parse_at_rule(c: &mut Cursor) -> Rule {
    let name = match c.next() {
        Some(Token::AtKeyword(n)) => n,
        _ => String::new(),
    };
    let (prelude, hit_brace) = c.collect_until_boundary();
    let prelude = render(&prelude).trim().to_string();
    if hit_brace {
        c.skip_block();
    } else if c.peek() == Some(&Token::Semicolon) {
        c.next();
    }
    Rule::At { name, prelude }
}

fn parse_qualified_rule(c: &mut Cursor) -> Option<Rule> {
    let (prelude, hit_brace) = c.collect_until_boundary();
    if !hit_brace {
        // No block: malformed; skip the stray `;` if present.
        if c.peek() == Some(&Token::Semicolon) {
            c.next();
        }
        return None;
    }
    let selectors = parse_selector_list(&prelude);
    if selectors.is_empty() {
        // Still consume the block to stay in sync.
        c.skip_block();
        return None;
    }
    c.skip_block_enter();
    let declarations = parse_declarations(c);
    Some(Rule::Style {
        selectors,
        declarations,
    })
}

impl Cursor {
    /// Consume the `{` that begins a declaration block.
    fn skip_block_enter(&mut self) {
        if self.peek() == Some(&Token::OpenBrace) {
            self.i += 1;
        }
    }
}

fn parse_declarations(c: &mut Cursor) -> Vec<Declaration> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    loop {
        c.skip_trivia();
        match c.peek() {
            None => break,
            Some(Token::CloseBrace) if depth == 0 => {
                c.next();
                break;
            }
            Some(Token::Semicolon) => {
                c.next();
                continue;
            }
            _ => {}
        }

        // Collect a candidate declaration up to `;`/`}` at depth 0.
        let mut buf: Vec<Token> = Vec::new();
        let mut stopped_at_close = false;
        while let Some(t) = c.peek() {
            match t {
                Token::OpenParen | Token::OpenBracket | Token::OpenBrace => depth += 1,
                Token::CloseParen | Token::CloseBracket => depth -= 1,
                Token::CloseBrace if depth == 0 => {
                    stopped_at_close = true;
                    break;
                }
                Token::Semicolon if depth == 0 => break,
                _ => {}
            }
            buf.push(c.next().unwrap());
        }
        if let Some(d) = parse_declaration(&buf) {
            out.push(d);
        }
        if stopped_at_close {
            c.next(); // consume '}'
            break;
        }
        if c.peek() == Some(&Token::Semicolon) {
            c.next();
        }
    }
    out
}

/// Interpret a token buffer as a single `property: value` declaration.
fn parse_declaration(buf: &[Token]) -> Option<Declaration> {
    let colon = buf.iter().position(|t| matches!(t, Token::Colon))?;
    let property = buf[..colon]
        .iter()
        .filter_map(|t| match t {
            Token::Ident(s) => Some(s.clone()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("");
    if property.is_empty() {
        return None;
    }
    let mut value = render(&buf[colon + 1..]).trim().to_string();
    let mut important = false;
    // Detect and strip a trailing `!important`.
    let lower = value.to_ascii_lowercase();
    if let Some(pos) = lower.rfind('!') {
        let tail: String = lower[pos + 1..]
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        if tail == "important" {
            important = true;
            value = value[..pos].trim().to_string();
        }
    }
    Some(Declaration {
        property,
        value,
        important,
    })
}

/// Split a selector-list token buffer on top-level commas and parse each.
fn parse_selector_list(toks: &[Token]) -> Vec<Selector> {
    let mut out = Vec::new();
    let mut current: Vec<Token> = Vec::new();
    let mut depth = 0i32;
    for t in toks {
        match t {
            Token::OpenParen | Token::OpenBracket => depth += 1,
            Token::CloseParen | Token::CloseBracket => depth -= 1,
            Token::Comma if depth == 0 => {
                if let Some(s) = parse_complex_selector(&current) {
                    out.push(s);
                }
                current.clear();
                continue;
            }
            _ => {}
        }
        current.push(t.clone());
    }
    if let Some(s) = parse_complex_selector(&current) {
        out.push(s);
    }
    out
}

fn parse_complex_selector(toks: &[Token]) -> Option<Selector> {
    let mut parts: Vec<SimpleSelector> = Vec::new();
    let mut i = 0;
    let mut pending_descendant = false;
    let mut pending_combinator: Option<char> = None;

    while i < toks.len() {
        match &toks[i] {
            Token::Whitespace => {
                if !parts.is_empty() {
                    pending_descendant = true;
                }
                i += 1;
            }
            Token::Delim('>') | Token::Delim('+') | Token::Delim('~') => {
                let c = match &toks[i] {
                    Token::Delim(c) => *c,
                    _ => unreachable!(),
                };
                pending_combinator = Some(c);
                pending_descendant = false;
                i += 1;
            }
            _ => {
                if !parts.is_empty() {
                    if let Some(c) = pending_combinator.take() {
                        parts.push(SimpleSelector::Combinator(c));
                    } else if pending_descendant {
                        parts.push(SimpleSelector::Combinator(' '));
                    }
                }
                pending_descendant = false;
                match &toks[i] {
                    Token::Ident(name) => {
                        parts.push(SimpleSelector::Type(name.clone()));
                        i += 1;
                    }
                    Token::Delim('*') => {
                        parts.push(SimpleSelector::Universal);
                        i += 1;
                    }
                    Token::Delim('.') => {
                        if let Some(Token::Ident(c)) = toks.get(i + 1) {
                            parts.push(SimpleSelector::Class(c.clone()));
                            i += 2;
                        } else {
                            i += 1;
                        }
                    }
                    Token::Hash(id) => {
                        parts.push(SimpleSelector::Id(id.clone()));
                        i += 1;
                    }
                    Token::OpenBracket => {
                        let start = i + 1;
                        let mut j = start;
                        let mut depth = 1i32;
                        while j < toks.len() {
                            match &toks[j] {
                                Token::OpenBracket => depth += 1,
                                Token::CloseBracket => {
                                    depth -= 1;
                                    if depth == 0 {
                                        break;
                                    }
                                }
                                _ => {}
                            }
                            j += 1;
                        }
                        let inner = render(&toks[start..j.min(toks.len())]).trim().to_string();
                        parts.push(SimpleSelector::Attr(inner));
                        i = (j + 1).min(toks.len());
                    }
                    Token::Colon => {
                        // `:ident` or `:func(...)`.
                        if let Some(Token::Ident(name)) = toks.get(i + 1) {
                            parts.push(SimpleSelector::Pseudo(format!(":{name}")));
                            i += 2;
                        } else if let Some(Token::Function(name)) = toks.get(i + 1) {
                            let mut j = i + 2;
                            let mut depth = 1i32;
                            while j < toks.len() {
                                match &toks[j] {
                                    Token::OpenParen => depth += 1,
                                    Token::CloseParen => {
                                        depth -= 1;
                                        if depth == 0 {
                                            break;
                                        }
                                    }
                                    _ => {}
                                }
                                j += 1;
                            }
                            let inner = render(&toks[i + 2..j.min(toks.len())]).trim().to_string();
                            parts.push(SimpleSelector::Pseudo(format!(":{name}({inner})")));
                            i = (j + 1).min(toks.len());
                        } else if let Some(Token::Colon) = toks.get(i + 1) {
                            // `::pseudo`
                            if let Some(Token::Ident(name)) = toks.get(i + 2) {
                                parts.push(SimpleSelector::Pseudo(format!("::{name}")));
                                i += 3;
                            } else {
                                i += 1;
                            }
                        } else {
                            i += 1;
                        }
                    }
                    _ => {
                        i += 1;
                    }
                }
            }
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(Selector { parts })
    }
}

/// Render a token buffer back to canonical CSS text.
pub fn render(toks: &[Token]) -> String {
    let mut out = String::new();
    for t in toks {
        match t {
            Token::Ident(s) => out.push_str(s),
            Token::Function(s) => {
                out.push_str(s);
                out.push('(');
            }
            Token::AtKeyword(s) => {
                out.push('@');
                out.push_str(s);
            }
            Token::Hash(s) => {
                out.push('#');
                out.push_str(s);
            }
            Token::Str(s) => {
                out.push('"');
                out.push_str(&s.replace('"', "\\\""));
                out.push('"');
            }
            Token::Url(s) => {
                out.push_str("url(");
                out.push_str(s);
                out.push(')');
            }
            Token::Number(n) => out.push_str(&fmt_num(*n)),
            Token::Percentage(n) => {
                out.push_str(&fmt_num(*n));
                out.push('%');
            }
            Token::Dimension(n, u) => {
                out.push_str(&fmt_num(*n));
                out.push_str(u);
            }
            Token::Delim(c) => out.push(*c),
            Token::Colon => out.push(':'),
            Token::Semicolon => out.push(';'),
            Token::Comma => out.push_str(", "),
            Token::OpenParen => out.push('('),
            Token::CloseParen => out.push(')'),
            Token::OpenBracket => out.push('['),
            Token::CloseBracket => out.push(']'),
            Token::OpenBrace => out.push('{'),
            Token::CloseBrace => out.push('}'),
            Token::Comment(_) => {}
            Token::Whitespace => {
                if !out.ends_with(' ') && !out.is_empty() {
                    out.push(' ');
                }
            }
        }
    }
    out
}

fn fmt_num(n: f64) -> String {
    if n.fract() == 0.0 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_rule() {
        let s = parse_stylesheet("p { color: red; }");
        assert_eq!(s.rules.len(), 1);
        match &s.rules[0] {
            Rule::Style {
                selectors,
                declarations,
            } => {
                assert_eq!(selectors.len(), 1);
                assert_eq!(selectors[0].parts, vec![SimpleSelector::Type("p".into())]);
                assert_eq!(
                    declarations,
                    &vec![Declaration {
                        property: "color".into(),
                        value: "red".into(),
                        important: false
                    }]
                );
            }
            _ => panic!("expected style rule"),
        }
    }

    #[test]
    fn selector_list_and_specificity_parts() {
        let s = parse_stylesheet("#main .a > p[href^=\"http\"]:hover { margin: 0 }");
        match &s.rules[0] {
            Rule::Style { selectors, .. } => {
                assert_eq!(selectors.len(), 1);
                assert_eq!(
                    selectors[0].parts,
                    vec![
                        SimpleSelector::Id("main".into()),
                        SimpleSelector::Combinator(' '),
                        SimpleSelector::Class("a".into()),
                        SimpleSelector::Combinator('>'),
                        SimpleSelector::Type("p".into()),
                        SimpleSelector::Attr("href^=\"http\"".into()),
                        SimpleSelector::Pseudo(":hover".into()),
                    ]
                );
                assert_eq!(selectors[0].to_css(), "#main .a > p[href^=\"http\"]:hover");
            }
            _ => panic!("expected style rule"),
        }

        let s = parse_stylesheet("h1, h2 , .x { color: blue }");
        match &s.rules[0] {
            Rule::Style { selectors, .. } => {
                assert_eq!(selectors.len(), 3);
                assert_eq!(selectors[0].to_css(), "h1");
                assert_eq!(selectors[1].to_css(), "h2");
                assert_eq!(selectors[2].to_css(), ".x");
            }
            _ => panic!("expected style rule"),
        }
    }

    #[test]
    fn important_is_detected() {
        let decls = parse_declaration_list("color: red !important; width: 10px");
        assert_eq!(decls.len(), 2);
        assert_eq!(decls[0].property, "color");
        assert_eq!(decls[0].value, "red");
        assert!(decls[0].important);
        assert_eq!(decls[1].value, "10px");
        assert!(!decls[1].important);
    }

    #[test]
    fn at_rule_is_recorded() {
        let s = parse_stylesheet("@media screen { p { color: red } }");
        assert_eq!(s.rules.len(), 1);
        assert_eq!(
            s.rules[0],
            Rule::At {
                name: "media".into(),
                prelude: "screen".into()
            }
        );
    }

    #[test]
    fn comments_are_ignored() {
        let s = parse_stylesheet("/* hi */ p /* x */ { color: red }");
        assert_eq!(s.rules.len(), 1);
    }

    #[test]
    fn malformed_declaration_is_skipped() {
        let decls = parse_declaration_list("color red; width: 5px");
        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].property, "width");
    }

    #[test]
    fn media_query_with_braces_in_value_survives() {
        let s = parse_stylesheet("a { background: url(x){ } ".trim());
        // The rule parses; the stray brace just terminates the block.
        assert!(!s.rules.is_empty());
    }
}

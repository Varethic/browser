//! Simplified HTML tree construction over [`crate::tokenizer::Token`]s.
//!
//! Produces a stack-based DOM tree (arena of [`Node`]s). This is a *subset* of
//! the WHATWG tree-construction algorithm: it handles nesting, void elements,
//! a set of implied-end-tag rules (`li`, `p`, `dt`/`dd`, `option`, and
//! block-level elements closing an open `p`), comments and doctype. It
//! deliberately does NOT synthesize `html`/`head`/`body`, and does not
//! implement the adoption agency algorithm, foster parenting, or table
//! scoping. Divergences are listed in docs/KNOWN_ISSUES.md.

use crate::tokenizer::{Token, Tokenizer};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    Document,
    Doctype {
        name: String,
    },
    Element {
        name: String,
        attrs: Vec<(String, String)>,
        children: Vec<usize>,
    },
    Text(String),
    Comment(String),
}

/// Void elements: they never get children and never open a scope.
pub fn is_void(name: &str) -> bool {
    matches!(
        name,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

/// Elements that close an open `<p>` when they start.
fn closes_p(name: &str) -> bool {
    matches!(
        name,
        "address"
            | "article"
            | "aside"
            | "blockquote"
            | "details"
            | "div"
            | "dl"
            | "fieldset"
            | "figcaption"
            | "figure"
            | "footer"
            | "form"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "header"
            | "hr"
            | "main"
            | "menu"
            | "nav"
            | "ol"
            | "p"
            | "pre"
            | "section"
            | "table"
            | "ul"
    )
}

/// A parsed document: an arena of [`Node`]s with node `0` as the root
/// `Document`.
#[derive(Debug, Clone)]
pub struct Dom {
    nodes: Vec<Node>,
    document_children: Vec<usize>,
}

impl Dom {
    /// Parse a complete document string.
    pub fn parse(input: &str) -> Dom {
        let mut builder = TreeBuilder::new();
        let mut tokenizer = Tokenizer::new(input);
        loop {
            let token = tokenizer.next_token();
            let eof = token == Token::Eof;
            builder.process(token);
            if eof {
                break;
            }
        }
        builder.finish()
    }

    pub fn root(&self) -> usize {
        0
    }

    pub fn node(&self, id: usize) -> &Node {
        &self.nodes[id]
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Depth-first iterator over the element `name`s in the document.
    pub fn element_names(&self) -> Vec<String> {
        let mut out = Vec::new();
        for &c in &self.document_children {
            self.collect_names(c, &mut out);
        }
        out
    }

    fn collect_names(&self, id: usize, out: &mut Vec<String>) {
        if let Node::Element { name, children, .. } = &self.nodes[id] {
            out.push(name.clone());
            for &c in children {
                self.collect_names(c, out);
            }
        }
    }

    /// First element with the given tag name, depth-first, if any.
    pub fn find(&self, name: &str) -> Option<usize> {
        for &c in &self.document_children {
            if let Some(found) = self.find_in(c, name) {
                return Some(found);
            }
        }
        None
    }

    fn find_in(&self, id: usize, name: &str) -> Option<usize> {
        if let Node::Element {
            name: n, children, ..
        } = &self.nodes[id]
        {
            if n == name {
                return Some(id);
            }
            for &c in children {
                if let Some(found) = self.find_in(c, name) {
                    return Some(found);
                }
            }
        }
        None
    }

    /// Concatenated text content under `id`.
    pub fn text_content(&self, id: usize) -> String {
        let mut out = String::new();
        self.collect_text(id, &mut out);
        out
    }

    fn collect_text(&self, id: usize, out: &mut String) {
        match &self.nodes[id] {
            Node::Text(t) => out.push_str(t),
            Node::Element { children, .. } => {
                for &c in children {
                    self.collect_text(c, out);
                }
            }
            _ => {}
        }
    }

    /// Serialize back to a normalized HTML string. Tag/attribute names are
    /// lowercase, attribute values are double-quoted, text is entity-escaped,
    /// and elements are always explicitly closed (unless void).
    pub fn to_html(&self) -> String {
        let mut out = String::new();
        for &c in &self.document_children {
            self.write(c, &mut out);
        }
        out
    }

    fn write(&self, id: usize, out: &mut String) {
        match &self.nodes[id] {
            Node::Document => {}
            Node::Doctype { name } => {
                out.push_str("<!DOCTYPE ");
                out.push_str(name);
                out.push('>');
            }
            Node::Element {
                name,
                attrs,
                children,
            } => {
                out.push('<');
                out.push_str(name);
                for (k, v) in attrs {
                    out.push(' ');
                    out.push_str(k);
                    out.push_str("=\"");
                    out.push_str(&escape_attr(v));
                    out.push('"');
                }
                out.push('>');
                if is_void(name) {
                    return;
                }
                for &c in children {
                    self.write(c, out);
                }
                out.push_str("</");
                out.push_str(name);
                out.push('>');
            }
            Node::Text(t) => out.push_str(&escape_text(t)),
            Node::Comment(c) => {
                out.push_str("<!--");
                out.push_str(c);
                out.push_str("-->");
            }
        }
    }
}

fn escape_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
    out
}

fn escape_attr(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

struct TreeBuilder {
    nodes: Vec<Node>,
    document_children: Vec<usize>,
    open: Vec<usize>,
}

impl TreeBuilder {
    fn new() -> Self {
        Self {
            nodes: vec![Node::Document],
            document_children: Vec::new(),
            open: Vec::new(),
        }
    }

    fn alloc(&mut self, node: Node) -> usize {
        self.nodes.push(node);
        self.nodes.len() - 1
    }

    fn current_parent(&self) -> Option<usize> {
        self.open.last().copied()
    }

    fn append(&mut self, parent: usize, child: usize) {
        match &mut self.nodes[parent] {
            Node::Element { children, .. } => children.push(child),
            Node::Document => self.document_children.push(child),
            _ => {}
        }
    }

    fn name_of(&self, id: usize) -> &str {
        match &self.nodes[id] {
            Node::Element { name, .. } => name,
            _ => "",
        }
    }

    fn open_index(&self, name: &str) -> Option<usize> {
        self.open.iter().rposition(|&id| self.name_of(id) == name)
    }

    fn add_top_level(&mut self, node: Node) -> usize {
        let id = self.alloc(node);
        match self.current_parent() {
            Some(p) => self.append(p, id),
            None => self.document_children.push(id),
        }
        id
    }

    fn process(&mut self, token: Token) {
        match token {
            Token::Doctype { name, .. } => {
                self.add_top_level(Node::Doctype { name });
            }
            Token::Comment(text) => {
                self.add_top_level(Node::Comment(text));
            }
            Token::Text(text) => {
                if text.is_empty() {
                    return;
                }
                let last = match self.current_parent() {
                    Some(p) => match &self.nodes[p] {
                        Node::Element { children, .. } => children.last().copied(),
                        _ => None,
                    },
                    None => self.document_children.last().copied(),
                };
                if let Some(l) = last {
                    if let Node::Text(prev) = &mut self.nodes[l] {
                        prev.push_str(&text);
                        return;
                    }
                }
                self.add_top_level(Node::Text(text));
            }
            Token::StartTag { name, attrs, .. } => {
                // Implied end tags.
                let pop_while_top = |b: &mut Self, names: &[&str]| {
                    while let Some(&id) = b.open.last() {
                        if names.contains(&b.name_of(id)) {
                            b.open.pop();
                        } else {
                            break;
                        }
                    }
                };
                if name == "li" {
                    pop_while_top(self, &["li"]);
                } else if name == "dt" || name == "dd" {
                    pop_while_top(self, &["dt", "dd"]);
                } else if name == "option" {
                    pop_while_top(self, &["option"]);
                }
                if closes_p(&name) {
                    if let Some(idx) = self.open.iter().rposition(|&id| self.name_of(id) == "p") {
                        self.open.truncate(idx);
                    }
                }

                let id = self.add_top_level(Node::Element {
                    name: name.clone(),
                    attrs,
                    children: Vec::new(),
                });
                // HTML ignores self-closing on non-void elements, so we push
                // unconditionally except for void elements.
                if !is_void(&name) {
                    self.open.push(id);
                }
            }
            Token::EndTag { name } => {
                if let Some(idx) = self.open_index(&name) {
                    self.open.truncate(idx);
                } else if name == "p" {
                    // A stray </p> inserts an empty paragraph, per spec.
                    self.add_top_level(Node::Element {
                        name: "p".into(),
                        attrs: Vec::new(),
                        children: Vec::new(),
                    });
                }
                // Otherwise ignore the stray end tag.
            }
            Token::Eof => {
                self.open.clear();
            }
        }
    }

    fn finish(self) -> Dom {
        Dom {
            nodes: self.nodes,
            document_children: self.document_children,
        }
    }
}

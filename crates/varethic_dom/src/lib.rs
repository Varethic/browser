//! varethic-dom: a safe-Rust HTML parser — the first native engine component
//! that is *not* delegated to WebView2/Chromium.
//!
//! It provides:
//! - a WHATWG-subset [tokenizer](tokenizer) (data, tags, attributes, comments,
//!   doctype, RAWTEXT/RCDATA, character references), and
//! - a simplified [tree constructor](tree) producing a DOM arena.
//!
//! This is an early successor to `//third_party/blink/renderer/core/html/parser`.
//! It is deliberately partial: see `docs/ENGINE_MIGRATION.md` and
//! `docs/KNOWN_ISSUES.md` for what is missing (full insertion modes, adoption
//! agency, foster parenting, `html`/`head`/`body` synthesis, foreign content).

pub mod entities;
pub mod tokenizer;
pub mod tree;

pub use tokenizer::{Token, Tokenizer};
pub use tree::{is_void, Dom, Node};

#[cfg(test)]
mod tests {
    use super::*;

    fn norm(html: &str) -> String {
        Dom::parse(html).to_html()
    }

    #[test]
    fn basic_nesting() {
        assert_eq!(
            norm("<p>Hello <b>world</b></p>"),
            "<p>Hello <b>world</b></p>"
        );
    }

    #[test]
    fn implied_end_tag_for_li() {
        assert_eq!(norm("<ul><li>a<li>b</ul>"), "<ul><li>a</li><li>b</li></ul>");
    }

    #[test]
    fn implied_end_tag_for_p() {
        assert_eq!(norm("<p>a<p>b"), "<p>a</p><p>b</p>");
    }

    #[test]
    fn block_element_closes_open_p() {
        assert_eq!(norm("<p>a<div>b</div>"), "<p>a</p><div>b</div>");
    }

    #[test]
    fn void_elements_never_close() {
        assert_eq!(
            norm("<div><br><img src=x></div>"),
            "<div><br><img src=\"x\"></div>"
        );
    }

    #[test]
    fn unquoted_attribute() {
        assert_eq!(norm("<a href=/x>t</a>"), "<a href=\"/x\">t</a>");
    }

    #[test]
    fn boolean_attribute_serializes_empty() {
        assert_eq!(norm("<input disabled>"), "<input disabled=\"\">");
    }

    #[test]
    fn comments_and_doctype() {
        assert_eq!(norm("<!-- c -->"), "<!-- c -->");
        assert_eq!(
            norm("<!DOCTYPE html><html><body>x</body></html>"),
            "<!DOCTYPE html><html><body>x</body></html>"
        );
    }

    #[test]
    fn raw_text_element() {
        assert_eq!(
            norm("<script>if (a < b) { x('<p>') }</script>"),
            "<script>if (a &lt; b) { x('&lt;p&gt;') }</script>"
        );
    }

    #[test]
    fn rcdata_decodes_entities() {
        assert_eq!(norm("<title>a &amp; b</title>"), "<title>a &amp; b</title>");
    }

    #[test]
    fn text_entities_roundtrip() {
        assert_eq!(norm("<p>a &amp; b</p>"), "<p>a &amp; b</p>");
        assert_eq!(norm("<p>&#65;&#x42;</p>"), "<p>AB</p>");
    }

    #[test]
    fn uppercase_names_are_folded() {
        assert_eq!(norm("<DIV CLASS=X>"), "<div class=\"X\"></div>");
    }

    #[test]
    fn nested_inline_elements() {
        assert_eq!(norm("<b><i>x</i></b>"), "<b><i>x</i></b>");
    }

    #[test]
    fn query_helpers() {
        let dom = Dom::parse("<div><span class=k>hi</span></div>");
        assert!(dom.find("span").is_some());
        assert!(dom.find("nope").is_none());
        let span = dom.find("span").unwrap();
        assert_eq!(dom.text_content(span), "hi");
        assert_eq!(dom.element_names(), vec!["div", "span"]);
    }

    #[test]
    fn stray_end_tag_is_ignored() {
        assert_eq!(norm("<div>x</div></span>"), "<div>x</div>");
    }

    /// Documented divergences from full HTML5 parsing. These assert *our*
    /// current behaviour so that changes are intentional, not accidental.
    #[test]
    fn divergences_from_full_html5() {
        // 1. No html/head/body synthesis.
        assert_eq!(norm("<p>x"), "<p>x</p>");
        // 2. No adoption agency: `</b>` pops through `<i>` and the trailing
        //    `</i>` is ignored (Chromium emits an extra empty <i>).
        assert_eq!(norm("<b><i>x</b></i>"), "<b><i>x</i></b>");
        // 3. Self-closing is ignored on non-void elements (matches HTML, but we
        //    make the behaviour explicit).
        assert_eq!(norm("<div/>x"), "<div>x</div>");
    }
}

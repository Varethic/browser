//! varethic-url: a safe-Rust component implementing the core of Chromium's
//! `//url` (GURL) role for the Varethic browser.
//!
//! Scope for v0.1.0:
//! - Parse and validate absolute URLs (scheme, host, port, path, query, fragment).
//! - Canonicalize common cases (scheme/host lower-casing, empty path -> "/").
//! - Omnibox classification: decide whether raw user input is a URL to
//!   navigate to, or a search query to send to the default search engine
//!   (Google: `https://www.google.com/search?q={searchTerms}`).
//!
//! Chromium's //url is a large, battle-tested component built on the WHATWG
//! URL Standard. This crate intentionally starts small and is extended
//! incrementally; it does not yet claim full WHATWG conformance.

use std::fmt;

/// Default search engine URL template. `{searchTerms}` is replaced with the
/// percent-encoded query.
pub const DEFAULT_SEARCH_URL: &str = "https://www.google.com/search?q={searchTerms}";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Url {
    pub scheme: String,
    pub host: String,
    pub port: Option<u16>,
    pub path: String,
    pub query: Option<String>,
    pub fragment: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UrlError {
    Empty,
    MissingScheme,
    InvalidScheme,
    MissingHost,
    InvalidPort,
    InvalidCharacter,
    InvalidHost,
}

impl fmt::Display for UrlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UrlError::Empty => write!(f, "empty URL"),
            UrlError::MissingScheme => write!(f, "missing scheme"),
            UrlError::InvalidScheme => write!(f, "invalid scheme"),
            UrlError::MissingHost => write!(f, "missing host"),
            UrlError::InvalidPort => write!(f, "invalid port"),
            UrlError::InvalidCharacter => write!(f, "invalid character in URL"),
            UrlError::InvalidHost => write!(f, "invalid host"),
        }
    }
}

impl std::error::Error for UrlError {}

/// WHATWG-style path normalization for absolute paths: removes `.` segments and
/// resolves `..` segments, recognizing percent-encoded dots as Chromium does
/// (only lowercase `%2e` is treated as a dot; uppercase `%2E` is preserved,
/// matching Chromium's canonicalizer). Non-absolute paths are returned
/// unchanged (opaque-path schemes).
fn normalize_path(path: &str) -> String {
    if !path.starts_with('/') {
        return path.to_string();
    }
    let mut segments: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        let s = seg.to_ascii_lowercase();
        match s.as_str() {
            "" => {
                if segments.is_empty() {
                    segments.push(""); // keep the leading slash marker
                }
            }
            "." | "%2e" => {}
            ".." | ".%2e" | "%2e." | "%2e%2e" => {
                segments.pop();
            }
            _ => segments.push(seg),
        }
    }
    let joined = segments.join("/");
    if joined.is_empty() {
        "/".to_string()
    } else if !joined.starts_with('/') {
        format!("/{}", joined)
    } else {
        joined
    }
}

/// Canonicalize a host: trim trailing dot, lower-case, and convert to ASCII
/// (IDN/punycode via `idna`) as WHATWG URL does. Bracketed IPv6 literals are
/// passed through unchanged.
fn normalize_host(host: &str) -> Result<String, UrlError> {
    let h = host.trim_end_matches('.').to_ascii_lowercase();
    if h.starts_with('[') {
        return Ok(h); // IPv6 literal, already validated
    }
    if is_ipv4(&h) {
        return Ok(h);
    }
    idna::domain_to_ascii(&h).map_err(|_| UrlError::InvalidHost)
}

fn is_scheme_char(c: char, first: bool) -> bool {
    if first {
        c.is_ascii_alphabetic()
    } else {
        c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.'
    }
}

impl Url {
    /// Parse an absolute URL. Returns an error for relative URLs.
    pub fn parse(input: &str) -> Result<Url, UrlError> {
        let input = input.trim();
        if input.is_empty() {
            return Err(UrlError::Empty);
        }
        if input.chars().any(|c| c.is_control() || c == ' ') {
            // Whitespace inside a URL is not allowed; callers should trim.
            return Err(UrlError::InvalidCharacter);
        }

        let scheme_end = input.find("://").ok_or(UrlError::MissingScheme)?;
        let scheme = &input[..scheme_end];
        if scheme.is_empty()
            || !scheme
                .chars()
                .next()
                .map(|c| is_scheme_char(c, true))
                .unwrap_or(false)
            || !scheme.chars().all(|c| is_scheme_char(c, false))
        {
            return Err(UrlError::InvalidScheme);
        }
        let scheme = scheme.to_ascii_lowercase();

        let rest = &input[scheme_end + 3..];
        // Split off fragment, then query, then path.
        let (rest, fragment) = match rest.find('#') {
            Some(i) => (&rest[..i], Some(rest[i + 1..].to_string())),
            None => (rest, None),
        };
        let (rest, query) = match rest.find('?') {
            Some(i) => (&rest[..i], Some(rest[i + 1..].to_string())),
            None => (rest, None),
        };
        let (authority, path) = match rest.find('/') {
            Some(i) => (&rest[..i], rest[i..].to_string()),
            None => (rest, "/".to_string()),
        };

        // userinfo@host:port  (IPv6 literals are bracketed: [::1]:8080)
        let (userinfo, hostport) = match authority.rfind('@') {
            Some(i) => (Some(&authority[..i]), &authority[i + 1..]),
            None => (None, authority),
        };
        let (username, password) = match userinfo {
            Some(ui) => match ui.find(':') {
                Some(i) => (Some(ui[..i].to_string()), Some(ui[i + 1..].to_string())),
                None => (Some(ui.to_string()), None),
            },
            None => (None, None),
        };

        let (host, port) = if let Some(stripped) = hostport.strip_prefix('[') {
            // Bracketed IPv6 literal, optional :port after the closing bracket.
            let close = stripped.find(']').ok_or(UrlError::MissingHost)?;
            let h = format!("[{}]", &stripped[..close]);
            let after = &stripped[close + 1..];
            if after.is_empty() {
                (h, None)
            } else if let Some(p) = after.strip_prefix(':') {
                if p.is_empty() || !p.chars().all(|c| c.is_ascii_digit()) {
                    return Err(UrlError::InvalidPort);
                }
                (h, Some(p.parse().map_err(|_| UrlError::InvalidPort)?))
            } else {
                return Err(UrlError::InvalidPort);
            }
        } else {
            match hostport.rfind(':') {
                Some(i) => {
                    let h = &hostport[..i];
                    let p = &hostport[i + 1..];
                    if p.is_empty() || !p.chars().all(|c| c.is_ascii_digit()) {
                        return Err(UrlError::InvalidPort);
                    }
                    (
                        h.to_string(),
                        Some(p.parse().map_err(|_| UrlError::InvalidPort)?),
                    )
                }
                None => (hostport.to_string(), None),
            }
        };

        if host.is_empty() {
            if scheme != "file" {
                return Err(UrlError::MissingHost);
            }
            // file:///path — WHATWG file URLs have an empty host.
        } else {
            let host = normalize_host(&host)?;
            return Self::finish(
                scheme, host, port, path, query, fragment, username, password,
            );
        }
        Self::finish(
            scheme,
            String::new(),
            port,
            path,
            query,
            fragment,
            username,
            password,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn finish(
        scheme: String,
        host: String,
        port: Option<u16>,
        path: String,
        query: Option<String>,
        fragment: Option<String>,
        username: Option<String>,
        password: Option<String>,
    ) -> Result<Url, UrlError> {
        Ok(Url {
            scheme,
            host,
            port,
            path: if path.is_empty() {
                "/".into()
            } else {
                normalize_path(&path)
            },
            query,
            fragment,
            username,
            password,
        })
    }

    /// Serialize to a canonical string.
    pub fn to_url_string(&self) -> String {
        let mut s = format!("{}://", self.scheme);
        if let Some(u) = &self.username {
            s.push_str(u);
            if let Some(p) = &self.password {
                s.push(':');
                s.push_str(p);
            }
            s.push('@');
        }
        s.push_str(&self.host);
        if let Some(p) = self.port {
            s.push(':');
            s.push_str(&p.to_string());
        }
        s.push_str(&self.path);
        if let Some(q) = &self.query {
            s.push('?');
            s.push_str(q);
        }
        if let Some(f) = &self.fragment {
            s.push('#');
            s.push_str(f);
        }
        s
    }

    /// True for potentially trustworthy schemes used for top-level navigation.
    pub fn is_navigable_scheme(&self) -> bool {
        matches!(
            self.scheme.as_str(),
            "http" | "https" | "file" | "about" | "data"
        )
    }
}

impl fmt::Display for Url {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_url_string())
    }
}

/// Classification of a parsed host, useful for omnibox and security decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostKind {
    Ipv4,
    Ipv6,
    Localhost,
    Domain,
}

/// Returns true if `s` is a dotted-quad IPv4 literal.
pub fn is_ipv4(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    parts.len() == 4
        && parts.iter().all(|p| {
            !p.is_empty()
                && p.len() <= 3
                && p.chars().all(|c| c.is_ascii_digit())
                && p.parse::<u8>().is_ok()
        })
}

impl Url {
    /// Compute the WHATWG origin of this URL. Tuple origins for
    /// http/https/ws/wss/ftp (default port filled in when absent);
    /// everything else is opaque.
    pub fn origin(&self) -> Origin {
        let default = match self.scheme.as_str() {
            "http" | "ws" => Some(80),
            "https" | "wss" => Some(443),
            "ftp" => Some(21),
            _ => None,
        };
        match default {
            Some(d) => Origin::Tuple {
                scheme: self.scheme.clone(),
                host: self.host.clone(),
                port: Some(self.port.unwrap_or(d)),
            },
            None => Origin::Opaque,
        }
    }

    /// Classify the host: IPv4 literal, [IPv6] literal, localhost, or domain.
    pub fn host_kind(&self) -> HostKind {
        let h = self.host.trim_start_matches('[').trim_end_matches(']');
        if is_ipv4(h) {
            HostKind::Ipv4
        } else if self.host.starts_with('[') || h.contains(':') {
            HostKind::Ipv6
        } else if h.eq_ignore_ascii_case("localhost") {
            HostKind::Localhost
        } else {
            HostKind::Domain
        }
    }
}

/// An origin per the WHATWG HTML Standard, as far as varethic-url implements
/// it: a tuple (scheme, host, port) for potentially-tuple schemes, else an
/// opaque origin. Cross-process isolation and the opaque-origin nonce are
/// out of scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    Opaque,
    Tuple {
        scheme: String,
        host: String,
        port: Option<u16>,
    },
}

impl Origin {
    /// Serialize like WHATWG `Origin.prototype.toString()` for tuple origins
    /// (`scheme://host[:port]`); opaque origins serialize as `"null"`.
    pub fn serialize(&self) -> String {
        match self {
            Origin::Opaque => "null".to_string(),
            Origin::Tuple { scheme, host, port } => {
                let mut s = format!("{}://{}", scheme, host);
                if let Some(p) = port {
                    s.push(':');
                    s.push_str(&p.to_string());
                }
                s
            }
        }
    }

    /// Same absolute origin (tuple components equal). Opaque == Opaque is false
    /// (opaque origins are never same-origin with each other), matching
    /// WHATWG's "same origin" definition.
    pub fn is_same_origin(&self, other: &Origin) -> bool {
        match (self, other) {
            (
                Origin::Tuple {
                    scheme: s1,
                    host: h1,
                    port: p1,
                },
                Origin::Tuple {
                    scheme: s2,
                    host: h2,
                    port: p2,
                },
            ) => s1 == s2 && h1 == h2 && p1 == p2,
            _ => false,
        }
    }
}

/// What the omnibox should do with raw user input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OmniboxTarget {
    /// Navigate to this URL.
    Navigate(Url),
    /// Run a search with the default search engine.
    Search(String),
}

/// Percent-encode a query string for use in a search URL (RFC 3986 unreserved kept).
pub fn percent_encode_query(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

fn looks_like_host(s: &str) -> bool {
    // Single label like "localhost" or dotted host with valid TLD-ish ending.
    if s.eq_ignore_ascii_case("localhost") {
        return true;
    }
    let labels: Vec<&str> = s.split('.').collect();
    if labels.len() < 2 {
        return false;
    }
    let tld = labels.last().unwrap();
    tld.len() >= 2
        && tld.chars().all(|c| c.is_ascii_alphabetic())
        && labels.iter().all(|l| {
            !l.is_empty()
                && l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
                && !l.starts_with('-')
                && !l.ends_with('-')
        })
}

/// Validate a search URL template: must contain `{searchTerms}` exactly once
/// and use http(s).
pub fn is_valid_search_template(template: &str) -> bool {
    template.matches("{searchTerms}").count() == 1
        && (template.starts_with("https://") || template.starts_with("http://"))
}

/// Classify omnibox input with an explicit search template. Falls back to the
/// Google default when the template is invalid.
pub fn classify_omnibox_input_with_search(input: &str, template: &str) -> OmniboxTarget {
    let effective = if is_valid_search_template(template) {
        template
    } else {
        DEFAULT_SEARCH_URL
    };
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return OmniboxTarget::Search(effective.replace("{searchTerms}", ""));
    }

    // Explicit scheme, e.g. https://example.com or http://...
    if let Ok(url) = Url::parse(trimmed) {
        return OmniboxTarget::Navigate(url);
    }

    // Bare host without scheme and without spaces: treat as https.
    let host_candidate = trimmed.split(['/', '?', '#']).next().unwrap_or("");
    let (host_only, _) = match host_candidate.rfind(':') {
        Some(i) => {
            let p = &host_candidate[i + 1..];
            if !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()) {
                (&host_candidate[..i], Some(p))
            } else {
                (host_candidate, None)
            }
        }
        None => (host_candidate, None),
    };

    if !trimmed.contains(' ') && looks_like_host(host_only) {
        let with_scheme = format!("https://{}", trimmed);
        if let Ok(url) = Url::parse(&with_scheme) {
            return OmniboxTarget::Navigate(url);
        }
    }

    OmniboxTarget::Search(effective.replace("{searchTerms}", &percent_encode_query(trimmed)))
}

/// Classify omnibox input, mirroring Chromium's `AutocompleteInput` logic in
/// simplified form: explicit scheme URLs navigate; bare hosts navigate to
/// https; everything else is a search.
pub fn classify_omnibox_input(input: &str) -> OmniboxTarget {
    classify_omnibox_input_with_search(input, DEFAULT_SEARCH_URL)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_https() {
        let u = Url::parse("https://Example.com/path?q=1#frag").unwrap();
        assert_eq!(u.scheme, "https");
        assert_eq!(u.host, "example.com");
        assert_eq!(u.path, "/path");
        assert_eq!(u.query.as_deref(), Some("q=1"));
        assert_eq!(u.fragment.as_deref(), Some("frag"));
        assert_eq!(u.to_url_string(), "https://example.com/path?q=1#frag");
    }

    #[test]
    fn empty_path_becomes_slash() {
        let u = Url::parse("http://example.com").unwrap();
        assert_eq!(u.path, "/");
    }

    #[test]
    fn parses_port_and_userinfo() {
        let u = Url::parse("https://user:pass@192.168.0.1:8080/a").unwrap();
        assert_eq!(u.host, "192.168.0.1");
        assert_eq!(u.port, Some(8080));
        assert_eq!(u.username.as_deref(), Some("user"));
        assert_eq!(u.password.as_deref(), Some("pass"));
    }

    #[test]
    fn rejects_invalid() {
        assert_eq!(Url::parse(""), Err(UrlError::Empty));
        assert_eq!(Url::parse("example.com"), Err(UrlError::MissingScheme));
        assert_eq!(Url::parse("https://"), Err(UrlError::MissingHost));
        assert_eq!(
            Url::parse("https://example.com:abc/"),
            Err(UrlError::InvalidPort)
        );
        assert_eq!(
            Url::parse("https://exam ple.com/"),
            Err(UrlError::InvalidCharacter)
        );
    }

    #[test]
    fn omnibox_navigates_explicit_urls() {
        match classify_omnibox_input("https://varethic.in.th") {
            OmniboxTarget::Navigate(u) => assert_eq!(u.host, "varethic.in.th"),
            other => panic!("expected navigate, got {:?}", other),
        }
    }

    #[test]
    fn omnibox_bare_host_defaults_to_https() {
        match classify_omnibox_input("varethic.in.th") {
            OmniboxTarget::Navigate(u) => {
                assert_eq!(u.scheme, "https");
                assert_eq!(u.host, "varethic.in.th");
            }
            other => panic!("expected navigate, got {:?}", other),
        }
    }

    #[test]
    fn origin_tuple_for_http() {
        let u = Url::parse("https://example.com/path?q=1").unwrap();
        assert_eq!(u.origin().serialize(), "https://example.com:443");
        let u2 = Url::parse("https://example.com:8443/").unwrap();
        assert_eq!(u2.origin().serialize(), "https://example.com:8443");
        let u3 = Url::parse("http://example.com/").unwrap();
        assert_eq!(u3.origin().serialize(), "http://example.com:80");
        assert!(u
            .origin()
            .is_same_origin(&Url::parse("https://example.com/").unwrap().origin()));
        assert!(!u
            .origin()
            .is_same_origin(&Url::parse("https://example.com:8443/").unwrap().origin()));
        assert!(!u
            .origin()
            .is_same_origin(&Url::parse("http://example.com/").unwrap().origin()));
    }

    #[test]
    fn origin_opaque_for_file() {
        let u = Url::parse("file:///tmp/x").unwrap();
        assert_eq!(u.origin(), Origin::Opaque);
        assert_eq!(u.origin().serialize(), "null");
        assert!(!u.origin().is_same_origin(&u.origin()));
    }

    #[test]
    fn idn_host_is_punycoded() {
        let u = Url::parse("https://bücher.de/").unwrap();
        assert_eq!(u.host, "xn--bcher-kva.de");
        let u2 = Url::parse("http://EXAMPLE.com./path").unwrap();
        assert_eq!(u2.host, "example.com");
        let u3 = Url::parse("http://192.168.0.1/").unwrap();
        assert_eq!(u3.host, "192.168.0.1");
    }

    /// Differential test vectors derived from Chromium's `url_unittest.cc`
    /// (//url, Apache-style BSD license, see docs/CHROMIUM_REFERENCE.md).
    /// Where our parser intentionally diverges, the case is in the mismatch list.
    #[test]
    fn chromium_differential_vectors() {
        let check = |input: &str, expected_path: &str| {
            let u = Url::parse(input).unwrap();
            assert_eq!(u.path, expected_path, "path mismatch for {}", input);
        };
        // Components: GetPath/GetQuery/GetRef equivalents.
        let u = Url::parse("http://user:pass@google.com:99/foo;bar?q=a#ref").unwrap();
        assert_eq!(u.scheme, "http");
        assert_eq!(u.username.as_deref(), Some("user"));
        assert_eq!(u.password.as_deref(), Some("pass"));
        assert_eq!(u.host, "google.com");
        assert_eq!(u.port, Some(99));
        assert_eq!(u.path, "/foo;bar");
        assert_eq!(u.query.as_deref(), Some("q=a"));
        assert_eq!(u.fragment.as_deref(), Some("ref"));

        // Path collapsing (GURLTest.Components).
        check("http://example.com/a/./b/c/d/../../e", "/a/b/e");
        // Percent-escaped dots (GURLTest.Components): lowercase %2e collapses,
        // uppercase %2E preserved.
        check(
            "http://example.com/a/./%2e/i%2E%2F%2fj?q=r%2Es",
            "/a/i%2E%2F%2fj",
        );
        // IDNA host (GURLTest.Components).
        let idna = Url::parse("http://Bücher.exAMple/").unwrap();
        assert_eq!(idna.host, "xn--bcher-kva.example");
        // Trailing-dot domain.
        let trailing = Url::parse("http://example.com./").unwrap();
        assert_eq!(trailing.host, "example.com");
    }

    #[test]
    fn known_divergences_from_chromium() {
        // Chromium tolerates missing slashes after the scheme
        // ("http:example.com" -> "http://example.com/"); varethic-url requires "://".
        assert_eq!(Url::parse("http:example.com"), Err(UrlError::MissingScheme));
        // Chromium percent-encodes non-ASCII in paths/queries; varethic-url keeps raw.
        let u = Url::parse("http://example.com/β").unwrap();
        assert_eq!(u.path, "/β");
        // file:// with empty host is supported; Windows drive-relative paths
        // ("c:\\foo.txt" -> "file:///c:/foo.txt") are NOT yet implemented.
    }

    #[test]
    fn search_template_validation() {
        assert!(is_valid_search_template(
            "https://www.google.com/search?q={searchTerms}"
        ));
        assert!(!is_valid_search_template("https://example.com/"));
        assert!(!is_valid_search_template("ftp://x/?q={searchTerms}"));
        assert!(!is_valid_search_template(
            "https://x/?q={searchTerms}&r={searchTerms}"
        ));
    }

    #[test]
    fn custom_search_template_is_used() {
        let t = "https://duckduckgo.com/?q={searchTerms}";
        match classify_omnibox_input_with_search("rust lang", t) {
            OmniboxTarget::Search(u) => {
                assert!(u.starts_with("https://duckduckgo.com/?q="));
                assert!(u.contains("rust+lang"));
            }
            other => panic!("expected search, got {:?}", other),
        }
        // Invalid template falls back to Google.
        match classify_omnibox_input_with_search("rust lang", "not a url") {
            OmniboxTarget::Search(u) => assert!(u.contains("google.com/search?q=")),
            other => panic!("expected search, got {:?}", other),
        }
    }

    #[test]
    fn host_kind_detection() {
        assert_eq!(
            Url::parse("http://[::1]:8080/").unwrap().host_kind(),
            HostKind::Ipv6
        );
        assert_eq!(
            Url::parse("http://192.168.0.1/").unwrap().host_kind(),
            HostKind::Ipv4
        );
        assert_eq!(
            Url::parse("http://localhost:3000/").unwrap().host_kind(),
            HostKind::Localhost
        );
        assert_eq!(
            Url::parse("https://example.com/").unwrap().host_kind(),
            HostKind::Domain
        );
        assert!(is_ipv4("10.0.0.255"));
        assert!(!is_ipv4("10.0.0.256"));
        assert!(!is_ipv4("example.com"));
    }

    #[test]
    fn omnibox_localhost_navigates() {
        match classify_omnibox_input("localhost:3000") {
            OmniboxTarget::Navigate(u) => assert_eq!(u.port, Some(3000)),
            other => panic!("expected navigate, got {:?}", other),
        }
    }

    #[test]
    fn omnibox_search_for_plain_words() {
        match classify_omnibox_input("rust browser engine") {
            OmniboxTarget::Search(u) => {
                assert!(u.starts_with("https://www.google.com/search?q="));
                assert!(u.contains("rust+browser+engine"));
            }
            other => panic!("expected search, got {:?}", other),
        }
    }

    #[test]
    fn percent_encoding_works() {
        assert_eq!(percent_encode_query("a b&c=d"), "a+b%26c%3Dd");
    }
}

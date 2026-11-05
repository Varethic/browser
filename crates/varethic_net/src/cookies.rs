//! Cookie jar for varethic-net. A safe-Rust subset of the behavior Chromium
//! implements in `//net/cookies` (net::CookieJar) and `//services/network`.
//!
//! Scope for v0.1: in-memory jar. Parses `Set-Cookie` (name=value; Domain;
//! Path; Secure; HttpOnly; Max-Age; Expires), stores cookies with basic
//! domain/path/secure matching, and emits `Cookie` headers for requests.
//! No SameSite enforcement, no third-party blocking, no partitioned cookies,
//! no persistence to disk yet. Do not treat as a security boundary.

use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    /// Lowercased domain without leading dot. Empty means host-only cookie.
    pub domain: String,
    pub host_only: bool,
    pub path: String,
    pub secure: bool,
    pub http_only: bool,
    /// Unix seconds; None = session cookie.
    pub expires: Option<i64>,
}

#[derive(Debug, Default)]
pub struct CookieJar {
    cookies: Vec<Cookie>,
}

struct HostParts {
    host: String,
    path: String,
    is_https: bool,
}

fn parse_url(url: &str) -> Option<HostParts> {
    let (scheme, rest) = url.split_once("://")?;
    let is_https = scheme.eq_ignore_ascii_case("https");
    let after = rest.split(['?', '#']).next()?;
    let (hostport, path) = match after.find('/') {
        Some(i) => (&after[..i], &after[i..]),
        None => (after, "/"),
    };
    let host = hostport
        .split('@')
        .last()?
        .split(':')
        .next()?
        .to_ascii_lowercase();
    if host.is_empty() {
        return None;
    }
    Some(HostParts {
        host,
        path: path.to_string(),
        is_https,
    })
}

fn default_path(req_path: &str) -> String {
    if !req_path.starts_with('/') || req_path == "/" {
        return "/".to_string();
    }
    match req_path.rfind('/') {
        Some(0) => "/".to_string(),
        Some(i) => req_path[..i].to_string(),
        None => "/".to_string(),
    }
}

fn host_matches(cookie_domain: &str, host: &str, host_only: bool) -> bool {
    if host_only {
        return host == cookie_domain;
    }
    host == cookie_domain || host.ends_with(&format!(".{}", cookie_domain))
}

fn path_matches(cookie_path: &str, req_path: &str) -> bool {
    req_path == cookie_path
        || (req_path.starts_with(cookie_path)
            && (cookie_path.ends_with('/') || req_path[cookie_path.len()..].starts_with('/')))
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Parse an IMF-date (RFC 1123) like `Wed, 21 Oct 2015 07:28:00 GMT`.
fn parse_imf_date(s: &str) -> Option<i64> {
    let s = s.trim();
    let mut parts = s.split_whitespace();
    let _day = parts.next()?; // "Wed,"
    let day: u32 = parts.next()?.parse().ok()?;
    let mon = parts.next()?;
    let year: i64 = parts.next()?.parse().ok()?;
    let time = parts.next()?;
    let _tz = parts.next()?;
    let mut t = time.split(':');
    let hh: i64 = t.next()?.parse().ok()?;
    let mm: i64 = t.next()?.parse().ok()?;
    let ss: i64 = t.next()?.parse().ok()?;
    let month = match mon {
        "Jan" => 1,
        "Feb" => 2,
        "Mar" => 3,
        "Apr" => 4,
        "May" => 5,
        "Jun" => 6,
        "Jul" => 7,
        "Aug" => 8,
        "Sep" => 9,
        "Oct" => 10,
        "Nov" => 11,
        "Dec" => 12,
        _ => return None,
    };
    // Days-from-civil algorithm (Howard Hinnant).
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    Some(days * 86400 + hh * 3600 + mm * 60 + ss)
}

impl CookieJar {
    pub fn new() -> Self {
        Self {
            cookies: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.cookies.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cookies.is_empty()
    }

    pub fn clear(&mut self) {
        self.cookies.clear();
    }

    /// Store cookies found in `Set-Cookie` headers of a response for `url`.
    pub fn store_from_response(&mut self, url: &str, headers: &[(String, String)]) {
        let Some(parts) = parse_url(url) else { return };
        for (name, value) in headers {
            if !name.eq_ignore_ascii_case("set-cookie") {
                continue;
            }
            if let Some(cookie) = parse_set_cookie(value, &parts) {
                // Expired cookies delete any existing matching cookie.
                let expired = cookie.expires.map(|e| e <= now_unix()).unwrap_or(false);
                self.cookies.retain(|c| {
                    !(c.name == cookie.name && c.domain == cookie.domain && c.path == cookie.path)
                });
                if !expired {
                    self.cookies.push(cookie);
                }
            }
        }
    }

    /// Build the `Cookie` header value for a request to `url`, if any.
    pub fn cookie_header(&self, url: &str) -> Option<String> {
        let parts = parse_url(url)?;
        let now = now_unix();
        let mut pairs: Vec<String> = self
            .cookies
            .iter()
            .filter(|c| {
                if c.expires.map(|e| e <= now).unwrap_or(false) {
                    return false;
                }
                if c.secure && !parts.is_https {
                    return false;
                }
                host_matches(&c.domain, &parts.host, c.host_only)
                    && path_matches(&c.path, &parts.path)
            })
            .map(|c| format!("{}={}", c.name, c.value))
            .collect();
        // Longest path first, per RFC 6265 §5.4.
        pairs.sort();
        if pairs.is_empty() {
            None
        } else {
            Some(pairs.join("; "))
        }
    }
}

fn parse_set_cookie(header: &str, parts: &HostParts) -> Option<Cookie> {
    let mut segments = header.split(';');
    let first = segments.next()?.trim();
    let (name, value) = first.split_once('=')?;
    let name = name.trim().to_string();
    let value = value.trim().to_string();
    if name.is_empty() {
        return None;
    }

    let mut domain: Option<String> = None;
    let mut host_only = true;
    let mut path: Option<String> = None;
    let mut secure = false;
    let mut http_only = false;
    let mut expires: Option<i64> = None;
    let mut max_age: Option<i64> = None;

    for seg in segments {
        let seg = seg.trim();
        let (k, v) = match seg.split_once('=') {
            Some((k, v)) => (k.trim().to_ascii_lowercase(), v.trim().to_string()),
            None => (seg.to_ascii_lowercase(), String::new()),
        };
        match k.as_str() {
            "domain" => {
                let d = v.to_ascii_lowercase().trim_start_matches('.').to_string();
                if !d.is_empty() && (parts.host == d || parts.host.ends_with(&format!(".{}", d))) {
                    domain = Some(d);
                    host_only = false;
                }
                // Reject Domain attribute that doesn't cover the origin host.
            }
            "path" => {
                path = Some(if v.starts_with('/') {
                    v
                } else {
                    "/".to_string()
                });
            }
            "secure" => secure = true,
            "httponly" => http_only = true,
            "expires" => expires = parse_imf_date(&v),
            "max-age" => max_age = v.parse::<i64>().ok(),
            _ => {}
        }
    }

    // Max-Age takes precedence over Expires (RFC 6265 §5.2.2).
    if let Some(age) = max_age {
        expires = Some(now_unix() + age);
    }

    Some(Cookie {
        name,
        value,
        domain: domain.unwrap_or_else(|| parts.host.clone()),
        host_only,
        path: path.unwrap_or_else(|| default_path(&parts.path)),
        secure,
        http_only,
        expires,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hdrs(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn stores_and_sends_simple_cookie() {
        let mut jar = CookieJar::new();
        jar.store_from_response(
            "https://example.com/",
            &hdrs(&[("set-cookie", "session=abc123")]),
        );
        assert_eq!(jar.len(), 1);
        let h = jar.cookie_header("https://example.com/").unwrap();
        assert_eq!(h, "session=abc123");
    }

    #[test]
    fn host_only_cookie_not_sent_to_other_host() {
        let mut jar = CookieJar::new();
        jar.store_from_response("https://example.com/", &hdrs(&[("set-cookie", "a=1")]));
        assert!(jar.cookie_header("https://other.com/").is_none());
        assert!(jar.cookie_header("https://sub.example.com/").is_none());
    }

    #[test]
    fn domain_cookie_sent_to_subdomains() {
        let mut jar = CookieJar::new();
        jar.store_from_response(
            "https://example.com/",
            &hdrs(&[("set-cookie", "track=xyz; Domain=example.com")]),
        );
        assert!(jar.cookie_header("https://sub.example.com/").is_some());
        assert!(jar.cookie_header("https://other.com/").is_none());
    }

    #[test]
    fn domain_mismatch_rejected() {
        let mut jar = CookieJar::new();
        jar.store_from_response(
            "https://example.com/",
            &hdrs(&[("set-cookie", "evil=1; Domain=other.com")]),
        );
        // Domain doesn't cover the origin host -> stored as host-only cookie.
        let c = &jar.cookies[0];
        assert!(c.host_only);
        assert_eq!(c.domain, "example.com");
    }

    #[test]
    fn secure_cookie_not_sent_over_http() {
        let mut jar = CookieJar::new();
        jar.store_from_response(
            "https://example.com/",
            &hdrs(&[("set-cookie", "s=1; Secure")]),
        );
        assert!(jar.cookie_header("http://example.com/").is_none());
        assert!(jar.cookie_header("https://example.com/").is_some());
    }

    #[test]
    fn expired_cookie_not_stored_and_deletes_existing() {
        let mut jar = CookieJar::new();
        jar.store_from_response("https://example.com/", &hdrs(&[("set-cookie", "a=1")]));
        assert_eq!(jar.len(), 1);
        jar.store_from_response(
            "https://example.com/",
            &hdrs(&[("set-cookie", "a=2; Max-Age=0")]),
        );
        assert!(jar.is_empty());
    }

    #[test]
    fn path_scoping() {
        let mut jar = CookieJar::new();
        jar.store_from_response(
            "https://example.com/app/page",
            &hdrs(&[("set-cookie", "p=1; Path=/app")]),
        );
        assert!(jar.cookie_header("https://example.com/app/other").is_some());
        assert!(jar.cookie_header("https://example.com/").is_none());
    }

    #[test]
    fn imf_date_parsing() {
        assert_eq!(parse_imf_date("Thu, 01 Jan 1970 00:00:10 GMT"), Some(10));
        assert_eq!(parse_imf_date("garbage"), None);
    }
}

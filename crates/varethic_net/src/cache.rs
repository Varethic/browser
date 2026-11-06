//! In-memory HTTP response cache for varethic-net — a first step toward the
//! `//net` cache (net::HttpCache) and `net/disk_cache`.
//!
//! Scope: in-memory only. Honors the `Cache-Control` directives `no-store`,
//! `no-cache`, `must-revalidate` (never cached), and `max-age` (entries
//! expire). Stale entries that carry an `ETag` or `Last-Modified` validator can
//! be revalidated with a conditional request (see [`HttpCache::stale_validators`]
//! and [`HttpCache::confirm_not_modified`]). No `Vary` support, no private vs
//! shared cache distinction, no disk backend, no partial content.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use bytes::Bytes;

use crate::Response;

#[derive(Debug, Clone)]
struct CacheEntry {
    status: u16,
    headers: Vec<(String, String)>,
    body: Bytes,
    stored_at: Instant,
    max_age: Duration,
}

impl CacheEntry {
    fn is_fresh(&self) -> bool {
        self.stored_at.elapsed() < self.max_age
    }
}

/// Parse `Cache-Control`; returns `None` when the response must not be stored
/// (`no-store`, `no-cache`, `must-revalidate`, `private`), else the max-age
/// to cache for (default 0 if unspecified — i.e. not cached).
pub fn cacheable_max_age(cache_control: &str) -> Option<Duration> {
    let mut max_age: Option<u64> = None;
    for token in cache_control.split(',') {
        let token = token.trim().to_ascii_lowercase();
        match token.as_str() {
            "no-store" | "no-cache" | "must-revalidate" | "private" => return None,
            _ => {
                if let Some(v) = token.strip_prefix("max-age=") {
                    max_age = v.trim_matches('"').parse::<u64>().ok();
                }
            }
        }
    }
    max_age.map(Duration::from_secs)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Validators {
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

impl Validators {
    /// True when at least one conditional header can be sent.
    pub fn is_empty(&self) -> bool {
        self.etag.is_none() && self.last_modified.is_none()
    }
}

#[derive(Debug)]
pub struct HttpCache {
    entries: HashMap<String, CacheEntry>,
    max_entries: usize,
    hits: u64,
    misses: u64,
}

impl Default for HttpCache {
    fn default() -> Self {
        Self::new(128)
    }
}

impl HttpCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: HashMap::new(),
            max_entries: max_entries.max(1),
            hits: 0,
            misses: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn hits(&self) -> u64 {
        self.hits
    }

    pub fn misses(&self) -> u64 {
        self.misses
    }

    /// Look up a fresh cached response for `url`. A stale entry is left in
    /// place (not deleted) so it can be revalidated later; this counts as a
    /// miss.
    pub fn get(&mut self, url: &str) -> Option<Response> {
        match self.entries.get(url) {
            Some(entry) if entry.is_fresh() => {
                self.hits += 1;
                Some(Response {
                    status: entry.status,
                    headers: entry.headers.clone(),
                    body: entry.body.clone(),
                })
            }
            Some(_) => {
                self.misses += 1;
                None
            }
            None => {
                self.misses += 1;
                None
            }
        }
    }

    /// Conditionally-usable validators for a *stale but stored* entry.
    /// Returns `None` when no entry exists or it has no ETag/Last-Modified.
    pub fn stale_validators(&self, url: &str) -> Option<Validators> {
        let entry = self.entries.get(url)?;
        if entry.is_fresh() {
            return None;
        }
        let find = |name: &str| {
            entry
                .headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(name))
                .map(|(_, v)| v.clone())
        };
        let v = Validators {
            etag: find("etag"),
            last_modified: find("last-modified"),
        };
        if v.is_empty() {
            None
        } else {
            Some(v)
        }
    }

    /// Called after a `304 Not Modified`: refresh the stored entry's age so it
    /// is fresh again, and return the (still-valid) stored response.
    pub fn confirm_not_modified(&mut self, url: &str) -> Option<Response> {
        let entry = self.entries.get_mut(url)?;
        entry.stored_at = Instant::now();
        self.hits += 1;
        Some(Response {
            status: entry.status,
            headers: entry.headers.clone(),
            body: entry.body.clone(),
        })
    }

    /// Drop a stored entry (revalidation failed, or explicit invalidation).
    pub fn invalidate(&mut self, url: &str) {
        self.entries.remove(url);
    }

    /// Force an entry to be treated as stale on the next lookup, keeping its
    /// validators. Used for explicit invalidation and by tests.
    pub fn expire(&mut self, url: &str) {
        if let Some(entry) = self.entries.get_mut(url) {
            entry.stored_at = Instant::now() - entry.max_age;
        }
    }

    /// Store a response if its `Cache-Control` permits. Returns true if stored.
    pub fn store(&mut self, url: &str, response: &Response) -> bool {
        let cc = response
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("cache-control"))
            .map(|(_, v)| v.as_str())
            .unwrap_or("");
        let Some(max_age) = cacheable_max_age(cc) else {
            return false;
        };
        if max_age.is_zero() || response.status != 200 {
            return false;
        }
        if self.entries.len() >= self.max_entries {
            // Evict an arbitrary (oldest-inserted-ish) entry; HashMap order is
            // not insertion order, so eviction is arbitrary but bounded.
            if let Some(key) = self.entries.keys().next().cloned() {
                self.entries.remove(&key);
            }
        }
        self.entries.insert(
            url.to_string(),
            CacheEntry {
                status: response.status,
                headers: response.headers.clone(),
                body: response.body.clone(),
                stored_at: Instant::now(),
                max_age,
            },
        );
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resp(cc: &str, body: &str) -> Response {
        Response {
            status: 200,
            headers: if cc.is_empty() {
                vec![]
            } else {
                vec![("cache-control".to_string(), cc.to_string())]
            },
            body: Bytes::from(body.as_bytes().to_vec()),
        }
    }

    #[test]
    fn no_store_not_cached() {
        let mut c = HttpCache::new(4);
        assert!(!c.store("https://a/", &resp("no-store", "x")));
        assert!(c.is_empty());
    }

    #[test]
    fn nothing_without_max_age() {
        let mut c = HttpCache::new(4);
        assert!(!c.store("https://a/", &resp("", "x")));
        assert!(!c.store("https://b/", &resp("public", "x")));
    }

    #[test]
    fn max_age_roundtrip_and_hit_count() {
        let mut c = HttpCache::new(4);
        assert!(c.store("https://a/", &resp("max-age=60", "cached-body")));
        let got = c.get("https://a/").unwrap();
        assert_eq!(got.body_string().unwrap(), "cached-body");
        assert_eq!(c.hits(), 1);
        assert_eq!(c.misses(), 0);
        assert!(c.get("https://b/").is_none());
        assert_eq!(c.misses(), 1);
    }

    #[test]
    fn private_not_cached() {
        let mut c = HttpCache::new(4);
        assert!(!c.store("https://a/", &resp("private, max-age=60", "x")));
    }

    #[test]
    fn non_200_not_cached() {
        let mut c = HttpCache::new(4);
        let mut r = resp("max-age=60", "x");
        r.status = 404;
        assert!(!c.store("https://a/", &r));
    }

    #[test]
    fn capacity_eviction_bounds_size() {
        let mut c = HttpCache::new(2);
        c.store("https://a/", &resp("max-age=60", "a"));
        c.store("https://b/", &resp("max-age=60", "b"));
        c.store("https://c/", &resp("max-age=60", "c"));
        assert_eq!(c.len(), 2);
    }

    #[test]
    fn must_revalidate_not_cached() {
        let mut c = HttpCache::new(4);
        assert!(!c.store("https://a/", &resp("max-age=60, must-revalidate", "x")));
    }

    fn resp_with(extra: &[(&str, &str)]) -> Response {
        let mut headers = vec![("cache-control".to_string(), "max-age=60".to_string())];
        for (k, v) in extra {
            headers.push((k.to_string(), v.to_string()));
        }
        Response {
            status: 200,
            headers,
            body: Bytes::from_static(b"v1-body"),
        }
    }

    #[test]
    fn stale_validators_extracted_and_refresh() {
        let mut c = HttpCache::new(4);
        assert!(c.store("https://a/", &resp_with(&[("ETag", "\"v1\"")])));
        // Fresh entry exposes no validators (nothing to revalidate).
        assert!(c.stale_validators("https://a/").is_none());
        c.expire("https://a/");
        let v = c.stale_validators("https://a/").expect("validators");
        assert_eq!(v.etag.as_deref(), Some("\"v1\"")); // case-insensitive header lookup
        assert!(v.last_modified.is_none());
        // 304 path: entry becomes fresh again and body is served.
        let body = c.confirm_not_modified("https://a/").unwrap();
        assert_eq!(body.body_string().unwrap(), "v1-body");
        assert!(c.stale_validators("https://a/").is_none());
    }

    #[test]
    fn last_modified_validator_and_invalidate() {
        let mut c = HttpCache::new(4);
        assert!(c.store(
            "https://a/",
            &resp_with(&[("last-modified", "Wed, 21 Oct 2015 07:28:00 GMT")])
        ));
        c.expire("https://a/");
        let v = c.stale_validators("https://a/").unwrap();
        assert_eq!(
            v.last_modified.as_deref(),
            Some("Wed, 21 Oct 2015 07:28:00 GMT")
        );
        c.invalidate("https://a/");
        assert!(c.stale_validators("https://a/").is_none());
        assert!(c.get("https://a/").is_none());
    }

    #[test]
    fn stale_without_validators_has_none() {
        let mut c = HttpCache::new(4);
        assert!(c.store("https://a/", &resp("max-age=60", "x")));
        c.expire("https://a/");
        assert!(c.stale_validators("https://a/").is_none());
    }
}

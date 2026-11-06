//! In-memory HTTP response cache for varethic-net — a first step toward the
//! `//net` cache (net::HttpCache) and `net/disk_cache`.
//!
//! Scope for v0.1: in-memory only. Honors the `Cache-Control` directives
//! `no-store`, `no-cache`, `must-revalidate` (never cached), and `max-age`
//! (entries expire). No ETag/Last-Modified revalidation, no Vary, no private
//! vs shared cache distinction, no disk backend, no partial content.

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

    /// Look up a fresh cached response for `url`.
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
                // Stale: drop it.
                self.entries.remove(url);
                self.misses += 1;
                None
            }
            None => {
                self.misses += 1;
                None
            }
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
}

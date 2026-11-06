//! varethic-net: a safe-Rust network stack subset, the first step in
//! replacing Chromium `//net` for the Varethic browser.
//!
//! Scope for v0.1: async HTTP/1.1 GET over HTTP and HTTPS (rustls,
//! webpki-roots). Redirect following, cookies, caching, proxies, HTTP/2,
//! and connection pooling beyond hyper's defaults are NOT yet implemented.
//! Do not claim //net parity.

pub mod cache;
pub mod cookies;
pub use cache::HttpCache;
pub use cookies::CookieJar;

use std::time::Duration;

use bytes::Bytes;
use http_body_util::{BodyExt, Empty};
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use thiserror::Error;

use varethic_url::Url;

#[derive(Debug, Error)]
pub enum NetError {
    #[error("URL error: {0}")]
    Url(#[from] varethic_url::UrlError),
    #[error("unsupported scheme: {0} (only http/https supported)")]
    UnsupportedScheme(String),
    #[error("HTTP error: {0}")]
    Http(String),
    #[error("timeout after {0:?}")]
    Timeout(Duration),
    #[error("too many redirects")]
    TooManyRedirects,
}

#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Bytes,
}

impl Response {
    pub fn body_string(&self) -> Result<String, std::string::FromUtf8Error> {
        String::from_utf8(self.body.to_vec())
    }
}

fn build_https_connector(
) -> hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector> {
    hyper_rustls::HttpsConnectorBuilder::new()
        .with_webpki_roots()
        .https_or_http()
        .enable_http1()
        .build()
}

/// Perform an HTTP(S) GET for `url` with a timeout, following redirects
/// (301/302/303/307/308) up to 10 hops.
pub async fn get(url: &Url, timeout: Duration) -> Result<Response, NetError> {
    let mut jar = CookieJar::new();
    get_with_jar(url, timeout, &mut jar).await
}

/// Like [`get_with_redirects`], but sends cookies from `jar` and stores any
/// `Set-Cookie` headers seen along the way (including on redirect hops).
pub async fn get_with_jar(
    url: &Url,
    timeout: Duration,
    jar: &mut CookieJar,
) -> Result<Response, NetError> {
    get_with_redirects_and_jar(url, timeout, 10, Some(jar)).await
}

/// Like [`get`], with an explicit redirect limit. Errors with
/// [`NetError::TooManyRedirects`] past the limit (mirrors Chromium's
/// net::ERR_TOO_MANY_REDIRECTS).
pub async fn get_with_redirects(
    url: &Url,
    timeout: Duration,
    max_redirects: u8,
) -> Result<Response, NetError> {
    get_with_redirects_and_jar(url, timeout, max_redirects, None).await
}

#[allow(clippy::too_many_arguments)]
async fn get_with_redirects_and_jar(
    url: &Url,
    timeout: Duration,
    max_redirects: u8,
    jar: Option<&mut CookieJar>,
) -> Result<Response, NetError> {
    get_with_redirects_jar_headers(url, timeout, max_redirects, jar, &[]).await
}

/// Core GET: follows redirects, carries a cookie jar, and applies `extra`
/// request headers to the *first* hop only (conditional validators must not be
/// replayed after a redirect to a different resource).
async fn get_with_redirects_jar_headers(
    url: &Url,
    timeout: Duration,
    max_redirects: u8,
    mut jar: Option<&mut CookieJar>,
    extra: &[(String, String)],
) -> Result<Response, NetError> {
    match url.scheme.as_str() {
        "http" | "https" => {}
        other => return Err(NetError::UnsupportedScheme(other.to_string())),
    }

    let client: Client<_, Empty<Bytes>> =
        Client::builder(TokioExecutor::new()).build(build_https_connector());

    let mut current = url.to_url_string();
    let mut hops = 0u8;

    loop {
        let mut builder = http::Request::builder()
            .method(http::Method::GET)
            .uri(&current)
            .header(
                http::header::USER_AGENT,
                concat!("Varethic/", env!("CARGO_PKG_VERSION")),
            );
        if hops == 0 {
            for (k, v) in extra {
                builder = builder.header(k.as_str(), v.as_str());
            }
        }
        if let Some(jar_ref) = jar.as_deref() {
            if let Some(cookie) = jar_ref.cookie_header(&current) {
                builder = builder.header(http::header::COOKIE, cookie);
            }
        }
        let req = builder
            .body(Empty::<Bytes>::new())
            .map_err(|e| NetError::Http(e.to_string()))?;

        let fut = client.request(req);
        let resp = tokio::time::timeout(timeout, fut)
            .await
            .map_err(|_| NetError::Timeout(timeout))?
            .map_err(|e| NetError::Http(e.to_string()))?;

        let status = resp.status().as_u16();
        if let Some(jar_ref) = jar.as_deref_mut() {
            let raw: Vec<(String, String)> = resp
                .headers()
                .iter()
                .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
                .collect();
            jar_ref.store_from_response(&current, &raw);
        }
        let is_redirect = matches!(status, 301 | 302 | 303 | 307 | 308);
        let location = resp
            .headers()
            .get(http::header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        if is_redirect {
            let loc = location.ok_or_else(|| {
                NetError::Http("redirect response without Location header".into())
            })?;
            if hops >= max_redirects {
                return Err(NetError::TooManyRedirects);
            }
            hops += 1;
            current = resolve_redirect(&current, &loc)?;
            continue;
        }

        let headers = resp
            .headers()
            .iter()
            .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let body = resp
            .into_body()
            .collect()
            .await
            .map_err(|e| NetError::Http(e.to_string()))?
            .to_bytes();

        return Ok(Response {
            status,
            headers,
            body,
        });
    }
}

/// Resolve a (possibly relative) redirect Location header against the URL
/// that produced it. Handles absolute http(s) URLs, absolute-path, and
/// relative-path locations.
fn resolve_redirect(current: &str, location: &str) -> Result<String, NetError> {
    if location.starts_with("http://") || location.starts_with("https://") {
        return Ok(location.to_string());
    }
    let scheme_end = current
        .find("://")
        .map(|i| i + 3)
        .ok_or_else(|| NetError::Http("malformed base URL".into()))?;
    let host_start = scheme_end;
    let host_end = current[host_start..]
        .find('/')
        .map(|i| host_start + i)
        .unwrap_or(current.len());
    let origin = &current[..host_end];
    if location.starts_with('/') {
        return Ok(format!("{}{}", origin, location));
    }
    let path_dir = match current.rfind('/') {
        Some(i) if i >= host_end => &current[..i],
        _ => origin,
    };
    Ok(format!("{}/{}", path_dir, location))
}

/// POST a body to `url`. Redirects are NOT followed for POST (303 would
/// require switching to GET; 301/302 POST-redirect semantics are ambiguous)
/// — the response is returned as-is for the caller to decide.
pub async fn post(
    url: &Url,
    content_type: &str,
    body: Vec<u8>,
    timeout: Duration,
) -> Result<Response, NetError> {
    match url.scheme.as_str() {
        "http" | "https" => {}
        other => return Err(NetError::UnsupportedScheme(other.to_string())),
    }

    use http_body_util::Full;
    let client: Client<_, Full<Bytes>> =
        Client::builder(TokioExecutor::new()).build(build_https_connector());

    let req = http::Request::builder()
        .method(http::Method::POST)
        .uri(url.to_url_string())
        .header(
            http::header::USER_AGENT,
            concat!("Varethic/", env!("CARGO_PKG_VERSION")),
        )
        .header(http::header::CONTENT_TYPE, content_type)
        .body(Full::new(Bytes::from(body)))
        .map_err(|e| NetError::Http(e.to_string()))?;

    let fut = client.request(req);
    let resp = tokio::time::timeout(timeout, fut)
        .await
        .map_err(|_| NetError::Timeout(timeout))?
        .map_err(|e| NetError::Http(e.to_string()))?;

    let status = resp.status().as_u16();
    let headers = resp
        .headers()
        .iter()
        .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();
    let body = resp
        .into_body()
        .collect()
        .await
        .map_err(|e| NetError::Http(e.to_string()))?
        .to_bytes();

    Ok(Response {
        status,
        headers,
        body,
    })
}

/// GET with an in-memory cache: serve from `cache` when a fresh entry exists.
/// When a stored entry is stale but carries an ETag / Last-Modified validator,
/// revalidate with a conditional request (`If-None-Match` /
/// `If-Modified-Since`); a `304 Not Modified` refreshes the entry without
/// re-downloading the body. Otherwise perform the request and store cacheable
/// responses.
pub async fn get_cached(
    url: &Url,
    timeout: Duration,
    cache: &mut HttpCache,
) -> Result<Response, NetError> {
    let key = url.to_url_string();
    if let Some(hit) = cache.get(&key) {
        return Ok(hit);
    }

    if let Some(v) = cache.stale_validators(&key) {
        let mut extra: Vec<(String, String)> = Vec::new();
        if let Some(etag) = &v.etag {
            extra.push(("if-none-match".into(), etag.clone()));
        }
        if let Some(lm) = &v.last_modified {
            extra.push(("if-modified-since".into(), lm.clone()));
        }
        let resp = get_with_redirects_jar_headers(url, timeout, 10, None, &extra).await?;
        if resp.status == 304 {
            if let Some(fresh) = cache.confirm_not_modified(&key) {
                return Ok(fresh);
            }
        } else {
            // Resource changed (or revalidation failed): drop the stale entry.
            cache.invalidate(&key);
        }
        cache.store(&key, &resp);
        return Ok(resp);
    }

    let resp = get(url, timeout).await?;
    cache.store(&key, &resp);
    Ok(resp)
}

/// Convenience: GET a raw string URL.
pub async fn get_str(url: &str, timeout: Duration) -> Result<Response, NetError> {
    let parsed = Url::parse(url)?;
    get(&parsed, timeout).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::Full;
    use hyper::service::service_fn;
    use hyper_util::rt::TokioIo;
    use std::convert::Infallible;
    use std::net::SocketAddr;
    use tokio::net::TcpListener;

    /// Spin up a local HTTP server returning a fixed body; returns its addr.
    async fn spawn_test_server() -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            loop {
                let (stream, _) = listener.accept().await.unwrap();
                let io = TokioIo::new(stream);
                tokio::spawn(async move {
                    let svc = service_fn(|req: http::Request<hyper::body::Incoming>| async move {
                        let path = req.uri().path().to_string();
                        let resp = match path.as_str() {
                            "/redirect" => http::Response::builder()
                                .status(301)
                                .header("location", "/final")
                                .body(Full::new(Bytes::new()))
                                .unwrap(),
                            "/redirect-loop" => http::Response::builder()
                                .status(302)
                                .header("location", "/redirect-loop")
                                .body(Full::new(Bytes::new()))
                                .unwrap(),
                            "/redirect-relative" => http::Response::builder()
                                .status(302)
                                .header("location", "dir/final")
                                .body(Full::new(Bytes::new()))
                                .unwrap(),
                            "/echo" => {
                                let body = req.into_body().collect().await.unwrap().to_bytes();
                                http::Response::builder()
                                    .status(200)
                                    .header("content-type", "text/plain")
                                    .body(Full::new(body))
                                    .unwrap()
                            }
                            "/set" => http::Response::builder()
                                .status(200)
                                .header("set-cookie", "session=abc123; Path=/")
                                .body(Full::new(Bytes::from_static(b"cookie-set")))
                                .unwrap(),
                            "/check" => {
                                let cookie = req
                                    .headers()
                                    .get(http::header::COOKIE)
                                    .and_then(|v| v.to_str().ok())
                                    .unwrap_or("none")
                                    .to_string();
                                http::Response::builder()
                                    .status(200)
                                    .body(Full::new(Bytes::from(format!("cookie: {}", cookie))))
                                    .unwrap()
                            }
                            _ => http::Response::builder()
                                .status(200)
                                .header("content-type", "text/plain")
                                .body(Full::new(Bytes::from_static(b"varethic-net-ok")))
                                .unwrap(),
                        };
                        Ok::<_, Infallible>(resp)
                    });
                    let _ = hyper::server::conn::http1::Builder::new()
                        .serve_connection(io, svc)
                        .await;
                });
            }
        });
        addr
    }

    #[tokio::test]
    #[ignore = "requires live internet access"]
    async fn live_https_google() {
        let resp = get_str("https://www.google.com/", Duration::from_secs(10))
            .await
            .unwrap();
        assert!(resp.status == 200 || resp.status == 301 || resp.status == 302);
    }

    #[tokio::test]
    async fn get_over_local_http() {
        let addr = spawn_test_server().await;
        let url = format!("http://{}/hello", addr);
        let resp = get_str(&url, Duration::from_secs(5)).await.unwrap();
        assert_eq!(resp.status, 200);
        assert_eq!(resp.body_string().unwrap(), "varethic-net-ok");
    }

    #[tokio::test]
    async fn follows_absolute_path_redirect() {
        let addr = spawn_test_server().await;
        let url = format!("http://{}/redirect", addr);
        let resp = get_str(&url, Duration::from_secs(5)).await.unwrap();
        assert_eq!(resp.status, 200);
        assert_eq!(resp.body_string().unwrap(), "varethic-net-ok");
    }

    #[tokio::test]
    async fn follows_relative_redirect() {
        let addr = spawn_test_server().await;
        let url = format!("http://{}/base/redirect-relative", addr);
        // /base/redirect-relative -> 302 Location: dir/final -> /base/dir/final (served 200 by catch-all)
        let resp = get_str(&url, Duration::from_secs(5)).await.unwrap();
        assert_eq!(resp.status, 200);
    }

    #[tokio::test]
    async fn too_many_redirects_errors() {
        let addr = spawn_test_server().await;
        let url = format!("http://{}/redirect-loop", addr);
        let err = get_str(&url, Duration::from_secs(5)).await.unwrap_err();
        assert!(matches!(err, NetError::TooManyRedirects));
    }

    #[tokio::test]
    async fn cookie_jar_roundtrip_over_http() {
        let addr = spawn_test_server().await;
        let mut jar = CookieJar::new();
        let set_url = Url::parse(&format!("http://{}/set", addr)).unwrap();
        let resp = get_with_jar(&set_url, Duration::from_secs(5), &mut jar)
            .await
            .unwrap();
        assert_eq!(resp.body_string().unwrap(), "cookie-set");
        assert_eq!(jar.len(), 1);

        let check_url = Url::parse(&format!("http://{}/check", addr)).unwrap();
        let resp = get_with_jar(&check_url, Duration::from_secs(5), &mut jar)
            .await
            .unwrap();
        assert_eq!(resp.body_string().unwrap(), "cookie: session=abc123");
    }

    async fn spawn_counting_server() -> (SocketAddr, std::sync::Arc<std::sync::atomic::AtomicUsize>)
    {
        use std::sync::atomic::Ordering;
        let counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let ctr = counter.clone();
        tokio::spawn(async move {
            loop {
                let (stream, _) = listener.accept().await.unwrap();
                let io = TokioIo::new(stream);
                let ctr = ctr.clone();
                tokio::spawn(async move {
                    let svc = service_fn(move |_req| {
                        let ctr = ctr.clone();
                        async move {
                            ctr.fetch_add(1, Ordering::SeqCst);
                            Ok::<_, Infallible>(
                                http::Response::builder()
                                    .status(200)
                                    .header("cache-control", "max-age=60")
                                    .body(Full::new(Bytes::from_static(b"cached-response-body")))
                                    .unwrap(),
                            )
                        }
                    });
                    let _ = hyper::server::conn::http1::Builder::new()
                        .serve_connection(io, svc)
                        .await;
                });
            }
        });
        (addr, counter)
    }

    #[tokio::test]
    async fn get_cached_serves_second_request_from_cache() {
        use std::sync::atomic::Ordering;
        let (addr, counter) = spawn_counting_server().await;
        let url = Url::parse(&format!("http://{}/cached", addr)).unwrap();
        let mut cache = HttpCache::new(8);
        let r1 = get_cached(&url, Duration::from_secs(5), &mut cache)
            .await
            .unwrap();
        let r2 = get_cached(&url, Duration::from_secs(5), &mut cache)
            .await
            .unwrap();
        assert_eq!(r1.body_string().unwrap(), "cached-response-body");
        assert_eq!(r2.body_string().unwrap(), "cached-response-body");
        assert_eq!(
            counter.load(Ordering::SeqCst),
            1,
            "second response must come from cache"
        );
        assert_eq!(cache.hits(), 1);
    }

    #[tokio::test]
    async fn post_echoes_body() {
        let addr = spawn_test_server().await;
        let url = Url::parse(&format!("http://{}/echo", addr)).unwrap();
        let resp = post(
            &url,
            "text/plain",
            b"hello-varethic".to_vec(),
            Duration::from_secs(5),
        )
        .await
        .unwrap();
        assert_eq!(resp.status, 200);
        assert_eq!(resp.body_string().unwrap(), "hello-varethic");
    }

    #[tokio::test]
    async fn rejects_unsupported_scheme_before_network() {
        let err = get_str("ftp://example.com/", Duration::from_secs(1))
            .await
            .unwrap_err();
        assert!(matches!(err, NetError::UnsupportedScheme(_)));
    }

    #[tokio::test]
    async fn timeout_on_unroutable() {
        // 10.255.255.1 is non-routable on typical networks; connect should not
        // complete within the (short) timeout.
        let err = get_str("http://10.255.255.1/", Duration::from_millis(500))
            .await
            .unwrap_err();
        assert!(matches!(err, NetError::Timeout(_) | NetError::Http(_)));
    }

    /// Server that emits an ETag and answers `If-None-Match` with 304.
    async fn spawn_revalidating_server(
    ) -> (SocketAddr, std::sync::Arc<std::sync::atomic::AtomicUsize>) {
        use std::sync::atomic::Ordering;
        let counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let ctr = counter.clone();
        tokio::spawn(async move {
            loop {
                let (stream, _) = listener.accept().await.unwrap();
                let io = TokioIo::new(stream);
                let ctr = ctr.clone();
                tokio::spawn(async move {
                    let svc = service_fn(move |req: http::Request<hyper::body::Incoming>| {
                        let ctr = ctr.clone();
                        async move {
                            ctr.fetch_add(1, Ordering::SeqCst);
                            let inm = req
                                .headers()
                                .get(http::header::IF_NONE_MATCH)
                                .and_then(|v| v.to_str().ok())
                                .unwrap_or("")
                                .to_string();
                            let resp = if inm == "\"v1\"" {
                                http::Response::builder()
                                    .status(304)
                                    .header("etag", "\"v1\"")
                                    .body(Full::new(Bytes::new()))
                                    .unwrap()
                            } else {
                                http::Response::builder()
                                    .status(200)
                                    .header("etag", "\"v1\"")
                                    .header("cache-control", "max-age=60")
                                    .body(Full::new(Bytes::from_static(b"revalidated-body")))
                                    .unwrap()
                            };
                            Ok::<_, Infallible>(resp)
                        }
                    });
                    let _ = hyper::server::conn::http1::Builder::new()
                        .serve_connection(io, svc)
                        .await;
                });
            }
        });
        (addr, counter)
    }

    #[tokio::test]
    async fn stale_entry_revalidated_with_304() {
        use std::sync::atomic::Ordering;
        let (addr, counter) = spawn_revalidating_server().await;
        let url = Url::parse(&format!("http://{}/revalidate", addr)).unwrap();
        let key = url.to_url_string();
        let mut cache = HttpCache::new(8);

        let r1 = get_cached(&url, Duration::from_secs(5), &mut cache)
            .await
            .unwrap();
        assert_eq!(r1.body_string().unwrap(), "revalidated-body");
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        // Force the entry stale; a conditional request must yield 304 and the
        // original body must still be served, without a second full download.
        cache.expire(&key);
        let r2 = get_cached(&url, Duration::from_secs(5), &mut cache)
            .await
            .unwrap();
        assert_eq!(r2.status, 200);
        assert_eq!(r2.body_string().unwrap(), "revalidated-body");
        assert_eq!(
            counter.load(Ordering::SeqCst),
            2,
            "exactly one revalidation"
        );
        assert_eq!(cache.hits(), 1, "304 confirmed from cache");
    }
}

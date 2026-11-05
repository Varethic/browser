//! varethic-net: a safe-Rust network stack subset, the first step in
//! replacing Chromium `//net` for the Varethic browser.
//!
//! Scope for v0.1: async HTTP/1.1 GET over HTTP and HTTPS (rustls,
//! webpki-roots). Redirect following, cookies, caching, proxies, HTTP/2,
//! and connection pooling beyond hyper's defaults are NOT yet implemented.
//! Do not claim //net parity.

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

/// Perform an HTTP(S) GET for `url` with a timeout.
pub async fn get(url: &Url, timeout: Duration) -> Result<Response, NetError> {
    match url.scheme.as_str() {
        "http" | "https" => {}
        other => return Err(NetError::UnsupportedScheme(other.to_string())),
    }

    let client: Client<_, Empty<Bytes>> =
        Client::builder(TokioExecutor::new()).build(build_https_connector());

    let target = url.to_url_string();
    let req = http::Request::builder()
        .method(http::Method::GET)
        .uri(&target)
        .header(
            http::header::USER_AGENT,
            concat!("Varethic/", env!("CARGO_PKG_VERSION")),
        )
        .body(Empty::<Bytes>::new())
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
                    let svc = service_fn(|_req| async {
                        Ok::<_, Infallible>(
                            http::Response::builder()
                                .status(200)
                                .header("content-type", "text/plain")
                                .body(Full::new(Bytes::from_static(b"varethic-net-ok")))
                                .unwrap(),
                        )
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
}

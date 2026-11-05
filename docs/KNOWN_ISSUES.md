# Known Issues

- **Not production-ready.** v0.1.0 is an early milestone. Do not ship.
- **Rendering is WebView2, not a native Rust engine.** All web platform
  behavior passes through Chromium inside WebView2. Engine migration is at
  the first component (`//url`, partial).
- **Full Chromium source not vendored or buildable here.** Disk available
  during setup was ~28 GB; a Chromium checkout + build needs roughly 100 GB.
  Engine work proceeds via reference reading (docs.rs, upstream sources) and
  incremental Rust crates instead.
- **varethic-url is not WHATWG-complete.** No IDN/punycode, no percent-decoding
  normalization, no trailing-dot domain rules beyond basics.
- **varethic-net is a GET-only subset.** No redirects, cookies, cache, HTTP/2,
  POST bodies, or connection pooling controls; shell navigation still uses
  WebView2, not varethic-net.
- **Per-tab state limitations.** Tabs each own a webview (good), but tab
  thumbnails, session restore, and hibernation are not implemented.
- **Toolbar is HTML in a webview** — it inherits WebView2 quirks; keyboard
  focus between toolbar and content needs care.
- **Only Windows/WebView2 path exists.** No macOS/Linux backends yet.
- **No automated UI tests**; smoke testing is manual (see docs/TESTING.md).
- **No code signing, installer, or auto-update.**
- **Security caveats:** default WebView2 permissions; camera/mic/geolocation
  not yet gated by a Varethic permission model; no sandbox work beyond what
  WebView2 provides.

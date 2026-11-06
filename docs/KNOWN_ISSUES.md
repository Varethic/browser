# Known Issues

Last reviewed: v0.20.0 (2026-11-06).

- **Not production-ready.** Do not ship. This is an incremental migration
  project, not a finished browser.
- **Rendering is WebView2, not a native Rust engine.** All HTML/CSS/DOM/JS
  execution goes through Chromium inside WebView2 (the Microsoft Edge runtime). No
  Rust implementation of Blink, DOM, CSS, V8, GPU or the sandbox exists yet.
- **Nothing is marked `Migrated`.** `varethic_engine_core::COMPONENT_REGISTRY`
  intentionally reports `InProgress`/`NotStarted` for every component until a
  Rust replacement reaches behavioural parity and is integrated.
- **Chromium is a reference checkout only.** `C:\Users\admin\Desktop\Varethic\chromium`
  is shallow and has never been built (no depot_tools/GN/Ninja build was run).
- **varethic-url is not WHATWG-complete.** It has IDN (UTS #46 via the `idna`
  crate), WHATWG tuple/opaque origins, and absolute-path dot-segment
  normalization, plus a differential corpus against Chromium's
  `url_unittest.cc` / `url_canon_unittest.cc`. Still missing: tolerance for
  missing `//` after a scheme, percent-encoding of non-ASCII in paths/queries,
  Windows drive-relative `file:` paths, full public-suffix handling.
- **Cookie `site_for_cookies` is a two-label approximation.** No bundled
  Public Suffix List, so `SameSite` decisions on multi-label suffixes
  (e.g. `.co.uk`) can be wrong. Cookies are also memory-only (no persistence).
- **varethic-net is HTTP/1.1 only.** No HTTP/2, no proxy support, no
  connection pooling beyond hyper defaults, no disk cache backend in the
  network path (the disk cache exists in `varethic-storage` but
  `get_cached` uses the in-memory cache), no `Vary` handling.
- **Disk HTTP cache is not wired into the shell.** `varethic-storage` exposes
  `cache_put`/`cache_get`, but the browser does not use it yet.
- **Per-tab state limitations.** Tab thumbnails, session restore, and tab
  hibernation are not implemented. Workspaces store tab URL sets but do not
  create isolated storage contexts.
- **varethic-storage settings are plaintext key/value** — fine for non-secrets
  (search template), do NOT store credentials there.
- **Incognito is per-tab via WebView2 incognito mode.** No separate window;
  storage/cache partitioning beyond WebView2 is unverified.
- **Toolbar is HTML in a webview** — it inherits WebView2 quirks; keyboard
  focus between toolbar and content needs care.
- **Only the Windows/WebView2 path exists.** No macOS/Linux backends.
- **UI validation is smoke-level only.** `tests/ui_smoke.ps1` (window appears,
  correct title, clean close) passes. Interaction-level automation does not
  yet work reliably: UIA `ValuePattern.SetValue` does not fire the omnibox's
  `keydown`, and synthetic keystrokes are not delivered in a non-interactive
  session. See `tests/experimental/README.md`.
- **No WPT, no differential HTML/CSS/DOM/JS test suite, no performance tests.**
- **No code signing, installer, auto-update, or crash reporting.**
- **Security caveats.** Default WebView2 permissions; camera/mic/geolocation
  are not gated by a Varethic permission model; no sandbox work beyond what
  WebView2 provides.
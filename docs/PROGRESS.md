# Progress Log

## 2026-11-05 — v0.6.0

Done:
- varethic-net: in-memory CookieJar (Domain/Path/Secure/HttpOnly/Max-Age/
  Expires, host-only vs domain match, path scoping, secure enforcement,
  RFC 1123 Expires), get_with_jar wires cookies into requests and stores
  Set-Cookie across redirect hops. 15 net tests pass (8 cookie unit + 1
  HTTP roundtrip + 6 prior).
- Pushed v0.5.0; v0.6.0 committed and pushed.

Verification:
- `cargo test -p varethic-net`: 15 passed, 1 ignored (live).
- Cookie roundtrip test: /set then /check through jar returns
  `cookie: session=abc123`.

## 2026-11-05 — v0.5.0

Done:
- Settings panel (\u2699): default search URL template editable, validated
  (http(s), exactly one {searchTerms}), persisted in `settings` table;
  Google remains the default; invalid values fall back to Google.
- Private browsing (\u{1F576} button): incognito WebView2 tab; history not
  recorded for incognito tabs; tab title prefixed with mask glyph.
- varethic-url: search-template validation + classify_omnibox_input_with_search
  (12 tests). varethic-storage: settings table (5 tests).
- Launch-tested; window alive. `cargo test --workspace`: 24 passed.

Not done / next:
- Profiles (multiple named profiles), vertical tabs/workspaces/split view,
  cookie jar in varethic-net, WHATWG ICU/IDN work, WPT/UI automation,
  signing/installer.

## 2026-11-05 — v0.4.0

Done:
- varethic-net: redirect following (301/302/303/307/308, max 10 hops,
  ERR_TOO_MANY_REDIRECTS analogue), absolute/relative Location resolution.
  6 local tests pass.
- Shell: history/bookmarks/downloads side panels (child webview) opened from
  new toolbar buttons; rows come from varethic-storage; clicking a
  history/bookmark navigates the active tab; panel closes on \u00D7 or goto;
  resize keeps panel bounds. Launch-tested, window alive.
- Pushed v0.1.0–v0.3.0 commits to origin/main (user authorized), committed
  v0.4.0.

Verification:
- `cargo test --workspace`: all pass (url 10, net 6, storage 4, registry 1).
- Launch test: window alive after 10 s.

## 2026-11-05 — v0.3.0

Done:
- New crate `varethic-storage` (rusqlite, bundled): history record/search/
  recent/clear with visit counting; bookmarks add/list/remove; downloads
  record/list. 4 unit tests incl. reopen persistence. Integrated into the
  shell: auto history on page load, "☆" bookmark button, WebView2 download
  started/completed handlers save to %USERPROFILE%\Downloads and record to
  the db (`%LOCALAPPDATA%\Varethic\varethic.db`). Launch-tested; db file
  created. Workspace 0.3.0. //storage + //sql tracked InProgress.

Not done / next:
- Download manager UI, history/bookmarks UI, search-in-history; profiles and
  private browsing; vertical tabs/workspaces; varethic-net redirects.

Verification:
- `cargo test --workspace`: 18 passed (10 url + 3 net + 4 storage + 1 registry).
- Launch test: window alive; varethic.db created.

## 2026-11-05 — v0.2.0

Done:
- New crate `varethic-net`: hyper + rustls HTTP/1.1 GET (HTTP and HTTPS via
  webpki-roots), timeout support, typed errors. Tests: local hyper server
  returns `varethic-net-ok` (pass); unsupported scheme rejected pre-network;
  timeout on unroutable host; ignored live test against
  https://www.google.com/ run manually — passed (status 200/30x).
- `varethic-url`: bracketed IPv6 parsing, HostKind (Ipv4/Ipv6/Localhost/
  Domain), is_ipv4. 10 unit tests pass.
- Registry/docs updated; //net now InProgress. Workspace version 0.2.0.

Not done / next:
- Redirects, cookies, download manager on varethic-net; history/bookmarks
  storage (rusqlite); shell integration of varethic-net for downloads.

Verification:
- `cargo test --workspace`: 14 passed (10 url + 3 net + 1 registry).
- `cargo test -p varethic-net -- --ignored`: live Google HTTPS passed.

## 2026-11-05 — v0.1.0

Environment: Windows 11, Rust 1.99.1 (stable-x86_64-pc-windows-msvc), VS2022
Community + Windows SDK 10.0.26100, ~28 GB free disk (full Chromium checkout
infeasible), WebView2 available.

Done:
- Cloned github.com/Varethic/browser (empty) → main branch.
- Git identity set to Varethic <application@varethic.in.th>.
- Cargo workspace created: varethic_url, varethic_engine_core, varethic_browser.
- varethic_url v0.1.0: parse/validate/canonicalize + omnibox classifier; 9 tests pass.
- varethic_engine_core: COMPONENT_REGISTRY + migration_summary; 1 test passes.
- varethic_browser v0.1.0: tao window + wry/WebView2 child views; tab strip and
  omnibox HTML toolbar with JSON IPC → typed UserEvent loop; per-tab webviews;
  navigate/new/close/switch/back/forward/reload/home/devtools; Google default
  search; URL-vs-search classification via varethic_url.
- Debug build OK; launch-tested: window renders and stays alive.
- Docs suite, VERSION=0.1.0, CHANGELOG, README.

Not done / next:
- WHATWG parity for varethic-url; varethic-net Phase 2; UI automation tests;
  signing/installer. See docs/ROADMAP.md.

Verification evidence:
- `cargo test --workspace`: 10 tests passed (see terminal output in session).
- `cargo build -p varethic-browser`: clean (0 warnings after fix).
- Launch test: process alive after 10 s; killed manually.

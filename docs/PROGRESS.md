# Progress Log

## 2026-11-06 — v0.21.0

Done:
- Integrated the migrated network into the shell: "Varethic Fetch" panel (⚡)
  fetches via varethic-net, memory -> disk cache -> network (revalidating),
  persists to varethic-storage's SQLite cache.
- varethic-net: HttpCache::seed (+1 test -> 32 net tests).
- varethic-browser: 2 end-to-end tests with a dependency-free HTTP/1.1 server
  (network -> memory -> disk layering, persistence, invalid URL).
- 62 workspace tests pass; ui_smoke.ps1 passes with the new panel.
- First-lookup order bug found by the E2E test (disk checked before memory);
  fixed to memory -> disk -> network.

## 2026-11-06 — v0.20.0

Done:
- varethic-net: cache revalidation via conditional requests (ETag/Last-Modified
  -> If-None-Match/If-Modified-Since, 304 handling). 31 net tests pass
  (was 27); workspace 59 tests.
- Fixed HttpCache::get deleting stale entries (blocked revalidation).
- docs/KNOWN_ISSUES.md rewritten to match v0.20.0 reality (was stale v0.1.0
  text claiming no IDN / SameSite-blind).
- tests/experimental/: parked UI-automation harnesses + honest README. Learned
  UIA exposes toolbar buttons by AutomationId, but omnibox navigation can't be
  driven (SetValue doesn't fire keydown; synthetic keys undelivered here).

## 2026-11-06 — v0.19.0

Done:
- varethic-storage: disk-backed HTTP cache (put/get/clear, max-age check +
  eviction), 1 test; 8 storage tests pass; 55 workspace tests.
- Compiler caught two layout mistakes (struct in impl block, mangled
  derive); both fixed and re-verified.

## 2026-11-06 — v0.18.0

Done:
- varethic-url: 12 more URLCanonHostTest.Host vectors (RTL/Bidi + large IDNA
  set). All pass. 54 workspace tests. Caught and fixed two of my own wrong
  Unicode literals during testing (documented in CHANGELOG).

## 2026-11-06 — v0.17.0

Done:
- tests/ui_smoke.ps1: scripted UI smoke harness (window appears titled
  'Varethic', live handle, WM_CLOSE clean exit). Verified this session:
  PASS with hwnd 1312800, exit code 0. Docs/TESTING.md updated.

## 2026-11-06 — v0.16.0

Done:
- varethic-url: differential host vectors from Chromium //url's
  URLCanonHostTest.Host (7 cases: case folding, fullwidth nameprep, U+3002 dot,
  IDN, ß, final sigma, zero-width strip). All pass. 53 workspace tests.

## 2026-11-06 — v0.15.0

Done:
- varethic-url: absolute-path dot-segment normalization (., .., %2e variants);
  differential test vectors hand-ported from Chromium //url's url_unittest.cc
  with a documented divergence list. 17 url tests; 52 workspace tests pass.

## 2026-11-06 — v0.14.0

Done:
- varethic-net: SameSite=Strict/Lax enforcement (two-label site
  approximation; cross-site blocked) and SameSite=None⇒Secure rejection at
  store time. 27 net tests pass; 50 workspace tests pass.
- CLAUDE.md added (agentic working notes; documents the no-fake-progress and
  push rules).
- Cookie tests caught two wrong host-only/Domain expectations during
  development; corrected tests match RFC 6265 host-only semantics.

## 2026-11-06 — v0.13.0

Done:
- Workspaces: storage schema + CRUD with roundtrip test; shell panel (\u29CA) to
  save/switch/delete named tab sets. Launch-tested. 47 workspace tests pass.
- Note: a stale cargo incremental cache reported phantom E0599 against
  varethic-storage methods; `cargo clean -p varethic-browser -p varethic-storage`
  resolved it.

## 2026-11-06 — v0.12.0

Done:
- varethic-net: HttpCache (Cache-Control aware, bounded in-memory) +
  get_cached; counting-server test proves second request served from cache.
  24 net tests pass; 46 workspace tests pass.

## 2026-11-06 — v0.11.0

Done:
- Split view: \u25A2 toggle splits content into two panes; right pane webview
  loads configurable split_url (settings-backed, default Google); \u25A6 panel
  edits it. Works alongside vertical tabs; resize keeps both panes bounded.
  Launch-tested. 38 workspace tests pass.

## 2026-11-06 — v0.10.0

Done:
- varethic-url: WHATWG Origin (tuple for http/https/ws/wss/ftp w/ default
  ports, opaque otherwise; serialize() + is_same_origin()); file: URLs parse
  with empty host. 15 url tests pass; 38 workspace tests pass.
- Chromium //v8 submodule fetched (f6b7d954); blink in-tree. Study notes in
  CHROMIUM_REFERENCE.md. //url Origin model implemented to match special
  scheme rules.

## 2026-11-06 — v0.9.0

Done:
- Vertical tabs mode: \u{2630} toggles 48px nav-only toolbar + 220px sidebar
  child webview with clickable tab list; content shifts right; bounds update
  on resize and toggle. Launch-tested.
- Chromium //url studied: gurl.rs/origin.rs/url.rs are cxx FFI wrappers over
  C++ GURL/Origin (not a native Rust parser). //v8 is a git submodule (not
  fetched by shallow clone). Findings recorded in ENGINE_MIGRATION.md and
  CHROMIUM_REFERENCE.md.
- 35 url/net/storage/registry tests pass.

## 2026-11-06 — v0.8.0

Done:
- Chromium reference checkout obtained (shallow, HEAD 4a5fcfa6ce) at
  Varethic\chromium; see docs/CHROMIUM_REFERENCE.md. NOT built, NOT vendored.
- varethic-net: post() with echo integration test. varethic-url: IDN/punycode
  host canonicalization + trailing-dot strip (13 tests). Workspace: 36 tests.
- README.md restored from git history (UTF-8) after accidental replacement.

Verification:
- `cargo test --workspace`: 36 passed, 0 failed, 1 ignored (live).
- Launch test (v0.7.0 binary) previously verified; see TESTING.md.

## 2026-11-05 — v0.7.0

Done:
- Named profiles: registry (profiles.db), per-profile dbs under
  %LOCALAPPDATA%\Varethic\profiles\<name>\varethic.db; legacy db migrated into
  Default; Profiles panel (\u{1F464}) to create/switch; switching swaps live
  storage backend. 34 workspace tests pass; launch-tested.

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

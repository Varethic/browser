# Changelog

All notable changes to this project are documented here. Version format:
`MAJOR.MINOR.PATCH`.

## [0.14.0] - 2026-11-06

### Added
- `varethic-net`: SameSite cookie enforcement — `SameSite=Strict/Lax` cookies
  (browsers default Lax) are not sent on cross-site requests (two-label site
  approximation, no PSL); `SameSite=None` requires `Secure` and is rejected at
  store time otherwise. 3 new tests (27 net tests pass).
- `CLAUDE.md`: agentic working notes — layout, build/test, docs discipline,
  push policy, no-fake-progress rule.

## [0.13.0] - 2026-11-06

### Added
- Workspaces: `varethic-storage` gains a `workspaces` + `workspace_tabs`
  schema and `save_workspace` / `list_workspaces` / `load_workspace` /
  `delete_workspace` (roundtrip test, 7 storage tests). Shell gains a \u29CA
  toolbar button and panel: save the current (non-incognito) tab set under a
  name (replaces on re-save), open a workspace into tabs (first tab activated),
  delete (panel re-renders). Launch-tested.
- Refactored panel IPC handling into a shared `recreate_panel` helper.

## [0.12.0] - 2026-11-06

### Added
- `varethic-net`: in-memory HTTP response cache (`HttpCache`) honoring
  `Cache-Control` `no-store` / `no-cache` / `must-revalidate` / `private` /
  `max-age`; arbitrary bounded eviction; hit/miss counters. `get_cached()`
  performs the request on miss, stores fresh 200s, and serves hits without
  network. 7 cache unit tests + 1 integration test proving the second request
  bypasses the server (hit counter stays 1). Not a disk cache; no Vary/ETag yet.

## [0.11.0] - 2026-11-06

### Added
- Split view: \u25A2 toolbar button toggles a two-pane layout — content area
  splits in half, right pane is a child webview loading `split_url` (sent-setting
  persisted as `split_url`, defaults to Google); the split-pane URL is editable
  via the \u25A6 panel (`set-split` IPC, URL validation). Bounds update on toggle,
  resize, and in combination with vertical tabs. Launch-tested.

## [0.10.0] - 2026-11-06

### Added
- `varethic-url`: WHATWG `Origin` — tuple origins for http/https/ws/wss/ftp
  (default ports 80/443/21 filled in), opaque otherwise; `serialize()` and
  `is_same_origin()` (opaque origins never same-origin). `file:` URLs now
  parse with empty host per the file-URL model. 15 tests.
- Chromium //v8 submodule fetched (`f6b7d954`) into the reference checkout;
  `third_party/blink` is in-tree. Study notes updated.

## [0.9.0] - 2026-11-06

### Added
- Vertical tabs layout: \u{2630} toolbar button toggles vertical mode — toolbar
  shrinks to nav-only height, the horizontal tab strip hides, a 220px sidebar
  child webview lists tabs (click to switch, \u{00D7} to close), and content
  webviews shift right. Layout state is restored on resize. Launch-tested.
- Chromium study findings written into docs (//url contains Rust FFI shims
  `url.rs`/`gurl.rs`/`origin.rs` wrapping C++ via `cxx`; v8 is a git submodule
  not fetched by the shallow clone).

### Changed
- Workspace version 0.9.0.

## [0.8.0] - 2026-11-06

### Added
- `varethic-net`: `post()` — POST with Content-Type and body; redirects are
  intentionally NOT followed (documented). Integration-tested against a local
  echo server (16 net tests pass).
- `varethic-url`: IDN/punycode host canonicalization via `idna`
  (`xn--bcher-kva.de` for `bücher.de`), trailing-dot stripping, `InvalidHost`
  error. 13 url tests pass.
- `docs/CHROMIUM_REFERENCE.md`: documents the local shallow Chromium reference
  checkout at `Varethic\chromium` (HEAD 4a5fcfa6ce, not vendored, not built).

### Changed
- Workspace version 0.8.0. README.md restored from history after the
  accidental "bro" replacement (user request; original preserved in git).

## [0.7.0] - 2026-11-05

### Added
- Named profiles: profile registry db (`profiles.db`) with `Default` auto-created;
  each profile gets its own browser-data dir (`%LOCALAPPDATA%\\Varethic\\profiles\\<name>\\varethic.db`);
  legacy pre-profiles `varethic.db` is migrated into the Default profile on first run.
- Profiles panel (\u{1F464} button): create + switch profiles; name validation
  (alnum/-/_, max 32); switching swaps the live storage backend.
  NOTE: switching does not migrate WebView2's in-memory state or per-profile tab sessions.
- `varethic-storage`: `profiles` table + `create_profile`/`list_profiles`/`delete_profile`
  (+1 test). Workspace: 34 tests pass; launch-tested.

## [0.6.0] - 2026-11-05

### Added
- `varethic-net`: in-memory `CookieJar` (`cookies` module) — parses and stores
  `Set-Cookie` (Domain/Path/Secure/HttpOnly/Max-Age/Expires), host-only vs
  domain matching, path scoping, secure-over-HTTPS enforcement, expiry
  deletion, RFC 1123 `Expires` parsing. `get_with_jar` sends the `Cookie`
  header on requests (including redirect hops) and stores cookies seen along
  the way. 8 cookie unit tests + 1 HTTP roundtrip integration test.
  Not a security boundary yet (no SameSite/partitioning; memory-only).

## [0.5.0] - 2026-11-05

### Added
- Settings panel (\u2699): configurable default search URL template persisted in
  the `settings` table (`%LOCALAPPDATA%\Varethic\varethic.db`); validation via
  `varethic_url::is_valid_search_template` (http(s) + exactly one
  `{searchTerms}`); invalid templates fall back to Google at query time.
- Private browsing: \u{1F576} button opens an incognito tab (WebView2 incognito
  mode); incognito tabs show a mask glyph in the tab strip and their page
  loads are NOT written to history. Bookmarks/downloads still work.
- `varethic-storage`: `settings` key/value table + `get_setting`/`set_setting`.
- `varethic-url`: `is_valid_search_template`,
  `classify_omnibox_input_with_search` (Google default preserved). 12 unit tests.
- Storage roundtrip test for settings. Workspace total: 24 tests pass.

## [0.4.0] - 2026-11-05

### Added
- `varethic-net`: redirect following (301/302/303/307/308, up to 10 hops,
  `NetError::TooManyRedirects`), absolute-path and relative `Location`
  resolution. New tests: absolute-path redirect, relative redirect,
  redirect-loop cutoff. 6 local tests pass (1 live test remains `#[ignore]`d).
- Shell: history/bookmarks/downloads side panels opened from the toolbar
  (\u{1F550} / \u{2605} / \u{2B07}). Panel is a child webview rendering rows from
  `varethic-storage`; click a history/bookmark row to navigate the active
  tab; downloads panel lists recent completed downloads with success state.
  Panels close via \u00D7 or on navigation.

### Changed
- Workspace version 0.4.0.

## [0.3.0] - 2026-11-05

### Added
- `varethic-storage`: SQLite (bundled rusqlite) browser-data store —
  history (record/search/recent/clear with visit counts), bookmarks
  (add/list/remove), downloads (record/list). 4 unit tests incl. on-disk
  persistence across opens.
- Shell: history is recorded on every non-`about:` page load into
  `%LOCALAPPDATA%\Varethic\varethic.db`; toolbar "☆" button bookmarks the
  current tab; downloads started via WebView2 are saved into the user's
  Downloads folder and recorded in the `downloads` table on completion.
- Launch-verified: window renders; db file is created on first run.

### Changed
- Workspace version 0.3.0.

## [0.2.0] - 2026-11-05

### Added
- `varethic-net`: safe-Rust network stack subset (successor path to Chromium
  `//net`) — async HTTP/1.1 GET over HTTP and HTTPS via hyper + rustls
  (webpki-roots), timeouts, `USER_AGENT: Varethic/<version>`. Tests: local
  hyper server (pass), unsupported-scheme rejection, timeout handling, plus a
  `#[ignore]`d live-HTTPS test that was run once and passed against
  `https://www.google.com/`.
- `varethic-url`: bracketed IPv6 literal parsing (`http://[::1]:8080/`),
  `HostKind` classification (IPv4 / IPv6 / localhost / domain), `is_ipv4`
  validator. 10 unit tests.

### Changed
- Migration registry: `//net` now tracked as `InProgress` with
  `varethic-net`. Nothing is marked `Migrated` yet.
- Workspace version 0.2.0.

## [0.1.0] - 2026-11-05

### Added
- Cargo workspace with crates: `varethic-url`, `varethic-engine-core`,
  `varethic-browser`.
- `varethic-url`: Rust successor to Chromium `//url` core — absolute-URL
  parsing and validation, canonicalization (scheme/host lower-case, empty
  path → `/`), RFC 3986 percent-encoding, Google-default omnibox classifier
  (URL vs search). 9 unit tests.
- `varethic-engine-core`: `COMPONENT_REGISTRY` machine-readable migration
  tracker + summary; consistency test.
- `varethic-browser`: Windows desktop shell (tao + wry/WebView2). Tab strip,
  address bar, back/forward/reload/home, DevTools, per-tab webviews with
  preserved history, Google as default search.
- Docs: ARCHITECTURE, ROADMAP, ENGINE_MIGRATION, COMPONENT_INVENTORY,
  TESTING, KNOWN_ISSUES, PROGRESS. README, VERSION, CHANGELOG.

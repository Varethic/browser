# Changelog

All notable changes to this project are documented here. Version format:
`MAJOR.MINOR.PATCH`.

## [0.22.0] - 2026-11-06

### Added
- **`varethic-dom`: the first native (non-WebView2) engine component.** A
  safe-Rust HTML5 tokenizer + simplified tree constructor, the early successor
  to Blink's HTML parser:
  - tokenizer: data, tag open/name, attributes (double/single/unquoted/boolean),
    self-closing, comments, doctype, bogus comments, RAWTEXT/RCDATA
    (`script`/`style`/`title`/`textarea`/...), character references
    (numeric + ~40 named).
  - tree construction: DOM arena, void elements, implied end tags for
    `li`/`p`/`dt`/`dd`/`option`, block elements closing open `p`, text merging,
    comment/doctype nodes; `to_html()` normalizing serializer plus
    `element_names`/`find`/`text_content` queries.
  - 27 unit tests including a `divergences_from_full_html5` test that pins the
    known gaps (no `html`/`head`/`body` synthesis, no adoption agency).
- `varethic_browser`: the Fetch panel now parses fetched HTML with
  `varethic-dom` and displays a tree summary (element count / node count /
  first tags), so the native parser runs on real bytes inside the product.
  +1 end-to-end test asserting the summary appears.
- `COMPONENT_REGISTRY`: new `//third_party/blink/renderer/core/html/parser`
  entry, `InProgress`, `varethic-dom`.

### Notes
- `varethic-dom` is **not used for rendering**; WebView2 still renders all
  pages. It is exercised for parsing/summary only. Nothing is marked
  `Migrated`.

## [0.21.0] - 2026-11-06

### Added
- **The migrated Rust network stack now serves the browser.** New "Varethic
  Fetch" panel (toolbar `⚡`) fetches a URL or search through `varethic-net`
  instead of WebView2, and displays the status, content type and body preview.
  Lookup order: in-memory cache → persistent disk cache → network (with
  ETag/Last-Modified revalidation). Responses are persisted to
  `varethic-storage`'s SQLite disk cache keyed by URL.
- `varethic-net`: `HttpCache::seed`, to prime the memory cache from a disk
  entry; 1 new test.
- `varethic-browser`: 2 end-to-end tests driving a dependency-free local
  HTTP/1.1 server — verifies network → memory cache → disk cache layering and
  that responses are actually persisted, plus invalid-URL handling.

### Changed
- `varethic-browser` now depends on `varethic-net` and `tokio`
  (`rt-multi-thread`); fetches run on a worker thread and marshal results back
  through the tao event loop proxy.

## [0.20.0] - 2026-11-06

### Added
- `varethic-net`: HTTP cache revalidation. Stale entries carrying an `ETag` or
  `Last-Modified` validator are revalidated with a conditional request
  (`If-None-Match` / `If-Modified-Since`); a `304 Not Modified` refreshes the
  entry and serves the stored body without a second download. New cache API:
  `Validators`, `stale_validators`, `confirm_not_modified`, `invalidate`,
  `expire`; conditional headers are applied to the first hop only. 4 cache
  tests + 1 end-to-end test with a 304 server (31 net tests total).
- `tests/experimental/`: parked, clearly-labelled UI-automation harnesses plus
  a README recording what was learned (UIA exposes toolbar buttons by
  AutomationId; `SetValue` does not fire `keydown`, so omnibox navigation is
  not yet automatable).

### Changed
- `docs/KNOWN_ISSUES.md` rewritten to match reality at v0.20.0 (was stale:
  claimed v0.1.0, no IDN, SameSite-blind). Now records the WebView2 dependency,
  nothing-Migrated status, unbuilt Chromium, HTTP/1.1-only net, unwired disk
  cache, and smoke-only UI validation.

### Fixed
- `HttpCache::get` no longer deletes a stale entry on lookup, so it remains
  available for conditional revalidation.

## [0.19.0] - 2026-11-06

### Added
- `varethic-storage`: disk-backed HTTP cache (`http_cache` table,
  `cache_put` / `cache_get` / `cache_clear`) honoring max-age expiry and
  bounded eviction; JSON-encoded headers; 1 integration test. 8 storage tests.
- Struct/derive layout fixes caught by the compiler this round (moved
  `CachedResponse` out of the impl block, restored derives).

## [0.18.0] - 2026-11-06

### Added
- `varethic-url`: second batch of differential host vectors from Chromium's
  `URLCanonHostTest.Host` — 12 IDNA/UTS#46 cases incl. MÜNCHEN→`xn--mnchen-3ya`,
  turned-F→`xn--oo-3tu`, ♥♥→`xn--ny-s0x.us`, ₠→`xn--wzg.com`, U+023A→`xn--bc-is1a.com`,
  fullwidth/math-mono C→`abc.xyz`, Devanagari+ZWJ→`xn--11bo0mv54g`, RTL+EN→
  `xn--2-ymcov`, RTL+AN→`xn--mgbjq0r`, and 4 Bidi-failure inputs expected to
  reject. Two of my Unicode literals were initially wrong (U+210C vs U+2132,
  U+037A vs U+023A); fixed and all pass. 19 url tests.

## [0.17.0] - 2026-11-06

### Added
- UI smoke harness `tests/ui_smoke.ps1`: launches the shell, asserts the main
  window appears titled `Varethic` with a live `MainWindowHandle`, then closes
  via `WM_CLOSE` and requires clean exit 0. Passing run verified
  (window hwnd 1312800, exit code 0). Wired into docs/TESTING.md procedure.

## [0.16.0] - 2026-11-06

### Added
- `varethic-url`: differential host canonicalization vectors from Chromium's
  `URLCanonHostTest.Host` — case folding, fullwidth (nameprep), U+3002 as dot,
  IDN (`你好你好`→`xn--6qqa088eba`), ß→`xn--fuball-cta.de`, final sigma→
  `xn--wxaijb9b`, zero-width/BOM stripping. All pass via the `idna` (UTS #46)
  crate, matching ICU expectations on these cases. 18 url tests.

## [0.15.0] - 2026-11-06

### Added
- `varethic-url`: WHATWG-style absolute-path normalization — `.`/`..`
  collapsing including percent-encoded dots (lowercase `%2e` collapses,
  uppercase `%2E` preserved, matching Chromium). Differential test vectors
  derived from Chromium's `url_unittest.cc` (`chromium_differential_vectors`)
  plus an explicit `known_divergences_from_chromium` test documenting
  intentional gaps (missing-slash tolerance, non-ASCII percent-encoding,
  Windows drive-relative file paths). 17 url tests.

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

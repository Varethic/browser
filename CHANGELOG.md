# Changelog

All notable changes to this project are documented here. Version format:
`MAJOR.MINOR.PATCH`.

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

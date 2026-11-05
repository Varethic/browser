# Changelog

All notable changes to this project are documented here. Version format:
`MAJOR.MINOR.PATCH`.

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

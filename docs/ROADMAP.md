# Roadmap

Version format `MAJOR.MINOR.PATCH`: MAJOR = breaking, MINOR = stable
milestone, PATCH = fixes.

## Phase 1 — Preparation (v0.1.0, current)
- [x] Environment and repo inspection (Rust 1.99.1 MSVC, VS2022, Windows SDK,
      WebView2; 28 GB disk — full Chromium checkout NOT feasible here)
- [x] Rust workspace, docs, VERSION/CHANGELOG
- [x] Browser shell: window, tabs, omnibox, back/forward/reload/home, DevTools
- [x] Google as default search; URL vs search classification via varethic-url
- [x] First engine component: varethic-url (successor to //url), unit tested

## Phase 2 — Engine (v0.2.x)
- [x] varethic_net (v0.2.0): HTTP/1.1 GET subset (rustls+hyper), tested
- [ ] varethic_net: redirects, cookies, cache, HTTP/2, download manager
- [x] varethic_url: IPv4/IPv6/host-kind + omnibox (10 tests); WHATWG full parity, IDN, origins pending
- [ ] Download manager on top of varethic_net
- [ ] Cookie jar in Rust
- [ ] Swap shell navigation to varethic_net for supported schemes
- [ ] varethic_storage: profiles, history, bookmarks (SQLite via rusqlite)

## Phase 3 — Validation (v0.3.x)
- [ ] WPT harness for url/net components
- [ ] Behavior comparison vs Chromium for URL parsing corpus
- [ ] Rendering smoke tests (HTML/CSS/JS/WASM) via WebView2 harness
- [ ] Security: sandbox assessment, permission model, CSP enforcement

## Phase 4 — Browser features (v0.4.x)
- [ ] Vertical tabs, workspaces, sidebar, split view
- [ ] History, bookmarks, downloads UI, profiles, private browsing
- [ ] Settings UI, default-search configuration

## Phase 5 — Integration (v0.5.x)
- [ ] Real-site compatibility campaign; crash reporting; stability fixes

## Phase 6 — Release (v1.0.0)
- [ ] Signed installers, auto-update, diagnostics, docs
- [ ] Engine parity gates before removing WebView2 backend

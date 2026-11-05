# Progress Log

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

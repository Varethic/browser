# Engine Migration Tracker

Chromium components are migrated one at a time. A component is **Migrated**
only when its Rust replacement passes tests and is integrated; until then the
original code path remains authoritative. Never inflate status.

## Status legend

`Legacy` — C/C++ authoritative · `InProgress` — Rust exists + unit tests,
not integrated · `Migrated` — integrated and verified

## Components

| Chromium path | Component | Status | Rust crate | Notes |
|---|---|---|---|---|
| `//url` | URL parsing / GURL / omnibox | InProgress | `varethic-url` | Parse/validate/canonicalize + omnibox classifier + IDN/punycode + IPv6/IPv4/host-kind. Chromium study (2026-11-06): //url ships Rust FFI shims (`url.rs`, `gurl.rs`, `origin.rs`) wrapping C++ via `cxx` — upstream deliberately avoids a native Rust parser to prevent origin-comparison divergence. varethic-url is a native Rust implementation; divergence risk accepted and tracked. WHATWG origin calc pending. |
| `//net` | Network stack | InProgress | `varethic-net` | HTTP/1.1 GET/POST + redirects + cookie jar (SameSite) + cache with ETag/Last-Modified revalidation (304). Serves the shell's Varethic Fetch panel. HTTP2/proxy pending. |
| `//services/network` | Network process | Legacy | — | Depends on //net work. |
| `//content` | Navigation / lifecycle | Legacy | — | Bootstrapped by WebView2. |
| `//mojo` | IPC | Legacy | — | Shell uses tao/wry IPC with typed events. |
| `//storage` | Browser-data SQLite | InProgress | `varethic-storage` | History/bookmarks/downloads records; wired into shell (auto-history, bookmark button, download recording). |
| `//sql` | `//sql` access layer | InProgress | `varethic-storage` | Via bundled rusqlite; also backs disk HTTP cache table (v0.19.0). Sync/FTS pending. |
| blink `//third_party/blink` | HTML/CSS/DOM | Legacy | — | Long-term; no start. |
| V8 `//v8` | JavaScript | Legacy | — | Long-term; no start. |
| `//gpu` | GPU process | Legacy | — | Via WebView2/D3D11 today. |
| `//sandbox` | Sandbox | Legacy | — | WebView2 inherits Chromium sandbox. |
| `//components/bookmarks` | Bookmarks | Legacy | — | Phase 4 (new Rust impl). |
| `//components/history` | History | Legacy | — | Phase 4 (new Rust impl). |
| `//components/omnibox` | Omnibox | InProgress | `varethic-url` | Classifier only; scoring/suggestions not started. |

The machine-readable version of this table lives in
`crates/varethic_engine_core/src/lib.rs` (`COMPONENT_REGISTRY`) and is
asserted by a unit test, so docs and code cannot silently drift apart.

## Method per component

1. Inspect dependencies and behavior (Chromium source, design docs).
2. Assess feasibility; choose minimal safe-Rust scope.
3. Implement with unit tests; compare behavior against Chromium where a
   reference binary is available.
4. Integrate behind existing interfaces; keep legacy path until parity.
5. Record results here and in `docs/PROGRESS.md`.

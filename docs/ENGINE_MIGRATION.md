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
| `//url` | URL parsing / GURL / omnibox | InProgress | `varethic-url` | Parse/validate/canonicalize + omnibox classifier, 9 unit tests. WHATWG parity + IDN pending. |
| `//net` | Network stack | Legacy | — | Planned Phase 2 (rustls + hyper). |
| `//services/network` | Network process | Legacy | — | Depends on //net work. |
| `//content` | Navigation / lifecycle | Legacy | — | Bootstrapped by WebView2. |
| `//mojo` | IPC | Legacy | — | Shell uses tao/wry IPC with typed events. |
| `//storage` | SQLite backends | Legacy | — | Phase 2. |
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

# Component Inventory

High-level inventory of the Chromium surface relevant to Varethic. "Usage" is
what the shell actually relies on today.

| Area | Chromium component(s) | Today | Target |
|---|---|---|---|
| URL | `//url`, `url::Origin`, omnibox scoring | `varethic-url` (partial) | Full WHATWG parity |
| Networking | `//net`, `//services/network` | WebView2 | rustls/hyper + sandboxed process |
| Rendering | blink (HTML/CSS/DOM/layout) | WebView2 | Long-term native Rust engine |
| JS | V8 | WebView2 | Long-term; evaluate rusty_v8 vs custom |
| GPU | `//gpu`, Dawn | WebView2/D3D11 | Via WebView2, later wgpu |
| IPC | mojo | tao `UserEvent` + wry IPC (JSON) | Typed, versioned schema |
| Storage | `//sql`, `//components/history` | None | rusqlite |
| Bookmarks/history | `//components/bookmarks`, `history` | None | New Rust crates |
| Sandbox | `//sandbox` | WebView2's sandbox | Inherit, then harden native path |
| Profiles | `//components/profile_metrics` etc. | None | Phase 4 |
| Downloads | `//components/download` | WebView2 handlers | `varethic-net` based |
| Settings/search | `//components/search` | Google hardcoded as default | Configurable, Google default |

License note: any file derived from Chromium must retain Chromium's license
headers (BSD-style, `// // Copyright ...`) and be attributed in a NOTICE file.
No Chromium-derived source is vendored yet.

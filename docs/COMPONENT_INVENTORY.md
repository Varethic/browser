# Component Inventory

High-level inventory of the Chromium surface relevant to Varethic. "Usage" is
what the shell actually relies on today.

| Area | Chromium component(s) | Today | Target |
|---|---|---|---|
| URL | `//url`, `url::Origin`, omnibox scoring | `varethic-url` (partial) | Full WHATWG parity |
| Networking | `//net`, `//services/network` | `varethic-net` (HTTP/1.1) for the Fetch panel; WebView2 for page loads | HTTP/2, proxies, sandboxed process |
| HTML parsing | blink `core/html/parser` | `varethic-dom` (tokenizer + simplified tree; parse only) | Full spec, feeding a native layout engine |
| Rendering | blink (CSS/layout/paint) | WebView2 | Long-term native Rust engine |
| JS | V8 | WebView2 | Long-term; evaluate rusty_v8 vs custom |
| GPU | `//gpu`, Dawn | WebView2/D3D11 | Via WebView2, later wgpu |
| IPC | mojo | tao `UserEvent` + wry IPC (JSON) | Typed, versioned schema |
| Storage | `//sql`, `//components/history` | `varethic-storage` (rusqlite) | Sync/FTS, migrations |
| Bookmarks/history | `//components/bookmarks`, `history` | `varethic-storage` + shell panels | New UI, richer queries |
| Sandbox | `//sandbox` | WebView2's sandbox | Inherit, then harden native path |
| Profiles | `//components/profile_metrics` etc. | `varethic-storage` (profiles registry) | Isolated per-profile storage contexts |
| Downloads | `//components/download` | WebView2 handlers, recorded in SQLite | `varethic-net` based |
| Settings/search | `//components/search` | Configurable template, Google default | Richer settings model |

License note: any file derived from Chromium must retain Chromium's license
headers (BSD-style, `// // Copyright ...`) and be attributed in a NOTICE file.
No Chromium-derived source is vendored yet.

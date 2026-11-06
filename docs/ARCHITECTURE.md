# Architecture

Varethic is a two-plane system:

1. **Browser shell (Rust)** — windowing, tabs, chrome (tab strip, address
   bar), profiles, settings, downloads. Rendered with `tao` (windows/event
   loop) and `wry` (WebView child views).
2. **Engine (migrating Chromium C/C++ → Rust)** — networking, Blink
   (HTML/CSS/DOM), V8 (JS), GPU, IPC, sandbox, storage, platform integration.

## Current reality (v0.21.0, honest)

- Rendering is delegated to the **system WebView2 runtime** (Chromium-based
  Edge engine) on Windows. This is the pragmatic bootstrap path, not the end
  state.
- Engine migration progress: `varethic_url` (successor to `//url` / GURL),
  `varethic_net` (successor to `//net`, now serving the shell's Varethic Fetch
  panel), and `varethic_storage` (SQLite layer, successor to part of `//sql`).
  `varethic-engine-core` holds a machine-readable registry of component
  migration status so claims can be audited in code.
- The long-term goal is to eliminate legacy C/C++ engine implementations by
  incrementally replacing them with Rust, keeping original code until
  replacements pass tests.

## Dual network paths (important)

The browser has two network stacks, deliberately:

1. **WebView2 content path** — normal page loads. Chromium inside WebView2 does
   all fetching. No Rust engine code is involved in the byte flow yet.
2. **Varethic engine path** — the "Varethic Fetch" panel (⚡ toolbar button).
   Requests go through `varethic-net` (HTTP/1.1, redirects, cookies,
   cache with revalidation) and are persisted in `varethic-storage`'s SQLite
   disk cache. This is how the migrated `//net` is exercised in the real
   product, and it is validated end-to-end by tests in the browser crate.

## Layering

```
+------------------------------------------------------+
| varethic_browser (shell)                             |
|   tao window -> EventLoop<UserEvent>                 |
|   toolbar child WebView (HTML chrome, IPC)           |
|   one content child WebView per tab (WebView2)       |
|   Varethic Fetch panel -> tokio worker thread        |
+--------------+---------------------------------------+
               | uses
+--------------v---------------------------------------+
| varethic_engine_core (registry, shared types)        |
+----+------------------+-------------------+---------+
     | uses             | uses              | uses
+----v----------+  +----v--------------+  +-v-----------------+
| varethic_url  |  | varethic_net      |  | varethic_dom      |
|  //url        |  |  //net            |  |  HTML tokenizer + |
|  successor    |  |  cache/revalidate |  |  tree constructor |
+---------------+  +----+--------------+  +-------------------+
                        | persists to        //blink html/parser
                  +-----v---------------+
                  | varethic_storage    |
                  |  SQLite, disk cache |
                  +---------------------+
```

## Key decisions

- **WebView2 for bootstrap.** A full Chromium build needs ~100 GB disk and
  hours of toolchain setup. WebView2 gives real Chromium-grade rendering today
  while the Rust engine is built milestone by milestone. The shell talks to
  content only through `wry`'s API so the rendering backend can be swapped for
  a native Rust engine later without changing the shell's event model.
- **Migrated network is exercised in-product.** Rather than leave
  `varethic-net` as a test-only crate, the Varethic Fetch panel routes real
  requests through it, proving the Rust stack works inside the shipped shell.
  Fetches run on a worker thread with a `tokio` runtime and post results back
  through the event-loop proxy, so the UI thread is never blocked.
- **Child webviews per tab** preserve per-tab history and state; inactive
  tabs are hidden with `set_visible(false)` rather than destroyed.
- **IPC, not globals.** The toolbar HTML posts JSON messages
  (`{type: "navigate", input: ...}`) via `window.ipc.postMessage`; Rust maps
  them to a typed `UserEvent` enum on the `tao` event loop. All mutations of
  tab state happen on the event loop thread.
- **Safe Rust first.** `unsafe` is currently unused anywhere in the
  workspace. Any future `unsafe` (e.g., FFI into Chromium) must be documented
  with a `// SAFETY:` comment.

## Renderer backend swap path

1. v0.1–v0.3: WebView2 content views.
2. Introduce a `varethic-render` abstraction crate (navigation, painting,
   input) with a WebView2 backend.
3. As engine components (url, net, dom, css, js) reach parity, they are
   linked in behind the same abstraction.
4. WebView2 backend is removed only when the Rust engine passes the
   validation suite in `docs/TESTING.md`.

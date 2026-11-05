# Architecture

Varethic is a two-plane system:

1. **Browser shell (Rust)** — windowing, tabs, chrome (tab strip, address
   bar), profiles, settings, downloads. Rendered with `tao` (windows/event
   loop) and `wry` (WebView child views).
2. **Engine (migrating Chromium C/C++ → Rust)** — networking, Blink
   (HTML/CSS/DOM), V8 (JS), GPU, IPC, sandbox, storage, platform integration.

## v0.1.0 reality (honest)

- Rendering is delegated to the **system WebView2 runtime** (Chromium-based
  Edge engine) on Windows. This is the pragmatic bootstrap path, not the end
  state.
- The engine migration has started with `varethic-url` (successor to
  `//url` / GURL). `varethic-engine-core` holds a machine-readable registry
  of component migration status so claims can be audited in code.
- The long-term goal is to eliminate legacy C/C++ engine implementations by
  incrementally replacing them with Rust, keeping original code until
  replacements pass tests.

## Layering

```
+--------------------------------------------------+
| varethic_browser (shell)                         |
|   tao window -> EventLoop<UserEvent>             |
|   toolbar child WebView (HTML chrome, IPC)       |
|   one content child WebView per tab              |
+--------------+-----------------------------------+
               | uses
+--------------v-----------------------------------+
| varethic_engine_core (registry, shared types)    |
+--------------+-----------------------------------+
               | uses
+--------------v-----------------------------------+
| varethic_url (Rust successor to //url)           |
|   Url::parse, canonicalize, classify_omnibox      |
+--------------------------------------------------+
```

## Key decisions

- **WebView2 for bootstrap.** A full Chromium build needs ~100 GB disk and
  hours of toolchain setup that this environment does not have (28 GB free).
  WebView2 gives real Chromium-grade rendering today while the Rust engine is
  built milestone by milestone. The shell talks to content only through
  `wry`'s API so the rendering backend can be swapped for a native Rust
  engine later without changing the shell's event model.
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

# Varethic Browser

A production-grade Rust browser that progressively migrates Chromium's C/C++
engine components to safe Rust, preserving functionality and security at every
step.

> **Status: v0.1.0 (early).** The shell renders real web content via the
> system WebView2 runtime on Windows. The Chromium engine migration has begun
> with `//url` (see `docs/ENGINE_MIGRATION.md`). This is **not** yet a
> production-ready browser; see `docs/KNOWN_ISSUES.md`.

## Quick start (Windows)

Prerequisites: Rust stable (MSVC), Visual Studio 2022 Build Tools with the
Windows SDK, and the WebView2 Evergreen Runtime (preinstalled on Windows 11).

```powershell
cargo build -p varethic-browser
cargo test --workspace
.\target\debug\varethic-browser.exe
```

## Features (v0.1.0)

- Native window with tab strip and address bar (browser chrome rendered as an
  HTML toolbar child WebView, IPC to Rust)
- Multiple tabs with per-tab webview state (back/forward history preserved)
- Address bar distinguishes URLs from search queries; default search engine is
  Google (`https://www.google.com/search?q={searchTerms}`)
- Back / forward / reload / home / DevTools
- `varethic-url`: Rust implementation of the core of Chromium's `//url`
  (GURL) role — parsing, validation, canonicalization, omnibox classification

## Repository layout

```
crates/
  varethic_url/          Rust engine component replacing //url (in progress)
  varethic_engine_core/  Engine types + machine-readable migration registry
  varethic_browser/      Browser shell (tao + wry / WebView2)
docs/                    Architecture, roadmap, migration status, testing
```

## Documentation

- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
- [docs/ROADMAP.md](docs/ROADMAP.md)
- [docs/ENGINE_MIGRATION.md](docs/ENGINE_MIGRATION.md)
- [docs/COMPONENT_INVENTORY.md](docs/COMPONENT_INVENTORY.md)
- [docs/TESTING.md](docs/TESTING.md)
- [docs/KNOWN_ISSUES.md](docs/KNOWN_ISSUES.md)
- [docs/PROGRESS.md](docs/PROGRESS.md)

## Search & versioning

- Default search: `https://www.google.com/search?q={searchTerms}`
- Version format `MAJOR.MINOR.PATCH` (breaking / milestone / fixes). See
  `VERSION` and `CHANGELOG.md`.

## License

BSD-3-Clause. Chromium-derived components retain their Chromium licenses
(see NOTICE when applicable). "Chromium" is a trademark of Google LLC; this is
an independent project and is not affiliated with or endorsed by Google.

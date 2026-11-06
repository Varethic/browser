# CLAUDE.md — agentic working notes for this repository

This file tells AI/agentic contributors how to work in `Varethic/browser`.
It complements `README.md` and `docs/*` rather than replacing them.

## What this project is

A production-grade Rust browser that progressively migrates Chromium's C/C++
engine components to safe Rust. Rendering today is the system WebView2
runtime (Chromium-in-Edge); the native Rust engine is built incrementally in
`crates/`. Nothing may be marked `Migrated` in the component registry
(`crates/varethic_engine_core/src/lib.rs`) until a Rust replacement passes
tests and is integrated. **Never fake progress, never weaken a test to pass,
never claim unverified features are production-ready.**

## Repository layout

- `crates/varethic_url` — Rust URL engine component (//url successor)
- `crates/varethic_net` — Rust HTTP/1.1 client (//net successor, GET/POST,
  redirects, cookies with SameSite, in-memory cache)
- `crates/varethic_storage` — SQLite layer (rusqlite, bundled): history,
  bookmarks, downloads, settings, profiles, workspaces
- `crates/varethic_engine_core` — engine types + machine-readable migration
  registry (`COMPONENT_REGISTRY`), kept in sync with docs/ENGINE_MIGRATION.md
- `crates/varethic_browser` — Windows shell: windowing (tao), rendering
  (wry/WebView2), toolbar chrome (HTML + JSON IPC), per-tab child webviews,
  vertical tabs, split view, workspaces, profiles, private tabs, panels
- `docs/` — ARCHITECTURE, ROADMAP, ENGINE_MIGRATION, COMPONENT_INVENTORY,
  TESTING, KNOWN_ISSUES, PROGRESS, CHROMIUM_REFERENCE, VERSION, CHANGELOG

## Local reference checkout

Chromium's source is at `C:\Users\admin\Desktop\Varethic\chromium`
(shallow, not built, not vendored). See `docs/CHROMIUM_REFERENCE.md`.

## Build & test (Windows)

```powershell
cargo test --workspace          # all tests must pass
cargo build -p varethic-browser # needs MSVC: run from a VS2022 developer shell
.\target\debug\varethic-browser.exe
```

A stale incremental cache once produced phantom `E0599` errors: fix with
`cargo clean -p <crate>` if methods that clearly exist fail to resolve.

## Working agreement (from project owner)

- Git identity: `Varethic <application@varethic.in.th>`; remote `origin`.
- Commit completed, verified changes; **push only when the user authorizes it**
  (authorization given for this repo as of 2026-11-06, but re-confirm for
  unusual pushes). Never force-push without permission.
- For every meaningful change: inspect status, implement, `cargo fmt --all`,
  build, `cargo test --workspace`, update `VERSION` (MAJOR.MINOR.PATCH),
  `CHANGELOG.md`, and the docs listed above, then commit and verify.
- Keep docs honest: gaps go in `KNOWN_ISSUES.md`, verified evidence in
  `PROGRESS.md` and `TESTING.md`.
- Safe Rust preferred; document any `unsafe` with a `// SAFETY:` comment.
- Do not vendor Chromium-derived code without retaining license headers and a
  NOTICE file.

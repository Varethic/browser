# Testing

## Current (v0.1.0)

```powershell
cargo test --workspace
```

- `varethic-url`: 9 unit tests — parsing, canonicalization, ports,
  userinfo, omnibox navigate-vs-search classification, percent-encoding.
- `varethic-engine-core`: registry consistency test.
- `varethic-net`: 3 local tests (local hyper server, scheme rejection,
  timeout) + 1 `#[ignore]`d live-HTTPS test (run manually, passed).
- `varethic-storage`: 4 unit tests (history upsert/search/clear, bookmarks,
  downloads, on-disk persistence).
- Browser binary: built debug and launch-tested; window stays alive,
  `%LOCALAPPDATA%\Varethic\varethic.db` is created on first run.

## Scripted smoke test (v0.17.0+)

```powershell
cargo build -p varethic-browser
powershell -ExecutionPolicy Bypass -File tests\ui_smoke.ps1
# Expected: "PASS: window 'Varethic' appeared ..." and exit code 0
```

Launches the browser, asserts title and window handle, closes via WM_CLOSE,
requires clean exit. Run it after every shell change.

## Manual smoke checklist

1. Launch `target\debug\varethic-browser.exe`.
2. Type `varethic.in.th` → should navigate (https inferred).
3. Type `rust browser engine` → Google results page.
4. Open 3 tabs, close middle tab, confirm active tab switching works.
5. Back/forward/reload/home buttons.
6. `</>` opens WebView2 DevTools.

## Planned

- URL corpus differential testing vs Chromium (`url_test_data` style).
- WPT (web-platform-tests) subsets once native rendering exists.
- Integration tests driving the shell via Windows UI Automation.
- Performance: cold-start, tab-switch, and memory footprint budgets.

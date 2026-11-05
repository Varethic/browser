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
- Browser binary: built debug and launch-tested; window stays alive and
  loads WebView2 content (manual verification on Windows 11).

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

# Testing

## Current (v0.20.0)

```powershell
cargo test --workspace
```

59 tests, all passing (1 `#[ignore]`d live-network test excluded):

- `varethic-url`: 19 tests — parsing/canonicalization, ports, userinfo,
  percent-encoding, IDN/UTS #46, WHATWG origins, path dot-segment
  normalization, omnibox navigate-vs-search classification, plus differential
  vectors ported from Chromium's `url_unittest.cc` / `url_canon_unittest.cc`
  and a documented divergences test.
- `varethic-net`: 31 tests — local hyper servers exercise GET, absolute and
  relative redirects, redirect-limit errors, cookies (incl. SameSite), POST
  echo, scheme rejection, timeout, cache hits, and ETag/Last-Modified
  revalidation with a 304. One `#[ignore]`d live-HTTPS test.
- `varethic-storage`: 8 tests — history upsert/search/clear, bookmarks,
  downloads, settings, profiles, workspaces, disk HTTP cache, on-disk
  persistence.
- `varethic-engine-core`: registry consistency test.
- Browser binary: built and launch-tested (window stays alive; DB created on
  first run).

## Experimental (not asserted)

`tests/experimental/` holds UI-automation harnesses that do **not** currently
pass reliably; see its README for what was learned. They are not part of the
verified suite.

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

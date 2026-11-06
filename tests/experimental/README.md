# Experimental UI harnesses (NOT part of the verified test suite)

These PowerShell scripts exercise the Varethic shell through Windows UI
Automation. They are **exploratory** and their results are recorded here
honestly. They are not wired into CI and are not asserted to pass.

## What was learned (2026-11-06, v0.20.0)

- The Windows UI Automation tree for the shell **does** expose the chrome
  controls by `AutomationId`: the omnibox (`omnibox`) and every toolbar button
  (`newtab`, `back`, `forward`, `reload`, `home`, `bookmark`, `history`,
  `bookmarks`, `downloads`, `private`, `workspaces`, `split`, `spliturl`,
  `vtabs`, `profiles`, `settings`, `devtools`). Toolbar button `Name`s are the
  button glyphs (some render as `?` in a non-UTF console).
- `ui_interact.ps1` was run and **did not yet observe** the new-tab button
  adding a web-content pane in the automation tree. Whether the click reached
  the WebView2 `onclick` handler or the tree simply did not refresh is
  **undetermined**; the harness therefore reported FAIL and is parked here.
- `ui_navigate.ps1` was run and **failed**: setting the omnibox value via
  `ValuePattern.SetValue` does not fire the DOM `keydown` event, so the
  `{type:'navigate'}` IPC message is never sent; synthetic `keybd_event`
  keystrokes were not delivered in this (non-interactive) session either.

## Conclusion

Automated *interaction* validation of the WebView2-based chrome is not yet
reliable in this environment. Verified UI coverage remains:

- `../ui_smoke.ps1` (committed, passing): window appears, titled `Varethic`,
  live handle, clean `WM_CLOSE` exit.

To make these experimental harnesses reliable we would need a deterministic
in-page test hook (e.g. a JS function the shell exposes for tests) or a real
interactive desktop session. Tracked in `docs/KNOWN_ISSUES.md`.

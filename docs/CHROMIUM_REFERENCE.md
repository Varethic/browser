# Chromium reference checkout

A local, shallow reference checkout of Chromium lives at
`C:\Users\admin\Desktop\Varethic\chromium` (outside this repository — the
code is NOT vendored into it).

- Source: `https://github.com/chromium/chromium.git` (shallow, `--depth 1`), HEAD
  `4a5fcfa6ce` on `main` as of 2026-11-05/06.
- Purpose: study engine architecture for the Varethic migration
  (`//url`, `//net`, `//content`, `//services/network`, blink, v8, mojo,
  `//sql`, `//storage`, gpu, sandbox). No Chromium-derived code is copied
  into this repository without retaining its license headers and attribution.
- Build status: NOT built. Building Chromium requires depot_tools, ~100 GB
  and hours; the disk now has the room but the build has not been run.
- The GitHub mirror uses git submodules for some third_party trees and
  internal-only components (e.g. `chrome/browser/resources/preinstalled_web_apps/internal`);
  those are absent here and are not needed for study. `//v8` is also a git
  submodule and is NOT fetched (would be `git submodule update --init v8`;
  ~2 GB additional). `//url`, `//net`, `//services`, `//storage`, `//sql`,
  `//sandbox`, `//mojo`, `//gpu` are in-tree and present.
- If the checkout is missing or stale, re-create with:
  `git clone --depth 1 https://github.com/chromium/chromium.git chromium`

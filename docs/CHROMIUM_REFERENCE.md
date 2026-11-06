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
- //url origin model studied: Chromium `url::Origin` (see `origin.h`) computes
  tuple origins for special schemes (http/https/ws/wss/ftp, default ports)
  with opaque origins elsewhere; varethic-url now implements the same tuples
  (v0.10.0). Chromium itself binds this via cxx shims, not a native Rust impl.
- Submodule status (2026-11-06): `//v8` fetched (`git submodule update --init
  --depth 1 v8`, HEAD `f6b7d954`); `third_party/blink` is in-tree (not a
  submodule); some third_party and `chrome/.../internal` trees remain absent
  and are unneeded for study.
- If the checkout is missing or stale, re-create with:
  `git clone --depth 1 https://github.com/chromium/chromium.git chromium`

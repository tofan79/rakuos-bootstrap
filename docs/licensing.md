# Licensing

## Project license

`rakuos-bootstrap` is released under the MIT License. The full text is in
[`../LICENSE`](../LICENSE).

```text
MIT License

Copyright (c) 2026 tofan79
```

SPDX headers in the source files use:

```text
// SPDX-License-Identifier: MIT
```

The RPM spec declares `License: MIT`, matching `Cargo.toml`.

## What the MIT license allows

You may use, copy, modify, merge, publish, distribute, sublicense, and sell
copies of the software, provided that the copyright notice and permission
notice are included in all copies or substantial portions. The software is
provided without warranty.

## Third-party dependencies

The repository vendors its Rust dependencies under `vendor/`. Each crate keeps
its own license files. Do not remove them when updating vendored sources.

| Crate | Used for |
|---|---|
| `anyhow` | Error context and propagation |
| `console` | Terminal capabilities, colors, key input, text measurement |
| `serde` | Catalog deserialization |
| `toml` | TOML parsing |

The exact versions are pinned in `Cargo.lock`. If you add a dependency, check
that its license is compatible with MIT distribution and that its license files
remain in the vendor tree.

## Bundled data

- `data/catalog.toml`: package names, descriptions, and choices written for this
  project.
- `data/skel/`: minimal desktop configuration written for this project.
- `data/sessions/hyprland-uwsm.desktop`: session entry maintained here, based on
  the upstream Hyprland/uwsm session format.

Upstream package names and trademarks belong to their respective projects.
Nothing in this repository changes the license of the packages the wizard
installs.

## Contributions

By contributing, you agree that your contribution is licensed under the MIT
License used by this project. See [`../CONTRIBUTING.md`](../CONTRIBUTING.md).

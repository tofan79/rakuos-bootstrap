# Architecture

## Goals

`rakuos-bootstrap` is deliberately small and dependency-light:

- one static binary with no GTK/Qt runtime;
- a catalog-driven list of choices so adding a package or desktop rarely needs
  Rust changes;
- safe to run on bootc/immutable systems: persistent writes go to `/etc`,
  `/var`, and the target user's home; session-file writes under `/usr/share`
  are best-effort and a read-only root is downgraded to a warning when the
  packaged data still works;
- repeatable: re-running the wizard backs up user-edited files instead of
  overwriting them silently.

## Source layout

| Path | Responsibility |
|---|---|
| `src/main.rs` | CLI parsing, phase state machine, summary screen, reboot prompt |
| `src/catalog.rs` | Deserialize `data/catalog.toml`, sort choices by name, resolve effective login |
| `src/ui.rs` | Terminal UI: glyph sets, palette, frames, list/detail panes, key handling, summary and progress screens |
| `src/install.rs` | Repository setup, package installation, skel/session files, display-manager configuration, state, process execution, logging tees |
| `src/verify.rs` | Read-only post-install checks (packages, display manager, Wayland session, skel files) |
| `src/log.rs` | Append-only log file, ANSI stripping, progress-line collapsing |
| `src/testing/` | Unit tests, mostly pure functions and catalog invariants |
| `data/catalog.toml` | Built-in catalog (also embedded with `include_str!`) |
| `data/skel/` | Bundled minimal desktop configurations |
| `data/sessions/` | Session `.desktop` overrides installed into `/usr/share/wayland-sessions` |
| `data/rakuos-bootstrap.service` | Optional first-boot systemd unit |
| `scripts/buat-payload-uji.sh` | Builds a small ISO for VM testing |

## Data flow

```text
CLI args ──► Opts ──► Catalog::load(path or embedded)
                         │
                         ▼
              phase state machine (main.rs)
                Desktop → Terminal → File Manager → Browser
                    → Login (conditional) → Summary
                         │
                         ▼
                 Picks (references into Catalog)
                         │
                         ▼
                 install::apply()
   repos → cache clean → packages → skel/session files
   → login manager → state → verify
                         │
                         ▼
              reboot prompt (or automatic in --first-boot)
```

`Catalog` owns the choices for the whole process; `Picks` holds borrows of the
chosen entries. This keeps the install code independent from UI state and makes
it easy to test with a hand-built catalog.

## Phase state machine

`main.rs` uses numeric phases instead of an enum at the call site:

| Constant | Value | Screen |
|---|---|---|
| `PHASE_DESKTOP` | 0 | single-select desktop |
| `PHASE_TERMINAL` | 1 | multi-select terminals |
| `PHASE_FILE_MANAGER` | 2 | multi-select file managers |
| `PHASE_BROWSER` | 3 | multi-select browsers |
| `PHASE_LOGIN` | 4 | single-select login manager (only when the desktop sets `choose_login`) |
| `PHASE_SUMMARY` | 5 | review and confirm |

Back navigation restores the previously checked indexes. Going back from the
summary returns to the login screen only when that screen was actually shown.

## Error handling

- User cancellation (`Ctrl-C`) is a typed `Cancelled` error; `main` prints
  `Cancelled. No changes were made.` and exits with status 130.
- Missing prerequisites, invalid flags, or TTY failures exit with status 1.
- Command failures are wrapped with the program name, exit status, and a pointer
  to the log file.
- Log setup failure is non-fatal: the wizard warns and continues without a log.
- Read-only root filesystem errors while writing session files are warnings, not
  fatal errors, because the package-provided session remains available.

## Immutable root handling

RakuOS is image-based. `/usr` may be read-only at runtime. The code treats
`PermissionDenied`, `Unsupported`, and messages containing `read-only file
system` as `is_immutable_fs_error()`. Session-file writes that hit this error
are skipped with a warning; everything that must persist is written to `/etc`,
`/var`, or the user's home.

## Why the catalog is embedded

`data/catalog.toml` is embedded at compile time as a fallback, but
`/usr/share/rakuos/bootstrap/catalog.toml` is preferred when present. This lets
the RPM update the catalog without rebuilding the binary, while `--dry-run`
and standalone payload builds still work with no installed data directory.

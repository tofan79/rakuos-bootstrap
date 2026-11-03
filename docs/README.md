# rakuos-bootstrap documentation

`rakuos-bootstrap` is a post-install setup wizard for minimal RakuOS images. It
is not an operating-system installer: the wizard adds a desktop environment,
applications, and a login manager to an already booted RakuOS system, then
writes the desktop configuration that belongs in the target user's home.

The binary is a small Rust TUI with no GTK/Qt dependency. It is designed to run
in a text terminal, including the Linux virtual console, and to stay useful when
the machine is offline or the root filesystem is immutable (bootc).

## What it does

1. Shows a guided terminal UI for choosing a desktop, terminals, file managers,
   browsers, and (when needed) a login manager.
2. Enables the third-party repositories requested by the catalog.
3. Installs the selected RPM packages through `rum` (or `dnf` when `rum` is not
   available).
4. Copies bundled minimal desktop configurations (skel) into the target user's
   home, with backups of conflicting files.
5. Registers the Wayland session and configures the selected display manager
   (SDDM, greetd, or a built-in login path).
6. Writes state and a completion marker, verifies the result, and optionally
   reboots.

## Quick start

```bash
# Preview every command without changing the system
./target/release/rakuos-bootstrap --dry-run

# Run the real setup (root is required, a TTY is required)
sudo ./target/release/rakuos-bootstrap
```

The full option list and the Indonesian user-facing guide live in the root
[`README.md`](../README.md). The Indonesian UI guide is in [`ui.md`](../ui.md).

## Documentation map

| Document | Contents |
|---|---|
| [`architecture.md`](architecture.md) | Source layout, modules, data flow, and design decisions |
| [`catalog.md`](catalog.md) | Complete `data/catalog.toml` reference |
| [`install-flow.md`](install-flow.md) | What happens after the user confirms the summary |
| [`ui.md`](ui.md) | Terminal UI modes, layout, keyboard handling, and rendering |
| [`building-and-testing.md`](building-and-testing.md) | Toolchain, vendored offline builds, tests, lint, payload ISO |
| [`releasing.md`](releasing.md) | Version bump, tags, GitHub releases, and COPR builds |
| [`troubleshooting.md`](troubleshooting.md) | Common failures and how to diagnose them |
| [`licensing.md`](licensing.md) | Project license and vendored dependency licenses |

## Language policy

- `README.md` and the root `ui.md` are written in Indonesian for end users.
- Everything under `docs/` and `CONTRIBUTING.md` is written in English.
- Source comments keep the style of the surrounding code; most existing code
  comments are Indonesian.

## Project facts

| Item | Value |
|---|---|
| Language | Rust 2024 edition |
| Runtime dependencies | `anyhow`, `console`, `serde`, `toml` |
| Package manager | `rum` (fallback: `dnf`) |
| Install prefix | `/usr/bin/rakuos-bootstrap` |
| Data directory | `/usr/share/rakuos/bootstrap` |
| Log file | `/var/log/rakuos-bootstrap.log` |
| State file | `/var/lib/rakuos/bootstrap.toml` |
| Completion marker | `/var/lib/rakuos/bootstrap.done` |
| License | MIT |

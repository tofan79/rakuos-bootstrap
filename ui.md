# RakuOS Bootstrap UI

This guide describes the terminal UI rendered by `src/ui.rs` and the setup flow
in `src/main.rs`. The wizard runs in a text terminal, including the Linux VT. It
does not require GTK, Qt, or a mouse.

## Visual design

The interface uses a cyan accent for the RakuOS brand, clear step progress, a
highlighted current choice, concise descriptions, and a consistent keyboard
help line. Colors adapt to the terminal and `NO_COLOR`; the UI falls back to
plain text when colors are unavailable or output is not interactive.

| Mode | When it is used | Frame and markers |
|---|---|---|
| Full | UTF-8 terminal emulator | `╭─╮`, `✔ ● ○ ❯` |
| TTY | `TERM=linux` with a UTF-8 locale | `┌─┐`, `[x] [>] [ ]` |
| ASCII | Non-UTF-8 locale, `--ascii`, `RAKUOS_ASCII`, or `TERM=dumb` | `+-+`, `[x] [>] [ ]` |

The terminal uses bright cyan when supported. On the Linux VT it uses the
bright ANSI cyan color; on other terminals it uses 256-color cyan when
available.

## Keyboard controls

- `↑` / `↓`, `j` / `k`, or `Tab`: move the highlight.
- Multi-select lists: `Space` or `1`–`9` toggles an option.
- Single-select lists: `1`–`9` selects an option.
- `Enter`: continue when the selection is valid.
- `Esc`, `Backspace`, `←`, or `b`: go back one step.
- `Ctrl-C`: quit the wizard. On the first step, the back keys show a hint to
  use `Ctrl-C` to quit.

Terminal, File Manager, and Browser allow multiple selections and require at
least one option. Desktop and Login Manager allow one selection. Login Manager
appears only when the selected desktop has `choose_login = true`. On the review
screen, `Enter` or `y` continues, `n` cancels, and a back key returns to the
last visible selection step.

## Setup flow

```text
Desktop → Terminal → File Manager → Browser → [Login Manager when needed]
   ↓                                      ↓
Ryoku setup                          Review your setup
                                           ↓
                              Install, configure, and verify
```

Ryoku skips the generic app and login selections and uses its own setup path.
For other desktops, login is skipped when the desktop provides its own login
manager or the catalog sets `login_override`. On terminals under 72 columns,
the stepper shows only the active step name.

KineticWE uses the `theblackdon/kwee` COPR for its desktop package and skips
`mindset/Mindset-Apps`. Terra and RPM Fusion can still be enabled for separately
selected apps. Ryoku uses only its dedicated RPM and dependency COPR setup.

## Example screens

The examples use the current catalog. Options and descriptions come from
`data/catalog.toml` and may change.

### Choose a terminal

```text
╭────────────────────────────────────────────────────────────────────────────╮
│ RAKUOS setup                                                       v0.3.0 │
│ Set up your RakuOS desktop                                        DRY-RUN │
╰────────────────────────────────────────────────────────────────────────────╯

 ✔ Desktop ── ● Terminal ── ○ File Manager ── ○ Browser ── ○ Sign-in

 STEP 2 OF 5 · Terminal
 Terminal for running commands
 ────────────────────────────────────────────────────────────────────────────
   [ ] Alacritty        GPU-accelerated with a simple YAML configuration
   [ ] Foot             Lightweight, Wayland-only, ideal for minimal systems
   [ ] Ghostty          From the Terra repository; enabled automatically
 ❯ [x] Kitty            GPU-accelerated and commonly used with Hyprland
 1 selected

 ↑↓ Move  Space Toggle  Enter Continue  Esc/← Back  Ctrl-C Quit
```

`default = true` in the catalog determines which multi-select options start
checked. Options are sorted alphabetically. Selections remain checked when you
return to a step.

### Choose a desktop

```text
 STEP 1 OF 5 · Desktop
 Choose your Wayland desktop
 ────────────────────────────────────────────────────────────────────────────
   Budgie       Traditional desktop with a Wayland session
 ❯ Hyprland     Dynamic tiling with a minimal configuration
   KineticWE    KWin-based tiling desktop with the Noctalia shell
   LXQt         Lightweight desktop with a Wayland session through Miriway
   MangoWM      Lightweight tiling compositor based on dwl (wlroots)
   MiracleWM    Wayland tiling compositor; Fedora group includes SDDM
   Ryoku        Complete Wayland desktop; manages its own setup
   Sway         Lightweight Wayland tiling desktop
   Umbriel      Lightweight Wayland compositor; daily Git build

 ↑↓ Move  Enter Continue  Esc/← Back  1-9 Jump  Ctrl-C Quit
```

Selecting Ryoku skips the generic terminal, file manager, browser, and login
manager screens. It installs Ryoku's default Kitty, Nautilus, Firefox, Fish,
both compositor sessions, its default app bundle, and SDDM theme through the
dedicated RPM path.

### Review your setup

The review shows the selected software, login method, repositories, target user,
and the number of RPM packages that will be installed. Package names wrap to at
most two lines.

```text
 ✔ Desktop ── ✔ Terminal ── ✔ File Manager ── ✔ Browser ── ✔ Sign-in

 Review your setup
 ────────────────────────────────────────────────────────────────────────────
 Desktop                Budgie
 Terminal               Kitty
 File Manager           Nautilus
 Browser                Firefox
 Login                  SDDM
 Repositories           terra, rpmfusion, mindset
 Target user            mindset
 RPM packages (…)
   …
 ? Continue with installation?

 Y/Enter Continue  Esc/← Back  n Cancel  Ctrl-C Quit
```

## Installation and completion

After confirmation, the selection UI gives way to the cyan RakuOS header and
live command output. Commands and their output are written to the log. In
`--dry-run` mode, planned commands are prefixed with `+`; no packages are
installed and no files are written.

After verification, a normal run asks whether to reboot (`Y/n`). A `--first-boot`
run reboots automatically after successful verification. If verification fails,
automatic reboot is skipped and the log location is shown.

The screen width is capped at 78 columns and adapts down to 40 columns. Long
descriptions are shortened, and long lists scroll within the available terminal
height. The cursor is hidden while the wizard is interactive and restored when
it exits. The UI does not use the alternate screen, so the final output remains
in terminal scrollback.

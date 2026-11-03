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

                                           ↓
                              Install, configure, and verify
```


For other desktops, login is skipped when the desktop provides its own login
manager or the catalog sets `login_override`. On terminals under 72 columns,
the stepper shows only the active step name.

KineticWE disables `mindset/Mindset-Apps` if it is already enabled, then enables
the `theblackdon/kwee` COPR for its desktop package. Terra and RPM Fusion can
still be enabled for separately selected apps.
RPM and dependency COPR setup.

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
 Options                            │ Details
 ───────────────────────────────────┼───────────────────────────────────
   [ ] Alacritty                     │ Kitty
   [ ] Foot                          │
   [ ] Ghostty                       │ GPU-accelerated and commonly used
 ❯ [x] Kitty                         │ with Hyprland
                                     │ Selected
                                     │ Packages
                                     │ kitty
                                     │ + 3 more
 1 selected

 ↑↓ Move  Space Toggle  Enter Continue  Esc/← Back  Ctrl-C Quit
```

`default = true` in the catalog determines which multi-select options start
checked. Options are sorted alphabetically. Selections remain checked when you
return to a step. Wide terminals show the option list beside details for the
highlighted package; narrow terminals use the compact list layout.

### Choose a desktop

```text
 STEP 1 OF 5 · Desktop
 Choose your Wayland desktop
 ────────────────────────────────────────────────────────────────────────────
 Options  ↓1                         │ Details
 ───────────────────────────────────┼───────────────────────────────────
   Budgie                          │ Hyprland
 ❯ Hyprland                        │
   KineticWE                       │ Dynamic tiling with a minimal
   LXQt                            │ configuration and no panel
   MangoWM                         │ Selected
   MiracleWM                       │ Packages
   Sway                            │ + 4 more

 ↑↓ Move  Enter Continue  Esc/← Back  1-9 Jump  Ctrl-C Quit
```

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

The screen width is capped at 78 columns and adapts down to 40 columns. On wide
terminals, descriptions wrap in the details pane and long option lists scroll
within the available height. Narrow terminals use a compact list with shortened
descriptions. The cursor is hidden while the wizard is interactive and restored
when it exits. The UI does not use the alternate screen, so the final output
remains in terminal scrollback.

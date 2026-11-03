# Terminal UI

The UI is implemented in `src/ui.rs` with the `console` crate only. It renders
in a normal terminal, in the Linux virtual console, and in a plain ASCII
fallback. It never uses the alternate screen, so the final output stays in the
scrollback buffer.

The Indonesian end-user guide remains in [`../ui.md`](../ui.md). This page is
the English engineering reference.

## Terminal detection

Capability detection happens once in `Ui::new()`.

### Glyph sets

| Mode | Selected when | Frame | Markers |
|---|---|---|---|
| Full | UTF-8 locale and a normal terminal | `╭─╮` | `✔ ● ○ ■ □ ❯` |
| TTY | `TERM=linux` with UTF-8 | `┌─┐` | `[x] (*) [ ]` |
| ASCII | non-UTF-8 locale, `--ascii`, `RAKUOS_ASCII`, `TERM=dumb` | `+-+` | `[x] (*) [ ]` |

### Colors

| Level | Selected when | Accent |
|---|---|---|
| `Xterm256` | terminal name contains `256`, or `COLORTERM` is set | color 51 (bright cyan) |
| `Vt16` | Linux console or a basic terminal | bright ANSI cyan |
| `None` | output is not a TTY or `NO_COLOR` is set | plain text |

Warnings are yellow; success is green; dim text is used for labels, hints, and
secondary details.

## Geometry

- Width is the terminal width minus two columns, clamped to 40-78 columns.
- Height is the terminal height, at least 12 rows.
- Wide terminals (68 columns or more) use a two-pane layout: an options list on
  the left and a details pane on the right.
- Narrow terminals use a compact single-column list with truncated
  descriptions.
- Long lists scroll inside the available height; the options title shows how
  many entries are above or below the window.
- The cursor is hidden while the wizard is interactive and restored on exit or
  error.

## Screen structure

```text
┌ header ──────────────────────────────────────────────┐
│ brand + version                         badge (opt.) │
│ tagline                                              │
└──────────────────────────────────────────────────────┘

● Desktop ── ○ Terminal ── ○ File Manager ── ...   (stepper)

 Desktop                                    Step 1 of 5
 Choose your Wayland desktop
 ────────────────────────────────────────────────────────
 Options  ↓1                  │ Details
 ─────────────────────────────┼─────────────────────────
 ❯ ● Hyprland                 │ Hyprland
   ○ KineticWE                │ ...
                              │ Packages (5)

 ↑↓ Move  Enter Continue  Esc/← Back  Ctrl-C Quit
```

### Header

- Brand: `RakuOS Setup`.
- Version from `CARGO_PKG_VERSION`.
- Optional badges: `DRY-RUN` or `FIRST BOOT`.
- Tagline: `Post-install setup: choose your desktop and apps`.

### Stepper

`Desktop → Terminal → File Manager → Browser → Login`

Completed steps use the done glyph; the current step is bold with the current
glyph; future steps are dim. On terminals narrower than 72 columns, only the
active step name is shown.

The login step is skipped for desktops that do not ask for a login manager.

### Options list

Each row contains:

- the cursor (`❯`) on the highlighted row;
- a radio marker (`●`/`○`) for single-select lists;
- a checkbox (`■`/`□`) for multi-select lists;
- the choice name;
- status text when the entry is unavailable.

Unavailable entries are dimmed and cannot be selected.

There are no numeric jump shortcuts. Navigation is `↑`/`↓`, `j`/`k`, `Tab`,
and `Enter`.

### Details pane

The right pane shows, for the highlighted entry:

- the name;
- `Selected` / `Not selected` or `Not available`;
- a `Default` tag when `default = true`;
- up to four wrapped description lines;
- a package count and up to three package lines, with `… +N more` when
  truncated.

The panel height is fixed so the screen does not jump while moving the
highlight.

### Footer

Hints are rendered as bold keys and dim labels. If the terminal is too narrow,
hints are dropped from the right, keeping the most important ones.

| Screen | Hints |
|---|---|
| Single select | `↑↓ Move`, `Enter Continue`, `Esc/← Back`, `Ctrl-C Quit` |
| Multi select | `↑↓ Move`, `Space Toggle`, `Enter Continue`, `Esc/← Back`, `Ctrl-C Quit` |
| Summary | `Enter/Y Continue`, `Esc/← Back`, `N Cancel`, `Ctrl-C Quit` |

## Keyboard handling

| Key | Action |
|---|---|
| `↑`, `↓`, `j`, `k`, `Tab` | Move the highlight (wraps around) |
| `Space` | Toggle the highlighted multi-select entry |
| `Enter` | Continue; on the summary this means "yes" |
| `Esc`, `Backspace`, `←`, `b` | Go back one screen |
| `Ctrl-C` | Cancel the whole wizard |
| `y` / `Y` | Confirm (summary only) |
| `n` / `N` | Cancel without changes (summary only) |

Back on the first screen prints a hint to use `Ctrl-C`; it does not quit.

Multi-select screens require at least one checked entry. `default = true`
entries start checked; selections are preserved when navigating backwards.

## Summary screen

The summary lists:

- Desktop
- Terminal
- File Manager
- Browser
- Login (including "automatic from <desktop>" or "built-in for <desktop>")
- Repositories
- Target user
- Package count and a wrapped package preview (two lines maximum)

Long values wrap to at most two lines and then use `…`.

## Installation screen

After confirmation, the screen is cleared, the header and completed stepper
remain visible, and command output streams below. `run()` prefixes each command
with `→` on screen; `--dry-run` uses `+` and prints no output because commands
are not executed.

At the end:

- `Setup complete.` on success;
- a dry-run note when nothing was changed;
- a failure count and a pointer to the log when verification fails;
- an inline `Reboot now?` prompt on a successful normal run.

## Exit behavior

- `Ctrl-C`: prints `Cancelled. No changes were made.` and exits with status 130.
- Errors: printed to stderr, logged, and exit with status 1.
- The terminal cursor is restored through a guard even on errors.

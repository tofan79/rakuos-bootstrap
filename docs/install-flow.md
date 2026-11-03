# Install flow

This page describes what happens after the user confirms the summary screen.
The entry point is `install::apply()` in `src/install.rs`.

## 0. Target user

The target user is the account that receives desktop configuration and skel
files:

1. `SUDO_USER`, when set and not `root`;
2. otherwise the first regular user from `getent passwd` (UID 1000-59999, login
   shell, existing home), choosing the smallest UID;
3. otherwise `root`.

The fallback exists so `--first-boot` on a systemd unit does not copy
configuration into `/root`.

## 1. Enable repositories

For every repository listed in the catalog (unless excluded):

| Repository | Command |
|---|---|
| `terra` | `rum config-manager --set-enabled terra` |
| `rpmfusion` | `rum config-manager --set-enabled <stable IDs>` |
| `mindset` | `rum copr enable -y mindset/Mindset-Apps fedora-45-x86_64` |

RPM Fusion IDs come from `/usr/share/rakuos/rpmfusion-default-repos`. Entries
containing `-updates-testing` are filtered out. If the file is missing, the
fallback is `rpmfusion-free rpmfusion-nonfree`, never a glob.

When the selected desktop is KineticWE:

- `rum copr enable -y theblackdon/kwee fedora-45-x86_64` is run;
- `mindset` is excluded from the normal repository loop.

If `rum` is absent, repository setup is skipped with a warning and installation
falls back to `dnf`. KineticWE is the exception: enabling its upstream COPR
requires `rum`, so that path stops with an error.

## 2. Clear the package cache

```bash
rum clean all    # or: dnf clean all
```

This removes stale metadata before the install transaction. The install
commands also pass `--refresh` to refresh metadata again at transaction time.

## 3. Install packages

The package set is the union of:

- `base` from the catalog;
- `packages` of every selected terminal, browser, and file manager;
- `packages` of the selected desktop;
- `packages` of the effective login manager, when one exists.

Duplicate RPM names are removed by using a `BTreeSet`.

Packages listed in a login's `no_recommends` are installed in a separate
transaction with `--no-recommends`. The rest are installed together.

For MiracleWM, `fedora-release-common` is passed as `--exclude=` because
RakuOS's `rakuos-release-common` already provides that capability; the
MiracleWM-specific release package from the desktop group is still installed.

If `rum` is not available, the wizard warns that the RakuOS overlay is inactive
and uses `dnf`.

## 4. Desktop configuration

### Skel

When the desktop has `skel = "<name>"`, the wizard copies
`data/skel/<name>/` into the target home. Placeholders are substituted as
described in [`catalog.md`](catalog.md).

Existing files with identical content are left alone. Files with different
content are renamed to `<name>.bak` (or `.bak.1`, `.bak.2`, …) first, so a
re-run never silently destroys user edits. Finally, `.config` is recursively
chowned to the target user.

### Session file override

When the desktop has `session_file`, the file `data/sessions/<name>` replaces
the package-provided file in `/usr/share/wayland-sessions`. The Hyprland
override sets `UWSM_SILENT_START=2` so uwsm logs do not flash on the VT before
the compositor starts.

On bootc systems `/usr` may be read-only; in that case the write fails with a
warning and the packaged session file remains in use.

### Generated session

When the desktop has `choose_login = true`, no `session_file`, and an
`autologin_cmd`, the wizard checks whether a `.desktop` file already executes
that command. If not, it writes `/usr/share/wayland-sessions/rakuos-<id>.desktop`.

This covers packages that ship a compositor command but no session entry. The
same immutable-root warning applies.

## 5. Login manager

The effective login is the desktop's `login_override` when present, otherwise
the user's pick. Three paths exist:

### Display-manager unit

If the login entry has a `unit`:

1. all other known display managers are disabled;
2. the selected unit is enabled;
3. `graphical.target` becomes the default;
4. an old getty autologin drop-in is removed.

For `sddm.service`:

- `/etc/sddm.conf.d/50-rakuos-theme.conf` is written only when `sddm_theme` is
  set; otherwise a stale theme override is removed;
- `/etc/sddm.conf.d/60-rakuos-sessions.conf` always lists the Wayland and X11
  session directories. It deliberately does not set `DisplayServer`, because
  the greeter package variant owns that drop-in under `/usr/lib/sddm/`.

For greetd-based entries (`greetd_cmd` is set):

- a dedicated greeter user is created when needed and added to `video`,
  `render`, and `input`;
- `/etc/greetd/config.toml` is written (the old file is backed up once as
  `config.toml.bak`);
- `@COMPOSITOR@` is replaced with the desktop's compositor command;
- the package setup script (`greetd_setup`) is run;
- `/var/lib/<greeter>` and `/var/lib/<login-id>` are chowned to the greeter
  user and set to mode 700 so the greeter can write its state.

### Built-in login

When the desktop does not ask for a login and has no `login_override`, the
wizard looks for an already installed display manager (`lightdm`, `sddm`, `gdm`,
`greetd`) and enables the first one found. If none exists, the login setup is
left unchanged with a warning.

### Autologin (`none`)

The code supports a hidden `none` login entry: it configures getty autologin on
tty1 and launches the desktop with `dbus-run-session <autologin_cmd>`. The
current catalog marks this entry `available = false` and `hidden = true`, so it
is not offered.

## 6. State

Two files are written:

| File | Contents |
|---|---|
| `/var/lib/rakuos/bootstrap.toml` | timestamp, target user, selected IDs, repositories |
| `/var/lib/rakuos/bootstrap.done` | empty completion marker checked by the first-boot unit |

## 7. Verification

`verify()` performs read-only checks:

- all requested packages are installed (group entries starting with `@` are
  skipped);
- the selected display-manager unit is enabled, or some display manager is
  enabled when the desktop provides its own;
- the Wayland session is registered (session file exists, or a `.desktop`
  entry runs `autologin_cmd`) for desktops that asked for a login;
- every skel file exists in the target home.

Package checks consult the base RPM database first, then the overlay RPM
database reported by `rum paths` (overlay mode), so packages installed through
`rum` are found.

If any check fails, the wizard prints the failure count, does not reboot, and
points at the log file. A re-run fixes most issues.

## 8. Reboot

- `--first-boot`: reboot automatically after a successful verification.
- Normal run: ask `Reboot now?` (default yes).
- `--dry-run`: stop after printing the plan; no log is written.

## Command wrapper and logging

Every external command is run by `run()`:

- in `--dry-run`, it prints the command with a leading `+` and returns;
- otherwise it prints the command with `→`, streams stdout/stderr live, and
  writes both streams to the log through `log::LineLogger`;
- ANSI escapes are stripped and carriage-return progress lines are collapsed;
- a non-zero exit status aborts the flow with the command name, status, and log
  path in the error message.

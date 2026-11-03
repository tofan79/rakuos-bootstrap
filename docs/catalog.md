# Catalog reference

`data/catalog.toml` is the single source of truth for the choices shown by the
wizard and the packages they install. The same file is embedded in the binary
and installed to `/usr/share/rakuos/bootstrap/catalog.toml`; the installed copy
wins when present.

## Top-level keys

| Key | Type | Meaning |
|---|---|---|
| `base` | array of strings | RPM packages installed for every setup, regardless of choices |
| `repos` | array of strings | Third-party repositories to enable before installing; known values are `terra`, `rpmfusion`, and `mindset` |
| `terminals` | array of choice tables | Multi-select terminal emulators |
| `browsers` | array of choice tables | Multi-select browsers |
| `file_managers` | array of choice tables | Multi-select file managers |
| `desktops` | array of choice tables | Single-select Wayland desktops |
| `logins` | array of choice tables | Single-select login managers; hidden entries are used internally |

```toml
repos = ["terra", "rpmfusion", "mindset"]

base = [
    "dbus-tools",
    "egl-wayland",
    "qt6-qtsvg",
    "qt6-qtwayland",
    "rakuos-software-gtk",
    "rakuos-system-gtk",
    "rakuos-welcome-gtk",
]
```

Choices are sorted alphabetically by `name` at load time. The order in the file
does not control the order on screen, but keeping entries A-Z makes the file
easier to review.

Each phase must have exactly one `default = true` entry. This invariant is
covered by a unit test.

## Choice fields

Every category uses the same table shape. Only `id` and `name` are required.

| Field | Type | Default | Meaning |
|---|---|---|---|
| `id` | string | required | Stable identifier used in the state file, session names, and skel lookups |
| `name` | string | required | Display name |
| `description` | string | `""` | Short description shown in the UI and session comments |
| `packages` | array | `[]` | RPM packages installed for this choice |
| `command` | string | `id` | Binary name substituted into skel placeholders; useful when the package and binary names differ (for example `superfile` → `spf`) |
| `available` | bool | `true` | `false` shows the entry as unavailable and prevents selecting it |
| `default` | bool | `false` | Starts highlighted/checked; exactly one per phase |
| `hidden` | bool | `false` | Loaded by the code but never shown to the user |
| `skel` | string | unset | Directory under `data/skel/` copied into the target home |
| `session_file` | string | unset | File under `data/sessions/` written to `/usr/share/wayland-sessions/` |
| `autologin_cmd` | string | unset | Command used for the session file and for autologin when no display manager is configured |
| `login_override` | string | unset | Login entry that is always used for this desktop, regardless of the user's pick |
| `choose_login` | bool | `false` | Show the login-manager screen after the browser screen |
| `unit` | string | unset | systemd unit enabled for a login manager |
| `no_recommends` | array | `[]` | Packages installed with `--no-recommends` |
| `sddm_theme` | string | unset | SDDM theme written to `/etc/sddm.conf.d/50-rakuos-theme.conf`; empty means "use the SDDM default" and removes an old override |
| `greetd_cmd` | string | unset | `[default_session] command` for `/etc/greetd/config.toml`; `@COMPOSITOR@` is replaced per desktop |
| `greetd_user` | string | `greeter` | User that runs the greetd greeter |
| `greetd_setup` | string | unset | Package-provided setup script run after the greetd config is written |

## Desktops

```toml
[[desktops]]
id = "hyprland"
name = "Hyprland"
description = "Dynamic-tiling compositor with smooth animations; no panel, highly customizable"
packages = ["hyprland", "hyprland-guiutils", "hyprland-uwsm", "xdg-desktop-portal-hyprland", "hyprland-preview-share-picker"]
session_file = "hyprland-uwsm.desktop"
autologin_cmd = "Hyprland"
choose_login = true
```

A desktop with `choose_login = true` asks the user for a login manager. Without
it, the desktop keeps the display manager that ships with its package group, or
the hidden `login_override` entry.

The current catalog asks for a login manager for Hyprland, MangoWM, and Sway.
Budgie and LXQt use the display manager that ships with their desktop group;
KineticWE and Umbriel have `login_override`; MiracleWM uses a hidden SDDM
entry.

## Login managers

```toml
[[logins]]
id = "sddm"
name = "SDDM"
description = "SDDM on Wayland (Miriway greeter, as used by Budgie/LXQt); default theme"
packages = ["sddm-wayland-miriway"]
unit = "sddm.service"
default = true
```

- `unit` is enabled with `systemctl enable`, and all other known display
  managers are disabled first.
- `sddm.service` additionally writes session-directory drop-ins.
- `greetd_cmd` switches the login manager to the greetd path and writes
  `/etc/greetd/config.toml`.
- A hidden login entry can be referenced through a desktop's `login_override`.
- The catalog currently hides the `none` autologin option; the code path is
  available but not offered by default.

## Repositories

| Value | Action |
|---|---|
| `terra` | `rum config-manager --set-enabled terra` |
| `rpmfusion` | Enable the IDs listed in `/usr/share/rakuos/rpmfusion-default-repos`, skipping `-updates-testing`; fall back to `rpmfusion-free` and `rpmfusion-nonfree` |
| `mindset` | `rum copr enable -y mindset/Mindset-Apps fedora-45-x86_64` |

KineticWE gets special handling: `theblackdon/kwee` is enabled and `mindset` is
excluded from the normal repository list for that desktop.

`updates-testing` repositories are never enabled by the wizard. The RPM Fusion
ID list is filtered, and the fallback names the two stable repositories
explicitly.

## Skel substitution

Files under `data/skel/<name>/` are copied into the target home. Text files get
these placeholders replaced before writing:

| Placeholder | Value |
|---|---|
| `@TERMINAL@` | `command()` of the first selected terminal after A-Z sorting |
| `@BROWSER@` | `command()` of the first selected browser after A-Z sorting |
| `@FILEMANAGER@` | `command()` of the first selected file manager after A-Z sorting |

Fallbacks are `foot`, `firefox`, and `nautilus` when a category is empty.

If the destination file exists with different content, it is renamed to
`<name>.bak` (or `.bak.1`, `.bak.2`, …) before the new file is written. Files
whose content already matches are left untouched.

## Adding a choice

1. Add a `[[<category>]]` table with a unique `id`, a `name`, and the packages.
2. Keep exactly one `default = true` per category.
3. If a desktop needs a bundled config, add `data/skel/<name>/` and set
   `skel = "<name>"`.
4. Run the test suite; catalog invariants (defaults, skel directories) are
   checked automatically.

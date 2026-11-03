# Troubleshooting

The wizard writes a detailed log to `/var/log/rakuos-bootstrap.log` (override
with `--log`). Every external command, its output, warnings, and verification
results are recorded there. Check the log first.

## The wizard refuses to start

### `An interactive terminal (TTY) is required`

The UI needs a real terminal. Run it directly in a terminal, not through a pipe
or a service without a TTY. For the first-boot unit, `data/rakuos-bootstrap.service`
attaches the service to `/dev/tty1`.

### `Run with sudo: sudo rakuos-bootstrap`

A real installation requires root. `--dry-run` does not.

### `unknown argument: ...`

Check `rakuos-bootstrap --help`. Options that take a value need one
(`--catalog`, `--base-dir`, `--log`).

## Repository problems

### `rum was not found; skipping third-party repository setup`

The wizard continues with `dnf`. Packages that only exist in RakuOS/COPR
repositories will not be available. Reinstall `rum` or use a RakuOS image that
ships it.

### `unknown third-party repository: ...`

The catalog contains a `repos` value the code does not know. Valid values are
`terra`, `rpmfusion`, and `mindset`.

### KineticWE fails to resolve packages

KineticWE uses the upstream `theblackdon/kwee` COPR and excludes
`mindset/Mindset-Apps` from its repository setup. If `rum` is missing, the
KineticWE path fails because enabling the COPR requires `rum`.

### RPM Fusion updates-testing should not be enabled

The wizard filters `-updates-testing` from
`/usr/share/rakuos/rpmfusion-default-repos` and uses explicit stable fallback
names. If testing repositories are enabled, they were enabled by something
else; check `rum repolist --all`.

## Installation problems

### Package verification says packages are missing

On RakuOS overlay/split mode, `rum install` writes to the overlay RPM database,
not the base image database. The verifier checks `rum paths` for the overlay
database and queries it with `rpm --dbpath`. If verification still fails:

```bash
rum paths
rpm -q <package>
rpm -q --dbpath /var/lib/rakuos/rum-rpmdb <package>
```

### `fedora-release-common` conflict on MiracleWM

The MiracleWM path excludes `fedora-release-common` because RakuOS provides
`rakuos-release-common`. If the transaction still conflicts, check whether
another selected package pulls in the Fedora release package explicitly.

### SDDM/greeter package conflicts

Variants such as `sddm-wayland-miriway`, `sddm-wayland-miraclewm`,
`noctalia-greeter`, and `kineticwe-greeter` conflict with each other. The wizard
removes the known conflicting variant before installing the new one. If a
manual install left both installed, remove the stale variant and re-run.

### DMS Greeter cannot start

DMS Greeter needs a Quickshell runtime. The wizard installs `quickshell-git` when
neither `quickshell` nor `quickshell-git` is present. Do not install both; they
conflict.

## Session and login problems

### Wayland session is not listed

- Check `/usr/share/wayland-sessions/*.desktop`.
- For a desktop with `choose_login = true`, the wizard either installs a
  `session_file` override or generates `rakuos-<id>.desktop` from
  `autologin_cmd`.
- On bootc systems, `/usr` may be read-only. The log then contains a warning
  that the session file could not be written; the package-provided session is
  still used.

### greetd greeter cannot write its state

The log may show `Permission denied` for files under `/var/lib/<greeter>` or
`/var/lib/<login-id>`. The wizard fixes ownership and mode 700 after running
the package setup script. If it still fails, check:

```bash
ls -ld /var/lib/greeter /var/lib/<login-id>
```

### Login loop after choosing a different login manager

The wizard disables other known display managers before enabling the selected
one. If two are still enabled, disable the stale unit manually:

```bash
systemctl disable --now <stale-unit>
```

## UI problems

### Garbled box characters

Use a UTF-8 locale and a UTF-8 terminal, or run with `--ascii`. On the Linux
virtual console, `TERM=linux` selects the TTY glyph set automatically.

### No colors

Colors are disabled when output is not a TTY or `NO_COLOR` is set. On a basic
terminal, the wizard uses 16 ANSI colors.

### Terminal is too small

The UI clamps to at least 40 columns and 12 rows. Very small terminals may
truncate descriptions; resize the terminal or use the details pane on a wider
terminal (68 columns or more).

## Log and state

| Path | Purpose |
|---|---|
| `/var/log/rakuos-bootstrap.log` | Full command/output log |
| `/var/lib/rakuos/bootstrap.toml` | Last selections and target user |
| `/var/lib/rakuos/bootstrap.done` | Completion marker; remove to re-run first-boot setup |

`--log <path>` changes the log location. Log setup failure is non-fatal; the
wizard warns and continues.

## Getting help

When reporting a problem, include:

- the RakuOS version (`cat /etc/os-release`);
- `rum --version` or `dnf --version`;
- the wizard version (`rakuos-bootstrap --version`);
- the relevant part of `/var/log/rakuos-bootstrap.log`;
- whether the system uses the RakuOS overlay and, if so, `rum paths` output.

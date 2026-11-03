# Building and testing

## Requirements

- Rust stable (edition 2024)
- `cargo`, `rustfmt`, and `clippy`
- `gcc` / linker toolchain
- Optional: `xorriso` for building the VM payload ISO

All dependencies are vendored under `vendor/`; the repository is configured in
`.cargo/config.toml` to use vendored sources. No network access is needed for a
normal build.

## Offline commands

```bash
cargo fmt --check
cargo clippy --all-targets --offline
cargo test --offline
cargo build --release --offline
```

The release profile uses `strip = true`, LTO, and `opt-level = "s"` to keep the
binary small enough for a minimal image.

To update dependencies, replace the contents of `vendor/` and
`.cargo/config.toml` together. Do not mix a vendored build with crates.io
sources.

## Tests

Tests live in `src/testing/` and run as a unit-test target of the binary.

Current coverage includes:

- catalog invariants: exactly one default per phase, known desktop/login
  relationships, package membership, skel directories exist and are non-empty;
- TOML validity of bundled skel files;
- SDDM configuration bodies;
- overlay RPM database path parsing;
- target-user selection;
- skel copying, placeholder substitution, and backup suffixes;
- RPM fingerprint parsing helpers;
- catalog/package helpers.

Run a single test with:

```bash
cargo test --offline user_target
```

## Lint expectations

- `cargo fmt --check` must pass.
- `cargo clippy --all-targets --offline` must produce no warnings.
- Warnings from the deprecated system `gold` linker are environmental and do
  not come from the code; ignore them.

## Running the wizard without installing

```bash
cargo build
./target/debug/rakuos-bootstrap --dry-run
```

`--dry-run` still needs an interactive terminal. It prints every command,
skel copy, session file, and state write it would perform, but runs no external
commands and writes nothing.

Useful dry-run variants:

```bash
# Force ASCII rendering for a simple terminal
./target/debug/rakuos-bootstrap --dry-run --ascii

# Use a different catalog or data directory
./target/debug/rakuos-bootstrap --dry-run \
  --catalog /path/to/catalog.toml \
  --base-dir /path/to/data
```

## Manual PTY test

The UI requires a TTY, so plain shell redirection is not enough. A quick check
with Python:

```python
import os, pty

os.execvpe(
    "./target/debug/rakuos-bootstrap",
    ["./target/debug/rakuos-bootstrap", "--dry-run", "--base-dir", "data"],
    dict(os.environ, TERM="xterm-256color", LANG="en_US.UTF-8"),
)
```

Run it under `script` or a terminal multiplexer if you want to interact with it.

## VM payload ISO

`scripts/buat-payload-uji.sh` builds a small ISO containing the release binary,
`data/`, the systemd unit, and a launcher script:

```bash
./scripts/buat-payload-uji.sh
# output/rakuos-bootstrap-payload.iso
# output/rakuos-bootstrap-payload.iso.sha256
```

Inside a test VM:

```bash
sudo mount -o ro /dev/sr1 /mnt
/mnt/jalankan.sh --dry-run
/mnt/jalankan.sh
```

This avoids installing build tools or network access in the VM. The payload's
launcher passes `--base-dir` to the mounted ISO so the catalog and skel files
are read from the payload.

## CI

`.github/workflows/ci.yml` runs on every push and pull request:

1. `cargo fmt --check`
2. `cargo clippy --all-targets --offline`
3. `cargo test --offline`
4. `cargo build --release --offline`

CI uses the vendored dependencies, matching local offline builds.

## Adding tests

- Pure helpers should be `pub(crate)` and tested without touching the host.
- Catalog invariants belong in `src/testing/catalog.rs`.
- Filesystem tests must operate in a temporary directory and clean up.
- Do not require root, `rum`, or an installed desktop for unit tests.

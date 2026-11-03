# Contributing

Thanks for improving `rakuos-bootstrap`. This is a small Rust TUI, so changes
should stay focused and easy to review.

## Before you start

1. Read [`docs/README.md`](docs/README.md) for the architecture and document
   map.
2. Check existing issues and pull requests to avoid duplicate work.
3. For behavior changes, describe the user problem first. Small fixes can go
   straight to a pull request.

## Development setup

```bash
git clone https://github.com/tofan79/rakuos-bootstrap
cd rakuos-bootstrap
cargo build --offline
```

Dependencies are vendored in `vendor/`; no network access is required. See
[`docs/building-and-testing.md`](docs/building-and-testing.md) for the full
toolchain and testing instructions.

## Checks before submitting

```bash
cargo fmt --check
cargo clippy --all-targets --offline
cargo test --offline
cargo build --release --offline
```

All four must pass. Clippy must produce no warnings. A warning about the
deprecated system `gold` linker comes from the environment, not the code.

If you changed the UI, also run a dry-run in a real terminal:

```bash
cargo run -- --dry-run
```

## Code style

- Follow the existing module layout and naming.
- Keep functions small and testable; pure helpers belong in `src/testing/`.
- Do not add runtime dependencies without a strong reason. The binary is meant
  to stay small and dependency-light.
- Preserve the catalog-driven design: adding a choice should normally require
  only `data/catalog.toml` and, for a bundled config, `data/skel/`.
- Comments follow the surrounding file. Most existing comments are Indonesian;
  English is acceptable in new code when it improves clarity.
- Do not commit generated files from `target/` or `output/`.

## Catalog changes

- Keep exactly one `default = true` per category.
- Keep `data/catalog.toml` sorted A-Z by display name.
- If a desktop references `skel = "<name>"`, the directory must exist and be
  non-empty; the test suite checks this.
- Prefer adding a new `[[category]]` entry over hard-coding package names in
  Rust.
- Package names must exist in Fedora 45 or the RakuOS repositories.

## Documentation

- `README.md` and the root `ui.md` are end-user documentation in Indonesian.
- Everything under `docs/` and this file are written in English.
- Update the relevant document when behavior changes. In particular:
  - `docs/catalog.md` for catalog fields and repository behavior;
  - `docs/install-flow.md` for install steps;
  - `docs/ui.md` for UI behavior;
  - `docs/troubleshooting.md` for known failure modes.

## Commit messages

- Use the imperative mood: `Fix greeting timeout`, not `Fixed`.
- Keep the subject under ~72 characters.
- Explain the user-visible reason in the body when it is not obvious.
- Do not mix unrelated changes in one commit.

## Pull requests

Include:

- what changed and why;
- how it was tested;
- any user-visible behavior change;
- screenshots or captured terminal output for UI changes when useful.

Keep pull requests small. Large refactors should be discussed in an issue
first.

## Reporting bugs

Open an issue with:

- RakuOS version (`cat /etc/os-release`);
- wizard version (`rakuos-bootstrap --version`);
- package manager and `rum paths` output when relevant;
- the relevant part of `/var/log/rakuos-bootstrap.log`;
- steps to reproduce.

Never paste secrets, passwords, or private user data.

## License

By contributing, you agree that your contribution is licensed under the MIT
License used by this project. See [`LICENSE`](LICENSE) and
[`docs/licensing.md`](docs/licensing.md).

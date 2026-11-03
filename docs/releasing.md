# Releasing

Releases are published on GitHub and picked up by the COPR automation in the
`tofan79/Mindset-Apps` repository. A release is not complete until the COPR
build for the new version has succeeded.

## Versioning

- Versions use `MAJOR.MINOR.PATCH`.
- Patch releases: bug fixes and small UI/documentation changes.
- Minor releases: new choices, new install behavior, or noticeable UI changes.
- The version in `Cargo.toml` must match the Git tag and the GitHub release.

## Release checklist

1. Make sure the working tree is clean and tests pass.

   ```bash
   cargo fmt --check
   cargo clippy --all-targets --offline
   cargo test --offline
   cargo build --release --offline
   ```

2. Bump the version in `Cargo.toml` and rebuild so `Cargo.lock` follows.

   ```bash
   sed -i 's/^version = "OLD"/version = "NEW"/' Cargo.toml
   cargo build --release --offline
   ```

3. Update the changelog/release notes if the release contains user-visible
   changes.

4. Commit and push `main`.

   ```bash
   git add -A
   git commit -m "Release vNEW"
   git push origin main
   ```

5. Create and push an annotated tag.

   ```bash
   git tag -a vNEW -m "vNEW"
   git push origin vNEW
   ```

   Before pushing, confirm the tag does not already exist remotely:

   ```bash
   git rev-parse vNEW^{commit}
   git ls-remote origin refs/tags/vNEW
   ```

6. Create the GitHub release.

   ```bash
   gh release create vNEW --title "vNEW" --notes "Release notes..."
   ```

   The COPR workflow reads the latest GitHub release, not the raw tag.

7. Trigger the COPR build.

   The workflow lives in `tofan79/Mindset-Apps`:

   ```bash
   gh workflow run rakuos-bootstrap.yml -R tofan79/Mindset-Apps
   gh run list -R tofan79/Mindset-Apps \
     --workflow "rakuos-bootstrap - COPR Auto Build" -L 2
   ```

   The workflow also runs daily and on changes to
   `specs/rakuos-bootstrap.spec`.

8. Verify the build.

   ```bash
   curl -s "https://copr.fedorainfracloud.org/api_3/build/list?ownername=mindset&projectname=Mindset-Apps&packagename=rakuos-bootstrap&limit=1"
   ```

   Confirm the newest build is `succeeded` and its version matches the release.

## What the COPR workflow does

1. Reads the latest GitHub release tag.
2. Reads the latest successful COPR build version.
3. Skips the build when the versions match, unless `force` is set or the spec
   changed.
4. Downloads the release source archive.
5. Builds an SRPM using `specs/rakuos-bootstrap.spec`.
6. Submits it to the `Mindset-Apps` COPR project.
7. Polls the build until it finishes.

The spec substitutes `%{pkg_version}` from the upstream release, so the spec
itself does not need a version bump for every release.

## Installing the result

COPR metadata can lag right after a build. If `dnf`/`rum` still shows the old
version, use the direct RPM URL:

```text
https://download.copr.fedorainfracloud.org/results/mindset/Mindset-Apps/fedora-45-x86_64/<build-id>-rakuos-bootstrap/rakuos-bootstrap-<version>-1.fc45.x86_64.rpm
```

## Release notes

Keep notes short and user-facing:

- what changed;
- any behavior that requires a re-run of the wizard;
- known issues that are not fixed in the release.

Internal refactors belong in the commit history, not in the release notes.

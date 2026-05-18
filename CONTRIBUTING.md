# Contributing to Ankify

Thanks for your interest in Ankify! It is alpha software, so bug reports,
feedback, and pull requests are all welcome.

## Building and testing

The repository is a Cargo workspace (the `ankify` CLI and library) plus a Typst
package (`packages/ankify-typst/`).

```sh
cargo build --workspace
cargo test --workspace      # the integration tests shell out to `typst`
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

You need the [Typst](https://typst.app) CLI on your `PATH` to run the tests.

## Commit messages

Commits follow [Conventional Commits](https://www.conventionalcommits.org)
(`feat:`, `fix:`, `docs:`, `chore:`, …). The crate's version bump and changelog
are derived from them automatically.

## Releasing

Two artefacts are published from this repository:

- **The `ankify` crate** → crates.io. Releases are automated: `release-plz`
  opens a release pull request; merging it publishes the crate and tags a
  GitHub release.
- **The `ankify` Typst package** → Typst Universe. This is a manual step: open
  a pull request against [`typst/packages`](https://github.com/typst/packages)
  adding `packages/preview/ankify/<version>/` with the package's contents.

Keep the crate version (`packages/ankify-cli/Cargo.toml`) and the Typst package
version (`packages/ankify-typst/typst.toml`) in step.

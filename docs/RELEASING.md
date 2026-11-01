# Releasing

Publishing to crates.io happens from CI: pushing a tag that matches the workspace version runs
`.github/workflows/release.yml`.

## Steps

1. Bump `version` in the workspace `Cargo.toml` (all crates inherit it), in `editors/vscode/package.json`
   and in `npm/package.json`, and update `CHANGELOG.md`.
2. Commit on `main` and let the regular CI go green.
3. Tag and push:

   ```sh
   git tag v0.1.0 && git push origin v0.1.0
   ```

The workflow then checks that the tag equals `v<workspace version>` and that the tagged commit is on
`main`, runs `cargo test --workspace`, and publishes in dependency order: `plantuml-core`, then
`plantuml-cli` once the registry index resolves core. A crate whose version is already published is
skipped, so re-running a failed release is safe.

It also builds `plantuml-cli` on Linux x64, macOS x64/arm64 and Windows x64, and creates the GitHub
release with those archives, a `SHA256SUMS` file and the changelog section for the version.

## What gets published

| crate | published | note |
|---|---|---|
| `plantuml-core` | yes | the library |
| `plantuml-cli` | yes | the `plantuml-cli` binary |
| `plantuml-wasm` | no | `publish = false`; it ships as the npm package `plantuml-rs` |

## npm package

The `npm` job of the release workflow builds both wasm targets with `scripts/build-npm.sh` and runs `npm publish`,
after the crates are published. It uses npm **trusted publishing**: the job carries `id-token: write` and no
`NPM_TOKEN` secret exists. A version that is already on npm is skipped, so re-running a tag is safe. The package
version follows the workspace version; the job checks that the tag equals `npm/package.json`.

`scripts/build-npm.sh -p` builds and packs the tarball locally, to inspect or smoke-test it.

## One-time setup

No secret is needed: publishing uses crates.io **trusted publishing**. On crates.io, under each crate's
settings, add a trusted publisher of type GitHub with

| field | value |
|---|---|
| repository | `bjornharrtell/plantuml-rs` |
| workflow | `release.yml` |
| environment | only if you also set `environment:` on the job, and then both must match |

Do this for `plantuml-core` and for `plantuml-cli`. The workflow then exchanges its GitHub OIDC identity
for a short-lived token (`rust-lang/crates-io-auth-action`, pinned to `v1.0.6`), which is why the job
carries `id-token: write` and no long-lived `CARGO_REGISTRY_TOKEN` secret exists. Trusted publishing can
only be configured for a crate that already exists, which is why `0.1.0` was published by hand.

Trusted publishing covers CI only: publishing from a workstation still needs `cargo login` with a
personal token. The skip-if-already-published check in the workflow makes it safe to re-run a tag.

npm needs the same once, on npmjs.com: package `plantuml-rs` → Settings → Trusted Publisher → GitHub Actions, with
user `bjornharrtell`, repository `plantuml-rs`, workflow `release.yml` and no environment. The package has to exist
first (it does); until the trusted publisher is set up, the `npm` job fails and the rest of the release is unaffected.

## Notes

- crates.io does not allow deleting a published version, only yanking, and never reuses a version: bump first, then
  tag. Everything in the `abel`, `klimt`, `svek`, ... modules is public API.
- `cargo publish --dry-run -p plantuml-core` reproduces the packaging step locally; the CLI cannot be dry-run before
  core is published.

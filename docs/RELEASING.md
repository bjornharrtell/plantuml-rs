# Releasing

Publishing to crates.io happens from CI: pushing a tag that matches the workspace version runs
`.github/workflows/release.yml`.

## Steps

1. Bump `version` in the workspace `Cargo.toml` (all crates inherit it) and update `CHANGELOG.md`.
2. Commit on `main` and let the regular CI go green.
3. Tag and push:

   ```sh
   git tag v0.1.0 && git push origin v0.1.0
   ```

The workflow then checks that the tag equals `v<workspace version>` and that the tagged commit is on
`main`, runs `cargo test --workspace`, and publishes in dependency order: `plantuml-core`, then
`plantuml-cli` once the registry index resolves core. A crate whose version is already published is
skipped, so re-running a failed release is safe.

## What gets published

| crate | published | note |
|---|---|---|
| `plantuml-core` | yes | the library |
| `plantuml-cli` | yes | the `plantuml-cli` binary |
| `plantuml-wasm` | no | `publish = false`; it ships as the npm binding, see `docs/PLAN.md` |

## One-time setup

Add a crates.io API token as the repository secret `CARGO_REGISTRY_TOKEN` (Settings → Secrets and
variables → Actions). Without it the publish steps fail with `no token found`.

## Notes

- `0.1.0` of `plantuml-core` and `plantuml-cli` was published by hand before this workflow existed, and
  crates.io never reuses a version: bump the version first, then tag. The first automated release will
  therefore be the next version, and the `v0.1.0` tag (if pushed) only records history and publishes
  nothing, because both crates are skipped as already published.

- crates.io does not allow deleting a published version, only yanking. Check the public API surface
  (everything in the `abel`, `klimt`, `svek`, … modules is public) before the first tag.
- `cargo publish --dry-run -p plantuml-core` reproduces the packaging step locally; the CLI cannot be
dry-run before core is published.
- The workflow does not create a GitHub release. If that is wanted, add a step using
  `gh release create "$GITHUB_REF_NAME" --notes-file CHANGELOG.md` with `contents: write`.

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

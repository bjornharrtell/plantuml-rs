# Agent instructions

- Be terse at all times
- Do not open pull requests. Commit and push to `main` directly when you work in the repository checkout. In an isolated worktree or other standalone checkout, `main` is not yours to commit to: commit a local branch there and land it into `main` from the main checkout. Do not leave work only on a branch when you have the main checkout.
- Follow [Conventional Commits 1.0.0](https://www.conventionalcommits.org/en/v1.0.0/) for every commit message: use `type(optional-scope)!: description`, for example `fix(ui): keep event polling single-flight`, with a type such as `feat`, `fix`, `refactor`, `perf`, `test`, `docs`, `build`, `ci`, or `chore`. Mark breaking changes with `!` or a `BREAKING CHANGE:` footer and explain them in the body.
- Do not create merge commits. Keep history linear by rebasing your work onto the latest `main` and landing it with a fast-forward push, never with `git merge` or a merge commit. Prefer `git pull --rebase` and plain `git push`; use `git push --force-with-lease` only when rewriting your own unpublished commits.
- Target the 80% patch coverage threshold
- For Rust logging, use the existing `log` facade (`log::debug!`, `log::info!`, `log::warn!`, or `log::error!`) so output follows the configured native and wasm logger. Do not add direct `println!` or `eprintln!` logging.

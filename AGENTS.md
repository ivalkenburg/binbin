# Repository instructions

For build and test requirements, read [CONTRIBUTING.md](CONTRIBUTING.md). Before changing module boundaries, read [docs/architecture.md](docs/architecture.md).

## Commit messages

- Use semantic commit messages: `<type>(<optional scope>): <summary>`, such as `fix(web): preserve download retries` or `chore: initial commit`.
- Use an imperative summary. Choose a conventional type such as `feat`, `fix`, `refactor`, `perf`, `docs`, `test`, `build`, `ci`, or `chore`.
- Commit messages must contain no emoji and no coauthor attribution or `Co-authored-by` trailers.

## Releases

- Use numeric `MAJOR.MINOR.FIX` versions, such as `1.2.3`. Increment MAJOR for breaking changes, MINOR for compatible features, and FIX for compatible bug fixes. Reset lower components when incrementing a higher one.
- Keep `workspace.package.version` in `Cargo.toml` and the Git tag `vMAJOR.MINOR.FIX` consistent.
- Set the GitHub release title to the bare version, such as `1.2.3`.
- Write the release description as concise bullet points describing changes since the previous released version. End with `[Compare changes](https://github.com/OWNER/REPOSITORY/compare/vPREVIOUS...vCURRENT)`, using the actual repository and version tags.
- For the first release, describe the initial functionality and use the repository's initial commit as the comparison base because no previous version exists.
- Follow [docs/releases.md](docs/releases.md) when preparing a release or publishing images.

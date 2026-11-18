# Releasing Serenade

Serenade publishes every `serenade-*` crate and the `serenade-framework` facade to crates.io
at one shared version, and attaches prebuilt `serenade-cli` binaries to the GitHub Release.
Both are driven by [`release.yml`](../.github/workflows/release.yml) from an immutable
`v<version>` tag. The rolling [`release-cli-dev.yml`](../.github/workflows/release-cli-dev.yml)
pre-release (`serenade-cli-dev`) is separate and unaffected.

The repository documentation entry point is [docs/README.md](README.md), and the release
changelog is [docs/CHANGELOG.md](CHANGELOG.md). Do not add either file at the repository
root; package-level READMEs remain in their crate directories.

## Pieces

| Piece | Role |
|-------|------|
| [`make/publish/publish-crates.sh`](../make/publish/publish-crates.sh) | Dependency-ordered, resumable crates.io publisher |
| [`make/publish/publish.mk`](../make/publish/publish.mk) | `make publish-check`, `make publish-test` |
| [`make/release/package-cli.sh`](../make/release/package-cli.sh) | Per-target CLI archives and the verified Trunk bundle |
| [`.github/workflows/release.yml`](../.github/workflows/release.yml) | Tag validation, full CI, publication, binaries |

The publisher modes:

| Command | Effect |
|---------|--------|
| `publish-crates.sh check [cargo package args]` | `cargo package --locked` for every crate in publish order; never uploads |
| `publish-crates.sh versions v1.2.3` | Tag equals `v<workspace version>`, and every version source agrees |
| `publish-crates.sh publish` | Uploads versions not yet on crates.io; needs `CARGO_REGISTRY_TOKEN` |

Every mode first checks the publish list against `cargo metadata`. Every publishable member must
be listed exactly once, carry the workspace version, and come after each workspace crate it
depends on. Adding a crate therefore means adding it to `DEFAULT_CRATES` in the right layer.

`versions` also checks the nested example workspaces, the `serenade new` template
(`crates/serenade-cli/templates/app/Cargo.toml.tmpl`), the shipped recipes, and a
`## [<version>]` section in [CHANGELOG.md](CHANGELOG.md).

## One-time setup

1. **crates.io token.** Create an API token at <https://crates.io/settings/tokens> with only the
   `publish-new` and `publish-update` scopes. Once the crates exist, you can narrow it with the
   crate pattern `serenade-*`, which also covers `serenade-framework`. Give it an expiry and
   rotate it.
2. **GitHub environment.** Create the `crates-io` environment and add the token as the
   environment secret `CARGO_REGISTRY_TOKEN`. Do not add it as a repository secret. Restrict
   deployments to tags matching `v*` and add required reviewers. Only the `publish-crates` job
   references this environment, and the token is exposed only to its publish step. The `ci`
   job's `secrets: inherit` passes repository secrets, never environment secrets.
3. **First-release rate limit.** crates.io limits how quickly new crates can be created, and the
   first release creates 42. The publisher waits out every HTTP 429 automatically, for at most
   5 hours of waiting per run. To publish faster, email <help@crates.io> beforehand to ask for a
   temporary increase of the new-crate limit for the `serenade-*` crates.

## Cutting a release

1. Bump `[workspace.package] version` in the root `Cargo.toml` and the internal
   `version = "…"` requirements in `[workspace.dependencies]`. Also bump the version sources
   checked by `versions`, and move the `[Unreleased]` notes into a new
   `## [<version>]` section of [CHANGELOG.md](CHANGELOG.md).
2. Verify locally:

   ```sh
   make ci
   make msrv
   make framework-features
   make publish-test
   make publish-check                  # add PUBLISH_CHECK_ARGS=--allow-dirty before committing
   make/publish/publish-crates.sh versions v<version>
   ```

3. Merge, tag the release commit `v<version>`, and push the tag.
4. Create a GitHub Release for the tag, as a draft if you like, and **publish** it as a full
   release, not a prerelease. Publishing the release (directly or from a draft) triggers
   `release.yml`. Creating a draft alone does not.

`release.yml` then:

1. `prepare` checks that the run is for a tag and that the tag still points to the commit the
   run started on. It runs `publish-crates.sh versions <tag>` and requires a published GitHub
   Release that is neither a draft nor a prerelease.
2. `ci` runs the complete [`ci.yml`](../.github/workflows/ci.yml) on that commit. This includes
   the MSRV check, the facade feature matrix, the publisher tests and `make publish-check`.
3. `publish-crates` waits for environment approval. It is serialized by the `crates-io-publish`
   concurrency group and runs `publish-crates.sh publish`.
4. `cli-binaries` builds the CLI for each target from the same commit, packages it and attests
   its build provenance.
5. After both `publish-crates` and all `cli-binaries` jobs succeed, `release-assets` writes
   `SHA256SUMS` and uploads everything to the release with `--clobber`.

Every checkout in the release and reusable CI workflows uses the run's `github.sha`, so
they use the same validated commit even if the tag moves after validation.

Publication and binaries start only after CI succeeds. They run independently of each other.

## How the publisher behaves

For each crate, in dependency order:

1. It asks the crates.io API whether `<crate> <version>` exists. If it does, the upload is
   skipped.
2. Otherwise it runs `cargo publish -p <crate> --locked`. It never passes `--no-verify`.
3. It waits until the version is visible in the sparse index (`index.crates.io`) before moving
   on, so dependents always resolve. Skipped crates are awaited too.

HTTP 429 from either the upload or the API lookup:

- **Waiting.** It waits until the reset time reported by crates.io (the `Retry-After` header or
  the "try again after …" message), plus `RATE_LIMIT_SAFETY_MARGIN`. If no reset time is
  given, it waits `RATE_LIMIT_FALLBACK_DELAY` instead.
- **Recheck before retrying.** It then looks the version up again. An upload whose response
  was lost is therefore never repeated.
- **Limits.** It stops after `PUBLISH_MAX_ATTEMPTS` attempts for one crate, or once the total
  wait would exceed `RATE_LIMIT_MAX_TOTAL_WAIT`.

Any other failure stops the run immediately, unless the lookup shows the version actually
landed.

| Variable | Default | Meaning |
|----------|---------|---------|
| `PUBLISH_MAX_ATTEMPTS` | 6 | Upload/lookup attempts per crate |
| `RATE_LIMIT_FALLBACK_DELAY` | 120 | Seconds to wait when a 429 has no reset time |
| `RATE_LIMIT_SAFETY_MARGIN` | 5 | Seconds added to every reported reset time |
| `RATE_LIMIT_MAX_TOTAL_WAIT` | 18000 | Total rate-limit wait per run, in seconds (5 h) |
| `INDEX_ATTEMPTS` / `INDEX_INTERVAL` | 60 / 10 | Sparse-index polling |

Invalid values (not a non-negative integer, or zero where at least one is required) are
rejected before anything runs. `make publish-test` exercises all of this against mocked
`cargo`, `curl` and `sleep`.

The `CARGO`, `CURL`, and `SLEEP` overrides accept whitespace-separated command prefixes,
such as `CARGO="cargo +stable"`. Prefixes are split into arrays without interpreting shell
quoting or expanding wildcards.

## Resuming a failed release

Never move or recreate the tag, and never publish individual crates by hand.

1. Fix the cause outside the tagged code, for example environment approval, an expired token
   or a crates.io outage.
2. Re-run the failed jobs of the `release` run. Alternatively, open **Actions → release → Run
   workflow**, choose the `v<version>` **tag** under "Use workflow from", and run it.
3. The publisher skips everything already on crates.io and continues with the first missing
   crate. CLI assets are rebuilt from the same commit and re-uploaded over the old ones.

If the failure is in the tagged code itself, the version cannot be fixed in place. Crates that
already went out stay published. Release the fix as the next patch version.

## Release assets

| Asset | Contents |
|-------|----------|
| `serenade-cli-v<version>-<target>.tar.gz` (`.zip` on Windows) | `serenade`, `serenade-spa`, `serenade-html`, `serenade-universal`, `serenade-ssg`, `serenade-trunk` |
| `serenade-trunk-v<version>-<target>.tar.gz` (`.zip` on Windows) | Optional: the verified fork Trunk in serenade-cli's install layout |
| `*.sha256`, `SHA256SUMS` | SHA-256 checksums |

Targets: `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`, `aarch64-apple-darwin`,
`x86_64-apple-darwin`, and `x86_64-pc-windows-msvc`. The Trunk fork has no Linux ARM64 build, so
there is no Trunk bundle for that target. Install Trunk yourself there.

Each archive has a GitHub build provenance attestation:

```sh
gh attestation verify serenade-cli-v1.0.0-x86_64-unknown-linux-musl.tar.gz --repo Interchouette-ITC/Serenade
```

The Trunk bundle is not rebuilt. The fork's `trunk-itc-dev` archive is checked against its
SHA-256 and its attested provenance, and the binary must report the attested commit. These are
the same checks `serenade-cli` performs before first use. Extract the bundle into
`$SERENADE_HOME/tools` (default `~/.serenade/tools`) to skip that download.

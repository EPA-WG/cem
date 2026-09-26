# Releasing cem-url

`cem-url` is a separately versioned CEM-maintained fork. Its initial release
candidate is 0.1.0, based on upstream `url` 2.5.8. Upstream versions and revisions
belong in `package.metadata.cem-upstream` and `CEM-PATCH.md`; they do not control
the fork version. It is excluded from CEM's synchronized platform version group.
Its Rust library remains `url`, preserving existing imports and features.

Publication is deferred into release work by user decision on 2026-09-25.
Run this checklist when the planned release includes dependent CEM-QL Cargo
publication; it does not block ordinary development. The owning release contract
is [the CEM Cargo release prerequisite](https://github.com/EPA-WG/cem/blob/develop/docs/cem-ml-deployment-contract.md#cargo-source-release-prerequisite).

## Local preparation

From the CEM repository root:

```sh
yarn nx run cem_ql:verify:url-package
yarn nx run cem_ql:test:url-integration --skipNxCache
```

The first target runs Cargo packaging with build verification, checks the
archive against the reviewed source and provenance files, then runs the
packaged unit/WPT tests with all features and checks the no-default-features
build. It uses locked, offline dependencies;
if dependencies are not cached, fetch them first with
`cargo fetch --locked --manifest-path vendor/url/Cargo.toml`.
It writes the `.crate` and `cem-url-review.json` (archive SHA-256 and Git state)
under `dist/target/cem_url_package/package`. Dirty working trees are allowed for
review artifacts and recorded in the report. No registry upload occurs.

Preserve the upstream dual licenses, WPT license, original README/manifest,
patch and fixed-case records in every archive. Cargo replaces `Cargo.toml.orig`
with the pre-normalization fork manifest, so `UPSTREAM-Cargo.toml` is the retained
upstream original. Runtime sources remain identical to the reviewed patch.
Review the complete patch and pinned fixture evidence on each upstream update.

## Publication order and prerequisites

1. Confirm the `cem-url` name is available or owned by the intended CEM
   maintainers, and establish publishing access. The preparation session's
   registry lookup returned HTTP 403; it did not confirm name availability.
2. Obtain explicit authorization to publish the reviewed release. Commit the
   release version and CEM-QL's matching exact dependency; regenerate both lock
   files without unrelated dependency updates. Use a clean checkout and rerun
   the local gates above. Inspect the archive and its report.
3. Run a clean `cargo publish --dry-run --locked --manifest-path
   vendor/url/Cargo.toml`. After that succeeds, publish with the same command
   without `--dry-run`. Do not use `--allow-dirty` for release publication.
4. Verify `cem-url = "=0.1.0"` resolves from crates.io in a clean consumer
   outside this workspace, with no path override. Confirm the published source
   matches the reviewed runtime/provenance files. Record release evidence.
5. Package and verify CEM-QL against the registry fork and its other released
   dependencies before publishing CEM-QL. The workspace path dependency alone
   is insufficient evidence. Publish the remaining dependent crates only after
   their exact dependencies are available.

Subsequent fork releases update `vendor/url/Cargo.toml`, both lock files,
CEM-QL's exact dependency, the rustdoc root URL in `src/lib.rs`, and versioned
examples here and in the README. Refresh `CEM-PACKAGING.patch` accordingly. Keep
fork release tags independent (`cem-url-vVERSION`). Returning to upstream `url`
requires unchanged focused CEM fixtures to pass and an explicit dependency
migration; never silently substitute upstream while publishing CEM-QL.

Cargo's [packaging contract](https://doc.rust-lang.org/cargo/commands/cargo-package.html)
and [path/version dependency rules](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#multiple-locations)
describe archive normalization and registry resolution.

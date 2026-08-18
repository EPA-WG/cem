# CEM-ML 0.1.0-rc.2 Release Rehearsal

Date: 2026-08-18

This record closes roadmap Phase 2.6. It proves the protected CEM-ML GitHub
Release path for the two WASM/npm units and Linux AMD64, including immutable
remote-byte verification and same-run finalizer recovery. It does not publish
the npm packages to the npm registry or publish an APT repository.

## Result

- Release: [CEM-ML 0.1.0-rc.2](https://github.com/EPA-WG/cem/releases/tag/cem-ml-v0.1.0-rc.2)
- Release ID: `372445085`
- Workflow: [CEM-ML Release run 32152871319](https://github.com/EPA-WG/cem/actions/runs/32152871319)
- Tag: `cem-ml-v0.1.0-rc.2`
- Source commit: `f85b7703b2a7f8be2746824763ac4fdd8c911d6b`
- Published: `2026-08-18T15:30:41Z`
- State after the recovery rehearsal: prerelease, published, and immutable
- Assets: 37 total; all 37 report a GitHub API SHA-256 digest
- Aggregate manifest: 36 release assets are listed by
  `cem-ml-0.1.0-rc.2.SHA256SUMS`; that checksum manifest is the 37th asset

The aggregate contract contains exactly these release units:

1. `@epa-wg/cem-ml`
2. `@epa-wg/cem-ml-cli`
3. `native-linux-amd64`

The aggregate index records protected environment `cem-ml-release`, promotion
target `cem_ml:release:promote`, same-workflow-run recovery, a single permitted
draft-to-published transition, and forbidden asset mutation, rebuild, or
repack.

## Preflight and local verification

The release candidate version was synchronized from
`packages/cem_ml/Cargo.toml` into all 12 governed Cargo, npm, deployment, and
lockfile authorities. After setting the authority to `0.1.0-rc.2`, the exact
Nx-owned preparation and verification commands were:

```bash
NX_TUI=false yarn nx run cem_ml:sync:platform-version
NX_TUI=false yarn nx run cem_ml:test:platform-release
NX_TUI=false yarn nx run @epa-wg/cem-ml-cli:test
NX_TUI=false yarn nx run cem_ml:verify:platform
```

Observed results:

- platform-release tests: 73 passed;
- CLI JavaScript tests: 16 passed;
- aggregate platform verification: target plus all 64 dependencies passed;
- the aggregate included the CEM-ML WASM build, Rust and CLI suites,
  clean-consumer npm packages, browser worker parity, native/WASM/browser
  command parity, and the Linux package/sign/verify plus install, upgrade, and
  uninstall lifecycle.

The release tag was created as an annotated immutable coordinate and pushed:

```bash
git tag -a cem-ml-v0.1.0-rc.2 -m "CEM-ML 0.1.0-rc.2 release rehearsal"
git push origin cem-ml-v0.1.0-rc.2
```

## First-tag failure and correction

The first candidate, `cem-ml-v0.1.0-rc.1`, intentionally remains preserved at
commit `207e47be32c9fae2d37707e2f36d505a11b8be77`. Its workflow
[run 32103244537](https://github.com/EPA-WG/cem/actions/runs/32103244537)
failed before draft creation because the changelog boundary treated Git's
`fatal: No tags can describe '<parent sha>'` response as an unexpected error
instead of an empty CEM-ML release history. It created no release and uploaded
no release assets.

The release coordinator now recognizes that Git response as the valid
first-release case. A real temporary Git repository regression test covers the
behavior. Because a release tag is immutable evidence, the correction used the
new `0.1.0-rc.2` version and tag rather than moving or replacing `rc.1`.

## Protected workflow evidence

Attempt 1 of run `32152871319` completed every production and promotion job:

| Job                                        |        Job ID | Result  |     Elapsed |
| ------------------------------------------ | ------------: | ------- | ----------: |
| Create or resume protected draft           | `95762857513` | success |        43 s |
| Produce `@epa-wg/cem-ml`                   | `95763249470` | success |  4 min 48 s |
| Produce `@epa-wg/cem-ml-cli`               | `95763249267` | success | 10 min 22 s |
| Produce native Linux AMD64                 | `95763249336` | success |  9 min 34 s |
| Upload immutable CI-owned units to draft   | `95767346912` | success |  1 min 32 s |
| Collect and stage aggregate draft evidence | `95768088446` | success |   4 min 8 s |

The maintainer approved the four expected `cem-ml-release` protected
environment boundaries in attempt 1: draft coordination, Linux signing,
immutable upload, and aggregate promotion. Administrator bypass remained
disabled and the environment remained restricted to `cem-ml-v*` tags.

The Linux producer recorded GitHub artifact attestation `41393974`, available
at <https://github.com/EPA-WG/cem/attestations/41393974>. Its producer evidence
records Ubuntu 24.04 runner image `20260810.271.1`, Node `24.19.0`, Yarn
`4.12.0`, Rust/Cargo `1.96.0`, GitHub CLI `2.97.0`, GnuPG `2.4.4`, and successful
release, installation, upgrade, and uninstall gates.

The immutable package-channel inputs recorded by the aggregate index are:

| Channel identity         | Release asset                             | SHA-256                                                            |
| ------------------------ | ----------------------------------------- | ------------------------------------------------------------------ |
| `@epa-wg/cem-ml` npm     | `cem-ml-0.1.0-rc.2-wasm-runtime-npm.tgz`  | `faa55dc682e8cfd245906e8dbe5a2bfddc4ae963d9a57901b7b0d0f30206ff03` |
| `@epa-wg/cem-ml-cli` npm | `cem-ml-0.1.0-rc.2-universal-cli-npm.tgz` | `5860aa151715a20eebf846f1209adaeaa5899c524970cd0603c00d5c3ad66648` |
| `native-linux-amd64` APT | `cem-ml-0.1.0-rc.2-linux-amd64-gnu.deb`   | `e075f8b56564fafc25e15b02435edbcb1861b4b3d37bc6a6903e74eab511f409` |

GitHub emitted one non-failing maintenance annotation: the Node 20 action
runtime used by `actions/upload-artifact@v4` was forced to Node 24. This is an
upstream action-runtime compatibility signal, not a release verification
failure, and should be reviewed during routine workflow dependency updates.

## Independent remote-byte verification

After publication, all release assets were downloaded outside the workspace
and checked against the aggregate manifest:

```bash
release_dir="$(mktemp -d /tmp/cem-release-rc2.XXXXXX)"
gh release download cem-ml-v0.1.0-rc.2 \
  --repo EPA-WG/cem \
  --dir "$release_dir"
(
  cd "$release_dir"
  sha256sum --check cem-ml-0.1.0-rc.2.SHA256SUMS
)
```

All 36 manifest entries passed. The aggregate release index itself reports 35
unit assets; adding the release index and `SHA256SUMS` yields the 37 assets
reported by GitHub.

The API state was checked with:

```bash
gh api repos/EPA-WG/cem/releases/tags/cem-ml-v0.1.0-rc.2
gh api repos/EPA-WG/cem/actions/runs/32152871319
```

The release API reported `immutable: true`, 37 assets, and 37 non-null SHA-256
digests.

## Same-run recovery rehearsal

The aggregate finalizer from attempt 1 was selected for a GitHub **Re-run
jobs** recovery:

```bash
gh run rerun --repo EPA-WG/cem --job 95768088446
```

The protected environment was approved once for the rerun. Attempt 2 retained
the same workflow run ID, tag, and source commit. GitHub reused the successful
upstream producer and upload results and ran only the terminal aggregate
finalizer as new work. Finalizer job `95770240560` succeeded in 3 min 6 s.

The rerun recollected the exact published assets, regenerated and verified the
aggregate evidence, accepted identical remote bytes, and treated promotion as
an idempotent no-op verification. Afterward:

- run attempt: 2, success, updated `2026-08-18T15:36:05Z`;
- release ID: still `372445085`;
- assets: still 37;
- `published_at`: still `2026-08-18T15:30:41Z`;
- `updated_at`: still `2026-08-18T15:30:41Z`;
- immutable state: still true.

This proves that recovery must retain the recorded `GITHUB_RUN_ID`; a new
workflow dispatch cannot substitute different run-bound producer evidence.

## Release-lane isolation

The tag's workflow inventory was queried with:

```bash
gh run list --repo EPA-WG/cem \
  --branch cem-ml-v0.1.0-rc.2 \
  --limit 100 \
  --json databaseId,workflowName,conclusion,headSha,url

gh run list --repo EPA-WG/cem \
  --workflow publish.yml \
  --branch cem-ml-v0.1.0-rc.2 \
  --limit 100 \
  --json databaseId,workflowName,conclusion,headSha,url
```

The tag has exactly one workflow run: CEM-ML Release `32152871319`. The generic
`publish.yml` query returned an empty array, consistent with its explicit
`!cem-ml-v*` tag exclusion.

The retained `.github/workflows/cem-ml-native-release.yml` recipe has no
schedule or tag trigger; it is manual-dispatch-only and all four macOS/Windows
producer/publisher jobs carry `if: ${{ false }}`. It therefore schedules no
self-hosted work and has no path to mutate the three-unit release. The workflow
is not yet present on the default branch, so querying it directly by workflow
name returns GitHub's expected 404; the complete tag-run inventory above is the
authoritative execution proof.

## Operator recovery rules

- If a producer or upload fails before promotion, correct only missing work and
  preserve every identical uploaded asset. Changed bytes require a new common
  version and tag.
- If aggregate collection or promotion is interrupted, rerun the failed
  finalizer job on the same GitHub workflow run. Do not start a replacement
  dispatch for run-bound evidence.
- Never move an existing CEM-ML release tag, overwrite a release asset, rebuild
  a package-channel input, or repack an already verified tarball/deb.
- Keep macOS/Homebrew and Windows/WinGet publication disabled until a later
  roadmap decision restores those release units and their runner trust model.

# `@epa-wg/custom-element` History Import And Package Boundary

Status: accepted and executed on 2026-08-19. This document locks the history
topology, reversible execution procedure, active package boundary, and
post-import checks. The tree-neutral join is
`dfe142be5dadd7d84010b3954c9532ff2f85ddcb`.

It applies the recommendation from
[`custom-element-phase3.6-inventory.md`](custom-element-phase3.6-inventory.md):
preserve both source branches and every real source tag, join that history to the
existing monorepo adapter work, and keep the separate distribution repository out
of the product graph.

## Accepted History Topology

The import has three parents at its join commit:

1. the then-current `cem/develop` commit containing the existing
   `packages/custom-element/` snapshot and adapter work;
2. the path-prefixed external `main` tip;
3. the path-prefixed external `develop` tip that produced npm `0.0.39`.

The join uses Git's `ours` merge strategy. Its tree must be byte-identical to the
first parent. This is intentional: `packages/custom-element/` already contains the
published snapshot plus later adapter work, so the import adds missing provenance
without rolling the active package back to either external branch.

The external source commits are rewritten in an isolated clone so every historical
path begins with `packages/custom-element/`. Author, committer, timestamp, message,
parent topology, and file bytes are otherwise retained. The original repository
remains authoritative for the original commit IDs; the import record pins the
original-to-rewritten tip mapping.

### Retained Source Refs

| Meaning | Original ref/commit | Imported ref/commit |
| --- | --- | --- |
| External root | `e75c240156fb4585e1361016819407d24227a5ab` | `d6ba98b2aa59b726092453cb08e395a831529c0f` |
| `main` / `0.0.37` | `0282a74fb2c79223f7f216627c7d0e997272ed99` | `32e1246f01ebc7e46303a1c52fea49d390a7e9f9` |
| `develop` / npm `0.0.39` | `3ce6f57c49e5f1e840c2eb3bdad0740838cad4f2` | `c99632f04d4115c6a2eeec992fbd3003bc822d3d` |

All 282 commits reachable from `main` or `develop` are retained. The rewritten
tips receive permanent lightweight tags:

- `custom-element-history-main`;
- `custom-element-history-develop`.

Each of the 32 real lightweight release tags is renamed by prepending
`custom-element-v`. For example, `0.0.37` becomes
`custom-element-v0.0.37`, and the anomalous historical `0.05` becomes
`custom-element-v0.05`. Generic source tags are never imported because the
monorepo already owns generic fixed-release tags.

No tags are invented for npm versions whose source repository has no tag. In
particular, `0.0.38` and `0.0.39` remain documented npm releases rather than
synthetic Git tags; `custom-element-history-develop` pins the exact `0.0.39`
`gitHead` lineage.

The 469-commit `custom-element-dist` graph is not merged. Its original
`main=9887eec`, `develop=49d20ab`, npm `0.0.39` `gitHead=63ef0cfd`, and 88+3
behavioral surface remain provenance inputs for the separately checked reference
manifest.

## Why A Path Rewrite Plus An `ours` Join

The rejected alternatives are:

- a squashed subtree, because it discards commit and tag provenance;
- an ordinary subtree rooted at `/`, because historical paths would not follow
  the package's monorepo location;
- replacing the active directory with external `main`, because that loses the
  published `0.0.39` fixes and all subsequent adapter work;
- replacing it with external `develop`, because that restores the forbidden
  browser XSLT/XPath engine and still loses the monorepo work;
- replaying the interleaved monorepo commits onto the external graph, because it
  rewrites unrelated CEM history and creates unnecessary conflict risk;
- Git grafts or `git replace`, because they are local metadata rather than
  durable shared history.

The selected join makes the external graph permanently reachable while keeping
the current product tree unchanged. Later source changes remain ordinary
monorepo commits on top of the join.

## Execution Procedure

The import runs in throwaway clones and uses only Git included with the workspace
host. `git-filter-repo` is not installed, so the procedure uses
`git filter-branch` with an index-only path rewrite; this exact procedure was
rehearsed successfully against Git 2.53.0.

### 1. Preflight And Isolated Source Clone

Before starting, verify that `develop` equals `origin/develop`, the external
checkout is clean, and its remote tips still equal the pinned original hashes.
Abort on any mismatch and refresh the inventory rather than importing a different
graph.

```bash
SOURCE_CHECKOUT=/home/suns/aWork/custom-element
IMPORT_STAGE=$(mktemp -d)

git status --short --branch
git ls-remote git@github.com:EPA-WG/custom-element.git \
  refs/heads/main refs/heads/develop
git -C "$SOURCE_CHECKOUT" status --short --branch

git clone --no-local "$SOURCE_CHECKOUT" "$IMPORT_STAGE/source"
git -C "$IMPORT_STAGE/source" fetch "$SOURCE_CHECKOUT" \
  refs/remotes/origin/develop:refs/heads/develop
```

The explicit fetch is required because the local history checkout has only a
local `main`; its current `develop` tip is stored as `origin/develop`.

### 2. Prefix Both Branches And All Real Tags

```bash
mapfile -t tag_refs < <(
  git -C "$IMPORT_STAGE/source" for-each-ref \
    --format='%(refname)' refs/tags
)

(
  cd "$IMPORT_STAGE/source"
  FILTER_BRANCH_SQUELCH_WARNING=1 git filter-branch \
    --force \
    --tag-name-filter cat \
    --index-filter '
      git ls-files -s |
        sed "s#\t#&packages/custom-element/#" |
        GIT_INDEX_FILE="$GIT_INDEX_FILE.new" git update-index --index-info &&
      mv "$GIT_INDEX_FILE.new" "$GIT_INDEX_FILE"
    ' \
    -- refs/heads/main refs/heads/develop "${tag_refs[@]}"
)

mapfile -t old_tags < <(git -C "$IMPORT_STAGE/source" tag --list)
for old_tag in "${old_tags[@]}"; do
  git -C "$IMPORT_STAGE/source" tag \
    "custom-element-v${old_tag}" "$old_tag"
  git -C "$IMPORT_STAGE/source" tag -d "$old_tag"
done

git -C "$IMPORT_STAGE/source" tag custom-element-history-main main
git -C "$IMPORT_STAGE/source" tag custom-element-history-develop develop
```

Only the rewritten branches and renamed tags are later fetched. The temporary
`refs/original/*` backup refs created by `filter-branch` are deliberately not
transferred.

### 3. Create And Verify The Join In An Isolated Monorepo Clone

```bash
git clone --no-local /home/suns/cem "$IMPORT_STAGE/cem"
git -C "$IMPORT_STAGE/cem" switch develop
test "$(git -C "$IMPORT_STAGE/cem" rev-parse HEAD)" = \
  "$(git -C "$IMPORT_STAGE/cem" rev-parse refs/remotes/origin/develop)"
git -C "$IMPORT_STAGE/cem" remote add custom-element-history \
  "$IMPORT_STAGE/source"

git -C "$IMPORT_STAGE/cem" fetch custom-element-history \
  refs/heads/main:refs/remotes/custom-element-history/main \
  refs/heads/develop:refs/remotes/custom-element-history/develop \
  'refs/tags/custom-element-v*:refs/tags/custom-element-v*' \
  'refs/tags/custom-element-history-*:refs/tags/custom-element-history-*'

BASE_COMMIT=$(git -C "$IMPORT_STAGE/cem" rev-parse HEAD)
git -C "$IMPORT_STAGE/cem" branch \
  rollback/custom-element-history-import "$BASE_COMMIT"

git -C "$IMPORT_STAGE/cem" merge \
  --no-ff \
  --allow-unrelated-histories \
  --strategy=ours \
  refs/remotes/custom-element-history/main \
  refs/remotes/custom-element-history/develop \
  -m "chore(custom-element): join legacy source history"
```

The isolated join is accepted only when all of these checks pass:

```bash
IMPORT_COMMIT=$(git -C "$IMPORT_STAGE/cem" rev-parse HEAD)

git -C "$IMPORT_STAGE/cem" diff --exit-code \
  "$BASE_COMMIT" "$IMPORT_COMMIT"
test "$(git -C "$IMPORT_STAGE/cem" show -s --format='%P' \
  "$IMPORT_COMMIT" | wc -w)" -eq 3

git -C "$IMPORT_STAGE/cem" merge-base --is-ancestor \
  32e1246f01ebc7e46303a1c52fea49d390a7e9f9 "$IMPORT_COMMIT"
git -C "$IMPORT_STAGE/cem" merge-base --is-ancestor \
  c99632f04d4115c6a2eeec992fbd3003bc822d3d "$IMPORT_COMMIT"

test "$(git -C "$IMPORT_STAGE/source" rev-list main develop |
  sort -u | wc -l)" -eq 282
test "$(git -C "$IMPORT_STAGE/source" tag \
  --list 'custom-element-v*' | wc -l)" -eq 32

git -C "$IMPORT_STAGE/cem" fsck --full
```

Also assert programmatically that every file at both rewritten tips starts with
`packages/custom-element/`, and record the actual base/join IDs in the import
commit's companion provenance fixture.

### 4. Verify The Unchanged Product Tree Before Publishing Refs

Because the join tree is identical to the current `develop` tree, run the package
gate in the real workspace before moving any shared ref:

```bash
yarn nx run @epa-wg/custom-element:verify
```

Then fetch the isolated join to a local review branch without advancing the
checked-out `develop` branch:

```bash
git fetch "$IMPORT_STAGE/cem" \
  refs/heads/develop:refs/heads/phase-3.6/custom-element-history-import \
  'refs/tags/custom-element-v*:refs/tags/custom-element-v*' \
  'refs/tags/custom-element-history-*:refs/tags/custom-element-history-*'
```

Repeat the tree, parent, ancestry, tag, path-prefix, and `git fsck` checks against
that local review branch. This is the final no-side-effect rollback point: if any
check fails, do not update `develop` and do not push any imported ref.

### 5. Atomic Remote Update

Collect the exact local tag refs and push the import branch plus all 34 retained
tags atomically:

```bash
mapfile -t import_tag_refs < <(
  git for-each-ref --format='%(refname)' \
    refs/tags/custom-element-v* \
    refs/tags/custom-element-history-*
)

git push --atomic origin \
  refs/heads/phase-3.6/custom-element-history-import:refs/heads/develop \
  "${import_tag_refs[@]}"

git fetch origin develop
git merge --ff-only origin/develop
```

An atomic push failure leaves the remote branch and tags unchanged. After a
successful push, removing the imported parents would require rewriting shared
history and deleting tags; that is not a routine rollback. It requires explicit
approval, the saved base commit, a coordinated force update, and remote tag
deletion. The pre-push checks are therefore the primary rollback mechanism.

## Active Source And Generated-Output Boundary

The history import does not overlay either rewritten external tip onto the active
tree. The existing monorepo tree remains authoritative.

Active package-owned source includes:

- `custom-element.js`, `index.js`, `custom-element.d.ts`, and the four loose
  companion browser modules;
- `LICENSE`, `README.md`, `datasource.md`, package docs, demos, IDE metadata, and
  the browser landing page;
- current `material/` compatibility inputs;
- `scripts/`, `test-fixtures/`, `project.json`, `package.json`, `CHANGELOG.md`,
  and `IMPORT.md`;
- package-scoped repository metadata only where it remains useful to monorepo
  development.

Historical editor/build files remain inspectable in imported commits but are not
restored to the active tree merely because they existed in an old source ref or
npm tarball.

The release root is generated at `packages/custom-element/dist/`. It must exclude:

- `.claude/`, `.idea/`, `.vs/`, `.vscode/`, and nested `.github/` workflow data;
- `.gitignore`, `.editorconfig`, `project.json`, import-planning notes, tests, and
  workspace-only scripts/configuration;
- `node_modules/`, `package-lock.json`, coverage, Storybook output, caches,
  temporary reports, and TypeScript incremental state;
- source-only fixtures except demos/material files deliberately admitted by the
  final archive manifest.

Generated `dist/` remains ignored source state and the sole npm publication root.
The packed-archive fixture, not the broad historical `files: ['*']` field, must
prove the final byte inventory.

## npm Identity And Public Surface

The accepted identity rules are:

- preserve the package name `@epa-wg/custom-element`, Apache-2.0 license,
  author/funding metadata, public `<custom-element>` tag, and browser entrypoint
  filenames;
- keep `.` -> `index.js`, `./CustomElement` -> `custom-element.js`, and
  `./package.json` as the initial export map; continue shipping `module-url.js`
  without silently adding it to the root barrel;
- keep root re-exports for the main class plus `http-request`, `local-storage`,
  and `location-element`, including their import-time registration side effects;
- preserve the genuinely documented non-engine helper surface (`cloneAs`, `mix`,
  `deepEqual`, `mergeAttr`, `xml2dom`, `xmlString`, `obj2node`, and `tagUid`) or
  provide explicit next-major compatibility aliases. Do not restore
  XSLT/XPath-engine exports such as `createXsltFromDom`, `xPath`, or `toXsl`;
- repair the declaration file to describe actual JS exports. The stale declared
  `log` helper is not implemented merely to preserve an already-false type claim;
- retain the current adapter settlement/installation APIs as intentional
  next-major additions, subject to the package fixture and migration guide;
- preserve a dependency-free consumer manifest for the first adopted release by
  packaging the required substrate runtime privately, without exporting its
  internal vendor paths.

The external `0.0.39` version is provenance and compatibility input, not the
active manifest version. `@epa-wg/custom-element` remains in the fixed `cem` Nx
release group. `packages/custom-element/package.json` is the canonical versioned
manifest and must match the group version; generated `dist/package.json` must be
derived from it. The current `0.1.0` source versus `0.1.1` changelog/workspace
mismatch is a release-configuration defect to repair before publishing, not a
reason to downgrade the source to `0.0.39`.

On the next publication, repository metadata should identify the actual monorepo
and `packages/custom-element` directory. Package name and import paths remain
stable; the obsolete standalone-repository URL is provenance, not the new source
location.

## Nx Ownership

The resolved Nx project remains `@epa-wg/custom-element` at
`packages/custom-element`, tagged `npm:public` and owned as a library.

- `build` remains cached, produces only `{projectRoot}/dist`, and depends on the
  `cem-elements` build plus the CEM-QL WASM and CEM-ML CLI artifacts needed by its
  current packaging/material transform.
- `test` remains cached and owns package baseline, source/dist browser, companion,
  material-conversion, and theme-vendor evidence. The inventory-discovered
  manifest, adapter matrix, and archive checks join this target as they land.
- `lint` must become a real source/package lint boundary instead of only rerunning
  the package baseline verifier; that cleanup belongs with the fixture/target
  work, not the history-only import.
- `verify` stays the package aggregate and must eventually include all four
  inventory-discovered migration fixtures.
- `nx-release-publish` continues publishing publicly from
  `packages/custom-element/dist`.
- the root dependency remains `workspace:^`, and root web-types paths continue to
  resolve from `packages/custom-element/ide/`.

The resolved graph currently records the package's static dependency on
`cem-elements`. Theme vendor coupling is expressed through explicit target/file
inputs rather than a graph edge; the import does not alter either relationship.

## Import Exit Criteria

The next checklist item is complete only when:

- one tree-neutral three-parent join is reachable from `develop`;
- the rewritten main/develop tips equal the pinned hashes above and all 282
  rewritten commits are reachable;
- all 32 real tags plus the two source-tip tags exist remotely with the accepted
  names, and no generic tag was imported or synthetic release tag created;
- every rewritten tip path is under `packages/custom-element/`;
- the active tree is byte-identical to the pre-import first parent;
- Git integrity and `@epa-wg/custom-element:verify` pass;
- the provenance fixture records base, join, original and rewritten tips, ref
  counts, path policy, and the excluded distribution graph.

Source reconciliation, public-adapter behavior changes, archive cleanup, and the
release-version fix remain subsequent checklist work. They must not be mixed into
the topology-only import commit.

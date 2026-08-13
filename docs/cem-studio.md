# CEM Studio Web Application Proposal

Status: research and design proposal, 2026-08-09. This document is not an
active implementation checklist or compatibility contract. Promote accepted
stages into acceptance criteria, the roadmap, and `docs/todo.md` before
implementation.

The Phase 2.5 deployment, version, platform, capability, signing, and host-wire
decisions are canonical in
[`cem-ml-deployment-contract.md`](./cem-ml-deployment-contract.md). This
document remains the broader Phase 6.5 Studio product and persistence proposal.

## Outcome

CEM Studio should be an installable, offline-capable web application and its own
public `@epa-wg/cem-studio` npm package. It makes the useful `cem-ml` CLI
workflows available through a browser workbench and directly depends on the
separate `@epa-wg/cem-ml-cli` WebAssembly CLI npm package. That CLI package
depends on the generated `@epa-wg/cem-ml` WASM runtime. Studio must not
reimplement formats, validation, query, or transformation behavior in
TypeScript.

The recommended product shape is:

- a deployable `cem-studio` Progressive Web Application (PWA), published and
  versioned as `@epa-wg/cem-studio`;
- a typed browser command service in `@epa-wg/cem-ml-cli/browser`, backed by the
  separate `@epa-wg/cem-ml/wasm` runtime in a dedicated worker;
- a portable npm-installed `cem-ml` command that runs the WASM engine through a
  Node host adapter from the `@epa-wg/cem-ml-cli` npm package on supported
  operating systems;
- a deliberately small CLI platform matrix: WASM for Node, native Linux AMD64,
  native macOS ARM64 distributed through Homebrew, and native Windows AMD64;
- a separate Nx subproject and deployment package for each of those three
  native platform targets, with binaries preserved as tagged GitHub Release
  assets;
- a project/subproject explorer that owns data sets, resources, run
  configurations, conversions, queries, transformations, and transformation
  graphs;
- a bidirectional CLI Command view that serializes the active page, copies its
  command/inputs/config/output, and applies edited commands to an existing or
  newly created project page;
- editable inline sources and URL-backed sources with explicit content and
  schema identities;
- validation, data, result, report, source-map, and graph previews;
- local-first persistence in IndexedDB, with import/export as the user's durable
  backup boundary;
- UI built from light-DOM `@epa-wg/cem-components`, with Studio-specific
  components published from `@epa-wg/cem-components/studio`;
- Consumer Semantic Theming supplied by `@epa-wg/cem-theme`;
- optional account-backed storage adapters for S3, a NoSQL service, Git
  repositories, and GitHub Gists as a later wishlist, not an MVP dependency;
  and
- a later semi-native CLI experiment that bundles the Node host, CLI JavaScript,
  and WASM runtime into target-specific executables using Node.js Single
  Executable Applications (SEA), with deprecated `pkg` considered only as a
  comparison or migration reference.

The common `cem_ml` project owns the authoritative product version. The WASM
runtime npm package, CLI npm package, Studio npm/PWA package, and every native
platform deployment package must be built from the same release commit and
publish that exact CEM-ML version. Any semi-native package promoted from the
wishlist joins the same release family. They do not version independently.

The phrase “WASM version of the CLI” means portable browser and Node-hosted
parity with the CLI's typed operations and reports. In the browser it does not
mean compiling terminal formatting, `clap`, process exit behavior, or an
unrestricted operating-system filesystem into the page. In Node, a thin host
adapter may provide files, streams, environment, and exit-code projections
without creating another semantic engine.

## Goals

- Let a user learn and exercise CEM-ML without installing a native executable.
- Cover the structural lifecycle: parse, validate, inspect, convert, query,
  transform, and inspect the resulting reports and source maps.
- Demonstrate every browser-capable schema package from source content types
  through queries, templates, conversions, and transformation graphs.
- Allow users to add, rename, edit, copy, reorder, and remove projects,
  subprojects, data sets, resources, configurations, and transformations.
- Let users move between structured forms and an editable `cem-ml` command
  without losing the selected inputs, effective config, outputs, or project
  destination.
- Restore user changes after reload, browser restart, PWA update, and offline
  launch, subject to browser storage guarantees.
- Keep browser and native results comparable through shared request, capability,
  diagnostic, report, and source-map contracts.
- Keep local projects private by default and require an explicit action before
  fetching a remote resource or publishing data to an external service.
- Make useful Studio components available to other CEM applications without
  turning the application shell into a component package.
- Use the Consumer Semantic Theme consistently across layout, controls,
  editors, graphs, diagnostics, previews, and all supported theme modes.

## Non-Goals for the First Release

- An arbitrary shell, arbitrary JavaScript evaluator, or emulated POSIX
  environment in the page.
- Silent access to the user's local filesystem, network credentials, Git
  credentials, or cloud credentials.
- Full native plugin, native filesystem, or benchmark parity when a capability
  is unavailable in WASM.
- Multi-user collaboration, hosted accounts, or a required CEM cloud service.
- Executing scripts from user-authored HTML or transformation output in the
  Studio origin.
- Replacing desktop IDE integration. Studio complements the LSP/DAP and
  Chromium integrations described in [`integrations.md`](./integrations.md).

## Existing Workspace Foundation and Gaps

| Existing asset                                                                                                                                    | Value to Studio                                                                      | Required gap work                                                                                                                                                                     |
| ------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| The `cem_ml` Rust crate is built as `cdylib` and `rlib`, and `@epa-wg/cem-ml` packages synchronized browser/Node bindings.                       | One semantic implementation reaches native and WASM through a policy-free deployment. | Extend the common WASM request surface to accepted operation parity; Studio must consume this package rather than bundle a private runtime.                                           |
| The current WASM API exposes observers, resolver adapters, run-config normalization, input/output spec parsing, and bounded legacy conversion.    | Browser hosts already have event and resource-resolution seams.                      | Add typed parse, validate, inspect, convert, query, transform, trace, cancellation, and capability entry points. The current surface is not full CLI parity.                          |
| `cem_ml_cli` implements parse, validate, check, inspect, convert, trace, bench, fixtures, transform, and query dispatch.                          | Defines proven user workflows and report projections.                                | Extract CLI-independent request execution into the library; do not import CLI arguments, terminal streams, or filesystem assumptions into WASM.                                       |
| CEM-ML reports include diagnostics, parser stages, scheduler trace, conversion results, transform results, and transformation-graph results.      | Supplies rich browser result models.                                                 | Version the browser wire contract, bound payload sizes, and add incremental progress/cancellation semantics.                                                                          |
| Schema packages register content types, examples, formatters, colorizers, queries, schemas, and transformations.                                  | Can generate the Feature Tour and capability catalog instead of hard-coding samples. | Emit a browser-consumable package/capability manifest and bundle only WASM-compatible artifacts.                                                                                      |
| `@epa-wg/cem-components` supplies reusable TypeScript/light-DOM components and already depends on `@epa-wg/cem-elements` and `@epa-wg/cem-theme`. | Natural UI frame for the application.                                                | Add explicit package subpaths and Studio components without making application state or routing part of the component API.                                                            |
| `@epa-wg/cem-theme` publishes semantic token CSS and theme modes.                                                                                 | Canonical visual system for Studio.                                                  | Add only genuinely reusable missing semantic tokens; keep Studio-only aliases in the Studio component stylesheet.                                                                     |
| The current Nx fixed `cem` release group contains the theme/component/browser packages but not the proposed CEM-ML deployment projects.           | Existing fixed-version release automation is a useful pattern.                       | Add a distinct fixed `cem-ml-platform` release family whose version originates from `cem_ml` and is copied exactly to Studio, CLI npm, WASM npm, and every native deployment package. |

Relevant local contracts include the
[`cem-ml` CLI feature summary](./cem-ml-cli-contract.md),
[`cem-ml` run-config contract](./cem-ml-phase2-run-config-contract.md),
[`cem-element` WASM proposal](./cem-element-wasm-proposal.md), and the
[`@epa-wg/cem-components` conventions](../packages/cem-components/docs/conventions.md).

## Recommended Architecture

```text
CEM Studio PWA
├── application shell
│   ├── routing, commands, responsive workspace layout
│   ├── @epa-wg/cem-components
│   ├── @epa-wg/cem-components/studio
│   └── @epa-wg/cem-theme CSS and theme scope
├── workspace service
│   ├── IndexedDB project records and text/blob resources
│   ├── import/export and schema migrations
│   ├── URL resource snapshots and provenance
│   └── optional remote-storage adapters
├── engine client
│   ├── typed request/response and progress protocol
│   ├── cancellation, stale-result rejection, and limits
│   └── dedicated Web Worker
│       └── @epa-wg/cem-ml-cli/browser
│           └── @epa-wg/cem-ml/wasm
│               ├── CEM-ML engine and schema packages
│               ├── CEM-QL/query/transformation engines
│               └── JS resource resolver for studio:// and allowed URLs
└── service worker
    ├── versioned app-shell/WASM/sample cache
    ├── offline navigation fallback
    └── update coordination (never the project database owner)
```

A dedicated worker keeps parsing and transformations off the UI thread. Web
Workers run in a separate global context and communicate with the page by
messages, which fits a typed request/progress/result boundary
([MDN Web Workers](https://developer.mozilla.org/en-US/docs/Web/API/Web_Workers_API/Using_web_workers)).
The first version should use one dedicated engine worker per open Studio window.
A shared worker or worker pool should be added only after measurements show a
benefit and the cache/concurrency contract is defined.

### Responsibility Boundaries

| Layer                            | Owns                                                                                                                                            | Must not own                                                                                 |
| -------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `cem_ml` Rust library            | Parsing, validation, schemas, normalized run plans, resolver policy, conversion, reports, diagnostics, source maps, transformations.            | Browser widgets, routes, terminal formatting, account services.                              |
| `@epa-wg/cem-ml/wasm`            | Generated low-level WASM bindings and wire-safe data transfer.                                                                                  | UI state or a second semantic implementation.                                                |
| `@epa-wg/cem-ml-cli/browser`     | Worker-safe typed commands, capability discovery, resolver bridge, cancellation, event streaming.                                               | DOM rendering or product-specific project storage.                                           |
| `@epa-wg/cem-components`         | Reusable light-DOM controls and functional UI patterns.                                                                                         | Studio routing, persistence, or CEM-ML execution policy.                                     |
| `@epa-wg/cem-components/studio`  | Reusable Studio workbench composites such as explorer, source editor frame, diagnostics panel, preview frame, and graph view.                   | User accounts, provider credentials, service worker lifecycle.                               |
| `@epa-wg/cem-studio` app/package | Composition, routes, project state, command orchestration, PWA lifecycle, deployment configuration, and publishable static/bootstrap artifacts. | Duplicate format/query/transform semantics or a bundled private copy of the engine contract. |

## Workspace and Package Placement

The proposed eventual workspace placement is:

```text
packages/
├── cem_ml/                       # common engine and authoritative product version
├── cem_ml_cli/                   # common native CLI source/adapter
├── cem-ml-npm/                   # @epa-wg/cem-ml: generated low-level WASM runtime
├── cem-ml-cli-npm/               # @epa-wg/cem-ml-cli: browser/Node adapters + npm bin
├── cem-components/
│   └── src/lib/studio/           # @epa-wg/cem-components/studio
├── cem-studio/                   # @epa-wg/cem-studio: deployable Nx PWA + npm package
├── cem-ml-cli-native-linux-amd64/
├── cem-ml-cli-native-brew-arm64/
└── cem-ml-cli-native-windows-amd64/
```

The exact generated npm-package folder name should be selected during
scaffolding; the public package identity is the important contract.

Studio-specific components belong in the `@epa-wg/cem-components` project and
npm package, exposed through a `./studio` export. A component should move to the
general package surface when it expresses a reusable functional pattern that
does not require CEM Studio state, for example a split pane, accessible tree,
status list, diff viewer, or bounded preview frame. Components whose vocabulary
is inherently Studio-specific, such as a transformation-graph inspector, may
remain in the Studio subpath.

Angular Material parity is a prerequisite gate, not merely inspiration. Before
implementing any Studio control or interaction, classify it against the pinned
[Angular Material component catalog](https://material.angular.dev/components/categories).
If Angular Material has a counterpart, first create and verify the general
`@epa-wg/cem-components` equivalent on the light-DOM `<cem-element>` substrate;
Studio then consumes that component. Neither the `/studio` subpath nor the
application may temporarily duplicate it. A Studio component or application-
local implementation may lead only when the parity matrix records no Angular
Material counterpart. If that new behavior later proves reusable, move it into
the appropriate component-package surface.

Application orchestration stays in the publishable `@epa-wg/cem-studio`
project: provider authentication, routes, IndexedDB repositories,
service-worker registration, seed-project installation, and deployment
environment configuration are not reusable UI components.

Each folder above is an independent Nx subproject with its own build/package/
verify/publish targets. This is the complete initial native target list, not an
illustrative matrix. Other native architectures, ABIs, operating systems, and
package-manager channels are outside initial support. Any later expansion must
be an explicit product decision and a new deployment subproject; one project
must not publish opaque architecture variants selected only by a shell script.

## Deployment Subprojects and Synchronized Version Model

The common engine, three npm deployment packages, and native target packages
are separate Nx subprojects and deployment units. They form one fixed-version
CEM-ML platform release.

| Subproject/deliverable    | Public package/artifact                           | Direct dependency                                                                | Runtime and use                                                          |
| ------------------------- | ------------------------------------------------- | -------------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| Common CEM-ML             | Rust crate/project `cem_ml`                       | None; owns common engine contracts                                               | Semantic engine and authoritative version source.                        |
| WASM runtime npm          | `@epa-wg/cem-ml`                                  | Generated from common `cem_ml`                                                   | Low-level WASM/types/schema-package runtime; no executable npm `bin`.    |
| Universal CLI npm         | `@epa-wg/cem-ml-cli`                              | Exact same-version `@epa-wg/cem-ml`                                              | Browser command service, Node host adapter, and npm `cem-ml` executable. |
| Studio npm/PWA            | `@epa-wg/cem-studio`                              | Exact same-version `@epa-wg/cem-ml-cli`; compatible CEM component/theme packages | Installable/hostable Studio application and static PWA deployment.       |
| Native target deployments | One archive/package set per supported Rust target | Common `cem_ml` plus common `cem_ml_cli` source                                  | Native CLI for one OS/architecture/ABI target.                           |

The required build/release graph is:

```text
cem_ml (common version authority)
├── @epa-wg/cem-ml (WASM runtime npm)
│   └── @epa-wg/cem-ml-cli (universal WASM CLI npm)
│       └── @epa-wg/cem-studio (Studio npm + PWA)
└── cem_ml_cli (common native CLI source)
    ├── cem_ml_cli_native_linux_amd64
    ├── cem_ml_cli_native_brew_arm64
    └── cem_ml_cli_native_windows_amd64
```

### Initial CLI Platform Matrix

The first supported CLI matrix is intentionally limited to these four entries:

| CLI distribution | Platform/channel                              | Runtime identity       | Deployment project/package        |
| ---------------- | --------------------------------------------- | ---------------------- | --------------------------------- |
| WASM for Node    | Supported Node.js hosts                       | `wasm-node`            | `@epa-wg/cem-ml-cli` npm package  |
| Linux AMD64      | `x86_64-unknown-linux-gnu`                    | `native-linux-amd64`   | `cem_ml_cli_native_linux_amd64`   |
| Homebrew ARM64   | macOS `aarch64-apple-darwin` through Homebrew | `native-macos-arm64`   | `cem_ml_cli_native_brew_arm64`    |
| Windows AMD64    | `x86_64-pc-windows-msvc`                      | `native-windows-amd64` | `cem_ml_cli_native_windows_amd64` |

The browser command export used by Studio remains a supported programmatic
surface, but it is not another terminal CLI platform. Additional native targets
are not implied by the portability of the Node/WASM npm package.

### Common Version Authority

The common `cem_ml` project is the only source of the CEM-ML platform version.
With the current repository layout, the authoritative field is
`packages/cem_ml/Cargo.toml` `[package].version`. If a generated neutral version
manifest is introduced later, it must live under the common project and be
generated from or replace that one source; parallel manually edited version
files are forbidden.

A release synchronizer must copy the common version exactly into:

- the common `cem_ml_cli` crate manifest;
- `@epa-wg/cem-ml/package.json`;
- `@epa-wg/cem-ml-cli/package.json` and its exact runtime dependency;
- `@epa-wg/cem-studio/package.json` and its exact CLI dependency;
- every native target package manifest, archive name, package-manager manifest,
  embedded version record, checksum/provenance/SBOM record, and release index;
- every promoted semi-native target manifest, executable metadata, embedded
  runtime version record, and provenance record;
- Studio's capability manifest, service-worker cache/build id, and About view;
  and
- the machine version/capability output fixtures for all runtimes.

The proposed Nx `cem-ml-platform` release group must use a fixed project
relationship and exact internal dependency versions. No member derives a
version from its own conventional-commit history, package-manager metadata, or
publication time. A Studio-only, npm-CLI-only, or platform-packaging change that
requires publication still advances the common `cem_ml` version and rebuilds
the complete release family from the same commit.

Release order follows the dependency graph, but all artifacts are staged and
verified before public promotion. A partial registry/package-manager failure is
retried with the same immutable version/artifacts and recorded in a release
index; it does not create a different Studio, npm CLI, or platform version.
Package-manager-specific build revisions may describe repackaging metadata, but
the embedded CEM-ML semantic version remains the exact common version.

### `@epa-wg/cem-ml`: Separate WASM Runtime NPM Package

`@epa-wg/cem-ml` is a generated deployment subproject for the low-level common
engine runtime. It contains `@epa-wg/cem-ml/wasm`, generated types,
schema-package assets, ABI/capability metadata, and integrity/version records. It
does not own the user-facing CLI argument parser, Node launcher, browser command
client, or npm `bin`.

This separation lets browser/Node libraries consume the engine without pulling
Studio or CLI presentation code and gives the CLI npm subproject one exact
runtime dependency.

### `@epa-wg/cem-ml-cli`: Separate Universal CLI NPM Package

`@epa-wg/cem-ml-cli` is its own publishable Nx subproject and npm deployment
package. It depends on the exact same-version `@epa-wg/cem-ml` runtime and owns:

| Package surface              | Purpose                                                                                                                                            |
| ---------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| `@epa-wg/cem-ml-cli/browser` | Dedicated-worker client, browser resolver adapters, progress/cancellation, command parsing/serialization, and capability discovery used by Studio. |
| `@epa-wg/cem-ml-cli/node`    | Node host adapters for files, standard streams, environment, signals, and process exit-code projection.                                            |
| npm `bin` named `cem-ml`     | Small Node ESM launcher over the Node adapter and shared typed command service.                                                                    |

npm supports a `package.json` `bin` mapping that installs a command shim and
makes a dependency's executable available to `npm exec` and package scripts
([npm `package.json` documentation](https://docs.npmjs.com/cli/v11/configuring-npm/package-json#bin)).
That permits, conceptually:

```sh
npm install --global @epa-wg/cem-ml-cli
cem-ml validate data/catalog.json

# No global installation:
npm exec --package=@epa-wg/cem-ml-cli -- cem-ml validate data/catalog.json
```

These commands are illustrative until the shared command schema and exact flags
are accepted. Project documentation must not guess or separately maintain flags.

“Universal npm CLI” means this one npm deployment can run on supported Node
versions across operating systems/architectures on which the WASM and JS host
adapters work. It still requires Node and may lack native-only capabilities.
Machine version output identifies `runtime: wasm-node` and the one synchronized
CEM-ML version; capability output identifies Node, WASM ABI, and unavailable
features without inventing another package version.

The browser export must not import Node built-ins. The Node export must not
depend on DOM, service-worker, or IndexedDB APIs. Both converge on the exact
same-version `@epa-wg/cem-ml` runtime and normalized command/report contract.

If both npm and native installations expose `cem-ml`, ordinary `PATH` ordering
selects one. Documentation should recommend the explicit npm-exec form when the
WASM CLI is required. Runtime identity lets scripts reject an unintended host.

### `@epa-wg/cem-studio`: Separate Synchronized NPM Package

`@epa-wg/cem-studio` is a separate first-class publishable Nx subproject, not an
example inside the CLI, runtime, or component package. It contains:

- an application bootstrap for mounting/routing CEM Studio;
- production JS/CSS/assets plus a deployable static PWA artifact manifest;
- web app manifest, icons, worker and service-worker build inputs/outputs;
- Feature Tour/capability assets;
- types for supported bootstrap/deployment configuration; and
- package/build metadata recording the synchronized CEM-ML version and the CEM
  component/theme versions used by the build.

Its published runtime dependencies should be shaped like:

```json
{
    "version": "<cem-ml-common-version>",
    "dependencies": {
        "@epa-wg/cem-ml-cli": "<exact-cem-ml-common-version>",
        "@epa-wg/cem-components": "<tested-compatible-version>",
        "@epa-wg/cem-theme": "<tested-compatible-version>"
    }
}
```

The placeholders are generated release values, not ranges for manual editing.
Studio consumes the browser command service from `@epa-wg/cem-ml-cli/browser`;
the CLI package supplies its exact `@epa-wg/cem-ml` dependency. Studio must not
add a second independently resolved engine runtime.

The npm package and hosted PWA are two projections of one Studio deployment
build. A host may install the npm package and copy/bundle its static deployment
artifact, while the project can also publish an official hosted PWA from the
same verified output. Service-worker URLs and scope are deployment-origin
concerns; importing the npm module must not silently register a service worker
for an embedding application.

Studio has no independent release cadence or version. Even a UI-only Studio
publication uses the next common CEM-ML version and participates in the complete
fixed-version platform release.

### Native CLI Platform Subprojects and Deployment Packages

The common `cem_ml_cli` project owns platform-neutral native CLI source and
dispatch, but it is not itself a deployment package. Each of the three supported
native targets has an explicit Nx deployment subproject that cross-compiles or
natively compiles, packages, signs, verifies, and publishes only that target's
artifacts.

| Deployment subproject             | Rust target                | Required artifacts/channel inputs                                                                       |
| --------------------------------- | -------------------------- | ------------------------------------------------------------------------------------------------------- |
| `cem_ml_cli_native_linux_amd64`   | `x86_64-unknown-linux-gnu` | Tagged GitHub Release archive, `.deb`, APT metadata, checksum/signature, SBOM, and provenance.          |
| `cem_ml_cli_native_brew_arm64`    | `aarch64-apple-darwin`     | Tagged GitHub Release archive, Homebrew formula/tap metadata, checksum/signature, SBOM, and provenance. |
| `cem_ml_cli_native_windows_amd64` | `x86_64-pc-windows-msvc`   | Tagged GitHub Release `.zip`, installer/package metadata, checksum/signature, SBOM, and provenance.     |

Each platform project owns or produces the immutable inputs for its distribution
channel. An APT repository index or Homebrew tap may be a thin aggregation
project, but it consumes the corresponding platform project's artifact and exact
common version; it must not rebuild the binary or introduce another version.

Every published native CLI binary/archive must also be preserved as an asset on
the tagged GitHub Release for the common CEM-ML version. Asset names include the
version, operating system, architecture, and ABI where relevant. Checksums,
signatures, SBOMs, provenance, and a release index are uploaded beside the
binaries. Package-manager definitions resolve these immutable versioned assets,
not a mutable “latest” build URL. The complete asset set is staged before the
release is published; immutable releases should be enabled where available, and
a published binary is never replaced or deleted. Corrections use a new common
version. GitHub Releases are explicitly designed to package tagged software with
downloadable binary assets
([GitHub Releases documentation](https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases));
GitHub's release-management guidance describes staging assets in a draft before
making a release immutable
([GitHub release-management documentation](https://docs.github.com/en/repositories/releasing-projects-on-github/managing-releases-in-a-repository)).

Homebrew formulae are package definitions installed through `brew install`
([Homebrew Formula Cookbook](https://docs.brew.sh/Formula-Cookbook)). APT consumes
architecture-specific packages from configured repositories and manages
dependencies/upgrades
([Debian package management](https://www.debian.org/doc/manuals/debian-reference/ch02)).
`cargo install` remains a source-build fallback rather than a platform package
([Rust binary installation](https://doc.rust-lang.org/book/ch14-04-installing-binaries.html)).

The initial native scope stops at Linux AMD64, Homebrew ARM64, and Windows AMD64.
Other targets and channels are outside initial support. A later addition requires
an explicit scope change, a new project, signing and
install/upgrade/uninstall coverage, and a release-matrix entry.

### Wishlist: Semi-Native Node/WASM Executables

A later deployment option may package the exact synchronized
`@epa-wg/cem-ml-cli` Node launcher, its `@epa-wg/cem-ml` WASM runtime, and all
required runtime assets together with Node into a self-contained executable.
This would remove the end user's Node installation requirement while retaining
the `wasm-node` implementation rather than compiling a native Rust CLI. It is
therefore **semi-native**, not a replacement for the native target packages.

The preferred experiment is Node.js Single Executable Applications (SEA).
Current Node documentation describes SEA as a way to distribute a Node
application to a system without Node installed, supports embedded assets, and
marks the facility as active development
([Node.js SEA documentation](https://nodejs.org/api/single-executable-applications.html)).
The archived Vercel `pkg` project can be evaluated for compatibility lessons or
legacy migration only: its maintainers deprecated it at version 5.8.1
([Vercel `pkg` repository](https://github.com/vercel/pkg)). It should not become
the default production packager without a maintained successor and an explicit
security/ownership decision.

| Option                                 | Advantages                                                                                                                             | Costs and risks                                                                                                                                                                      |
| -------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Native Node.js SEA                     | Maintained with Node, embeds the launcher and WASM assets, removes the user-side Node prerequisite, and exposes an official asset API. | Still in active development; requires target-specific building/signing and validation of module, snapshot, code-cache, and asset constraints.                                        |
| Vercel `pkg` or a compatible successor | Familiar single-file Node packaging model and useful prior art for dependency/asset discovery.                                         | The original project is archived and deprecated; its frozen Node/runtime assumptions and security-update ownership make it unsuitable as the default without a maintained successor. |

If promoted from the wishlist, the experiment should initially produce only
Linux AMD64, macOS/Homebrew ARM64, and Windows AMD64 executables. Each output
must have its own Nx deployment subproject, use the exact common `cem_ml`
version, identify itself as `wasm-node-sea` (or `wasm-node-pkg` for an explicit
legacy experiment), and publish checksums, SBOM, provenance, signing, and
install/uninstall tests. It must not share an artifact name or runtime identity
with the native Rust package for the same platform. Any promoted semi-native
binary is also preserved as a distinctly named asset on the matching tagged
GitHub Release.

Before adoption, measure executable size, startup and transformation
performance, WASM asset loading, temporary-file behavior, command parity,
updater behavior, antivirus/notarization impact, signing reproducibility, Node
security-update cadence, licenses, and per-target build constraints. The Node
runtime and WASM engine remain embedded dependencies that must be rebuilt and
republished when either receives a relevant security update.

### Cross-Distribution Contract

All synchronized npm/WASM, native CLI, and any promoted semi-native deployment
projects must preserve:

- exactly one version originating from common `cem_ml`;
- one command/request schema, normalized run-plan semantics, diagnostic/report
  schemas, exit policy, and source-map model;
- the same command name and option meanings for shared capabilities;
- explicit `wasm-browser`, `wasm-node`, `wasm-node-sea`, `wasm-node-pkg`, or
  native target identity in capability output without changing the common
  version;
- parity fixtures and explicit expected native-only capability differences;
- immutable checksums/provenance linking every deployment artifact to the same
  source commit and common version;
- durable tagged GitHub Release assets for every native and promoted semi-native
  binary, with package channels resolving those versioned assets;
- no automatic fallback between native, npm/WASM, and semi-native runtime
  families; and
- no use of runtime-specific output as an undocumented interchange contract.

The Studio PWA always uses its exact synchronized `@epa-wg/cem-ml-cli`
dependency. It cannot discover or execute an installed OS binary directly. A
later native bridge must be an explicit, permissioned integration such as the
native-messaging option in [`integrations.md`](./integrations.md), with
capability and common-version negotiation.

## Consumer Semantic Theme Contract

CEM Studio must consume `@epa-wg/cem-theme` from this workspace as its only
visual design-token system. The application should:

- import `@epa-wg/cem-theme/dist/lib/css/cem-combined.css` exactly once at the
  app shell;
- place one supported theme scope on the shell: `cem-theme-light`,
  `cem-theme-dark`, `cem-theme-contrast-light`, `cem-theme-contrast-dark`, or
  `cem-theme-native`;
- persist the user's selected mode as a small preference and honor the native
  mode when selected;
- use semantic action, state, surface, layering, stroke, focus, selection,
  typography, timing, coupling, control, shape, and breakpoint tokens;
- keep all CEM components and Studio composites in light DOM;
- never copy generated CEM CSS into the app or introduce another design-token
  or utility-framework vocabulary; and
- verify light, dark, both contrast modes, forced colors, keyboard focus, and
  CEM interaction-size requirements.

Graph node states, diagnostic severity, source selections, diffs, and execution
states must never be communicated by color alone. Studio-specific CSS may define
aliases such as `--cem-studio-graph-running` only by mapping them to an existing
semantic CEM token. If a missing semantic is broadly useful, add it to
`@epa-wg/cem-theme` with its specification and generation tests instead of
inventing a private raw value.

The package-local
[`CEM Theme AI Instructions`](../packages/cem-theme/src/lib/tokens/cem-theme-ai-instructions.md)
remain the styling implementation reference.

## Browser Command Service

### Typed Operations, Not a Shell Process

The browser should call a versioned request API conceptually shaped like:

```ts
import { createCemMlBrowser } from '@epa-wg/cem-ml-cli/browser';

const engine = await createCemMlBrowser({ worker: true });
const result = await engine.run({
    protocolVersion: 1,
    command: 'validate',
    inputs: [
        {
            uri: 'studio://project/catalog/data/catalog.json',
            contentType: 'application/json',
            schema: 'studio://project/catalog/schema/catalog.schema.json',
        },
    ],
    report: { projection: 'json' },
});
```

This is illustrative, not a frozen TypeScript API. The actual request must be a
projection of the normalized library run plan. It should return structured
outputs and a report; terminal text is one optional presentation generated from
that result.

The app may later provide a CLI-like command bar for users who know `cem-ml`
syntax. That parser must map an allowlisted command grammar to the same typed
request. It must not invoke a shell or permit command substitution, arbitrary
JavaScript, native paths, or undeclared network access.

### Command-to-Workbench Mapping

| Native workflow          | Studio presentation                                                                                                                | Browser target                                                               |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| `parse`                  | Source editor plus AST/DOM/event projection tabs and parser-stage report.                                                          | MVP when the selected parser is WASM-capable.                                |
| `validate`               | Inline ranges, diagnostics list, effective schema/identity, and source trace.                                                      | MVP.                                                                         |
| `check`                  | Combined validation and configured policy checks for a project or data set.                                                        | MVP after multi-resource run plans.                                          |
| `inspect`                | Expandable document/resource/schema summary and selected-node details.                                                             | MVP.                                                                         |
| `convert`                | Immutable input on the left, generated output on the right, identity selectors, loss/diagnostic report, and download/copy actions. | MVP after validation.                                                        |
| `query`                  | Data source, explicit query language, query editor, variable/config editor, and typed result preview.                              | Second milestone.                                                            |
| `transform`              | Input data set, transformation config/template, output preview/diff, stage trace, and graph view.                                  | Second milestone.                                                            |
| `trace`                  | Timeline/table over structured observability events with source navigation.                                                        | Second milestone.                                                            |
| `bench`                  | Repeatable browser profile with environment disclosure.                                                                            | Optional; never compare browser numbers directly with native budgets.        |
| `fixture`                | Run bundled or user project examples as tests. File globs and repository parity suites are native-only.                            | Later browser-specific example runner.                                       |
| schema/plugin management | Read-only bundled capability catalog first.                                                                                        | Mutation/installation is later and must be signed, versioned, and sandboxed. |

### Capability Discovery and Parity

The engine must expose a versioned capability manifest containing:

- engine, npm package, WASM ABI, report schema, and schema-package versions;
- available commands and command options;
- supported input, schema, query, transform, formatter, colorizer, and output
  identities;
- unavailable or degraded native capabilities and a reason code;
- resolver schemes and resource limits;
- threading/streaming support; and
- maximum recommended source, result, event, and graph sizes.

The UI must disable or label unsupported choices. It must never silently choose
a different parser, query language, schema, network resolver, or transform
implementation to make an operation appear successful.

Native/WASM parity tests should run the same portable request fixtures through
the Rust API, native CLI projection, and browser package and compare normalized
reports, diagnostics, outputs, and source maps. Environment-specific metadata
may be excluded explicitly.

### Worker Protocol

The page/worker protocol should support:

- `initialize`, capability negotiation, and schema-package preload;
- `run`, progress events, observability events, final result, and structured
  failure;
- a stable request id, project revision, and resource-version map;
- cancellation and stale-result rejection;
- lazy retrieval of large output, trace, graph, and variable chunks;
- worker restart after a fatal error or hard cancellation; and
- deterministic disposal of resource handles and large buffers.

Transfer `ArrayBuffer` payloads where useful rather than repeatedly cloning
large byte arrays. Do not expose raw Rust pointers or WASM memory views as a
public contract.

### Virtual Resources

All project resources should have logical URIs. Suggested schemes are:

| Scheme                                | Meaning                                                                      |
| ------------------------------------- | ---------------------------------------------------------------------------- |
| `studio://<project-id>/<path>`        | Editable local project resource resolved from IndexedDB.                     |
| `sample://<package>/<version>/<path>` | Immutable bundled schema-package/example resource.                           |
| `https://…`                           | Explicit user-selected remote resource, subject to CORS and resolver policy. |
| `blob:`                               | Ephemeral imported/downloaded bytes for the current session only.            |

Human paths are presentation metadata; stable ids and logical URIs own
references. Renaming a tree label must not silently break a transformation
graph. `file://` is not a portable browser resource scheme and should not be
accepted as a hidden alias.

## Project and Subproject Model

One browser origin may contain several workspaces. A workspace owns projects;
projects and subprojects form the navigation tree. Executable resources are
referenced by stable id and URI, not nested as anonymous blobs in every run
configuration.

| Entity               | Purpose                                                                                                                                               | Important fields                                                                                                        |
| -------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| Workspace            | Local collection and preference boundary.                                                                                                             | id, name, active project, storage schema version, created/updated revision.                                             |
| Project              | Top-level portable unit and export/sync boundary.                                                                                                     | id, name, description, root URI, project schema version, child ordering, provider binding.                              |
| Subproject           | Nested feature, scenario, or domain grouping.                                                                                                         | id, parent id, name, description, children, tags.                                                                       |
| Workbench page       | Persisted executable view under a project/subproject. Its kind selects validation, inspection, conversion, query, transformation, graph, or trace UI. | id, parent id, name, kind, run-config id, referenced resources, presentation state, optional latest result-snapshot id. |
| Data set             | Named ordered selection of one or more data resources with effective input defaults.                                                                  | resource ids, content/schema defaults, namespace/module map, variables, base URI.                                       |
| Resource             | Source/config/query/template/schema content.                                                                                                          | id, logical URI, role, source mode, content type, schema, encoding, text/blob or URL reference, revision, hash.         |
| Run configuration    | Shared normalized options for an operation.                                                                                                           | command, inputs, outputs, policy, budgets, formatter/colorizer/report preferences.                                      |
| Conversion           | Input/output identity scenario.                                                                                                                       | input data set, target identities, output options, expected losses/diagnostics.                                         |
| Query                | Query source and explicit query identity.                                                                                                             | language, source resource, data set, variables, expected result.                                                        |
| Transformation       | Template/module/config plus its input and output contracts.                                                                                           | config/template ids, input data set, entry point, query language, parameters, result identity.                          |
| Transformation graph | Nodes and explicit data/control edges over transformations.                                                                                           | graph config id, inputs, stages, exports, edge mapping, layout hints.                                                   |
| Result snapshot      | Optional pinned derived result for comparison or examples.                                                                                            | run-config revision, input hashes, engine/package versions, outputs, report, created time.                              |

Every mutable record needs a stable id, monotonically changing revision, and
timestamps. Every run result needs the exact project revision, resource hashes,
engine/package versions, effective identities, resolver-policy stamp, and run
config provenance required to decide whether it is stale.

### Source Modes

A resource supports these source modes:

| Mode               | Behavior                                                                                                                                       |
| ------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| Inline text        | Edited in Studio and saved transactionally to IndexedDB. Best default for examples, configs, queries, and modest data.                         |
| Inline binary/blob | Imported bytes with explicit content type; previewed through a safe projection.                                                                |
| URL reference      | Stores URL, request policy, declared identity, and optionally a last successful local snapshot. Fetch is explicit and provenance is preserved. |
| Bundled sample     | Read-only versioned app/package asset. “Edit” first clones it into an inline project resource.                                                 |
| Provider resource  | Later remote-storage reference with local working copy, base revision, sync status, and conflict metadata.                                     |

URL credentials, authorization headers, signed URLs, cookies, and bearer tokens
must never be serialized into a project export or ordinary local resource
record. Authentication belongs to a session-scoped provider adapter.

### Mutation and Recovery

- Autosave inline edits after a short debounce and on explicit save.
- Commit related record/resource changes in one IndexedDB transaction.
- Show `saving`, `saved`, `failed`, `offline`, `stale result`, and `conflict`
  states without relying only on color.
- Use a recoverable trash state for project/subproject/resource deletion before
  permanent removal.
- Preserve an undo/redo command history for the current session; later add
  lightweight local checkpoints for structural changes.
- Export a project before destructive migration or when a storage quota warning
  is reached.
- Coordinate concurrent tabs with revision checks and `BroadcastChannel`;
  same-origin browsing contexts and workers can communicate through that API
  ([MDN Broadcast Channel](https://developer.mozilla.org/en-US/docs/Web/API/Broadcast_Channel_API)).

## Initial Feature-Tour Project

On first launch, Studio should install an editable copy of a versioned,
read-only seed named **CEM-ML Feature Tour**. Updating the PWA may install a new
seed version, but must never overwrite the user's copy. “Reset” creates another
copy and names the source seed/version.

```text
CEM-ML Feature Tour
├── 00 Start Here
│   ├── Parse a CEM document
│   ├── Validate and navigate diagnostics
│   └── Inspect AST, DOM, events, report, and source map
├── 01 Content Types
│   ├── CEM data and projections
│   ├── Web and structured data
│   ├── Styling and math/vector data
│   ├── Schemas and schema packages
│   └── Queries, templates, and transforms
├── 02 Validation
│   ├── Valid documents
│   ├── Syntax diagnostics
│   ├── Schema diagnostics
│   └── Multi-source and referenced-schema cases
├── 03 Conversion
│   ├── JSON ⇄ YAML
│   ├── XML/HTML/CEM projections
│   └── Loss, output identity, and source-map examples
├── 04 Query Lab
│   ├── CEM-QL
│   ├── CSS selector
│   └── XPath
├── 05 Transformations
│   ├── Native CEM template
│   ├── CEMT configuration
│   ├── XSLT compatibility example
│   └── Multiple inputs and outputs
├── 06 Transformation Graphs
│   ├── Linear graph
│   ├── Branched graph
│   ├── Collections and exports
│   └── Stage trace, diagnostics, and source provenance
└── 07 Reports and Safety
    ├── Structured report projections
    ├── Resolver policy and budgets
    ├── Remote URL/CORS behavior
    └── Safe HTML preview
```

The Content Types branch must be generated from the bundled capability/package
manifest. At the time of this proposal, the workspace package manifests include:

- CEM-ML, native templates, transform configuration, CEM-QL, CSS selectors, and
  XPath;
- HTML, XHTML, XML, JSON, YAML, CSV, Markdown, CSS, SCSS, SVG, and MathML;
- CEM schemas, schema packages, JSON Schema, Relax NG, XSLT; and
- CEM AST, DOM, and event projections.

The tree should show only capabilities in the loaded WASM build. A package that
is present in the repository but not browser-capable remains visible only as an
explanatory disabled item if that helps teach the difference. Example content
should come from schema-package manifests so samples, formatters, colorizers,
and expected diagnostics do not drift from the engine.

## Application Workbench

### Responsive Layout

An expanded desktop layout should provide:

```text
┌──────────────── command bar / run status / theme / install ────────────────┐
│ Project explorer │ editor or configuration form │ preview / inspector      │
│                  │                              │                          │
├──────────────────┴──────────────────────────────┴──────────────────────────┤
│ diagnostics / report / event trace / graph execution console              │
└─────────────────────────────────────────────────────────────────────────────┘
```

At compact widths, these become navigable panes rather than squeezed columns.
Use CEM semantic breakpoints based on available space, not device names. Preserve
the selected resource, run, diagnostic, and graph node while switching panes.

### Primary Screens

1. **Home and projects:** recent projects, create/import, Feature Tour, storage
   health, offline/update status, and optional provider status.
2. **Project explorer:** accessible tree with project, subproject, data set,
   resource, configuration, conversion, query, transformation, and graph node
   types; keyboard reordering and context actions.
3. **Resource editor:** source, identity, schema, base URI, URL/snapshot, revision,
   and validation state.
4. **Run workbench:** command-specific form generated from the typed request
   schema, with an advanced normalized-config preview.
5. **CLI Command:** editable round-trip command, resolved input/config/output
   panes, copy actions, change preview, and an Apply target for the current,
   existing, or newly named project page.
6. **Results:** structured data, rendered-safe preview, raw text/bytes,
   diagnostics, report, event trace, source trace, and download.
7. **Transformation graph:** authored nodes/edges, validation, runnable stages,
   active/completed/failed state, edge data preview, timings, and click-to-source.
8. **Project settings:** defaults, resolver/network policy, budgets, storage,
   export, provider wishlist integration, and destructive actions.

### CLI Command View and Round Trip

Every executable Workbench page should have a **CLI Command** view. It is both a
reproducibility view and an alternate editor for the page's typed run
configuration.

The view contains:

| Pane         | Contents                                                                                              | Copy behavior                                                                                                              |
| ------------ | ----------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| Command      | Canonical `cem-ml` command generated from the active page, editable as plain text.                    | **Copy command** writes the exact displayed text.                                                                          |
| Inputs       | Ordered resolved inputs with URI/path, content type, schema, revision/hash, and text or safe preview. | Copy one input or all textual inputs; binary inputs offer download, not guessed text.                                      |
| Config       | Authored config plus a separate effective normalized-config view, with inherited/default provenance.  | Copy authored config, effective config, or the selected section.                                                           |
| Output       | Current primary output(s), identity, diagnostics/report summary, and stale/current status.            | Copy textual output, selected output, report, or download binary output.                                                   |
| Reproduction | Command plus a manifest of the referenced inputs/config/output identities and versions.               | **Copy all** produces one redacted plain-text reproduction block; it does not inline secrets or unbounded/binary payloads. |

Clipboard writes must happen from an explicit user action, report success or
failure accessibly, and retain selectable text as a fallback. The asynchronous
Clipboard API is restricted to secure contexts and browsers may impose
additional user-activation/permission rules
([MDN Clipboard API](https://developer.mozilla.org/en-US/docs/Web/API/Clipboard_API)).

#### Command Projections

The same typed run plan has two useful command projections:

1. **Studio command:** uses stable `studio://` and `sample://` logical URIs and
   can be pasted into another CEM Studio command page after its referenced
   project resources are imported or available.
2. **Exported native command:** uses project-relative paths from the portable
   project tree and is runnable after project export with a compatible native
   `cem-ml` version.

The UI must label which projection is displayed. A `studio://` command must not
be presented as directly runnable by a native CLI that has no Studio resolver.
Native command serialization should be generated from the shared command
schema, disclose its minimum CLI/capability versions, and quote arguments for a
selected target shell. Do not hand-maintain a second list of flags in the web
app.

Command generation includes all semantically relevant current inputs, output
specs, transformation/conversion config, explicit content/schema/query
identities, policy/budget choices, and report options. Defaults may be omitted
only when the generated command records the compatible CLI version and omission
cannot change meaning. A “fully explicit” projection should be available for
diagnosis and long-lived reproductions.

#### Editing and Applying a Command

Editing switches the view from generated to draft mode. The browser command
parser continuously produces syntax/option diagnostics and a structured change
preview, but typing alone does not mutate project records or start a run.

The preview must show:

- the parsed command and effective run-plan change;
- inputs, configs, schemas, queries, transformations, and outputs that will be
  reused, relinked, created, or left unresolved;
- values changed from the selected page, including inherited/default values;
- capability, policy, missing-resource, type, and destructive replacement
  diagnostics; and
- whether the last output becomes stale and whether **Apply & Run** will create
  a new result snapshot.

The **Apply to** control is an editable, accessible combobox with these targets:

- **Current page**, including the current transformation or conversion;
- any compatible existing Workbench page in a selectable project/subproject;
- **New page** under the selected project/subproject; or
- a newly typed page name. If that name does not exist, Apply creates it. If it
  already exists, the UI resolves the stable existing id and requires an update
  or save-copy choice rather than creating an ambiguous duplicate.

Page creation happens only on **Apply**, **Apply & Run**, or **Save as new**.
The target parent and page name remain visible before confirmation. Applying to
an existing page updates its typed run configuration and references in one
transaction. Applying a command of a different kind, such as changing a
conversion to a transformation, should default to a new correctly typed page;
replacing an incompatible page requires explicit confirmation.

The command may create a declared output resource as part of the transaction
when the change preview identifies it. A missing input, schema, query,
transformation config, or template is not silently fabricated: the user must
select an existing resource, import it, or explicitly create the proposed
placeholder/content. Failed parsing, unresolved required resources, duplicate
target names, capability gaps, or policy errors leave the project unchanged.

After a successful apply:

- the structured form, command text, and project tree represent the same typed
  run configuration;
- the new/updated page is selected;
- old results remain visible only as clearly stale history until rerun;
- **Apply & Run** executes exactly the applied revision; and
- the whole structural update is undoable according to project recovery rules.

This round trip requires a tested invariant:

```text
page state -> typed run plan -> command -> parsed run plan -> page state
```

The two page states must be semantically equivalent after normalization, apart
from explicitly excluded presentation state and generated ids. Comments or
formatting in manually authored commands may be preserved as draft text, but
the typed run plan remains execution authority.

### Editing Options

| Editor                          | Advantages                                                                                     | Disadvantages                                                                                                           | Recommendation                                                            |
| ------------------------------- | ---------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- |
| Native `<textarea>`             | Small, accessible baseline; dependable text value and selection offsets; no editor dependency. | No syntax highlighting, folding, inline widgets, or rich completion UI.                                                 | MVP source editor behind a reusable adapter.                              |
| `contenteditable` source editor | Can style syntax spans directly.                                                               | Complex selection/IME/clipboard/undo behavior and easy source-offset drift; unsafe if authored text is treated as HTML. | Do not use as the canonical source editor.                                |
| CodeMirror/Monaco-class editor  | Mature language UI, diagnostics, completion, and large-document ergonomics.                    | Bundle size, theming/accessibility integration, worker complexity, and another adapter contract.                        | Evaluate after the textarea workflow and language-service API are stable. |

Editable HTML means editing HTML **source text**. Rendered HTML is a separate,
read-only-by-default preview. Editor adapters must use the same versioned
document model so a richer editor can replace the textarea without changing
project storage or engine requests.

### Preview Rules

- Structured data uses an expandable tree/table plus a raw-source tab.
- Conversion always shows input and output identities, text/bytes, diagnostics,
  loss metadata, and source mapping side by side.
- Transformation shows input data set, effective config/template/query, result,
  stage trace, and diff where meaningful.
- Transformation graphs show both the authored graph and the latest execution
  overlay; graph layout hints are presentation metadata, not execution order.
- Selecting a diagnostic, result node, graph stage, or source-map frame
  navigates to the exact resource/range.
- Large results are paged or lazily expanded. The app must not stringify an
  unbounded tree into the DOM.
- Binary data is never decoded by guessing. Use its declared identity and a
  safe registered projection, otherwise provide metadata and download.

User-authored or generated HTML must not be inserted into the Studio DOM. A
rendered preview should use a sandboxed iframe without `allow-scripts` or
`allow-same-origin` by default, a restrictive preview CSP, bounded resources,
and an explicit opt-in if a future scenario needs more capability. `srcdoc` is
an injection sink and unsandboxed content can access its parent origin
([MDN `srcdoc` security](https://developer.mozilla.org/en-US/docs/Web/API/HTMLIFrameElement/srcdoc)).
Plain source views must use text nodes or equivalent escaped rendering.

## Local-First Persistence

### Storage Choice

“Preserved in local storage” should mean durable browser-local application
storage, not putting full projects into the synchronous `window.localStorage`
API.

| Browser storage            | Use in Studio                                                                                             | Reason                                                                                                                                                                                                                                   |
| -------------------------- | --------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| IndexedDB                  | Canonical local working store for projects, records, source text, blobs, reports, and migration metadata. | Asynchronous, transactional, indexed, and intended for significant structured data/files ([MDN IndexedDB](https://developer.mozilla.org/en-US/docs/Web/API/IndexedDB_API)).                                                              |
| `localStorage`             | Tiny preferences only: last workspace id, theme mode, pane preference, dismissed update notice.           | Synchronous operations block JavaScript and Web Storage is size-limited ([MDN Web Storage](https://developer.mozilla.org/en-US/docs/Web/API/Web_Storage_API)).                                                                           |
| Cache Storage              | Versioned app shell, WASM, bundled capability catalog, and immutable sample assets.                       | Service-worker/offline asset cache, not a transactional project database.                                                                                                                                                                |
| Origin Private File System | Optional later cache for very large blobs or engine scratch data.                                         | Fast worker-oriented storage, but origin-private, quota-bound, and invisible to the user; it is not backup or Git integration ([MDN OPFS](https://developer.mozilla.org/en-US/docs/Web/API/File_System_API/Origin_private_file_system)). |

Suggested IndexedDB stores are `workspaces`, `nodes`, `resources`, `blobs`,
`runs`, `resultSnapshots`, `providerBindings`, `syncQueue`, `trash`, and
`settings`. Schema upgrades must be versioned, transactional, tested against old
fixtures, and recoverable by export when migration fails.

Browser storage is best-effort by default and quotas/eviction differ by browser.
Studio should display `navigator.storage.estimate()`, request
`navigator.storage.persist()` after the user creates/imports meaningful work,
handle quota errors, and keep export prominent. Persistence reduces automatic
eviction risk but is not a backup and users can still clear site data
([MDN storage quotas and eviction](https://developer.mozilla.org/en-US/docs/Web/API/Storage_API/Storage_quotas_and_eviction_criteria)).

### PWA and Offline Contract

The application needs a web app manifest, HTTPS deployment, install icons,
standalone display metadata, and a service worker. PWA installation varies by
browser/OS, so “installable” is a capability with browser-specific presentation,
not a guaranteed custom prompt
([web.dev PWA installation](https://web.dev/learn/pwa/installation)).

The service worker should precache a versioned minimal shell, theme assets,
worker, WASM, capability manifest, and Feature Tour assets. Service workers can
intercept requests and serve cached resources offline
([web.dev service workers](https://web.dev/learn/pwa/service-workers)). Project
records remain in IndexedDB; never copy mutable projects into an opaque static
asset cache.

| Operation                                                  | Offline behavior                                                                            |
| ---------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| Open/edit local project                                    | Fully available.                                                                            |
| Parse/validate/query/transform with bundled WASM resources | Available when the needed engine/package assets are cached.                                 |
| Open bundled Feature Tour                                  | Available after initial installation/cache.                                                 |
| Fetch URL-backed source                                    | Use an explicitly selected last snapshot or report offline; never pretend it is current.    |
| Sync to account provider                                   | Queue only after the provider defines idempotency, base revision, and conflict rules.       |
| Install/update                                             | Online; active work remains on the current app/DB contract until update migration succeeds. |

An update must not activate halfway through an engine run or unsaved edit. Show
that an update is ready, finish/abort active requests, persist state, then reload
under an explicit migration plan.

## URL-Backed Data

URL resources use `fetch()` in the worker or a bounded host resolver. For a
cross-origin response to be readable, the remote server must allow the Studio
origin through CORS; `no-cors` returns an opaque response whose body is not
available to the engine
([MDN Fetch and CORS](https://developer.mozilla.org/en-US/docs/Web/API/Fetch_API/Using_Fetch#making_cross-origin_requests)).

Each fetch should apply:

- explicit user action for the first network read and clear active-network
  status;
- resolver allow/deny/substitution policy and scheme/host/port rules;
- byte, time, redirect, decompression, recursion, and dependency-count budgets;
- declared or response content type with conflict diagnostics, never silent
  extension guessing;
- requested URL, final URL, response metadata, content hash, fetch time, and
  effective policy in provenance;
- optional local snapshot with `ETag`/`Last-Modified` revalidation metadata;
- no ambient cross-origin credentials by default; and
- cache controls that avoid placing private/authenticated content in the service
  worker's shared asset cache.

A hosted fetch proxy is a later service because it introduces SSRF, credential,
privacy, bandwidth, abuse, and content-retention obligations. It must not be a
transparent workaround for CORS.

## Portable Project Format

No single serialization is ideal for IndexedDB, Git review, NoSQL queries,
single-object storage, documentation, and quick sharing. Define one versioned
**logical project model** and several lossless or generated projections.

### Recommended Canonical and Projections

1. Define a CEM-ML Studio project schema. `project.cem` is the preferred
   human-authorable portable manifest and Git source of truth once that schema
   is accepted.
2. Keep sources, schemas, queries, templates, and transformation configs as
   separate files in their native formats. Do not embed every resource into one
   huge manifest.
3. Use normalized JSON records internally for IndexedDB, NoSQL documents, API
   requests, migrations, and deterministic snapshots. JSON is a projection of
   the same project schema, not an alternate semantic model.
4. Generate `README.md` for explanation, example previews, and links. Markdown
   is not a lossless project database.
5. Offer YAML only as an optional import/export projection if there is user
   demand. YAML typing, aliases, and dialect choices make it a poor sole
   canonical store; normalize it through the CEM-ML YAML package and the Studio
   project schema.

Suggested Git/directory layout:

```text
project.cem
README.md                         # optional/generated
data/
schemas/
queries/
transforms/
graphs/
expected/                         # optional pinned result fixtures
.cem-studio/
└── project.lock.json             # generated versions/hashes; optional in Git
```

The manifest references logical relative paths plus content/schema identities.
Provider ids, OAuth details, local absolute paths, signed URLs, UI pane sizes,
and transient run output do not belong in the portable manifest.

### Deployment Format Matrix

| Target                 | Preferred representation                                                                          | Notes                                                                                                         |
| ---------------------- | ------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| Browser local store    | Normalized IndexedDB records plus blobs.                                                          | Efficient mutable working copy; export required for independent backup.                                       |
| Git repository         | `project.cem` plus native resource files and optional generated `README.md`.                      | Reviewable diffs, branches, history, and CI. Keep generated JSON only when another tool requires it.          |
| GitHub Gist            | Small multi-file project with `project.cem`; optionally a single JSON export for simple examples. | Convenient sharing, but weaker hierarchy, project size, and repository workflow.                              |
| S3/object storage      | Versioned ZIP/bundle containing the directory layout, or manifest plus content-addressed objects. | A single bundle simplifies atomic save/download; object sets improve deduplication but need a commit pointer. |
| NoSQL document service | Normalized JSON metadata/documents with large source/output blobs in object storage.              | Queryable account/project indexes; transactions and object consistency need service design.                   |
| Static demo deployment | Read-only `project.cem` and resources bundled with the PWA/site.                                  | Good for Feature Tours and documentation examples.                                                            |
| Documentation          | Generated Markdown plus fenced native sources and links to an importable bundle.                  | Human presentation only.                                                                                      |
| CI interchange         | Deterministic JSON run plan/report and the repository project tree.                               | CI should not reconstruct execution semantics from Markdown.                                                  |

The export contract must include schema version, engine constraints,
capabilities, resource hashes, and migration history. Import validates before
writing, shows conflicts/unsupported capabilities, assigns new local ids where
needed, and performs the final write atomically.

## Permanent Account Storage Wishlist

Account-backed storage is a provider layer over the portable project model. The
MVP remains useful without an account.

```ts
interface StudioProjectProvider {
    capabilities(): Promise<ProviderCapabilities>;
    listProjects(): Promise<RemoteProjectSummary[]>;
    pull(ref: RemoteProjectRef): Promise<ProjectBundle>;
    push(bundle: ProjectBundle, baseRevision?: string): Promise<RemoteRevision>;
    delete?(ref: RemoteProjectRef, baseRevision: string): Promise<void>;
}
```

The actual contract also needs authentication state, optimistic concurrency,
conflicts, progress, cancellation, retry/idempotency, provider metadata, and
secret redaction.

| Provider                         | Advantages                                                                                    | Disadvantages and service requirements                                                                                                                                                | Suggested use                                                                                                                                                                                                                                                                    |
| -------------------------------- | --------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Download/upload local bundle     | No account or server; user controls the file; works across deployments.                       | Manual backup and conflict management; browser file APIs vary.                                                                                                                        | MVP durable portability baseline.                                                                                                                                                                                                                                                |
| S3-compatible object storage     | Durable object/version model; cheap static bundles; content-addressed resources are possible. | Never ship permanent S3 credentials to the page. Needs account service, authorization, conflict metadata, CORS, lifecycle policy, and cleanup.                                        | Hosted project bundles and large artifacts. Use short-lived presigned requests; AWS documents that presigned uploads avoid giving the browser AWS credentials ([AWS S3 presigned uploads](https://docs.aws.amazon.com/AmazonS3/latest/userguide/PresignedUrlUploadObject.html)). |
| NoSQL database plus object store | Natural user/project indexes, metadata queries, sharing flags, quotas, and sync cursors.      | Largest custom-service commitment: schema migrations, tenancy, authorization, backup, deletion/export, cost, privacy, and consistency.                                                | Full CEM account service if product demand justifies it.                                                                                                                                                                                                                         |
| Git repository                   | Open, portable, reviewable, branchable, and CI-friendly.                                      | OAuth/Git provider integration, commit conflicts, binary/large output handling, branch UX, and non-technical-user complexity. Browser-only local Git is not uniformly portable.       | Advanced users and project-as-code deployment. GitHub's Contents API can create/update files but requires revision-aware conflict handling and repository content permissions ([GitHub repository contents API](https://docs.github.com/en/rest/repos/contents)).                |
| GitHub Gist                      | Fast account-backed sharing; multi-file API; revision history; public or secret visibility.   | GitHub-specific authentication, provider limits, flatter organization, secret gists are unlisted rather than a general authorization system, and not suited to large/binary projects. | Small examples, bug reproductions, and shareable demos. The API supports multi-file create/update and requires authentication for writes ([GitHub Gists API](https://docs.github.com/en/rest/gists/gists)).                                                                      |

Provider binding should be optional metadata outside the portable project. A
local project remains fully editable when signed out. The UI distinguishes
`saved locally` from `synced remotely`; those states must never be collapsed
into one checkmark.

### Potential CEM Studio Service

If a hosted service is introduced, keep the browser engine local by default.
The service may provide:

- OAuth/OIDC login and short-lived provider tokens;
- project metadata, sharing policy, quotas, and sync revisions;
- presigned object upload/download URLs;
- optional NoSQL indexes and bundle commit records;
- CORS-safe remote fetch only through explicit policy;
- encrypted-at-rest project storage, deletion/export, retention, and audit
  behavior; and
- server-side CI/preview execution only as a distinct opted-in service with the
  same run contract and version disclosure.

Do not send project sources, queries, transformation inputs, or results to the
service merely for telemetry, installation, or account login.

## Security and Privacy

- Treat all source text, URL responses, imported bundles, transform output,
  schema-package assets, and provider metadata as untrusted.
- Run the engine in a worker and enforce resolver and execution budgets in Rust,
  not only in disabled UI controls.
- Use a restrictive application CSP. WASM compilation may require the specific
  `wasm-unsafe-eval` CSP source expression; do not broaden this to general
  `unsafe-eval`
  ([MDN CSP](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Content-Security-Policy#wasm-unsafe-eval)).
- Do not use `eval`, `Function`, injected module scripts, or event-handler
  attributes for user-authored code.
- Render source as text. Render HTML only in the isolated preview boundary.
- Keep preview origin permissions, navigation, downloads, forms, popups,
  storage, scripts, and network access denied unless a scenario explicitly
  grants the minimum capability.
- Make all network access visible and cancellable; enforce URL and redirect
  policy after every resolution step.
- Never persist provider access tokens, cloud secret keys, arbitrary request
  headers, signed URLs, or captured passwords in exports or ordinary project
  data.
- Redact sensitive values from reports, event traces, recent-project thumbnails,
  crash reports, and share operations by default.
- Sign and version bundled schema packages; a future third-party package loader
  needs provenance, integrity, capability, and trust UI.
- Provide “delete local project,” “clear all local data,” “disconnect provider,”
  and “export my projects” workflows with clear consequences.

## Accessibility and Interaction

- Meet the existing `@epa-wg/cem-components` accessibility contract for names,
  keyboard operation, focus, validation, loading, and live regions.
- Use a real accessible tree/grid/list pattern as appropriate; do not simulate
  the project hierarchy with non-semantic clickable `div` elements.
- Keep source editing, run actions, diagnostic navigation, pane switching,
  graph inspection, resize controls, and context actions keyboard reachable.
- Announce run start/completion/cancellation and diagnostic-count changes without
  flooding live regions with every parser event.
- Provide text/icon/state alternatives for graph and diagnostic colors.
- Respect reduced motion, forced colors, zoom, text reflow, and CEM coupling
  safety zones.
- Keep a native textarea fallback if a richer editor cannot meet the required
  accessibility or browser capability.

## Performance and Reliability

- Split the app shell from the engine/schema-package payload and load packages
  on demand where practical.
- Preload the minimum Feature Tour capability set; disclose downloads before
  loading unusually large packages.
- Debounce validation while typing, validate immediately on explicit run/save,
  and cancel superseded requests.
- Cache by engine/package version, normalized run config, resource hashes,
  policy stamp, and output identity. Never reuse a result across a mismatched
  resolver or schema identity.
- Bound source, graph, trace, preview, report, and retained-result sizes and
  explain limit diagnostics.
- Stream/lazily page large results and release buffers when tabs or projects
  close.
- Recover from a crashed worker without losing edits; only the current run may
  be lost.
- Surface offline, quota, migration, worker, package-load, CORS, and provider
  failures as distinct actionable states.

## Verification Strategy

1. **Rust contract tests:** command requests, normalized plans, resolver policy,
   reports, diagnostics, source maps, cancellation, and limits remain native
   tests first.
2. **WASM parity tests:** run portable cases in native and WASM and compare
   normalized structured results.
3. **Browser package tests:** worker lifecycle, transfers, progress ordering,
   cancellation, URL resolver, capability negotiation, command
   parse/serialize/normalize round trips, and native-path versus Studio-URI
   projections.
4. **Component tests:** the pinned Angular Material parity matrix verifies that
   every equivalent Studio need is supplied by a completed general
   `<cem-element>`-based CEM component. Studio-only components cover
   accessibility, keyboard, theme modes, loading/error/empty/stale/conflict
   states, and light-DOM rules, and record the absence of an Angular counterpart.
5. **Persistence tests:** IndexedDB schema creation, every migration, atomic
   autosave, quota failures, multi-tab revision conflicts, trash/restore, and
   export/import round trips. Cover Apply to current/existing/new page,
   duplicate names, incompatible command kinds, unresolved resources, atomic
   failure, stale results, and undo.
6. **PWA browser tests:** install metadata, offline reload, service-worker
   update, cached WASM/sample execution, and no project loss across update.
7. **Security tests:** active HTML, unsafe URLs, redirects, oversized/decompression
   inputs, malicious bundles, CSP, preview sandbox, resolver limits, and secret
   redaction.
8. **Feature Tour drift tests:** generate the tree from the actual bundled
   capability manifest and execute every advertised example.
9. **Npm distribution tests:** pack/install `@epa-wg/cem-ml`,
   `@epa-wg/cem-ml-cli`, and `@epa-wg/cem-studio` separately into clean
   temporary consumers. Verify the runtime exports, CLI browser/Node exports,
   npm `cem-ml` executable, Studio bootstrap/static artifact, package contents,
   exact internal dependency versions, and one resolved engine runtime on the
   supported Node/browser/OS matrix.
10. **Native target distribution tests:** build and test the Linux AMD64,
    Homebrew ARM64, and Windows AMD64 deployment subprojects independently;
    verify target identity, archive/package contents, checksums/signatures,
    machine version/capability output, and install/upgrade/uninstall on that
    exact OS/architecture/ABI. Verify every binary and its integrity/provenance
    companions appear on the matching tagged GitHub Release.
11. **Release-family tests:** before promotion, verify that every npm package,
    native target artifact, package-manager projection, capability manifest,
    service-worker build id, checksum, SBOM, and provenance record contains the
    exact version from common `cem_ml` and points to the same release commit.

All build, test, lint, browser, and PWA verification should be exposed as Nx
targets on the eventual projects.

## Dependency-Ordered Delivery Plan

This ordering is a design proposal. It does not add work to the active roadmap
or todo list by itself.

1. **Portable project contract.** Define the logical project model, stable ids,
   CEM-ML schema, JSON projection, logical URI rules, bundle layout, migration,
   and import/export validation.
2. **Library command boundary.** Move typed CLI-independent parse, validate,
   inspect, convert, query, transform, trace, report, capability, cancellation,
   and resolver requests into `cem_ml` with native contract tests.
3. **Common version and release-family contract.** Keep the authoritative
   version in common `cem_ml`; define the fixed `cem-ml-platform` release graph,
   version synchronizer, exact internal dependency policy, staged promotion,
   release index, and cross-package/version verification gates.
4. **WASM runtime npm deployment.** Create the separate `@epa-wg/cem-ml`
   subproject with its `/wasm` runtime, generated types, schema-package assets,
   ABI/capability manifest, integrity records, and native/WASM parity fixtures.
   It has no CLI launcher or npm `bin`.
5. **Universal CLI npm deployment.** Create the separate
   `@epa-wg/cem-ml-cli` subproject with an exact same-version dependency on
   `@epa-wg/cem-ml`, `/browser` and `/node` exports, shared command parsing,
   generated command types, and the npm `cem-ml` executable.
6. **Native target deployment projects.** Establish exactly three initial Nx
   subprojects: Linux AMD64, macOS ARM64 through Homebrew, and Windows AMD64.
   Produce independently signed/checksummed artifacts and platform package
   metadata, preserve the binaries and integrity/provenance companions on the
   tagged GitHub Release, and use thin APT or Homebrew index aggregation where
   required.
7. **Worker engine client.** Implement initialization, request/progress/result,
   transfers, cancellation, stale-result rejection, lazy payloads, restart, and
   JS resolver adapters in `@epa-wg/cem-ml-cli/browser`.
8. **Angular parity prerequisites and themed Studio frame.** Classify every
   required control against the pinned Angular Material catalog. Land and verify
   the general `<cem-element>`-based CEM component first wherever a counterpart
   exists; only then compose it in Studio. Add `/studio` components for
   workbench behavior with no Angular counterpart, using `@epa-wg/cem-theme`
   exclusively.
9. **Studio npm/PWA deployment.** Create the separate publishable
   `@epa-wg/cem-studio` Nx subproject with an exact same-version dependency on
   `@epa-wg/cem-ml-cli` plus tested-compatible component/theme dependencies.
   Add the responsive theme scope/mode selector, manifest, static deployment
   artifact, service worker, offline shell, update flow, and worker loading.
10. **Local project store.** Add IndexedDB repositories/migrations, autosave,
    trash/restore, multi-tab revision checks, storage persistence/health, and
    validated import/export.
11. **Generated Feature Tour.** Build the editable seed from bundled
    schema-package examples and capability manifests; add drift and execution
    tests.
12. **Parse/validate/inspect MVP.** Deliver inline/URL resources, data sets,
    source identities, diagnostics/range navigation, AST/DOM/event/report/source
    map previews, offline behavior, and the CLI Command view with copy plus
    current/existing/new-page round trip.
13. **Conversion workbench.** Add input/output identity selectors, side-by-side
    previews, losses, source trace, copy/download, and pinned expected results.
14. **Query and transformation workbench.** Add explicit query-language
    selection, variables, templates/configs, outputs, diffs, trace, and relevant
    Feature Tour scenarios.
15. **Transformation graph.** Add graph editing/validation, stage execution
    overlay, edge/scoped-data preview, source navigation, collections, and
    exports.
16. **Hardening and richer editing.** Profile bundle/memory costs, add large-data
    paging, richer editor adapter if justified, accessibility/contrast coverage,
    CSP and sandbox tests, migration recovery, supported Node/browser/OS matrix,
    npm installation/packing checks, and browser support disclosure.
17. **Semi-native executable experiment.** After the Node/WASM CLI is stable,
    prototype target-specific self-contained executables with Node SEA for the
    same Linux AMD64, macOS/Homebrew ARM64, and Windows AMD64 matrix. Compare
    deprecated `pkg` only for migration knowledge; gate adoption on size,
    performance, asset loading, signing, security-update, and installation
    results.
18. **Account-storage providers.** Start with one validated need—likely GitHub
    Gist for small sharing or Git repositories for project-as-code—then add S3
    bundles or a NoSQL account service behind the same revisioned provider
    contract.

## Decisions to Preserve

- One CEM-ML semantic engine; native CLI and browser Studio are adapters.
- Common `cem_ml` is the sole version authority for the complete fixed
  `cem-ml-platform` release family.
- `@epa-wg/cem-ml`, `@epa-wg/cem-ml-cli`, and `@epa-wg/cem-studio` are separate
  public npm deployment subprojects, but they always publish the exact common
  CEM-ML version from the same release commit.
- `@epa-wg/cem-ml` owns only the low-level WASM runtime; it has no npm CLI
  launcher or `bin`.
- `@epa-wg/cem-ml-cli` owns the browser and Node CLI adapters plus the npm
  `cem-ml` executable, and depends on the exact same-version
  `@epa-wg/cem-ml` package.
- `@epa-wg/cem-studio` depends on the exact same-version
  `@epa-wg/cem-ml-cli`; it receives the runtime transitively and must not resolve
  another copy independently.
- Common `cem_ml_cli` owns native CLI source but is not a deployment package.
  Linux AMD64, Homebrew ARM64, and Windows AMD64 each have their own Nx
  subproject and deployment package, all stamped with the exact common version.
- The initial CLI support matrix contains only WASM for Node, Linux AMD64,
  Homebrew ARM64, and Windows AMD64.
- Every native binary is preserved as a versioned asset on the tagged GitHub
  Release with its checksum, signature, SBOM, provenance, and target identity;
  package-manager projections resolve those immutable assets.
- Target-specific executables that bundle Node plus WASM through Node SEA are a
  wishlist item and remain `wasm-node`-derived rather than native Rust. Archived,
  deprecated `pkg` is only a comparison or migration reference.
- UI-only, npm-CLI-only, or target-packaging publication still advances the
  common version and rebuilds the complete release family; members have no
  independent release cadence.
- Every version/capability response identifies its precise WASM, semi-native, or
  native runtime and target; no distribution silently falls back to another
  runtime.
- Browser commands are typed operations, not an arbitrary shell.
- The CLI Command view and structured Workbench forms are lossless projections
  of the same normalized run plan.
- Editing a command does not mutate the tree; Apply targets a stable existing
  page or explicitly creates a named page transactionally.
- The browser exposes capability differences instead of hiding them.
- Projects are local-first and useful without an account or network.
- IndexedDB owns mutable local projects; `localStorage` owns only small
  preferences.
- Built-in examples are versioned seeds; updates never overwrite user copies.
- Content/schema/query/transform identities are explicit and never guessed.
- Logical URIs and stable ids survive tree-label changes.
- `@epa-wg/cem-components` owns reusable functional UI;
  `@epa-wg/cem-components/studio` owns Studio-specific reusable composites; the
  PWA owns orchestration.
- An Angular-Material-equivalent control is implemented and verified first as a
  general `<cem-element>`-based CEM component. Studio-first components are
  allowed only when the pinned parity matrix records no Angular counterpart.
- `@epa-wg/cem-theme` is the sole visual semantic/token system, applied once at
  the light-DOM app shell.
- Preview output is untrusted, sandboxed, bounded, and non-scriptable by default.
- URL access is explicit, policy-controlled, CORS-aware, and provenance-rich.
- `project.cem` plus native resource files is the preferred portable/Git form;
  JSON is the normalized browser/API/NoSQL projection; Markdown is generated
  presentation; YAML is optional interchange.
- Import/export precedes remote sync so users always have a provider-independent
  escape hatch.
- S3, NoSQL, Git, and Gist persistence are optional providers, not alternate
  project semantics.

## Open Contract Questions

Phase 2.5 resolved the runtime/CLI deployment roots and identities, fixed Nx
release group and Cargo-derived version synchronization, Node/host matrix,
native signing and package channels, first-release capability matrix, and
versioned host protocol. Those decisions are owned by the canonical
[`cem-ml-deployment-contract.md`](./cem-ml-deployment-contract.md), not by this
Studio proposal.

- What executable-size, startup, performance, signing, asset-loading, and
  security-update thresholds must a Node SEA semi-native package meet before it
  leaves the wishlist?
- What is the accepted CEM-ML schema/content identity for a Studio project
  manifest?
- Which source/result size limits are safe defaults for desktop and mobile
  browsers?
- Is textarea sufficient for the first public release, or is schema-driven
  completion a launch requirement?
- Which output formats may be rendered, and which only receive text/tree/download
  previews?
- Which first permanent provider has validated user demand and a sustainable
  authentication/service model?

## Research References

- [web.dev: Learn PWA](https://web.dev/learn/pwa/welcome)
- [web.dev: PWA installation](https://web.dev/learn/pwa/installation)
- [web.dev: Service workers](https://web.dev/learn/pwa/service-workers)
- [MDN: IndexedDB API](https://developer.mozilla.org/en-US/docs/Web/API/IndexedDB_API)
- [MDN: Web Storage API](https://developer.mozilla.org/en-US/docs/Web/API/Web_Storage_API)
- [MDN: Storage quotas and eviction](https://developer.mozilla.org/en-US/docs/Web/API/Storage_API/Storage_quotas_and_eviction_criteria)
- [MDN: Origin Private File System](https://developer.mozilla.org/en-US/docs/Web/API/File_System_API/Origin_private_file_system)
- [MDN: Web Workers](https://developer.mozilla.org/en-US/docs/Web/API/Web_Workers_API/Using_web_workers)
- [MDN: Clipboard API](https://developer.mozilla.org/en-US/docs/Web/API/Clipboard_API)
- [MDN: Fetch and CORS](https://developer.mozilla.org/en-US/docs/Web/API/Fetch_API/Using_Fetch#making_cross-origin_requests)
- [MDN: iframe `srcdoc` security](https://developer.mozilla.org/en-US/docs/Web/API/HTMLIFrameElement/srcdoc)
- [MDN: Content Security Policy](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Content-Security-Policy)
- [npm: `package.json` `bin` field](https://docs.npmjs.com/cli/v11/configuring-npm/package-json#bin)
- [Node.js: Single executable applications](https://nodejs.org/api/single-executable-applications.html)
- [Vercel: archived and deprecated `pkg`](https://github.com/vercel/pkg)
- [GitHub: About releases and binary assets](https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases)
- [Angular Material: Component catalog](https://material.angular.dev/components/categories)
- [Homebrew: Formula Cookbook](https://docs.brew.sh/Formula-Cookbook)
- [Debian: Package management](https://www.debian.org/doc/manuals/debian-reference/ch02)
- [Rust: Installing binaries with `cargo install`](https://doc.rust-lang.org/book/ch14-04-installing-binaries.html)
- [GitHub: REST API for Gists](https://docs.github.com/en/rest/gists/gists)
- [GitHub: REST API for repository contents](https://docs.github.com/en/rest/repos/contents)
- [AWS: Uploading S3 objects with presigned URLs](https://docs.aws.amazon.com/AmazonS3/latest/userguide/PresignedUrlUploadObject.html)

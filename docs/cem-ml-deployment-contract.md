# CEM-ML Deployment Contract

**Status:** Canonical Phase 2.5 deployment contract, accepted 2026-08-11.

This document fixes the deployment, version, platform, capability, host-wire,
signing, and promotion boundaries that Phase 2.5 implementation must follow.

## Outcome

Phase 2.5 turns the existing common `cem_ml` engine and `cem_ml_cli` native
source projects into one version-synchronized product family with five new
deployment projects:

- a low-level `@epa-wg/cem-ml` WASM runtime npm package;
- a universal `@epa-wg/cem-ml-cli` npm package for browser and Node hosts;
- native Linux AMD64, macOS ARM64/Homebrew, and Windows AMD64 packages.

The common engine remains the semantic implementation. Deployment packages add
bindings, host adapters, packaging, integrity metadata, and distribution
policy; they do not copy parser, validator, query, transform, or report logic.

## Project identities and ownership

| Responsibility | Workspace root | Nx project | Public identity |
| --- | --- | --- | --- |
| Common engine and version authority | `packages/cem_ml` | `cem_ml` | Rust crate `cem-ml` |
| Common native CLI source/adapter | `packages/cem_ml_cli` | `cem_ml_cli` | Rust crate/binary `cem-ml-cli` / `cem-ml` |
| Low-level WASM npm deployment | `packages/cem-ml-npm` | `@epa-wg/cem-ml` | npm `@epa-wg/cem-ml` |
| Universal browser/Node CLI npm deployment | `packages/cem-ml-cli-npm` | `@epa-wg/cem-ml-cli` | npm `@epa-wg/cem-ml-cli`; bin `cem-ml` |
| Linux AMD64 native deployment | `packages/cem-ml-cli-native-linux-amd64` | `cem_ml_cli_native_linux_amd64` | runtime `native-linux-amd64` |
| macOS ARM64/Homebrew deployment | `packages/cem-ml-cli-native-brew-arm64` | `cem_ml_cli_native_brew_arm64` | runtime `native-macos-arm64` |
| Windows AMD64 native deployment | `packages/cem-ml-cli-native-windows-amd64` | `cem_ml_cli_native_windows_amd64` | runtime `native-windows-amd64` |

Folder names identify deployment concerns and may differ from the public npm
name. The two existing underscore-named Rust roots remain unchanged. Each new
root is one Nx project with its own build, package, verify, and publish targets;
no single project hides an OS/architecture matrix behind a shell switch.

The dependency graph is fixed:

```text
cem_ml
├── cem_ml_cli
    ├── cem_ml_cli_native_linux_amd64
    ├── cem_ml_cli_native_brew_arm64
    └── cem_ml_cli_native_windows_amd64
└── @epa-wg/cem-ml
    └── @epa-wg/cem-ml-cli
```

`@epa-wg/cem-studio` joins the release family later but is not a Phase 2.5
project. The npm CLI receives the WASM runtime through one exact direct
dependency. Studio and downstream consumers must not add a second private
runtime copy.

## Version and release authority

`packages/cem_ml/Cargo.toml` `[package].version` is the sole authoritative
CEM-ML product version. Every other Cargo manifest, npm manifest, exact internal
dependency, runtime response, capability manifest, checksum record, SBOM,
provenance record, and release index is a projection of that value.

The workspace adds a fixed Nx release group named `cem-ml-platform`. Nx fixed
groups keep members on one version and update in-group dependencies together;
the group is separate from the existing `cem` group because it has a different
version authority and release tag. The group uses `cem-ml-v{version}` tags so it
cannot collide with the existing `{version}` CEM package tags. See the current
[Nx release-group contract](https://nx.dev/docs/guides/nx-release/release-groups).

The release sequence is:

1. Author the intended version only in `packages/cem_ml/Cargo.toml`.
2. Run an Nx-owned write target that copies that exact version into every
   projection and exact internal dependency.
3. Run a separate read-only Nx verification target that fails on any drift and
   proves the working tree is unchanged.
4. Run Nx Release for only `cem-ml-platform`, supplying the exact authoritative
   Cargo version rather than deriving another value from conventional commits
   or an independent version plan.
5. Build and verify all artifacts from the tagged commit, stage a draft GitHub
   Release, attach the complete asset/index set, and publish it only after every
   artifact verifies.

The sync target may write during an intentional version-preparation change. The
verification and release gates never repair drift. Failed publication may retry
the same immutable version and bytes; changed bytes require a new version.

## Supported runtime matrix

The first npm release supports Node `^22.12.0 || ^24.0.0`. At acceptance
(2026-08-11), Node 22 and 24 are the supported LTS lines while Node 20 is EOL;
Node 26 remains Current until its LTS promotion. The repository's Nx 22 line
also supports Node 24 and `^22.12.0`. This intentionally excludes Current and
EOL Node lines from the production contract even when they happen to run. See
the [Node release table](https://nodejs.org/en/about/previous-releases) and
[Nx Node compatibility matrix](https://nx.dev/docs/technologies/node/introduction).

| Distribution | Required verification hosts | Runtime identity |
| --- | --- | --- |
| Universal npm CLI | Ubuntu x64 on Node 22 and 24; macOS ARM64 and Windows x64 on Node 24 | `wasm-node` |
| Browser API | Current project Playwright Chromium plus a real dedicated worker | `wasm-browser-worker` |
| Linux native | Ubuntu 24.04 x64; `x86_64-unknown-linux-gnu` | `native-linux-amd64` |
| Homebrew native | GitHub-hosted `macos-14` ARM64; `aarch64-apple-darwin` | `native-macos-arm64` |
| Windows native | GitHub-hosted `windows-2025` x64; `x86_64-pc-windows-msvc` | `native-windows-amd64` |

GitHub currently provides x64 Ubuntu/Windows runners and ARM64 macOS runners,
so each release artifact is built and smoke-tested on its native architecture
rather than cross-signed on an unrelated host. See the
[GitHub-hosted runner matrix](https://docs.github.com/en/actions/reference/runners/github-hosted-runners).

Node support is reviewed at each CEM-ML minor release. A newly promoted Node LTS
line is not claimed until the complete npm/browser matrix passes; an EOL line is
removed in the next CEM-ML minor or major permitted by compatibility policy.

## WASM and npm boundaries

The WASM npm build pins the Rust toolchain through `rust-toolchain.toml` and pins
the `wasm-bindgen-cli` version to the resolved `wasm-bindgen` crate version. It
generates release-mode browser and Node loaders from the common crate, TypeScript
declarations, and one package-owned capability/ABI manifest. The generated
outputs are package artifacts, not sources of semantic behavior.

`@epa-wg/cem-ml` exposes low-level runtime entry points under `./wasm` and a
Node loader used by the CLI package. It has no npm `bin`, terminal parser,
filesystem policy, UI state, or independent release cadence. `wasm-bindgen`
supports distinct web and Node deployment targets and generated TypeScript
declarations; see its [deployment](https://wasm-bindgen.github.io/wasm-bindgen/reference/deployment.html)
and [CLI](https://wasm-bindgen.github.io/wasm-bindgen/reference/cli.html)
documentation.

`@epa-wg/cem-ml-cli` is an ESM npm package with:

- `./browser`: dedicated-worker client, typed request/event/result API,
  resolver bridge, progress, cancellation, and capability discovery;
- `./node`: Node filesystem/URL/stream/signal adapters over the same request
  model;
- `cem-ml`: the Node/WASM executable using the same command grammar and exit
  policy as the native CLI.

The npm package contains one exact dependency on `@epa-wg/cem-ml`; clean-install
verification rejects a second resolved version or bundled private runtime.

## First-release capability matrix

Availability means the operation reaches the same common engine request and
result contract. Host I/O and presentation may differ only where the capability
manifest says so.

| Operation | Native CLI | Node/WASM CLI | Browser API | First-release rule |
| --- | --- | --- | --- | --- |
| `parse` | Required | Required | Required | Same typed parse result and source maps |
| `validate` / `check` | Required | Required | Required | Same diagnostics, fail policy, and report result |
| `inspect` | Required | Required | Required | Same projections; host chooses display/export |
| `convert` | Required | Required | Required | Same registered adapters; host resolver differences explicit |
| `query` | Required | Required | Required | CSS selector, CEM-QL, and XPath only when advertised |
| `transform` | Required | Required | Required | Only implemented CEMT/CEM-QL/XSLT compatibility capabilities advertised |
| `trace` | Required | Required | Required | Same bounded event model; terminal rendering is host-owned |
| version/capabilities | Required | Required | Required | Exact common version plus runtime/target/ABI identity |
| `bench` | Native only | Unavailable | Unavailable | Results are host-specific and cannot satisfy parity |
| `fixture *` | Development targets only | Development targets only | Unavailable | Not a public consumer capability |
| `schema *` / `plugin *` mutation | Unavailable | Unavailable | Unavailable | Reserved commands stay explicit capability gaps |

Native local-file resolution is built in. Node local-file and HTTPS resolution
are built in with the existing policy boundary. Browser requests use supplied
virtual resources and explicit URL resolver callbacks; they never receive an
implicit filesystem capability. Network, plugin, schema-installation, and
process-spawn capabilities remain unavailable unless a later reviewed contract
adds them.

## Versioned machine contract

The host contract is a typed API with an explicit wire projection, not an
internal JSON object model. Common Rust request/result types retain native AST,
event, item, artifact, and source-map ownership until a host boundary explicitly
projects them.

The canonical operation-handle, scoped cancellation, pause/resume, worker-pool,
resource-failure, and debugger extensions to this envelope are defined in
[`cem-ml-operation-control-design.md`](cem-ml-operation-control-design.md).
Where the compact message table below omits an operation-control field or
status, that later focused design is authoritative.

The first protocol family has these messages:

| Message | Required fields |
| --- | --- |
| `initialize` | protocol range, host/runtime identity, requested capabilities, resolver-policy identity, transfer limits |
| `initialized` | selected protocol, common version, WASM ABI, report/schema-package versions, runtime/target identity, supported operations and gaps, effective limits |
| `run` | request id, project/input revision, operation, normalized run request, input/output specs, policy/budget stamp |
| `progress` | request id, monotonic sequence, stage, completed/total where knowable, bounded message/details |
| `event` | request id, monotonic sequence, versioned observability event |
| `result` | request id, status, typed result/artifact handles, diagnostics, report, source maps, effective identity/version stamps |
| `cancel` | request id and reason |
| `cancelled` | request id, terminal status, retained/discarded artifact statement |
| `fatal` | protocol/runtime failure, restartability, bounded diagnostics |

Required rules:

- request ids are unique within one initialized host session;
- progress/event sequence numbers are monotonic per request;
- exactly one terminal `result`, `cancelled`, or `fatal` follows an accepted
  request;
- cancellation is cooperative first, with worker/process termination as the
  hard fallback; partial outputs are never committed silently;
- superseded project/input revisions are rejected or returned as stale without
  mutating current state;
- all arrays, strings, byte transfers, diagnostics, events, artifacts, and
  source maps are bounded by values disclosed during initialization;
- unknown protocol majors fail negotiation; unknown additive fields on the
  same major are ignored and preserved only at explicit report/export
  boundaries;
- native and npm executables retain the existing exit-code contract; browser
  callers receive the same stable status code without a process exit;
- JSON/JSONL is allowed for explicit machine reports and observability exports,
  while browser workers prefer structured clone and transferable byte buffers.

The existing run-config, report, observability, diagnostics, and source-map
schemas remain canonical inputs to this envelope. Phase 2.5 adds protocol and
capability schemas; it does not duplicate them as npm-only DTOs.

### Command service protocol v1

Accepted 2026-08-13: the universal CLI uses one host-neutral command-service
contract owned by common Rust. TypeScript declarations must be generated
projections of the implementing Rust types; browser and Node adapters must not
introduce a second request or result model. The field names below are the exact
structured-clone/JSON projection for protocol version 1.

`CommandServiceRequestV1` has these required fields:

| Field | Type and rule |
| --- | --- |
| `protocolVersion` | The integer `1`. |
| `requestId` | Non-empty session-unique identity, at most `MAX_IDENTITY_BYTES` UTF-8 bytes. |
| `project` | `{ projectId, revision }`; `projectId` has the identity bound and `revision` is an integer from `0` through `9007199254740991`. |
| `resourceVersions` | URI-keyed map of `{ revision, sha256 }` for every resource the operation may read. Revisions use the same safe-integer range and `sha256` is exactly 64 lower-case hexadecimal characters. |
| `operation` | The discriminated `PortableOperationRequestV1` union below. |
| `runPlan` | The common `NormalizedRunPlan`, or `null` only for `version-capabilities`. It owns input/output specs, resolver declarations, scheduler policy, budgets, diagnostics mode, and report destinations. |
| `resources` | URI-keyed map of `VirtualResourceV1` values containing `bytes: Uint8Array` and optional common `FormatIdentity`. Every entry must have an identical key in `resourceVersions`; the SHA-256 of `bytes` must match before execution. |
| `policyStamp` | `{ resolver, safety, budget }`; each value is a non-empty bounded identity for the effective host policy, not executable policy text. |

Resource URI keys are non-empty and no longer than `MAX_SOURCE_URI_BYTES`.
`resources` contains at most the negotiated
`maxTransferBuffersPerMessage` entries and its aggregate bytes do not exceed
`maxTransferBytesPerMessage`. `resourceVersions`, diagnostics, and returned
references are bounded by the negotiated operation-host limits. A missing,
duplicate after decoding, over-bound, digest-mismatched, or version-mismatched
entry fails admission before engine state is created.

`PortableOperationRequestV1` is a `kind`-discriminated union. Sources refer to
`NormalizedRunPlan.inputs[*].inputId` or to a URI present in
`resourceVersions`; live engine contexts, abort signals, resolver functions,
scheduler scope ids, and native handles are never request fields.

| `kind` | Additional fields | Common operation projection |
| --- | --- | --- |
| `parse` | `inputId`, `projection`, `preserveSourceOffsets` | `ParseRequest` |
| `validate` | `inputIds`, `projection` | `ValidateRequest` |
| `check` | `inputIds`, `projection`, `zeroHardViolations` | `CheckRequest` |
| `inspect` | `inputId`, `show` | `InspectRequest` |
| `convert` | `inputId`, `toFormat`, `preserveSourceOffsets` | `ConvertRequest` |
| `query` | `dataInputId`, `queryUri`, `output` | `QueryRunRequest` and its registered exporter |
| `transform` | `source`, `params`, `templateEntrypoint`, `preserveSourceOffsets`; `source` is either `{ kind: "direct", dataInputId, templateUri }` or `{ kind: "graph", configUri }` | `TransformRequest` or `TransformGraphRequest` |
| `trace` | `inputId`, `projection` | `TraceRequest` |
| `version-capabilities` | No additional fields | `ProductVersion` and `CapabilityManifest` |

For a direct transform source, `params` and `templateEntrypoint` configure the
projected `TransformRequest`. For a graph source, `params` MUST be empty and
`templateEntrypoint` MUST be implicit because each graph `transform` stage owns
its params and entrypoint. Command admission rejects an override with stable code
`cem.command_service.transform_graph_stage_local`.

All enum values use the serialization already owned by the named common Rust
type. Fail level, target identity/scope, output pipeline, resolver bindings,
budgets, report projection/destinations, terminal presentation preferences, and
the effective config come only from `runPlan`; the operation union does not
duplicate them. Only the nine portable capability paths are admitted by v1.

`CommandServiceResultV1` has these fields:

| Field | Type and rule |
| --- | --- |
| `protocolVersion`, `requestId`, `project`, `resourceVersions` | Exact echoes of the admitted request identity and snapshot. |
| `operation` | The admitted operation `kind`. |
| `status` | `succeeded`, `failed`, `cancelled`, `fatal`, or `stale`. Exactly one terminal result is emitted. |
| `exitCode` | `0`, `1`, `2`, `3`, `6`, `7`, or `130` under the existing CLI policy; `null` for `stale`. It is data until the npm or native executable projects it to a process exit. |
| `result` | Optional `CommandPayloadV1<T>` containing the typed common operation result. |
| `diagnostics` | Common `BoundedList<Diagnostic>`. |
| `report` | Optional `CommandPayloadV1<Report>`. Terminal text is a host presentation derived from this report, never the report model. |
| `artifacts` | Common `BoundedList<CommandArtifactHandleV1>` for outputs, reports, traces, graphs, and other retained payloads. |
| `sourceMaps` | Common `BoundedList<CommandSourceMapReferenceV1>` associated with result or artifact handles. |
| `identity` | Effective common version, runtime, target, ABI, schema-package, resolver-policy, safety-policy, and budget-policy stamps. |
| `stale` | Required only when `status` is `stale`: `{ currentProjectRevision, changedResources }`, where each changed resource carries its current `{ uri, revision, sha256 }`. |

`CommandPayloadV1<T>` is exactly either `{ storage: "inline", value: T }` or
`{ storage: "artifact", handle: CommandArtifactHandleV1 }`. Inline values must
fit the negotiated transfer bounds. `CommandArtifactHandleV1` is request-scoped
and contains `handleId`, `kind`, optional logical `uri`, `contentType`,
`byteLength`, `sha256`, and optional `sourceMapId`. Its `kind` is `output`,
`report`, `source-map`, `trace`, `graph`, or `variables`.
`CommandSourceMapReferenceV1` contains `sourceMapId`, an `owner` discriminated as
the operation `result` or an `artifact` `handleId`, and a
`CommandPayloadV1<SourceMapStack>`. Large payload retrieval and deterministic
handle disposal are service methods keyed by `requestId` and `handleId`; raw
Rust pointers and WASM memory views never cross this boundary.

Prepared `convert` and direct `transform` operations may fan out across the
normalized output records. Their typed result is therefore a
`CommandFanoutResultV1<T>` containing a negotiated `BoundedList` of
`{ outputId?, destination?, response }` entries in preparation order; it is not
a singular engine response. The list is non-empty and cannot exceed
`maxArtifactReferences`. Transform-graph results keep their graph-owned artifact
aggregate because graph export identities and destinations are already carried
by `TransformGraphResponse`.

The host supplies a read-only current project/resource revision ledger when it
constructs the service; the service owns both freshness comparisons. It checks
the same snapshot immediately before admission and immediately before
transactional publication. If either comparison is obsolete, the service
returns `stale`, rolls back or discards every staged output, emits no published
artifact handle, and does not mutate current project state. A normal failure or
cancellation likewise cannot silently commit partial output.

Resolver bindings are constructor-time host capabilities, not serializable
request properties. A browser host may provide explicit async read and
transactional write callbacks; Node may additionally install the accepted file
and HTTPS adapters. A read returns bytes, identity, revision, and digest and is
accepted only when it matches `resourceVersions`. Writes use prepare/commit/
rollback and pass the freshness check before commit. Resolver callbacks,
filesystem paths, streams, environment, signals, stdout/stderr, and process
exit stay in their host adapter. Only the Node npm executable and native binary
may terminate a process.

The low-level WASM deployment exposes this lifecycle through the generated
asynchronous `executeCommandServiceV1` binding. It accepts only the canonical
request JSON, common capability request, and constructor-supplied current-
revision, read, prepare-write, commit-write, and rollback-write callbacks.
Callback request and response records serialize the Rust host-capability types;
write bytes travel separately as a `Uint8Array`. The companion
`normalizeCommandRunPlanV1` binding keeps normalized input/output identity,
scope, resolver, budget, and destination semantics in Rust. Both bindings are
policy-free prerequisites for the typed browser and Node adapters rather than a
second public command model.

## Native build, package, and signing profiles

All tool versions and runner images are pinned in the owning Nx project or
workflow and upgraded through reviewed changes. Release builds use Cargo
`--locked --release` and the workspace Rust toolchain. Each package embeds the
common version, source commit, target/runtime identity, and capability digest.
A pinned Syft release generates SPDX 2.3 JSON from each final package root so
the SBOM describes shipped bytes rather than only source manifests.

### Linux AMD64

- Build on Ubuntu 24.04 x64 for `x86_64-unknown-linux-gnu`.
- Publish a versioned `.tar.gz`, a `.deb`, SHA-256 manifest, SPDX JSON SBOM,
  GitHub artifact attestation, and release-index entry.
- Generate the Debian package with pinned Debian packaging tools; generate the
  APT repository with pinned `reprepro` and sign its Release metadata with the
  EPA-WG release GPG identity.
- Publish the thin APT index from a dedicated `EPA-WG/cem-apt` repository. Its
  package URLs resolve immutable assets from the matching `cem-ml-v{version}`
  GitHub Release.

### macOS ARM64 and Homebrew

- Build on a GitHub-hosted `macos-14` ARM64 runner for
  `aarch64-apple-darwin`.
- Sign the binary with an EPA-WG Apple Developer ID, notarize the versioned
  archive, and verify both before promotion.
- Publish a versioned `.tar.gz`, SHA-256 manifest, SPDX JSON SBOM, GitHub
  artifact attestation, and release-index entry.
- Publish formula `cem-ml` from dedicated tap `EPA-WG/homebrew-cem`; it resolves
  the immutable release archive and SHA-256, installs `cem-ml`, and runs a
  functional conversion/validation smoke test rather than only `--version`.

Homebrew formulae are URL/checksum package definitions with executable test
blocks, and taps are normally Git repositories. See the
[Homebrew Formula Cookbook](https://docs.brew.sh/Formula-Cookbook).

### Windows AMD64

- Build on `windows-2025` x64 for `x86_64-pc-windows-msvc`.
- Publish a portable `.zip` and a WiX v4 `.msi` with silent install/uninstall,
  plus SHA-256 manifest, SPDX JSON SBOM, GitHub artifact attestation, and
  release-index entry.
- Authenticode-sign the executable and MSI through Microsoft Artifact Signing
  using an EPA-WG public-trust profile; verify signature and timestamp before
  packaging and after download.
- Publish versioned WinGet manifests to `microsoft/winget-pkgs` that resolve the
  immutable MSI and validate with `winget validate` plus a Windows Sandbox
  install/upgrade/uninstall smoke test.

WinGet supports MSI and portable packages and validates public manifest
submissions; see the [WinGet overview](https://learn.microsoft.com/en-us/windows/package-manager/winget/)
and [repository workflow](https://learn.microsoft.com/en-us/windows/package-manager/package/repository).
Microsoft Artifact Signing provides managed certificate lifecycle and HSM-backed
signing; see its [service overview](https://learn.microsoft.com/en-us/azure/artifact-signing/overview).

## Release assets and supply-chain evidence

Every release publishes one machine-readable index covering all npm and native
artifacts. Each record contains version, source commit, runtime/target identity,
filename or npm identity, byte size, SHA-256, signature/attestation reference,
SBOM filename, capability-manifest digest, and publication state.

GitHub artifact attestations are the common provenance mechanism for executable
archives, installers, npm tarballs, checksum manifests, SBOMs, and the release
index. Platform trust remains additive: GPG for APT metadata, Apple signing and
notarization for macOS, and Authenticode for Windows. GitHub attestations use
Sigstore/OIDC provenance and can carry an associated SBOM; see
[GitHub artifact attestations](https://docs.github.com/en/actions/concepts/security/artifact-attestations).

The release workflow creates a draft, uploads and verifies the complete asset
set, then publishes once. Repository-level immutable releases must be enabled
before the first public Phase 2.5 release. GitHub recommends attaching all
assets to a draft before publication when immutability is enabled; see
[release management](https://docs.github.com/en/repositories/releasing-projects-on-github/managing-releases-in-a-repository).

No package-channel job rebuilds an executable. APT, Homebrew, and WinGet records
only project immutable release artifacts. A packaging correction that changes
bytes requires a new common CEM-ML version.

## Verification and promotion

The eventual aggregate `yarn nx run cem_ml:verify:platform` target must depend
on:

- common `cem_ml` lint, tests, WASM build, schema/report validation, and
  capability generation;
- common `cem_ml_cli` lint, tests, e2e, fixture validation, and converter parity;
- WASM npm browser/Node ABI tests and clean-consumer pack/install checks;
- universal CLI browser-worker, Node API, executable, command-round-trip, and
  single-runtime-resolution checks;
- the accepted operation/capability parity fixture across native and WASM hosts;
- every available native build/package/sign/install smoke target;
- version, dependency, tag, source-commit, checksum, signature, SBOM,
  attestation, capability, and release-index drift verification.

Unavailable credentials or hosted toolchains skip publication, never artifact
correctness. Pull requests run deterministic unsigned package-shape and
verification fixtures; protected release jobs add real platform signatures,
attestations, channel validation, and immutable publication.

## Accepted decisions

The accepted contract commits the project to these organization-level choices:

1. Approve the public identities, workspace roots, fixed release group, and
   `cem-ml-v{version}` tag family above.
2. Approve Node 22.12+ and Node 24 as the first supported npm lines, with the
   three-host representative CI matrix.
3. Approve dedicated `EPA-WG/cem-apt` and `EPA-WG/homebrew-cem` repositories and
   the public `microsoft/winget-pkgs` channel.
4. Confirm that EPA-WG will provision and govern the release GPG identity,
   Apple Developer ID/notarization credentials, and Microsoft Artifact Signing
   public-trust profile before public native promotion.
5. Approve the first-release capability matrix and the explicit exclusion of
   benchmark, fixture, schema-mutation, and plugin-mutation parity.

All five decisions were accepted on 2026-08-11. Phase 2.5 implementation may
proceed in [`todo.md`](todo.md) checklist order. No deployment project was
scaffolded as part of accepting this contract.

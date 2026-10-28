# Owned binary Studio commands — 2026-10-28

## Completed contracts

- [x] Implement explicit owned binary command execution with immutable sources.
- [x] Add native admission/mutation fixtures and shared Studio control guards.
- [x] Add ordinary binary configurations, documentation and saved/offline checks.

- Shared native invocation lowering validates complete binary source bytes and
  declared/effective MIME, schema and namespace scope. Known passive DOM, AST
  and event codecs emit informational `binaryInputs` facts binding input ID,
  URI, byte length and SHA-256. These contain no execution tokens or native
  handles. Unknown compiled formats, corrupt payloads and identity relabeling
  are refused.
- Common command preparation independently verifies snapshot digests and repeats
  identity/payload admission before execution. Missing binary scope fields may
  be supplied by the plan; explicit conflicts fail. Nonbinary identity guards
  are unchanged. Parsing remains in the shared native import layer.
- Studio transports declared binary identity with its exact local resource bytes.
  Bare binary file selection has no implicit command. Only an explicit saved
  configuration with matching native URI, size and hash facts enables Run.
- Binary workbenches provide no source draft/edit/save, ad-hoc Parse/Inspect,
  graph authoring, live-template preview or command replacement. Existing
  declarative CEM controls provide Run, Reload, exact downloads and a read-only
  saved-command view. The command resource can be edited through the project
  tree. No component/application UI behavior JavaScript was introduced.
- Binary byte offsets remain visible as data, with text-coordinate navigation
  disabled. Source counts describe actual bytes, never a decoded placeholder.
  External changes require reload, and changes during execution make results
  stale. Byte downloads remain available independently of command admission.
- The ordinary `native-projections` project, version 1.0.5, adds six saved binary
  configurations: DOM Parse, AST/event Inspect, and DOM-to-HTML, AST-to-XML and
  event-to-CEM conversion. Existing catalog, cloning and offline workflows use
  the same shared workspace and components.
- Source/resource limits, 4 MiB recovery groups, 32,768-node witnesses and all
  allocation/resume deadlines remain unchanged. Entered passwords or one-time
  codes block Login recovery until explicitly cleared; automatic recovery
  remains disabled. No source bytes or native authority were added to app state.

## Verification

- Four native admission tests cover all three codecs, corruption, relabeling,
  unsupported formats, missing snapshots and post-preview digest/payload
  mutation. Initial failing admission fixtures were recorded before repair.
- Native command regressions: 74/74; native projection codecs: 12/12; complete
  CLI tests: 550/550. All six new configurations also execute through the CLI.
- Public WASM: 28/28 tests across Node and browser-worker exports. Distribution
  verification checks 658 integrity records and 32 packages. All 12 CEM-QL WASM
  boundary scripts pass.
- Studio focused workbench tests: 58/58, including five new binary admission,
  immutable-source, read-only control, stale-result and live-preview guards.
  The real native projection project browser test passes with six binary
  configurations and three directly opened binary workbenches.
- Catalog unit/audit: 48/48; authored Studio lint and generated-catalog checks
  pass. The complete native catalog executes 283 configurations: 247 successes
  and 36 intentional diagnostic cases, with zero known limits or policy
  refusals. Catalog identity:
  `48d5e0a2b315f99f5475e214c80c1ad5281c76c610816f729093265e62323827`.
- Authored inventory: 22 projects, 246 configurations, 251 documented workflows
  and 32 packages. The six new configurations are ordinary catalog entries.

- Deployed native-projections acceptance passes its complete configuration,
  repair, Unicode/source navigation, exact binary transfer, independent-copy
  and offline workflows. All six binary configurations pass online and offline;
  bare binary files retain no implicit Run operation.
- Final demo verification checks all 898 managed hashes, all 333 copied Studio
  files, native gallery factories/rendering/highlighting, saved/offline Studio
  builds and worker scope. All 10 theme generators retain their CSS baseline;
  all 120 playground endpoints and the live tree gallery pass. Four WASM files
  remain below 25 MiB: paired engines are 23,967,944 and 24,645,453 bytes.

- Final isolated Login recovery: 2/2 worker/fallback cases pass, with three
  successful recoveries and twelve edits per runtime, two credential refusals
  per runtime, the unchanged 4 MiB group limit and 32,768-unit witness limit.
  Allocation/resume deadlines and credential-clearing assertions are unchanged.

## Release-check corrections and limits

Initial WASM/browser corruption assertions expected a parser result after command
acceptance. They now assert native pre-execution admission errors, no publication
and no artifact/source-map records. An overbroad new generic Run assertion on
legacy validation configurations was corrected to require Run specifically for
binary configurations; validation assertions remain intact. These corrections
required no command execution changes, weakened guards or increased deadlines.
An early catalog launch during rebuilding stopped on temporarily absent package
metadata; the complete final catalog run used frozen release output.

Existing version-specific module-map and formatter reader-trace fixture issues
remain separate TODO items, as recorded in the previous projection-import and
formatter-provenance archives. This change does not weaken their native semantic
or resource freshness guards. Direct configuration URL text admission is the
next functional item.

The first isolated Login run exposed a real shell witness overflow (32,772
units against the unchanged 32,768 limit). The first whitespace trim reached
32,769 and still refused; making adjacent command declarations and closing
instructions contiguous removed the remaining layout whitespace, following the
existing component checkpoint contract. The release was rebuilt; all 58 focused
workbench checks, the complete demo verifier and isolated worker/fallback Login
recovery pass again. Native binaries,
catalog configuration bytes and execution policies are unchanged; the longer
saved/offline native-project run above precedes this whitespace-only edit.

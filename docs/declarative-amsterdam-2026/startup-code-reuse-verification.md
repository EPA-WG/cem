# Command worker startup: installed compiled-code reuse

The owner-reported deployment `2ea68bcd-f2ab-444f-a215-deda0fd9b624` serves
the prepared 905-file inventory `5deaa842…`; all 11 critical asset byte checks
match. The default Chromium capture confirms overlapping main WASM downloads
(about 12.4 seconds each) but records `CEM-ML browser command worker startup
timed out`. Studio never publishes its application in that capture. The worker
is terminated, so its own transfer timing is unavailable. No failed network
requests were recorded. The [public readback](public-demo-2ea68bcd-readback.json)
keeps measured facts separate from the selected repair and public acceptance.

The shared command-service host now obtains the adjacent installed asset's
immutable `WebAssembly.Module` from the existing owned-factory code cache. It
structured-clones that code through the private worker bootstrap. No instance,
memory, binding closure, native owner, AST, host grant or retirement permit is
shared. Each worker still instantiates its own memory and checks native headroom,
protocol and ABI. There is no caller-selectable code URL or module option.
Pool bootstraps without compiled code retain their existing initialization path.

The original 10-second worker budget starts before code acquisition; a cold
standalone caller must still finish loading and initializing inside that budget.
Timeout, cancellation, clone errors and failed loads remove initialization
listeners; a late compiled-code result cannot bootstrap the terminated worker.
The Studio readiness limit remains 30 seconds. Optional schema-assets activation
ordering, native resource authorities and checkpoint budgets are unchanged.

## Regression

The browser fixture first fails against the old implementation: no compiled
module reaches the worker and an invalid bootstrap value is accepted. The
candidate checks one network acquisition for two main generations and two
command workers, distinct main memories and worker identities, successful native
commands and rejection of an invalid compiled-code value. A separate fresh
context delays initial acquisition past a short explicit startup budget and
checks worker termination with zero late bootstrap messages.

The full shared CLI browser suite and all 57 Node/native-boundary tests pass.
The browser suite's two obsolete 29-registry assertions now require the existing
36-registry native/unit contract; exact census parsing and ownership checks stay
in force. CEM-ML WASM remains byte-identical to the deployed copy (24,860,607
bytes, SHA-256 `966df03a97851e50fa69cb62551bc3fe29409865e49d2ffc8767c9b1719aeb43`).
CLI and Studio lint and Studio typecheck pass; CLI lint retains its pre-existing
unused `copyFileSync` warning in the unchanged build script.

The controlled Studio fixture delays normal WASM responses by eight seconds and
any duplicate CEM-ML acquisition by eleven seconds. Its optional baseline uses
the graph-emitted command-service module from the deployed 905-file copy, while
retaining the same current main startup coordinator. Run it with:

```sh
CEM_STUDIO_WORKER_STARTUP_BASELINE=/absolute/path/to/saved/browser-command.js \
yarn nx run @epa-wg/cem-studio:verify:startup-latency
```

The baseline must fail with the original command-worker startup timeout; the
candidate must publish inside 30 seconds with one CEM-ML download and three
total WASM requests. All original runtime owners remain; only immutable CEM-ML
code acquisition is reused. This controlled local test cannot establish public
network acceptance. The archived [startup-overlap comparison](startup-overlap-readback.json)
describes the previous four-download implementation separately.

The final controlled comparison reproduces the baseline's worker timeout and
failed readiness at 30.277 seconds. The candidate is ready at 20.572 seconds
and settled at 20.582 seconds, without page errors. Both measurements use the
same emitted main module; only the command-service host module is overridden
for the baseline. The candidate's measured main/command hashes match the
prepared copy. No late baseline timestamp is labeled as successful settling.

The [companion readback](startup-code-reuse-readback.json) retains the full
waterfall, module identities, cold code-reuse checks and final walkthrough
receipt. All 19 shell/startup tests and all 59 presentation/resource guards pass.
General demo acceptance passes: exact 335-file Studio freshness, paired gallery
runtimes, actual playground editing/Render, offline SCSS rebuild, worker scope,
10 rendered theme generators, 120 playground HTTP checks and the live tree.
All 23 selected offline Login/Supplier configurations pass against the exact
new alias `http://127.0.0.1:8081/studio/__cem_preview__/a6fda0f562cd0b74/`, including
failure/repair, graph provenance, independent copies and offline draft reload.

The next prepared copy is `/home/suns/cem-bin`: 905 files, inventory SHA-256
`253278d16f34b620b2bbde2d9085f03950b69bee9d8938d250036334c623a55d`.
Eleven asset paths changed from the deployed overlap copy. All four WASM files
are unchanged and remain below 25 MiB. No styling, theme, component declaration
or native transformation behavior changed. The graph's source/destination
module maps explicitly declare the command host's new factory import.

Use the owner's signed-in `npx wrangler@4 deploy` route from the checkout whose
`../cem-bin` is this verified folder. Public general browser and full selected
Login/Supplier acceptance remain open until those bytes are uploaded and the
unchanged public gates pass. Existing cached-client upgrades require a separate
check preserving saved projects and drafts. The discussion comment stays unposted.

Follow-up: the owner uploaded this `253278d1…` copy. Its public byte checks pass,
but default readiness still fails during slow main WASM transfers. A controlled
transport run reaches readiness, then reveals redirected module identities.
See the [public readback](public-demo-253278d1-readback.json) and subsequent
[hosting/cache correction](hosting-upgrade-verification.md), which supersedes
the prepared copy above. These findings do not undo the measured code-reuse
regression or imply a public acceptance pass.

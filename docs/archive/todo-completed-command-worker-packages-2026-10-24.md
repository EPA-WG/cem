# Completed: command-service worker package transport

- [x] Add command-service worker package transport to the shared handoff: acquire
  original bytes under the pinned pack contract, activate on the exact worker
  heap, release only under original checkpoint/embedding confirmation, and restore
  packages alongside command clients. Keep native owners/grants out of
  transferable status/checkpoint records and refuse missing/late/unknown work.
- [x] Add real optional-WASM worker fixtures for offline package demand, ordinary
  CSV commands, consecutive complete handoffs, credential/incomplete-owner
  refusal, exclusive loader ownership, worker-owned grants, stale aliases and
  late cleanup acknowledgments. Add async resource-release deadline, bounded
  transport DTO and unknown activation-outcome tests; deliver the raw-worker
  protocol dependency in the existing CLI browser fixture.

`BrowserCommandServiceClient.packageResourceBinding()` supplies an original
asynchronous endpoint to the shared `CemSchemaAssetPackLoader`. Its identity is
that exact Worker. The loader snapshots original endpoint functions, permits one
transport loader per physical identity, authenticates the worker's compiled pin,
and sends original index/source bytes to native import and activation. The
synchronous owned-WASM adapter remains supported. Native source interpretation
and package compilation stay in Rust; no JavaScript AST or component-local UI
behavior was introduced.

The native owner remains private to the worker. Its replacement grants come
from original, snapshotted worker startup configuration, renewed by the original
fresh-worker allocator. Loader configuration, demand URLs and package messages
cannot override them. Messages reject unknown operations/fields, sparse source
lists, shared mutable buffers, oversized sources/indices/selections and storage
above the 32 MiB aggregate message bound. Bounds count actual backing buffers,
since structured cloning a small typed-array view also copies the rest of its
buffer. Pack loading retains its separate 32 MiB resident/pending bound.

Attached packages require the command client's original complete embedding
acknowledgment and a current checkpoint handoff within the existing 1 MiB command
budget. The client closes managed and raw admission, awaits resource release,
rechecks original witnesses, then validates an empty native inventory. The
original five-second checkpoint deadline covers the entire sequence. Pending,
failed, stale or late release cannot produce a retirement receipt. Explicit
shutdown and worker failure fence/dispose resources and terminate the physical
worker; successful retirement preserves only replay bytes for the original
complete binding transaction.

The shared resource registry now supports asynchronous release without weakening
its synchronous adapter. A failed or late asynchronous release poisons that
source attempt. Aborted or lost asynchronous activation acknowledgment likewise
blocks replay until explicit disposal, because native publication may already
have happened. Ordinary read cancellation does not wait for an unresponsive
worker. The cancellation fixture waits for the opener to start before aborting,
so it continues to verify late response-body cleanup across both synchronous and
asynchronous endpoints.

Fresh preparation authenticates and recompiles retained bytes on the new worker
before command routes bind. Source/target checks use original physical
participants. Resources re-enroll after publication for consecutive handoffs;
old aliases and previous revocation cannot retarget or fence the latest route.
The entered-password/OTP recovery refusal remains in effect, and automatic
whole-Studio recovery stays disabled. No credentials, native owner IDs or grant
records were added to checkpoint/status data.

## Verification

Passed against the final generated runtime/deployment outputs:

- 51 Node host/native-generation/transport tests with forced GC enabled. New
  coverage exercises asynchronous release failure, witness changes, pending/late
  acknowledgment, protocol fields, sparse arrays, shared buffers, backing-buffer
  limits and message capacity.
- 40 optional native schema-asset/HTTP tests, including real package activation
  with deliberately lost async acknowledgment, replay refusal, cleanup and
  cancellation of a missing worker response.
- 27 shared checkpoint-resume/recovery-scope tests.
- Full CLI Chromium verifier: worker-pool control, owned invocation recovery,
  dedicated-worker commands, callbacks, progress, cancellation and artifact
  cleanup. The raw-worker fixture explicitly serves its protocol dependency.
- Default Studio native build and output verifier: 215 declared assets, 310
  emitted files, matching source/output byte lengths and digests.
- Optional Studio native build and isolated offline Chromium verifier. The
  `workerPackageHandoff` report records two consecutive handoffs across three
  distinct command workers, ordinary CSV validation, credential/incomplete-owner
  refusal, required bounded handoffs, empty native inventory, worker-owned grants,
  duplicate-loader refusal, stale aliases and a deliberately delayed cleanup
  acknowledgment. Both optional factories and all 205 pack assets remain paired;
  default/profile cache isolation and default output preservation pass.
- Independent production-service-worker offline loader fixture: native package
  demand, source reimport/reactivation, exact cleanup and missing/incomplete or
  invalid deployment refusal.
- CLI and shared runtime TypeScript builds, scoped ESLint and `git diff --check`.

The browser verifiers use isolated storage; saved projects and drafts in the
user's browser session were not modified. Generated reports, screenshots and
Vitest attachments are not part of this commit.

## Remaining boundaries

Rendering-worker and owned-fallback adapters remain next, followed by explicit
shared Studio demand across complete command/render recovery groups. Neither
this transport nor its acceptance fixture silently enables whole-Studio package
adoption. The default complete deployment and optional embedded source bodies
remain unchanged. Optional body exclusion and binary/latency/headroom measurement
remain separate TODOs; no download, memory or checkpoint savings are claimed.

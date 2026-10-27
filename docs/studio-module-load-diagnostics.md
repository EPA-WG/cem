# Studio module-load investigation — 2026-10-27

The historical aggregate fallback failure remains unattributed. Two current
complete cohorts pass 275/275: one with loading diagnostics, one with diagnostics
disabled and an independent fresh development cache. Successful repeats are not
evidence that the historical cause has been repaired.

## Observations

The 2026-10-26 full cohort passed 273/274 cases. Whole-workspace fallback recovery
failed in 287 ms, after `native ready` and before workspace construction, while
importing `cem-elements/dist/index.js`. The file existed and a subsequent HTTP
request returned 200. That later response does not describe the failed request
or its dependency graph. The unchanged isolated case then passed in 45.6 s.

The historical build and copy logs finished before that cohort started. They do
not establish a publication race. No request-failure reason, HTTP response for
the failing attempt, or matching frame-lifecycle trace was retained, so the
historical evidence cannot distinguish cancellation, server errors, blocking,
dependency failure or another loading condition.

The current observed cohort passes **275/275 across four files**, in **730.75 s**.
All eight original recovery cases pass, including the previously failing case
in **48.661 s**. The initial provider-side capture records **11** entry responses
(ten HTTP 200, one 304) and no failed script requests. The diagnostic tool uses public
Vitest browser commands and Playwright APIs rather than provider internals.

The independent, initially empty development-cache run also passes **275/275**
with diagnostics disabled, in **693.87 s**. Whole-workspace fallback recovery
passes in **45.756 s**. This uses the ordinary provider without recorder hooks,
while preserving the normal development cache and user storage.

Live Login acceptance retains two credential refusals followed by three admitted
recoveries after explicit clearing. Worker checkpoint/group sizes are
**2,906,881 / 2,906,979 bytes**; fallback sizes are **2,906,334 / 2,906,432 bytes**.
All fit the original **4,194,304-byte** limits. The original recovery assertions
and deadlines remain in place. Automatic whole-Studio recovery remains disabled.
The fresh-cache worker/fallback group sizes are **2,907,109 / 2,906,562 bytes**,
with the same credential refusals and admitted recoveries.

## Capture a recurrence

From the repository root:

```bash
CEM_STUDIO_LOAD_DIAGNOSTICS=/tmp/cem-studio-module-loads.jsonl \
NX_DAEMON=false yarn nx run @epa-wg/cem-studio:test:workbench \
  --excludeTaskDependencies --skipNxCache \
  --args='src/feature-tour.browser.spec.js --reporter=verbose'
```

Capture is opt-in and starts in the test setup's `beforeAll`, before recovery
iframes are created. Initial test-module imports can precede that hook. Ordinary
runs use the original Playwright provider and have no recorder or CDP session.

The JSONL records include:

- Entry-script HTTP status, including 200/304, and unsuccessful script responses.
- Failed script requests and Chromium network error/blocking/CORS metadata.
- Script frame IDs, frame attachment/removal and named recovery stages.
- An explicit `diagnostic-unavailable` event if CDP capture is unavailable.

Only HTTP(S) origins and paths are recorded. URL credentials and query/fragment
values are excluded; other URL schemes retain only their scheme. There are no
request/response bodies, headers, document contents, control values, screenshots
or heap exports. The recorder observes existing requests without interception,
retries or module cache-busting, and releases its listeners when its page closes.

On failure, correlate the script's frame ID with frame removal and compare the
entry status with dependency failures. A 200 entry response alone does not prove
the graph loaded. Preserve the original exception and assertions while gathering
the network evidence; a later successful fetch is a separate observation.

An independent cache can test cold startup without clearing the normal cache or
any projects/drafts:

```bash
studio_test_cache=$(mktemp -d "$PWD/node_modules/.vite/cem-studio-load-probe.XXXXXX")
CEM_STUDIO_TEST_CACHE_DIR="$studio_test_cache" NX_DAEMON=false \
  yarn nx run @epa-wg/cem-studio:test:workbench \
  --excludeTaskDependencies --skipNxCache \
  --args='src/feature-tour.browser.spec.js --reporter=verbose'
```

## Diagnostic regression fixture

```bash
node --test packages/cem-studio/scripts/browser-load-diagnostics.test.mjs
```

The controlled local server returns HTTP 503 for an entry import. The fresh
iframe receives the original `TypeError`, exactly one request is made, and the
capture links the HTTP/network failure to that iframe's removal. A separate
fresh realm then imports the same URL successfully. The test also checks query
and body redaction, recorder deduplication and listener release. This validates
the recorder and error propagation; it does not reproduce or explain the
historical Studio failure.

Only test tooling and documentation change. Application/component code and
native runtimes retain their existing behavior. Temporary acceptance databases
belong to the fixtures; user projects and drafts are preserved.

Additional checks pass: the public recorder command in a real Studio worker
test, Studio typechecking and assembled demo acceptance. The demo retains all
333 Studio files, 898 total files, independent runtime memories, saved/offline
builds and HTML/CSS previews. All ten theme generators retain their CSS hashes;
120 canonical playground pages and the live tree gallery pass. Generated
diagnostic logs, screenshots and Vitest attachments are excluded from Git.

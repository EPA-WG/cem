# Completed work — 2026-09-27

This records follow-up work after the [TODO snapshot](todo-snapshot-2026-09-27.md).
The [active TODO](../todo.md) remains the authority for unfinished work.

- [x] Fixture: activate staged registry connections inside queued CSS/DOM
  publication; verify cleanup ordering, empty source groups, rejection and
  activation failure recovery.
  `QueuedConnectionActivation` exercises worker and fallback hosts. Publication
  adopts the connection after the DOM patch and before deferred old-load cleanup.
  Rejected candidates release preparations, reused active handles remain live,
  and activation failure or cleanup cancellation blocks the queue until recovery.
- [x] Update legacy viewer evidence checks to read completed fixture IDs from the
  archived checklist while continuing to check open IDs in the active TODO.

Validation: 585 unit tests and 304 browser cases in each of the default and
retained-CSS lanes passed. Typecheck and lint passed, with the two existing
non-null assertion warnings. All 31 existing open TODO items were preserved;
the active checklist contains no completed entries.

## Runtime payload stylesheet replacements

- [x] Fixture: retain active runtime instance CSS during replacement imports,
  failed loads and superseded updates; verify successful replacement, clearing
  and disconnect cleanup in both host modes.
  `PayloadReplacementPreservesActiveStyles` reproduced the removed live style
  node while an import was pending. Runtime replacements now use the staged
  installer, which retains active styles until a complete replacement succeeds.
  Initial installation retains its independent-source failure behavior.
  The fixture covers worker and fallback execution, node reuse, failed imports,
  superseded late results, empty payload CSS and disconnect cancellation.

Runtime CSS/DOM joint publication remains an open integration task.

Validation: 585 unit tests and 305 browser cases in each CSS lane passed,
along with typecheck and lint (two existing non-null assertion warnings).

## Native CSS migration closure

- [x] Close native CSS migration with these three delivery gates. Existing
      parsing, URL resolution, staging and queue helpers are prerequisites already
      implemented; do not expand them into separate completion milestones.
    - [x] Runtime fixture: publish retained declaration CSS, instance CSS and DOM
          through the per-host queue, adopt runtime state before cleanup, reject
          superseded loads and recover invalid DOM with a full render. Keep
          resource settlement outside the queue. Cover worker and fallback modes.
    - [x] Edge fixture: transport and retain changed stylesheet batches, validate
          them with the patch transaction, publish through the same ownership
          boundary and recover authoritatively. Verify cancellation, stale CAS,
          shared contexts and browser cleanup before removing the unchanged-CSS
          capability guard.
    - [x] Default cutover fixture: make retained CSS the browser default after
          SSR/hydration and component verification; retain an explicit legacy
          comparison lane and document the separately owned result-style path.
          Audit source profiles and diagnostic gates, document explicit
          compatibility boundaries, and close the native CSS migration.

Runtime DOM and processing-host updates now prepare declaration CSS, payload CSS
and DOM patches inside the per-host queue. They adopt connection, style and
render-plan state before cleanup, cancel superseded work, preserve live output
on CSS failure and retry invalid patches with a full render. Resource settlement
runs outside the queue. Hydration keeps server style nodes while imports load.
CSS source compilation uses its own artifact, so declaration startup cannot
prime the render cache with incomplete instance bindings.

Native Edge updates load a complete changed stylesheet batch before committing
retained state or exposing frames. Protocol v14 carries the content-addressed
batch; `publishEdgeCssDomUpdate` verifies it and publishes emitted CSS with the
DOM transaction without parsing CSS again. Fixtures cover changed and unchanged
inputs, held-import cancellation, competing CAS writes, failed imports, tampered
content, stale browser state, shared/private ownership, callback-induced recovery,
node reuse and teardown.

Native retained CSS is now the browser default. `retainedStylesheets: false` and
`cem-elements:test:legacy-css` provide explicit compatibility comparison. XSLT
result styles remain branch-local under their accepted contract. Unsupported CSS
syntax keeps the documented native diagnostics; automatic fragment substitution
remains in the wishlist. These are explicit boundaries, not further migration
prerequisites.

Validation:

- 585 runtime unit tests and 53 Edge unit tests passed.
- 306 browser cases passed in each CSS lane; all 10 SSR browser cases passed.
- Native retained-stylesheet/WASM verification, typecheck, lint and package
  verification passed. Lint retains two existing non-null assertion warnings.
- Rebuilt `cem-elements` before running component verification because the legacy
  suite consumes packaged output. Committed baseline `5f8e7172` and this cutover
  both produce 81 passing / 48 failing tests, with identical failing test names.
  No new component failures were introduced. The legacy component package gate
  remains red and is tracked under the remaining declarative UI migration.

The active TODO retains its 25 unrelated open items and no completed checkboxes.

## Site v3 XHTML deployment

    - [x] Implement v3 XHTML deployment before resuming the action cutover.
        - Native support complete: the v3 schema gate, 27 focused module-map
          tests, v1/v2/v3 CLI publication tests, cache-key fixture, TypeScript
          projections and WASM build pass. Generated examples document XHTML.
        - [x] Adopt typed v3 maps and exact module edges in Site, then prove
              static search/interactive XHTML delivery before registry removal.
            - [x] Fixture: verify Site catalog/search layouts query retained
                  generic-data trees, including rendered values and row counts;
                  retain one stable ID per heading without copying a second ID.
            - [x] Fixture: deploy the existing canonical select XHTML on both
                  routes; verify byte/digest identity, named-template loading,
                  scoped styles, interaction and complete declared module assets.

    - [x] Fixture: distinguish bounded document imports from small expression
          imports; accept a catalog-sized native tree with source provenance,
          reject document value/depth/byte overflow, and preserve string/CSS limits.
    - [x] Diagnose the restored Site build's token-browser failure:
          `cem.transform_template.adapter_failed` reports the 64-level / 4096-value
          JSON import limit before asset deployment. Resolve before end-to-end
          XHTML cutover verification; do not attribute it to the new resource.

Validation: 37 focused native import tests passed. The full `cem-site:verify`
gate passes, including deterministic builds, search, interactive behavior and
all 26 production routes. Both browser routes load the canonical select XHTML,
choose its second option and verify native scoped CSS. Static checks preserve
all catalog rows, component provenance and source/output asset digests.
The 65,536-value document profile fixes the token-catalog blocker while retaining
16 MiB and 64-level limits and the smaller string/CSS profiles. Site layouts now
read the retained generic-data tree. Completed items are removed from active TODO.
Site lint passes with one existing unused-variable warning in `search-runtime.js`.

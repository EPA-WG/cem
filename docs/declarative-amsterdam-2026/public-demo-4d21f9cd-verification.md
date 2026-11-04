# Public demo acceptance — 2026-11-03

The owner-deployed [public demo](https://green-morning-fd6e.trans-acf.workers.dev/)
passes the general browser verifier, the complete selected Login/Supplier
workflow and all 14 frozen slide/demo destinations in default Chromium
148.0.7778.96. The [readback](public-demo-4d21f9cd-readback.json) retains exact
identities and individual results, including the final inventory recheck.

The 906-file inventory SHA-256 is
`4d21f9cd63ddb3c7c989b9bc9c123040e7472612274f3be1cb56c97d532a0770`,
from source commit `f9bc2ef96630e581ed98770e7a3471c56f61aadb`.
All 11 critical asset byte checks pass. Additional direct checks confirm the
startup module, graph-emitted revision and service worker match the prepared
copy without redirects. The inventory is unchanged after all browser cohorts.
No new Wrangler version ID was supplied; this record identifies observed bytes.

## Accepted behavior

The general run passes native gallery rendering/highlighting, shared initializer
and isolated memories, production Studio navigation/scope, an independent saved
copy, native HTML/CSS preview, offline SCSS rebuild and the real playground
editing/Render action. All ten theme generators render; 120 canonical playground
pages return successfully, and the live tree gallery renders. Those HTTP checks
do not establish interaction coverage for every playground.

The selected-workflow run starts in a separate fresh context, passes the original
30-second application-readiness gate, verifies shell styling/responsive retention
and forced colors, then executes 20 Login and three Supplier configurations.
Validation failure/repair, template output, saved graph provenance, live view,
source inspection/formatting/coloring, standalone expressions, independent
copies and offline draft restoration pass. This is the complete selected suite;
it does not claim all six Supplier configurations.

Every frozen link passes independently in a fresh context: exact final URL,
collection/sample/resource, Structure-tree selection, selected preview
configuration, available sample source, zero saved projects and no page errors.
Observed setup/navigation-to-ready durations range from 23.952 to 28.555 seconds;
the original 30-second readiness wait remains unchanged. No retries, transport
flags, executable override or warmed-context substitute were used. Browser
cohorts ran sequentially. The link verifier now retains every destination's
result even when an earlier one fails, and requires every result to pass.

The service-worker startup scheduling correction is therefore accepted on the
public bytes above. Earlier failed uploads, transport traces and controlled
regressions remain historical evidence; this does not erase their limits.
No product asset changed during this verification batch.

## Reproduce

Run the browser cohorts sequentially against the matching prepared `../cem-bin`:

```sh
CEM_DEMO_HOSTING=production yarn verify:demo /home/suns/cem-bin https://green-morning-fd6e.trans-acf.workers.dev/
CEM_STUDIO_PUBLIC_ACCEPTANCE=amsterdam-2026 \
CEM_STUDIO_ACCEPTANCE_URL=https://green-morning-fd6e.trans-acf.workers.dev/studio/ \
node packages/cem-studio/scripts/verify-static-worker.mjs --presentation-only
yarn nx run @epa-wg/cem-studio:verify:public-demo-links
```

Studio lint and all 59 presentation/resource guards pass. Temporary isolated
contexts were closed; existing user projects, drafts and profiles were untouched.
No real credentials were entered. Worker startup stays at 10 seconds;
checkpoint limits stay at 5 seconds/4 MiB, automatic whole-Studio recovery stays
off, and entered-password/OTP recovery refusal remains required.

Presentation Site publication, review/delivery of discussion #24's prepared
comment, the timed rehearsal and final handoff remain open. No comment was posted.

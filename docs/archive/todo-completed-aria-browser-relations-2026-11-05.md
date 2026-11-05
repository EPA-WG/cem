# ARIA browser relation evidence — 2026-11-05

The new `cem-elements:verify:aria-relations` target compares native CEM exports
with equivalent literal HTML using Chromium's internal accessibility tree.
It runs six fresh-context cases: worker and forced fallback for the pinned
Recommendation's single target and the draft sequences `first second first`
and `second first second`. Every case checks invalid state true and false.
Actual worker-ready/success counters distinguish worker execution from fallback.
Fallback has no created workers and must observe the deliberate factory refusal.

The probe asserts exact native DOM order and repetition for both `aria-details`
and `aria-errormessage`, exact selected profile, no fatal/error diagnostics,
browser errors or network failures, and native-export/literal-control AX parity.
It resolves related backend DOM nodes individually instead of inferring targets
from an ID-reference string. Initial true/false URL selections also pass through
native rendering. Later DOM invalid-state changes isolate browser AX exposure
without rewriting the original target sequences. Failure observations are copied
before teardown can introduce request cancellations.

Observed environment: **Chromium 148.0.7778.96**, headless, **Ubuntu 26.04 LTS**,
WSL2 kernel **6.18.54.1-microsoft-standard-WSL2**, Node **v24.16.0**. No assistive
technology was invoked.

| Observed boundary | Result in worker and fallback |
| --- | --- |
| Recommendation target | Single target retained in DOM and AX |
| Draft target sequences | Every target, order and repetition retained in DOM and AX |
| Invalid state true | Details and error-message AX relations exposed |
| Invalid state false | Details exposed; error-message AX relation absent |
| DOM attributes when valid | Both complete target sequences remain intact |
| Field name | Profile field, matching literal HTML |

- [x] `yarn nx run cem-elements:verify:aria-relations`: **6 cases / 12 state comparisons pass**.
- [x] `yarn nx run cem-elements:lint`: **0 errors**, existing **818 warnings**.
- [x] `yarn nx run cem-elements:serve:aria-relations`: persistent server lists
      **12 URLs** across profiles, modes, target sequences and initial invalid states.
- [x] Browser-smoke two complementary served pages: worker/draft/reversed/valid
      and fallback/Recommendation/invalid. Both named fieldsets, field names,
      native initial states and exact target sequences pass, with no page errors.
- [x] Serving preserves the automated receipt. The smoke server is stopped after
      verification; starting it alone never records an AT acceptance result.
- [x] Frozen presentation archive remains 17,321,262 bytes, SHA-256
      `bbcb37da0a36491e5c021a5cf0c67304cc41f001a98247fc5c101ed6c45a9864`.

The [receipt](aria-browser-relations-2026-11-05.json) retains versions, runtime
asset hashes, mode counters, exact normalized DOM/AX observations and manual-server
smoke results. Production runtime, declarations, reference export policy,
cardinality defaults, CSS/theme tokens and frozen presentation materials have
no source changes. The normal Nx build/package prerequisites were run; there
is no public deployment change.

An earlier six-case run passed. A subsequent repeat passed the first case, then
hit the unchanged 30-second initialization deadline in the ordered draft worker
case, recording `net::ERR_NETWORK_CHANGED`. The
[failed receipt](aria-browser-relations-network-failure-2026-11-05.json) retains
that result. Its request capture contained the failure code but no path, so it
cannot attribute the precise loading boundary or a runtime defect. The probe
now retains sanitized failed paths, request kinds/owners and initialization
phase/counters before teardown. No retry, transport override, extra readiness
time or runtime repair is introduced. Keep that failed observation alongside
the passing matrix; successful repeats do not establish its cause.

The broader acceptance item remains open. CDP does not establish OS accessibility
bridge mappings, screen-reader speech/navigation or physical-device behavior.
Use the [manual fixture procedure](../cem-element-aria-reference-profile.md#repeatable-chromium-accessibility-tree-probe)
for actual Firefox/NVDA, Chromium/Edge with JAWS/NVDA, Safari/VoiceOver and Linux
Orca runs. Record actual versions and results, including unsupported or
inconclusive cases. The experimental draft remains opt-in; a browser/AT that
exposes only one relation must not cause native export to truncate its sequence.

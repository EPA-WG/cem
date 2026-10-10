# ARIA relationship profile review

Reviewed 2026-10-07. Keep native export on the pinned
`wai-aria-1.2-rec-20230606` profile by default. Do not infer a profile from browser
support, authored values or the current contents of an unversioned standards URL.

| Export profile | `aria-details` | `aria-errormessage` | Status |
| --- | --- | --- | --- |
| `wai-aria-1.2-rec-20230606` | Exactly one element | Exactly one element | Implemented default |
| `wai-aria-1.3-wd-20260604` | Nonempty ordered sequence | Nonempty ordered sequence | Explicit experimental opt-in |

The [WAI-ARIA 1.2 Recommendation, 6 June 2023](https://www.w3.org/TR/2023/REC-wai-aria-1.2-20230606/#aria-details)
defines a single details target;
[its error-message property](https://www.w3.org/TR/2023/REC-wai-aria-1.2-20230606/#aria-errormessage)
also has the ID-reference value type. The
[WAI-ARIA 1.3 Working Draft, 4 June 2026](https://www.w3.org/TR/2026/WD-wai-aria-1.3-20260604/#aria-details)
supports multiple details targets, and
[its error-message property](https://www.w3.org/TR/2026/WD-wai-aria-1.3-20260604/#aria-errormessage)
uses an ID-reference list. These changes justify a separate export profile;
the draft's publication does not by itself establish browser or assistive
technology interoperability.

For the experimental profile, preserve authored target order and
repetitions through the shared reference traversal. Never truncate a sequence
to make it pass the default profile, clone declarations, or rewrite source AST
references. The profile changes export cardinality only: node identity,
placement grants, unresolved-link policies and traversal bounds stay the same.
Literal browser attributes remain literal under both profiles. Role support,
error visibility and `aria-invalid` obligations remain separate accessibility
validation responsibilities.

Expose profile selection through explicit native export options and the
embedding runtime's configuration. Carry its exact identity through worker
requests, export/result cache identity, SSR output and hydration compatibility
checks. A mismatch requires a new render under the requested profile. Unknown
profiles fail preparation rather than silently choosing the newest profile.

Native consumers select `ElementReferenceExportOptions.aria_profile`; existing
projection entry points retain the Recommendation default. Browser embeddings
select `CemElementRuntimeOptions.ariaReferenceProfile`, and retained template
processing carries the same field in its execution identity. Coordinated SSR
uses `new CemSsrPlacementCoordinator({ ariaReferenceProfile })` and requires all
staged plans to match that profile. Both supported exact identifiers are
exported from the package as `DEFAULT_ARIA_REFERENCE_PROFILE` and
`EXPERIMENTAL_ARIA_REFERENCE_PROFILE`.

New snapshots and native DOM export responses retain the exact profile. Missing
legacy metadata means the pinned Recommendation. Draft identity separates
revision, patch, runtime policy and export cache keys; selecting it never changes
the source template or original references. Hydration with a different profile
retains the understood data island and producer identity, clears foreign placement
routes, and rerenders. Other policy mismatches remain subject to their existing
compatibility checks. Serialized profile and placement hints confer no authority.

### Browser and assistive-technology evidence

Recorded 2026-10-07. The automated browser environment is headless Chromium
148.0.7778.96 on Linux. The following matrix describes evidence for **both**
`aria-details` and `aria-errormessage`; passing DOM export tests do not establish
accessibility API mappings, announcement order or assistive-technology support.

| Environment | Recommendation profile | Draft profile | Evidence boundary |
| --- | --- | --- | --- |
| Chromium 148, Linux; worker and fallback | Single target exported | Ordered/repeated targets exported | Automated DOM attributes and target IDs |
| Chromium 148, Linux; SSR hydration | Profile retained; mismatch rerenders | Profile retained; mismatch rerenders | Automated resume and fresh native export |
| Firefox + NVDA, Windows | Not tested | Not tested | Accessibility relations and navigation remain to verify |
| Chromium/Edge + JAWS/NVDA, Windows | Not tested | Not tested | Error announcements, relation order and repetitions remain to verify |
| Safari + VoiceOver, macOS/iOS | Not tested | Not tested | Details navigation and error-message exposure remain to verify |
| Chromium/Firefox + Orca, Linux | Not tested | Not tested | Accessibility API mappings remain to verify |

The pinned [draft details contract](https://www.w3.org/TR/2026/WD-wai-aria-1.3-20260604/#aria-details)
allows accessibility APIs that cannot represent multiple relations to expose
only the first target. That is a possible interoperability limit, not a reason
for this exporter to truncate the authored sequence. The draft remains an
explicit experimental choice without a browser/AT interoperability claim.

Native one/many/empty/repeated cases, separate WASM workers, fallback, SSR export,
unknown-profile rejection and hydration mismatch fixtures are maintained with
the [archived consumer actions](archive/todo-snapshot-2026-10-10.md#next-consumer-actions). Manual interoperability
work remains a separate [delivery action](todo.md#native-surface-delivery-follow-ups).

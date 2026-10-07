# ARIA relationship profile review

Reviewed 2026-10-07. Keep native export on the pinned
`wai-aria-1.2-rec-20230606` profile by default. Do not infer a profile from browser
support, authored values or the current contents of an unversioned standards URL.

| Export profile | `aria-details` | `aria-errormessage` | Status |
| --- | --- | --- | --- |
| `wai-aria-1.2-rec-20230606` | Exactly one element | Exactly one element | Implemented default |
| `wai-aria-1.3-wd-20260604` | Nonempty ordered sequence | Nonempty ordered sequence | Reviewed; future explicit experimental opt-in |

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

For a future experimental profile, preserve authored target order and
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

Before enabling the draft profile, add native one/many/empty/repeated target
fixtures, worker/fallback and SSR/resume profile propagation fixtures, and a
documented browser/assistive-technology matrix for both relationships. Keep that
compatibility evidence separate from a passing DOM ID export test. Implementation
is tracked in [todo.md](todo.md#next-consumer-actions).

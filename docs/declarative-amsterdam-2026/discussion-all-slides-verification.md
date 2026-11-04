# Discussion #24: complete slide-resource review

Reviewed 2026-11-03. The [comment draft](discussion-resource-comment-draft.md)
now covers all 30 physical slides, including optional references 25 and 27–29.
There are 31 rows because slide 16 has separate query and expression links:
14 unchanged Studio destinations across 13 slides, plus supporting sources for
the other 17 slides. The [readback](discussion-all-slides-readback.json) records
the live deck, existing comment, exact references and public checks.

The existing [published comment](https://github.com/EPA-WG/cem/discussions/24#discussioncomment-18867103)
contained examples for slides 1, 3, 4, 5 and 8 and placeholders for the others.
Its demo-site, native-deck and playground headers are retained. The original
Login, Hello, embedded-XSLT and schema-package examples remain listed with their
slide numbers. The fragment-source link now uses ordinary HTTPS because GitHub
does not offer a usable `view-source:` link. The existing comment was not edited.

Google Drive readback of the actual 30-slide presentation at revision
`Eq03wuK7FUhGiQ` matches every recorded stable slide ID in order. No slide,
speaker note, native timing or original artwork was changed. Conceptual slides
link to the relevant explanation, source pair, schema guide, platform/host
contract, security boundary or measured performance record; these are labelled
references rather than advertised as new interactive demonstrations.

All new source references and the updated footer pin repository revision
`bc51404ef57639c4b627ce88f615b629cbfeb5e8`. Eighteen distinct GitHub source
links return 200; their raw contents match the pinned Git blobs, and selected
heading fragments exist in those sources. The dated local-kit index is labelled
as a fallback; the public verification footer now points to the successful
`4d21f9cd…` acceptance instead of the older pending-public edition.

Seven retained HTTP destinations return 200. In fresh default Chromium contexts,
both source-bearing playground URLs load their exact source and render the
expected Login/Hello headings after choosing **Render**. The original XSLT
fragment example renders both fruits and its details control toggles. Initial
probes omitted Render; the corrected probes follow the intended flow. The draft
now makes that step explicit. No product code or deployment bytes changed.

The 14 Studio URLs exactly match the earlier accepted public receipt, and the
public inventory still matches `4d21f9cd…`; that existing navigation/full-workflow
acceptance is reused, not described as a new run. Its selected-configuration,
playground interaction and host coverage limits still apply.

Studio lint and all 65 presentation/resource guards pass, including new refusal
cases for omitted/duplicate references, wrong native IDs, changed anchors or
revisions, and removed original examples. The existing foreign-host, frozen
project/configuration, privacy and posting guards remain. Reproduce local checks:

```sh
yarn nx run @epa-wg/cem-studio:verify:presentation-resources
yarn nx run @epa-wg/cem-studio:lint
```

The draft is complete for review. Updating the existing GitHub comment is a
separate explicit delivery action. Presentation Site publication, timed rehearsal
and final handoff remain open.

## Publication follow-up — 2026-11-03

The owner continued after review. The [existing resource comment](https://github.com/EPA-WG/cem/discussions/24#discussioncomment-18867103)
is now updated in place with the completed 30-slide map. The
[published body](discussion-resource-comment-published.md) matches GitHub's
separate readback exactly; all 31 rendered rows and every rendered link match.
The [delivery receipt](discussion-resource-comment-delivery.json) records the
same comment ID/URL, before/after hashes, update time and preserved boundaries.
The public inventory still matches the accepted `4d21f9cd…` deployment. No new
comment, product deployment or native-deck change was made. Statements above
about pending delivery describe the earlier review stage.

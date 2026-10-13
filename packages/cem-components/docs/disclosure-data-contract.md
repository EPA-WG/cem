# Declarative disclosure and data owners

`cem-expansion`, `cem-table`, `cem-sort-header` and `cem-paginator` are canonical
XHTML/CEM-ML declarations with embedded scoped CSS, colocated CSF Next plays,
property playgrounds and five-theme galleries. Their legacy declarations,
behavior modules and separate browser specs are retired. The expansion,
sort-header and paginator contracts retain their full native-input assertions.

Reusable `disclosure-panel`, `sort-action` and `pagination` capabilities belong
to `cem-elements`. Declarations own structure, presence predicates, ARIA and
paint. The runtime owns generic normalization, event handling and coordination.
Fragment-src consumers repeat the capability on their declaration wrapper.

`cem-table` is a passive named table, not an interactive grid. It exposes a
`part=root` table role and projects authored rows/cells without reordering them.
Its default label is Table; empty content supplies one No rows cell. It creates
no focus stop, loading state, selection or announcement. Data typography,
including numeral features, inherits from the theme. Applications own row
layout, sort results, loading and result announcements.

## CEM principle audit — 2026-10-12

| Principle | Implementation and evidence |
| --- | --- |
| Semantic intent | Disclosure reveals content; sorting requests order; pagination requests a range; table stays passive. Contextual actions use D0 action states, panel/navigation surfaces use comfort pairs, and numeric ranges/table content use complete D6 data typography. |
| Bounded variation | Heading levels are 1–6; sort has three states; pagination normalizes finite integer values and clamps pages. Presence flags include empty/false spellings. No component color/density/focus options or local public token defaults. |
| Accessibility by construction | Native buttons/select own input. Persistent disclosure IDs give reciprocal references. Columnheaders own aria-sort. Native disabled controls leave tab order; paginator boundaries suppress activation while retaining existing focus. The range is a polite atomic status. |
| Cross-dimension harmony | D1 spacing, D2 target floors, D2c control/indicator geometry, D3 bends, D5 external focus outlines and D6 typography keep transient paint from shifting layout. Exact inherited overrides are exercised across all five modes. |
| Theme survivability | Embedded scoped rules retain normal and forced-color contracts. System colors preserve labels, indicators, boundaries and focus; character indicators do not rely on color alone. |
| Public token ownership | All public variables remain theme-owned. Source declarations consume inherited overrides without aliases or copied defaults. No new token family or CSS exception is needed. |

The fixture suite preserves all 15 legacy cases and adds a passive-table play,
four five-mode inheritance audits and shared renamed-host worker/fallback
lifecycle cases. Browser checks do not establish spoken assistive-technology or
physical-device acceptance; those tasks remain explicit in the main TODO.

The audit corrected a legacy D2 mismatch: the page-size select used the 40px
D2c visual height without the 48px D2 operable floor. Select and icon actions now
clamp both dimensions to the inherited zone minimum; paginator control gaps
clamp to the inherited guard minimum. Disclosure and sort headers explicitly
retain the inline floor as well. The inheritance audit intentionally reduces
visual size/gap tokens below the safety floors and verifies that D2 wins.

Bundle verification also reproduced a shared processing-host cache defect:
stylesheet begin did not reacquire an artifact evicted from the bounded worker
cache. It now performs the same compile preflight as rendering. Delivery and
release retain their generation checks; cache capacity is unchanged. A one-entry
worker regression fails before the fix, passes after it, and the combined
component bundle passes in source and packed consumers.

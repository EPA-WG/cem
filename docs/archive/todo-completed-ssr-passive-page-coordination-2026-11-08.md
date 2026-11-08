# Native SSR passive page coordination — 2026-11-08

This increment adds `prepare_inline_page`, a native first-connection worklist for
multiple passive HTML participants in one direct document-body registry. It
compiles each unique inline declaration once, then settles hosts in parent-first
preorder. Generated descendants, projected source descendants and siblings use
the existing shared island capture, render plan and atomic host transaction.
Inert template contents and canonical islands remain outside active discovery.

Native applications continue from the exact current retained page tree. Original
HTML event handles remap through each application without reparsing output.
An existing DOM projection is refused instead of silently rebuilding its original
HTML source; a native refusal fixture reproduced and closed that admission gap.
Generated nodes retain producer source maps and URI, receive process-local runtime
keys and never fabricate authored line/event coordinates. Source copies retain
their native provenance. Projection namespace declarations remain structural
metadata rather than canonical host aliases; authored reserved writes still
refuse publication.

Each participant receives a deterministic distinct instance ID. One original
operation retains source/arena permits and shares instance, depth and render
ceilings. Active elapsed admission observes the original clock with the stricter
wall/CPU limit as a conservative bound, without reporting it as measured CPU.
Unknown/customized participants, scoped or duplicate declarations, forged resume
markers, stale source/control, exhaustion and cancellation refuse the staged
candidate. No intermediate page is exposed. Native source, canonical island and
output-boundary ownership remap after every pass. Native memory remains charged
until the final retained tree owner releases it.

Fixtures preceded implementation. Native cases cover ordering, identities,
ownership, exact continuation, inert/dropped payloads, recursive discovery,
budgets, original-operation cancellation, terminal cleanup and final-tree memory
lifetime. Three shared observation vectors compare complete public host/rendered
subtrees and literal canonical payloads with the browser runtime. Runtime trees
never pass through JSON; the named JSON files are verification exports only.

The active nested lifecycle parent task remains open. This profile deliberately
refuses cross-participant declaration/instance CSS and CEM-ML source payloads.
Those require shared declaration leases and resource tickets before admission can
widen. Quiescence remains `Partial(Incomplete)` with hydration disabled. Final
version, island, identity/policy and owned-range proof precedes CLI/export or
browser adoption. Existing feature selection and named JS/WASM ABI stay fixed.

Verification results and exact source/artifact hashes are recorded in the
[verification receipt](ssr-passive-page-coordination-2026-11-08.json).

- [x] Expanded source-admission gate: **260 native checks** across default and
      actual stripped profiles, including **11/11** page-coordinator cases,
      **11/11** tree-application cases and **1/1** element-continuation case per
      profile. Existing page-artifact regressions pass **14/14 per profile**.
- [x] Standalone stripped SSR suite **97/97**; component receiver **17/17**;
      shared receiver units **19/19 per profile**.
- [x] Browser slot profiles **4/4**, host/island publication profiles **4/4**,
      existing full native/browser observations **19/19** and passive page
      observations **3/3**. No divergences or external requests. Public rendered
      comparisons exclude runtime attributes throughout inert contents; canonical
      payload comparisons retain authored attributes. Namespace declarations are
      structural metadata in both observation adapters.
- [x] Paired runtime packaging, Clippy with existing warnings, focused Rustfmt
      and optimized WASM template/CSS consumers pass.
- [x] Optimized core: **23,990,533 bytes**, SHA-256
      `d194026c80c5f6610019302e34fbb01672d4949f6b7260aa4a0ac2746c461b09`,
      below the 26,214,400-byte ceiling, with **28 imports/175 exports**.
      Packaged core matches; no named ABI or default feature change.
- [x] Frozen presentation SHA-256 remains
      `bbcb37da0a36491e5c021a5cf0c67304cc41f001a98247fc5c101ed6c45a9864`.

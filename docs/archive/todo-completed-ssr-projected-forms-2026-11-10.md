# Projected form controls for SSR browser adoption

Base commit `35adb3adc96f45c77d06337b3a8e490393741cc7`.

- [x] Compare HTML payload controls with the parent’s fresh render and follow
  their native capture identities when projecting them into the receiving range.
- [x] Check the slice against the author’s declaration and bind the receiving
  form separately. Verify live form associations and processing baselines before
  committing any participant in the group.
- [x] Retain edited controls and route input through main-thread and worker
  patching. Refuse altered payloads, external form members and stale form values
  while keeping the published ranges.

The native fixture checks the form and author identities separately, including
refusal when a control would cross into an enclosing participant’s form. The
108-case Chromium suite includes six projected-form cases covering a standalone
HTML payload form and a nested receiver with a same-named local slice. The
profile admits local native controls from compatible inline declarations.

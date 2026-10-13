# Opening idea: DNA for structured data

Sasha added a teaser before the original title slide. Use the thought → DNA →
CEM sequence as an opening metaphor: a pattern can produce a present form while
allowing later forms to change. This introduces Evolutionary Architecture before
the feature discussion.

## Slide copy

**CEM — DNA for structured data**

A template for today's structure and tomorrow's evolution.

| Thought | DNA | CEM |
| --- | --- | --- |
| One idea gives rise to others. | A pattern carried through change. | A template that can evolve. |

## Talking points · 45 seconds

“In the beginning was a thought. It felt lonely, so it changed—and gave rise to
other thoughts.” Pause before moving to the biological analogy.

“In the biological world, DNA offers a vivid parallel: a pattern that life can
copy and express, while variation makes evolution possible.”

“CEM is an attempt to create DNA for structured data: a declarative template for
a system's current state and a structure its future versions can build upon.”

“That is the ambition behind evolutionary architecture. Let's see what it means
for one document.”

Tell this as a metaphor, not a scientific claim that thought caused DNA. The
lonely thought is the poetic opening; DNA supplies the analogy of inheritance
and variation; CEM supplies the platform ambition.

## Design and timing

The first slide uses native editable branching, helix and template diagrams.
The [prepared animation](opening-animation.md) reveals these groups with
2-second fades starting at 0, 12 and 24 seconds after one starting click.
Its native timing survived import; copying it into the main deck and checking
Slideshow playback remain open.
Its colors come from the [CEM palette](../../packages/cem-theme/src/lib/tokens/cem-colors.md):
comfort `#F1FEFE`, comfort-text `#001010`, creativity-x `#6A1B9A`, creativity
`#E1BEE7`, trust `#D7E3FF`, trust-x `#002F65`, calm-x `#006A6A`.

The teaser takes 00:00–00:45. The original title slide, now physical slide 2,
stays dark and takes 00:45–01:10. The visible login result takes 01:10–03:00;
the opening seven-minute block stays intact. The current deck has 30 physical
slides: 26 timed slides and four optional references at positions 25 and 27–29.
Sasha's “Browser vs DCE security” diagram takes 50:00–50:30 within the platform block;
questions remain 54:00–60:00 on slide 30. The [slide story](slide-story.md)
follows the native order and owns the per-slide notes.

The deck-wide CEM light palette and dark terminal samples are recorded in
[docs/todo.md](../todo.md). This update applies light mode to the teaser only;
the bulk restyle remains a separate presentation action.

The native deck includes Sasha's changes and is authoritative. Rebuilding the
original PowerPoint would overwrite that work. [update-opening-slide.cjs](update-opening-slide.cjs)
builds a bounded native update from the saved live readback instead.

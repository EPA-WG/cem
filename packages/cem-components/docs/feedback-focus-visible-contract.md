# Feedback focus-visible contract

`cem-dialog` and `cem-dialog-shell` use one native dialog, with explicit modal
mode. A focused native fallback owner receives an external outline using
`--cem-stroke-focus`, `--cem-stroke-indicator-offset` and `--cem-zebra-color-1`.
Generated launcher and close controls have their own focus outlines. Eligible
authored descendants retain their own focus styling; the host never receives
a descendant-wide outline or a synthetic tab stop.

Forced colors preserve outline width/offset and use `Highlight`; surfaces use
`Canvas`/`CanvasText` with automatic forced-color adjustment. The colocated
`KeyboardFallbackFocus` stories use trusted browser keyboard input. The
`verify-surfaces` gate verifies actual source and installed declarations under
forced colors. The former feedback-focus target delegates to that gate.

`cem-tooltip` keeps focus on the authored native trigger. Tooltip description
and presentation add no tab stop. A canonical `cem-sheet` remains a focus-neutral
region; only its authored controls receive keyboard focus and focus paint.
See [dialog](./dialog-contract.md), [tooltip](./tooltip-contract.md) and
[sheet visibility](./feedback-expanded-contract.md) contracts.

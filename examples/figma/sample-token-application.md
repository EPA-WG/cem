# Figma Token Application Fixture

Use this as the manual validation fixture after importing the generated `cem-*.tokens.json` files into one CEM
collection.

## Frame

- Name: `CEM token smoke`
- Background fill: `cem/palette/comfort`
- Width: 640
- Height: 360

## Button

- Name: `Primary action`
- Fill: `cem/action/primary/default/background`
- Text fill: `cem/action/primary/default/text`
- Corner radius: `cem/bend/control`
- Horizontal padding: `cem/inset/control`
- Height: `cem/control/height`
- Label font family: `cem/typography/ui/font/family`
- Label font size: `cem/typography/ui/font/size`

## Card

- Name: `Comfort surface`
- Fill: `cem/palette/comfort`
- Text fill: `cem/palette/comfort/text`
- Corner radius: `cem/bend/surface`
- Padding: `cem/inset/surface`
- Gap: `cem/gap/group`
- Border color: `cem/zebra/color/0`

## Mode Check

Switch the collection mode through `light`, `dark`, `contrast-light`, `contrast-dark`, and `native`.
The button and card should keep the same variable names while values change with the active mode.

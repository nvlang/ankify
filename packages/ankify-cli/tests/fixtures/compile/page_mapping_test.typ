#import "../../../../ankify-typst/lib.typ": note, configure

#configure(
  defaults: (
    deck: "Page-Mapping-Test",
    model: "Basic",
    format: "svg",
    tags: ("test", "mapping"),
  ),
)

#note(
  "card-one",
  data: (
    Front: [
      #rect(width: 100pt, height: 50pt, fill: rgb("#ff0000"), stroke: none)
      Card 1 Front - Red Rectangle
    ],
    Back: [
      #rect(width: 100pt, height: 50pt, fill: rgb("#00ff00"), stroke: none)
      Card 1 Back - Green Rectangle
    ],
  ),
)

#note(
  "card-two",
  data: (
    Front: [
      #rect(width: 100pt, height: 50pt, fill: rgb("#0000ff"), stroke: none)
      Card 2 Front - Blue Rectangle
    ],
    Back: [
      #rect(width: 100pt, height: 50pt, fill: rgb("#ffff00"), stroke: none)
      Card 2 Back - Yellow Rectangle
    ],
  ),
)

#note(
  "card-three",
  data: (
    Front: [
      #rect(width: 100pt, height: 50pt, fill: rgb("#ff00ff"), stroke: none)
      Card 3 Front - Magenta Rectangle
    ],
    Back: [
      #rect(width: 100pt, height: 50pt, fill: rgb("#00ffff"), stroke: none)
      Card 3 Back - Cyan Rectangle
    ],
  ),
)

= Page Mapping Test Document

This document contains three cards with distinctive colored rectangles:
- Card 1: Front (Red), Back (Green)
- Card 2: Front (Blue), Back (Yellow)
- Card 3: Front (Magenta), Back (Cyan)

Each field contains a colored rectangle with a unique hex color to verify
that the output pages are correctly associated with the right cards and fields.

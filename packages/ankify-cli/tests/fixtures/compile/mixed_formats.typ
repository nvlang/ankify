#import "../../../../ankify-typst/lib.typ": note, configure

#configure(
  defaults: (
    deck: "Mixed-Test",
    model: "Basic",
    tags: ("mixed", "test"),
  ),
)

#note(
  "plain-note",
  format: "plain",
  data: (
    Front: "What is the capital of France?",
    Back: "Paris",
  ),
)

#note(
  "svg-note",
  format: "svg",
  data: (
    Front: [What is $e^{i\pi}$?],
    Back: [Euler's identity: $e^{i\pi} + 1 = 0$],
  ),
)

#note(
  "png-note",
  format: "png",
  data: (
    Front: [Complex formula with colors],
    Back: [
      The quadratic formula:
      #text(fill: red)[$x = (-b plus.minus sqrt(b^2 - 4 a c)) / (2 a)$]
    ],
  ),
)

= Document with Mixed Formats

This document contains notes with different format types:
- Plain text notes (no compilation needed)
- SVG vector graphics
- PNG raster graphics

This tests the compile module's ability to handle multiple formats in a single document.

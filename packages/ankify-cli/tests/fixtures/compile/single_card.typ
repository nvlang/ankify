#import "../../../../ankify-typst/lib.typ": note, configure

#configure(
  defaults: (
    deck: "Single-Card-Test",
    model: "Basic",
    format: "svg",
    tags: ("test",),
  ),
)

#note(
  "single-card",
  data: (
    Front: [
      #rect(width: 50pt, height: 25pt, fill: rgb("#abcdef"), stroke: none)
      Single Card Front
    ],
    Back: [
      #rect(width: 50pt, height: 25pt, fill: rgb("#fedcba"), stroke: none)
      Single Card Back
    ],
  ),
)

= Single Card Test

This tests the page mapping with just one card.

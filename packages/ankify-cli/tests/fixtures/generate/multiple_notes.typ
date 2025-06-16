#import "../../../../ankify-typst/lib.typ": note, configure

#configure(
  defaults: (
    deck: "Multiple-Test",
    model: "Basic",
    tags: ("multiple", "test"),
  ),
)

#note(
  "note-one",
  format: "plain",
  data: (
    Front: "First question",
    Back: "First answer",
  ),
)

#note(
  "note-two",
  format: "svg",
  data: (
    Front: [What is $e^{i\pi}$?],
    Back: [Euler's identity: $e^{i\pi} + 1 = 0$],
  ),
)

#note(
  "note-three",
  model: "Cloze",
  format: "png",
  data: (
    Text: [The capital of France is {{c1::Paris}}],
  ),
)

= Document with Multiple Notes

This document contains multiple notes to test batch processing.

== Mathematics
Some mathematical content here.

== Geography
Some geographical content here.

== History
Some historical content here.

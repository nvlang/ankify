#import "../../../../ankify-typst/lib.typ": note, configure

#configure(
  defaults: (
    deck: "Test-Deck",
    tags: ("test",),
  ),
)

#note(
  "simple-note",
  model: "Basic",
  format: "plain",
  data: (
    Front: "What is 2 + 2?",
    Back: "4",
  ),
)

= Simple Test Document

This is a simple document for testing the generate module.

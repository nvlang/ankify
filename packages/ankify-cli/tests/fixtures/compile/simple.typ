#import "../../../../ankify-typst/lib.typ": note, configure

#configure(
  defaults: (
    deck: "Test-Deck",
    model: "Basic",
    format: "png",
    tags: ("test",),
  ),
)

#note(
  "simple-note",
  data: (
    Front: "What is 2 + 2?",
    Back: "4",
  ),
)

= Simple Test Document

This is a simple document for testing the compile module.

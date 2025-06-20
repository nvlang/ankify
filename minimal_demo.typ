#import "packages/ankify-typst/lib.typ": note, configure

#configure(
  ankiconnect-url: "http://localhost:8765",
  verbose: true,
  defaults: (
    model: "Basic",
    deck: "Ankify Minimal Demo",
    tags: ("ankify", "minimal2"),
    format: "png"
  )
)

= Minimal Ankify Demo

This is a minimal demonstration of Ankify functionality using the real ankify-typst library.

#note(
  "minimal-demo-unique-test-1",
  data: (
    Front: "What is the square root of 169?",
    Back: "14"
  ),
  tags: ("math", "test")
)

#note(
  "minimal-demo-unique-test-2",
  data: (
    Front: "What package manager does Rust use by default?",
    Back: "Cargo"
  )
)

#note(
  "minimal-demo-unique-test-3",
  data: (
    Front: "What file extension does Typst use for its documents?",
    Back: ".typ"
  )
)

#note(
  "minimal-demo-unique-test-4",
  data: (
    Front: "What file extension does Typst use for its documents? 2",
    Back: [$x^2 + y^2 = r^2$]
  )
)

This demo creates 4 simple flashcards to test the basic Ankify sync functionality.

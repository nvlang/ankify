#import "../../../../ankify-typst/lib.typ": note, configure

// Advanced configuration with all options including defaults
#configure(
  ankiconnect-url: "http://advanced:9000",
  verbose: true,
  cache: (
    enabled: true,
    custom-file: "advanced.json",
  ),
  checks: (
    typst: true,
    ankiconnect: (
      model: true,
      deck: true,
      tags: true,
    ),
  ),
  defaults: (
    model: "Cloze",
    deck: "Default-Deck",
    format: "svg",
    tags: ("default-tag",),
  ),
)

// Advanced math note
#note(
  "advanced-math",
  deck: "Mathematics",
  model: "Basic",
  format: "svg",
  tags: ("calculus", "derivatives"),
  data: (
    Front: [What is the derivative of $sin(x)$?],
    Back: [
      The derivative of $sin(x)$ is $cos(x)$.

      #block[
        This follows from the fundamental trigonometric limit:
        $lim_(h -> 0) (sin(h)) / h = 1$
      ]
    ],
    Extra: "This is a fundamental result in calculus",
  ),
)

// Programming concept note
#note(
  "programming-concept",
  deck: "Computer Science",
  model: "Basic",
  format: "plain",
  tags: ("programming", "rust"),
  data: (
    Front: "What is ownership in Rust?",
    Back: [
      Ownership is Rust's unique approach to memory management.

      Key rules:
      - Each value has a single owner
      - When the owner goes out of scope, the value is dropped
      - Ownership can be moved or borrowed
    ],
    Extra: "This prevents memory leaks and data races at compile time",
  ),
)

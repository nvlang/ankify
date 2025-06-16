#import "../../../../ankify-typst/lib.typ": note, configure

#configure(
  setup: () => {
    set page(
      margin: 0.5cm,
      width: 12cm,
      height: auto,
    )
    set text(size: 11pt)
  },
  defaults: (
    deck: "Setup-Test",
    model: "Basic",
    format: "svg",
  ),
)

#note(
  "setup-note",
  data: (
    Front: [What is the integral of $x^2$?],
    Back: [The integral is $x^3 / 3 + C$],
  ),
)

= Document with Custom Setup

This document tests custom setup functionality with page and text settings.

The setup function configures:
- Custom page margins
- Specific page width
- Custom text size

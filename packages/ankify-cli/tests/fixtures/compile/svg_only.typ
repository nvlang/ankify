#import "../../../../ankify-typst/lib.typ": note, configure

#configure(
  defaults: (
    deck: "SVG-Test",
    model: "Basic",
    format: "svg",
    tags: ("svg", "vector"),
  ),
)

#note(
  "math-equation",
  data: (
    Front: [What is the quadratic formula?],
    Back: [
      For $a x^2 + b x + c = 0$:
      $ x = frac(-b plus.minus sqrt(b^2 - 4 a c), 2 a) $
    ],
  ),
)

#note(
  "geometry-theorem",
  data: (
    Front: [State the Pythagorean theorem],
    Back: [
      In a right triangle:
      $ a^2 + b^2 = c^2 $

      #align(center)[
        #rect(width: 3cm, height: 2cm, stroke: 1pt)
        Triangle with sides $a$, $b$, and hypotenuse $c$
      ]
    ],
  ),
)

= SVG Only Document

This document contains only SVG format notes for testing vector graphics compilation.
All mathematical content should be rendered as scalable vector graphics.

Mathematical expressions and geometric diagrams work well in SVG format
because they remain crisp at any zoom level.

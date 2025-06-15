#import "../../../../ankify-typst/lib.typ": note, configure

// Test file for various data types in note fields
#configure(
  ankiconnect-url: "http://localhost:8765",
  verbose: false,
  render: "ankify-render",
)

// Note with simple string data types
#note(
  label: "simple-strings",
  deck: "Test-Deck",
  model: "Basic",
  format: "plain",
  tags: ("simple", "strings"),
  data: (
    Front: "What is 2 + 2?",
    Back: "4",
    Extra: "This is basic arithmetic",
  ),
)

// Note with complex Typst content and mixed data types
#note(
  label: "complex-content",
  deck: "Test-Deck",
  model: "Basic",
  format: "svg",
  tags: ("complex", "math"),
  data: (
    Front: [What is the derivative of $x^2$?],
    Back: [
      The derivative is $2x$.

      #block[
        Proof: Using the power rule, $(x^n)' = n x^(n-1)$
      ]

      #table(
        columns: 2,
        [Function], [Derivative],
        [$x^2$], [$2x$],
        [$x^3$], [$3x^2$],
      )
    ],
    Extra: [
      This follows from the power rule for differentiation.

      $lim_(h -> 0) (f(x+h) - f(x))/h$
    ],
  ),
)

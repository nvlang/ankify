#import "ankify.typ": card

= Mathematics Study Notes

This document contains basic mathematics flashcards for Anki.

#card(
  "pythagoras-theorem",
  model: "Basic",
  deck: "Mathematics",
  tags: ("geometry", "theorems", "triangles"),
  data: (
    Front: [What is the Pythagorean theorem?],
    Back: [For a right triangle with legs $a$ and $b$ and hypotenuse $c$: $ a^2 + b^2 = c^2 $]
  )
)

#card(
  "quadratic-formula",
  model: "Basic", 
  deck: "Mathematics",
  tags: ("algebra", "formulas", "equations"),
  data: (
    Front: [What is the quadratic formula?],
    Back: [For $a x^2 + b x + c = 0$: $ x = frac(-b plus.minus sqrt(b^2 - 4 a c), 2 a) $]
  )
)

#card(
  "derivative-power-rule",
  model: "Cloze",
  deck: "Mathematics",
  tags: ("calculus", "derivatives"),
  data: (
    Text: [The derivative of $x^n$ is {{c1::$n x^(n-1)$}}.]
  )
)

#card(
  "integral-power-rule",
  model: "Basic",
  deck: "Mathematics", 
  tags: ("calculus", "integrals"),
  format: "svg",
  data: (
    Front: [What is the integral of $x^n$ where $n ≠ -1$?],
    Back: [$ integral x^n d x = frac(x^(n+1), n+1) + C $]
  )
)

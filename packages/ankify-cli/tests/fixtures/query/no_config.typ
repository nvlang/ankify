#import "../../../../ankify-typst/lib.typ": note

// This file has notes but no configuration
// Used to test that notes can be queried even without explicit configuration

#note(
  label: "basic-addition",
  deck: "Math-Basics",
  model: "Basic",
  format: "plain",
  tags: ("arithmetic", "basic"),
  data: (
    Front: "What is 5 + 3?",
    Back: "8",
  ),
)

#note(
  label: "simple-geometry",
  deck: "Math-Basics",
  model: "Basic",
  format: "svg",
  tags: ("geometry", "area"),
  data: (
    Front: [What is the area of a rectangle with width $w$ and height $h$?],
    Back: [The area is $w times h$],
    Extra: "This is the basic formula for rectangular area",
  ),
)

// Regular Typst content mixed in
= Document with Notes Only

This document contains ankify notes but no explicit configuration.
It tests that the system can handle notes without requiring a configure() call.

== Some Mathematics

The notes above cover basic arithmetic and geometry concepts.

$sum_(i=1)^n i^2 = (n(n+1)(2n+1))/6$

#block[
  This document demonstrates notes without configuration.
]

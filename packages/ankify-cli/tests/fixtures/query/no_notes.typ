#import "../../../../ankify-typst/lib.typ": note, configure

// Configuration with all options but no notes
#configure(
  ankiconnect-url: "http://localhost:8765",
  verbose: false,
  render: "ankify-render",
  cache: (
    enabled: true,
    custom-file: none,
  ),
  checks: (
    typst: (
      data: true,
      format: true,
    ),
    ankiconnect: (
      model: true,
      deck: true,
      tags: true,
    ),
  ),
)

// This file has configuration but no notes
// Used to test that query_ankify_notes returns an empty array

= Document with Configuration Only

This document has ankify configuration but contains no notes.
It's used to test that the query functions can handle files with
configuration but no note content.

== Some Regular Content

Here's some regular Typst content that doesn't involve ankify notes:

$sum_(i=1)^n i = (n(n+1))/2$

#block[
  This is just regular Typst content.
]
